#!/usr/bin/env python3
"""Roll up shadow-mode records from real repos into one table.

Every shadow run uploads a `blast-shadow-*` artifact with a small JSON record. This pulls
them down with the gh CLI and adds them up.

  python3 bench/collect.py owner/repo1 owner/repo2 --out bench/results/real-use.md
"""

from __future__ import annotations

import argparse
import io
import json
import subprocess
import sys
import zipfile
from pathlib import Path


def gh_json(*args: str):
    out = subprocess.run(["gh", "api", *args], check=True, capture_output=True)
    return json.loads(out.stdout)


def records(repo: str, limit: int) -> list[dict]:
    out = []
    page = 1
    while len(out) < limit:
        data = gh_json(f"repos/{repo}/actions/artifacts?per_page=100&page={page}")
        arts = [a for a in data.get("artifacts", []) if a["name"].startswith("blast-shadow-")]
        if not data.get("artifacts"):
            break
        for a in arts:
            if a.get("expired"):
                continue
            blob = subprocess.run(
                ["gh", "api", f"repos/{repo}/actions/artifacts/{a['id']}/zip"],
                check=True,
                capture_output=True,
            ).stdout
            with zipfile.ZipFile(io.BytesIO(blob)) as z:
                rec = json.loads(z.read("shadow.json"))
            rec["repo"] = rec.get("repo") or repo
            out.append(rec)
        page += 1
    return out[:limit]


def summarize(rows: list[dict]) -> str:
    lines = [
        "# blast in real use (shadow mode)",
        "",
        "Shadow mode runs the full suite and records what blast would have skipped, so these",
        "numbers come from real PRs with nothing actually skipped. Times are test time from",
        "pytest's junit report, not whole-job minutes (setup and install aren't counted).",
        "",
        "| repo | runs | full-suite runs | test time | blast would run | saved | missed failures |",
        "|---|---|---|---|---|---|---|",
    ]
    by_repo: dict[str, list[dict]] = {}
    for r in rows:
        by_repo.setdefault(r["repo"], []).append(r)
    all_total = all_sel = 0.0
    for repo, rs in sorted(by_repo.items()):
        total = sum(r["seconds_total"] for r in rs)
        sel = sum(r["seconds_selected"] for r in rs)
        fulls = sum(r["mode"] == "full" for r in rs)
        missed = sum(1 for r in rs if r["missed_failure_files"])
        with_fail = sum(1 for r in rs if r["failures_total"])
        all_total += total
        all_sel += sel
        lines.append(
            f"| {repo} | {len(rs)} | {fulls} | {total / 60:.1f} min | {sel / 60:.1f} min "
            f"| {pct(total - sel, total)} | {missed} of {with_fail} runs with failures |"
        )
    lines += [
        "",
        f"Overall: {(all_total - all_sel) / 60:.1f} of {all_total / 60:.1f} test minutes would have been skipped "
        f"({pct(all_total - all_sel, all_total)}).",
    ]
    misses = [r for r in rows if r["missed_failure_files"]]
    if misses:
        lines += ["", "## Misses", ""]
        for r in misses:
            lines.append(f"- {r['repo']} PR {r.get('pr') or '?'} `{r['sha'][:10]}`: {', '.join(r['missed_failure_files'])}")
    return "\n".join(lines) + "\n"


def pct(a: float, b: float) -> str:
    return f"{100 * a / b:.0f}%" if b else "-"


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("repos", nargs="+")
    p.add_argument("--limit", type=int, default=500, help="max records per repo")
    p.add_argument("--out", type=Path)
    args = p.parse_args()

    rows = []
    for repo in args.repos:
        got = records(repo, args.limit)
        print(f"{repo}: {len(got)} shadow runs", file=sys.stderr)
        rows += got
    report = summarize(rows)
    if args.out:
        args.out.write_text(report)
    print(report)
    return 0


if __name__ == "__main__":
    sys.exit(main())
