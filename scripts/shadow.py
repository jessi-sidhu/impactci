#!/usr/bin/env python3
"""Shadow mode: the full suite ran, compare it with what blast would have run.

  shadow.py SELECTION_JSON JUNIT_XML OUT_JSON [COMMENT_MD]

Writes a small JSON record (for collecting numbers across repos) and, if given, appends
a section to the PR comment. Stdlib only, runs on any runner with python3.
"""

from __future__ import annotations

import json
import os
import sys
import xml.etree.ElementTree as ET


def load_junit(path: str) -> list[dict]:
    cases = []
    root = ET.parse(path).getroot()
    for case in root.iter("testcase"):
        file = case.get("file")
        classname = case.get("classname") or ""
        if not file:
            # xunit2 has no file attr: tests.test_app.TestX -> tests/test_app.py (best effort)
            parts = classname.split(".")
            while parts and parts[-1][:1].isupper():
                parts.pop()
            file = "/".join(parts) + ".py" if parts else ""
        failed = case.find("failure") is not None or case.find("error") is not None
        cases.append({"file": file, "seconds": float(case.get("time") or 0), "failed": failed})
    return cases


def secs(s: float) -> str:
    if s >= 120:
        return f"{s / 60:.1f} min"
    return f"{s:.1f}s" if s < 10 else f"{s:.0f}s"


def main() -> int:
    sel_path, junit_path, out_path = sys.argv[1:4]
    comment_path = sys.argv[4] if len(sys.argv) > 4 else None

    sel = json.load(open(sel_path))
    cases = load_junit(junit_path)
    chosen = None if sel["mode"] == "full" else set(sel["selected_tests"])

    def picked(c):
        return chosen is None or c["file"] in chosen

    total_s = sum(c["seconds"] for c in cases)
    sel_s = sum(c["seconds"] for c in cases if picked(c))
    failed = [c for c in cases if c["failed"]]
    missed = sorted({c["file"] for c in failed if not picked(c)})

    record = {
        "repo": os.environ.get("GITHUB_REPOSITORY", ""),
        "pr": os.environ.get("BLAST_PR", ""),
        "sha": os.environ.get("GITHUB_SHA", ""),
        "mode": sel["mode"],
        "tests_total": len(cases),
        "tests_selected": sum(1 for c in cases if picked(c)),
        "seconds_total": round(total_s, 2),
        "seconds_selected": round(sel_s, 2),
        "failures_total": len(failed),
        "missed_failure_files": missed,
    }
    json.dump(record, open(out_path, "w"), indent=2)

    if comment_path:
        saved = total_s - sel_s
        pct = 100 * saved / total_s if total_s else 0
        lines = ["", "**Shadow mode** (the full suite ran, nothing was skipped)", ""]
        lines.append(
            f"- blast would have run {record['tests_selected']} of {record['tests_total']} tests, "
            f"saving {secs(saved)} of {secs(total_s)} test time ({pct:.0f}%)"
        )
        if missed:
            lines.append(f"- :warning: **it would have missed failures in** {', '.join(f'`{m}`' for m in missed)}")
        elif failed:
            lines.append(f"- all {len(failed)} failing tests were in its selection")
        else:
            lines.append("- no failures to compare against")
        with open(comment_path, "a") as f:
            f.write("\n".join(lines) + "\n")

    print(json.dumps(record))
    return 0


if __name__ == "__main__":
    sys.exit(main())
