#!/usr/bin/env python3
"""Selection-only benchmark: how much of the suite would blast pick, and how fast is it?

Doesn't run any tests, so it works on repos like Django whose suite doesn't run under
pytest or takes too long to replay. For safety numbers use replay.py.

  python3 bench/static.py --repo ../django -n 100 -- --python-files tests.py --python-files 'test_*.py'
"""

from __future__ import annotations

import argparse
import json
import statistics
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from replay import checkout, first_parent_commits, sh  # noqa: E402


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--repo", required=True, type=Path)
    p.add_argument("--blast", default=str(Path(__file__).resolve().parent.parent / "target/release/blast"))
    p.add_argument("-n", "--commits", type=int, default=100)
    p.add_argument("--name")
    p.add_argument("--out", type=Path, default=Path(__file__).resolve().parent / "results")
    p.add_argument("blast_args", nargs="*")
    args = p.parse_args()

    repo = args.repo.resolve()
    name = args.name or repo.name
    commits = first_parent_commits(repo, args.commits, "HEAD")
    wt = Path(sh("mktemp", "-d").strip()) / "wt"
    sh("git", "worktree", "add", "-q", "--detach", str(wt), commits[0], cwd=repo)

    rows = []
    try:
        for sha in commits:
            checkout(wt, sha)
            parent = sh("git", "rev-parse", f"{sha}~1", cwd=wt).strip()
            t = time.time()
            try:
                out = sh(args.blast, "--diff", f"{parent}..{sha}", "--json", *args.blast_args, str(wt))
            except subprocess.CalledProcessError as e:
                print(f"{sha[:10]} failed: {e.stderr[:200]}", file=sys.stderr)
                continue
            secs = time.time() - t
            sel = json.loads(out)
            picked = sel["total_tests"] if sel["mode"] == "full" else len(sel["selected_tests"])
            rows.append(
                {
                    "sha": sha[:10],
                    "subject": sh("git", "log", "-1", "--format=%s", sha, cwd=wt).strip()[:72],
                    "mode": sel["mode"],
                    "selected": picked,
                    "total": sel["total_tests"],
                    "seconds": round(secs, 2),
                    "reason": (sel["reasons"] or [""])[0],
                }
            )
            print(f"{sha[:10]} {sel['mode']:8} {picked:4}/{sel['total_tests']} {secs:.1f}s", file=sys.stderr)
    finally:
        sh("git", "worktree", "remove", "--force", str(wt), cwd=repo, check=False)

    fracs = [r["selected"] / r["total"] for r in rows if r["total"]]
    fulls = sum(r["mode"] == "full" for r in rows)
    lines = [
        f"# {name} (selection only)",
        "",
        f"{len(rows)} commits. Median {100 * statistics.median(fracs):.0f}% of test files selected, "
        f"mean {100 * statistics.mean(fracs):.0f}%. Full suite on {fulls}/{len(rows)} commits. "
        f"Median analysis time {statistics.median(r['seconds'] for r in rows):.2f}s.",
        "",
        "| test files selected | commits |",
        "|---|---|",
    ]
    buckets = [(0, 0, "none"), (0, 0.1, "under 10%"), (0.1, 0.5, "10–50%"), (0.5, 0.999, "50–99%"), (0.999, 1, "all")]
    for lo, hi, label in buckets:
        if label == "none":
            n = sum(f == 0 for f in fracs)
        elif label == "all":
            n = sum(f >= 0.999 for f in fracs)
        else:
            n = sum(lo < f <= hi if lo else 0 < f <= hi for f in fracs)
        lines.append(f"| {label} | {n} |")
    lines += ["", "| commit | subject | mode | selected | reason |", "|---|---|---|---|---|"]
    for r in rows:
        subject = r["subject"].replace("|", "\\|")
        reason = r["reason"].replace("|", "\\|")
        lines.append(f"| `{r['sha']}` | {subject} | {r['mode']} | {r['selected']}/{r['total']} | {reason} |")

    args.out.mkdir(parents=True, exist_ok=True)
    (args.out / f"{name}-static.json").write_text(json.dumps(rows, indent=2) + "\n")
    report = "\n".join(lines) + "\n"
    (args.out / f"{name}-static.md").write_text(report)
    print(report)
    return 0


if __name__ == "__main__":
    sys.exit(main())
