#!/usr/bin/env python3
"""Replay a repo's history and check how blast's test selection holds up.

For each commit C (first-parent, so a merged PR counts as one change):

  1. run `blast --diff C~1..C` (strict and --shallow-init) and note the selected test files
  2. run the full suite at C to get per-test timings and anything already failing
  3. plant a bug in every function C touched (a `raise` at the top of the body),
     run the full suite again, and call every newly failing test a regression
  4. a regression in a test file blast didn't select is a miss
  5. optionally do the same with pytest-testmon, giving it its best case: a coverage
     database built from a full run at C~1

Usage:
  python3 bench/replay.py --repo ../flask --python ../venv/bin/python -n 30 --testmon

Stdlib only. Needs the repo's test deps (and pytest-testmon for --testmon) installed
in the interpreter passed with --python.
"""

from __future__ import annotations

import argparse
import ast
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
import xml.etree.ElementTree as ET
from dataclasses import asdict, dataclass, field
from pathlib import Path

MUTANT = 'raise RuntimeError("blast-mutant")'


def sh(*args, cwd=None, check=True, env=None) -> str:
    out = subprocess.run(
        args, cwd=cwd, env=env, check=check, capture_output=True, text=True
    )
    return out.stdout


def is_test_file(path: str) -> bool:
    name = os.path.basename(path)
    return name.endswith(".py") and (name.startswith("test_") or name.endswith("_test.py"))


# --- git ---------------------------------------------------------------------


def first_parent_commits(repo: Path, n: int, rev: str) -> list[str]:
    """Last n first-parent commits that touch a .py file."""
    out = []
    for sha in sh("git", "rev-list", "--first-parent", rev, cwd=repo).split():
        if len(out) >= n:
            break
        parents = sh("git", "rev-list", "--parents", "-n", "1", sha, cwd=repo).split()[1:]
        if not parents:
            continue
        files = changed_files(repo, parents[0], sha)
        if any(f.endswith(".py") for f in files):
            out.append(sha)
    return out


def changed_files(repo: Path, a: str, b: str) -> list[str]:
    return sh("git", "diff", "--name-only", a, b, cwd=repo).split()


def checkout(wt: Path, sha: str, keep: tuple[str, ...] = ()) -> None:
    sh("git", "checkout", "-q", "--force", "--detach", sha, cwd=wt)
    excludes = [arg for k in keep for arg in ("-e", k)]
    sh("git", "clean", "-q", "-fdx", *excludes, cwd=wt)


def touched_lines(wt: Path, parent: str, sha: str, path: str) -> set[int]:
    """Line numbers in the new version of `path` that the commit added or changed."""
    diff = sh("git", "diff", "-U0", parent, sha, "--", path, cwd=wt)
    lines = set()
    for m in re.finditer(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@", diff, re.M):
        start, count = int(m.group(1)), int(m.group(2) or 1)
        # a pure deletion (count 0) still "touches" the spot it happened
        lines.update(range(start, start + max(count, 1)))
    return lines


# --- mutation ----------------------------------------------------------------


def mutate_file(file: Path, lines: set[int]) -> list[str]:
    """Put a raise at the top of the innermost function around each touched line.
    Returns the qualified names of mutated functions."""
    src = file.read_text()
    try:
        tree = ast.parse(src)
    except SyntaxError:
        return []

    funcs: list[tuple[ast.AST, str]] = []

    def visit(node, prefix=""):
        for child in ast.iter_child_nodes(node):
            if isinstance(child, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                name = f"{prefix}{child.name}"
                if not isinstance(child, ast.ClassDef):
                    funcs.append((child, name))
                visit(child, name + ".")
            else:
                visit(child, prefix)

    visit(tree)

    def contains(f, line):
        return f.lineno <= line <= f.end_lineno

    targets = {}
    for line in lines:
        hits = [(f, n) for f, n in funcs if contains(f, line)]
        if not hits:
            continue
        # innermost = the one that starts last
        f, n = max(hits, key=lambda h: h[0].lineno)
        targets[id(f)] = (f, n)

    src_lines = src.splitlines(keepends=True)
    inserts = []
    for f, name in targets.values():
        first = f.body[0]
        first_line = min([first.lineno] + [d.lineno for d in getattr(first, "decorator_list", [])])
        if first_line <= f.lineno:
            continue  # one-liner `def f(): ...`
        # skip the def's own multi-line signature: first body line must come after it
        indent = " " * first.col_offset
        inserts.append((first_line, f"{indent}{MUTANT}\n", name))

    done = []
    for line, text, name in sorted(inserts, reverse=True):
        src_lines.insert(line - 1, text)
        done.append(name)

    new_src = "".join(src_lines)
    try:
        compile(new_src, str(file), "exec")
    except SyntaxError:
        return []
    file.write_text(new_src)
    return sorted(done)


def mutate_commit(wt: Path, parent: str, sha: str) -> dict[str, list[str]]:
    mutated = {}
    for path in changed_files(wt, parent, sha):
        if not path.endswith(".py") or is_test_file(path) or os.path.basename(path) == "conftest.py":
            continue
        file = wt / path
        if not file.exists():
            continue
        names = mutate_file(file, touched_lines(wt, parent, sha, path))
        if names:
            mutated[path] = names
    return mutated


# --- pytest ------------------------------------------------------------------


@dataclass
class Run:
    # nodeid -> (file, seconds, failed)
    tests: dict[str, tuple[str, float, bool]] = field(default_factory=dict)
    wall: float = 0.0
    returncode: int = 0

    def failed(self) -> set[str]:
        return {t for t, (_, _, bad) in self.tests.items() if bad}

    def files(self) -> dict[str, float]:
        out: dict[str, float] = {}
        for f, secs, _ in self.tests.values():
            out[f] = out.get(f, 0.0) + secs
        return out


def run_pytest(wt: Path, python: str, extra: list[str], timeout: int) -> Run:
    with tempfile.TemporaryDirectory() as tmp:
        xml = Path(tmp) / "junit.xml"
        env = dict(os.environ)
        # make sure the worktree's code wins over any editable install
        src = wt / "src"
        env["PYTHONPATH"] = str(src if src.is_dir() else wt)
        env.pop("PYTEST_ADDOPTS", None)
        cmd = [
            python, "-m", "pytest", "-q",
            "-o", "junit_family=xunit1", f"--junitxml={xml}", *extra,
        ]
        start = time.time()
        code = -1
        try:
            code = subprocess.run(cmd, cwd=wt, env=env, capture_output=True, text=True, timeout=timeout).returncode
        except subprocess.TimeoutExpired:
            pass
        wall = time.time() - start
        run = Run(wall=wall, returncode=code)
        if not xml.exists():
            return run
        for case in ET.parse(xml).getroot().iter("testcase"):
            file = case.get("file")
            classname = case.get("classname") or ""
            name = case.get("name") or ""
            if not file:
                # collection errors: name is the module path
                mod = classname or name
                file = mod.replace(".", "/") + ".py"
            nodeid = f"{file}::{classname}::{name}"
            bad = case.find("failure") is not None or case.find("error") is not None
            run.tests[nodeid] = (file, float(case.get("time") or 0), bad)
        return run


def pytest_id(nodeid: str) -> str:
    """Our `file::classname::name` back to something pytest accepts."""
    file, classname, name = nodeid.split("::", 2)
    module = file[:-3].replace("/", ".")
    parts = [file]
    if classname.startswith(module + "."):
        parts += classname[len(module) + 1 :].split(".")
    return "::".join(parts + [name])


def fails_alone(wt: Path, python: str, nodeids: list[str], extra: list[str], timeout: int) -> set[str]:
    """Of these tests, which still fail when run on their own (bug still planted)?
    Anything that passes alone only failed because an earlier test left state behind."""
    still = set()
    for t in nodeids:
        run = run_pytest(wt, python, [*extra, pytest_id(t)], timeout)
        if run.failed() or run.returncode not in (0, 5):
            still.add(t)
    return still


# --- blast -------------------------------------------------------------------


def blast_select(blast: str, wt: Path, parent: str, sha: str, flags: list[str]) -> dict:
    out = sh(blast, "--diff", f"{parent}..{sha}", "--json", *flags, str(wt))
    return json.loads(out)


# --- main --------------------------------------------------------------------


@dataclass
class Variant:
    mode: str
    tests: int
    seconds: float
    missed: list[str]


@dataclass
class Result:
    sha: str
    subject: str
    changed: list[str]
    mutated: dict[str, list[str]]
    total_tests: int
    total_seconds: float
    regressions: int
    regression_files: list[str]
    variants: dict[str, Variant]
    notes: list[str]


def replay_commit(args, wt: Path, sha: str) -> Result:
    parent = sh("git", "rev-parse", f"{sha}~1", cwd=wt).strip()
    subject = sh("git", "log", "-1", "--format=%s", sha, cwd=wt).strip()
    notes = []

    variants: dict[str, Variant] = {}
    missed_ids: dict[str, set[str]] = {}
    checkout(wt, sha)
    selections = {
        "blast": blast_select(args.blast, wt, parent, sha, []),
        "blast --shallow-init": blast_select(args.blast, wt, parent, sha, ["--shallow-init"]),
    }

    base = run_pytest(wt, args.python, args.pytest_args, args.timeout)
    already_failing = base.failed()
    if already_failing:
        notes.append(f"{len(already_failing)} tests already failing before mutation")
    file_secs = base.files()
    total_secs = sum(file_secs.values())
    file_counts: dict[str, int] = {}
    for f, _, _ in base.tests.values():
        file_counts[f] = file_counts.get(f, 0) + 1

    mutated = mutate_commit(wt, parent, sha)
    regressions: set[str] = set()
    if mutated:
        mut = run_pytest(wt, args.python, args.pytest_args, args.timeout)
        regressions = mut.failed() - already_failing
        if not regressions:
            notes.append("mutants survived the full suite (untested code)")
    reg_files = sorted({mut.tests[t][0] for t in regressions})

    for name, sel in selections.items():
        if sel["mode"] == "full":
            chosen = set(file_counts)
        else:
            chosen = {str(p) for p in sel["selected_tests"]}
        missed = sorted(set(reg_files) - chosen)
        missed_ids[name] = {t for t in regressions if mut.tests[t][0] not in chosen}
        variants[name] = Variant(
            mode=sel["mode"],
            tests=sum(c for f, c in file_counts.items() if f in chosen),
            seconds=sum(s for f, s in file_secs.items() if f in chosen),
            missed=missed,
        )

    if args.testmon:
        # best case for testmon: a database from a full run of the parent
        tm_args = ["--testmon", *args.testmon_args, *args.pytest_args]
        checkout(wt, parent)
        warm = run_pytest(wt, args.python, tm_args, args.timeout)
        checkout(wt, sha, keep=(".testmondata", ".testmondata-shm", ".testmondata-wal"))
        mutate_commit(wt, parent, sha)
        tm = run_pytest(wt, args.python, tm_args, args.timeout)
        ran = set(tm.tests)
        # pytest exit codes 2-4 are interrupted / internal error / usage error: testmon
        # didn't run, which isn't the same as testmon choosing to run nothing
        if warm.returncode in (-1, 2, 3, 4) or tm.returncode in (-1, 2, 3, 4):
            notes.append(f"testmon crashed or timed out (exit {warm.returncode}/{tm.returncode})")
            variants["testmon (warm db)"] = Variant(mode="error", tests=0, seconds=0, missed=[])
            tm = None
    if args.testmon and tm is not None:
        tm_missed = sorted(t for t in regressions if t not in ran)
        missed_ids["testmon (warm db)"] = set(tm_missed)
        missed = sorted({mut.tests[t][0] for t in tm_missed})
        if tm_missed:
            notes.append(f"testmon skipped {len(tm_missed)} failing tests, e.g. {tm_missed[0]}")
        variants["testmon (warm db)"] = Variant(
            mode="selected",
            tests=len(ran),
            # time it with the unmutated run so it's comparable with blast
            seconds=sum(base.tests[t][1] for t in ran if t in base.tests),
            missed=missed,
        )
    if args.testmon:
        checkout(wt, sha)

    # double-check misses: a test that only fails after other tests broke shared state
    # (env vars, globals) isn't something a selector could reasonably be blamed for
    suspects = sorted(set().union(*missed_ids.values())) if missed_ids else []
    if suspects:
        mutate_commit(wt, parent, sha)
        real = fails_alone(wt, args.python, suspects[:50], args.pytest_args, args.timeout)
        real |= set(suspects[50:])  # too many to check one by one, count them as real
        checkout(wt, sha)
        flaky = [t for t in suspects if t not in real]
        if flaky:
            notes.append(f"{len(flaky)} missed tests pass on their own (order-dependent), not counted: {flaky[0]}")
        for name, v in variants.items():
            ids = missed_ids.get(name, set()) & real
            v.missed = sorted({mut.tests[t][0] for t in ids})

    return Result(
        sha=sha[:10],
        subject=subject[:72],
        changed=changed_files(wt, parent, sha),
        mutated=mutated,
        total_tests=len(base.tests),
        total_seconds=round(total_secs, 2),
        regressions=len(regressions),
        regression_files=reg_files,
        variants={k: Variant(**{**asdict(v), "seconds": round(v.seconds, 2)}) for k, v in variants.items()},
        notes=notes,
    )


def summarize(name: str, results: list[Result]) -> str:
    variants = list(results[0].variants) if results else []
    total_tests = sum(r.total_tests for r in results)
    total_secs = sum(r.total_seconds for r in results)
    with_mutants = [r for r in results if r.regressions]

    lines = [f"# {name}", ""]
    lines.append(
        f"{len(results)} commits replayed, {len(with_mutants)} with planted bugs the full suite caught."
    )
    lines.append("")
    lines.append("| | tests run | test time | full-suite runs | regressions missed |")
    lines.append("|---|---|---|---|---|")
    lines.append(f"| full suite | 100% | 100% | {len(results)}/{len(results)} | 0 |")
    for v in variants:
        ok = [r for r in results if r.variants[v].mode != "error"]
        ran = sum(r.variants[v].tests for r in ok)
        secs = sum(r.variants[v].seconds for r in ok)
        fulls = sum(1 for r in ok if r.variants[v].mode == "full")
        ok_mut = [r for r in ok if r.regressions]
        missed_commits = sum(1 for r in ok_mut if r.variants[v].missed)
        errors = len(results) - len(ok)
        label = f"{v} ({errors} runs errored, left out)" if errors else v
        lines.append(
            f"| {label} | {pct(ran, sum(r.total_tests for r in ok))} "
            f"| {pct(secs, sum(r.total_seconds for r in ok))} | {fulls}/{len(ok)} "
            f"| {missed_commits}/{len(ok_mut)} commits |"
        )
    lines.append("")
    lines.append("## Per commit")
    lines.append("")
    header = "| commit | subject | mutated fns | regressions | " + " | ".join(variants) + " |"
    lines.append(header)
    lines.append("|" + "---|" * (4 + len(variants)))
    for r in results:
        cells = []
        for v in variants:
            x = r.variants[v]
            cell = f"{pct(x.tests, r.total_tests)}"
            if x.mode == "full":
                cell = "full"
            if x.mode == "error":
                cell = "error"
            if x.missed:
                cell += f" **missed {len(x.missed)}**"
            cells.append(cell)
        nfn = sum(len(v) for v in r.mutated.values())
        subject = r.subject.replace("|", "\\|")
        lines.append(f"| `{r.sha}` | {subject} | {nfn} | {r.regressions} | " + " | ".join(cells) + " |")

    misses = [(r, v) for r in results for v in variants if r.variants[v].missed]
    if misses:
        lines += ["", "## Misses", ""]
        for r, v in misses:
            lines.append(f"- `{r.sha}` {v}: {', '.join(r.variants[v].missed)}")
    notes = [(r, n) for r in results for n in r.notes]
    if notes:
        lines += ["", "## Notes", ""]
        for r, n in notes:
            lines.append(f"- `{r.sha}`: {n}")
    return "\n".join(lines) + "\n"


def pct(a: float, b: float) -> str:
    return f"{100 * a / b:.0f}%" if b else "-"


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--repo", type=Path)
    p.add_argument("--python", help="interpreter with the repo's test deps")
    p.add_argument("--blast", default=str(Path(__file__).resolve().parent.parent / "target/release/blast"))
    p.add_argument("-n", "--commits", type=int, default=20)
    p.add_argument("--rev", default="HEAD")
    p.add_argument("--sha", action="append", help="replay just these commits (repeatable)")
    p.add_argument("--testmon", action="store_true")
    p.add_argument("--testmon-args", type=str, default="", help="extra pytest args for testmon runs only")
    p.add_argument("--from-json", type=Path, help="just re-render the report from a saved results file")
    p.add_argument("--note", help="paragraph to add under the title (with --from-json)")
    p.add_argument("--merge-into", type=Path, help="replace matching commits in an existing results json")
    p.add_argument("--drop-variant", action="append", default=[], help="leave a variant out of the report")
    p.add_argument("--timeout", type=int, default=900, help="per pytest run, seconds")
    p.add_argument("--name", help="label for the report (default: repo dir name)")
    p.add_argument("--out", type=Path, default=Path(__file__).resolve().parent / "results")
    p.add_argument("pytest_args", nargs="*", help="extra args for pytest (after --)")
    args = p.parse_args()
    args.testmon_args = args.testmon_args.split()

    if args.from_json:
        raw = json.loads(args.from_json.read_text())
        results = []
        for r in raw:
            r["variants"] = {
                k: Variant(**v) for k, v in r["variants"].items() if k not in args.drop_variant
            }
            for dropped in args.drop_variant:
                word = dropped.split()[0]
                r["notes"] = [n for n in r["notes"] if not n.startswith(word)]
            results.append(Result(**r))
        name = args.name or args.from_json.stem
        report = summarize(name, results)
        if args.note:
            report = report.replace("\n\n", f"\n\n{args.note}\n\n", 1)
        (args.out / f"{name}.md").write_text(report)
        print(report)
        return 0

    repo = args.repo.resolve()
    name = args.name or repo.name
    if args.sha:
        commits = [sh("git", "rev-parse", c, cwd=repo).strip() for c in args.sha]
    else:
        commits = first_parent_commits(repo, args.commits, args.rev)
    print(f"replaying {len(commits)} commits of {name}", file=sys.stderr)

    tmp = Path(tempfile.mkdtemp(prefix=f"blast-replay-{name}-"))
    wt = tmp / "wt"
    sh("git", "worktree", "add", "-q", "--detach", str(wt), commits[0], cwd=repo)
    results = []
    try:
        for i, sha in enumerate(commits, 1):
            t = time.time()
            try:
                r = replay_commit(args, wt, sha)
            except subprocess.CalledProcessError as e:
                print(f"  {sha[:10]} skipped: {e.stderr.strip()[:200]}", file=sys.stderr)
                continue
            results.append(r)
            summary = ", ".join(
                f"{k}={'full' if v.mode == 'full' else pct(v.tests, r.total_tests)}"
                + (f" MISSED {len(v.missed)}" if v.missed else "")
                for k, v in r.variants.items()
            )
            print(
                f"[{i}/{len(commits)}] {r.sha} regressions={r.regressions} {summary} ({time.time() - t:.0f}s)",
                file=sys.stderr,
            )
    finally:
        sh("git", "worktree", "remove", "--force", str(wt), cwd=repo, check=False)
        shutil.rmtree(tmp, ignore_errors=True)

    args.out.mkdir(parents=True, exist_ok=True)
    if args.merge_into:
        # rerun a few commits and patch them into an earlier full run
        fresh = {r.sha: r for r in results}
        merged = []
        for r in json.loads(args.merge_into.read_text()):
            if r["sha"] in fresh:
                merged.append(fresh[r["sha"]])
            else:
                r["variants"] = {k: Variant(**v) for k, v in r["variants"].items()}
                merged.append(Result(**r))
        results = merged
    (args.out / f"{name}.json").write_text(json.dumps([asdict(r) for r in results], indent=2) + "\n")
    report = summarize(name, results)
    (args.out / f"{name}.md").write_text(report)
    print(report)
    return 0


if __name__ == "__main__":
    sys.exit(main())
