#!/usr/bin/env python3
"""Render docs/demo.gif from real command output.

Runs each command below for real, then animates it being typed and its output
appearing. Needs Pillow:

  pip install pillow && python docs/make_demo.py

Run from the repo root with target/release/blast built and the try-blast-action branch
around (it's the demo PR).
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs" / "demo.gif"
BRANCH = "try-blast-action"

# (what the viewer sees, what we actually run)
SCRIPT = [
    (
        f"git diff --stat main..{BRANCH}",
        f"git diff --stat main..{BRANCH} -- examples/demo",
    ),
    (
        f"blast --diff main..{BRANCH} examples/demo",
        f"target/release/blast --diff main..{BRANCH} examples/demo",
    ),
    (
        "blast why tests/test_discounts.py shop/money.py",
        "target/release/blast why tests/test_discounts.py shop/money.py --root examples/demo",
    ),
    (
        "pytest $(blast --diff main..HEAD --format pytest-args)",
        f"git worktree add -q --detach {{wt}} {BRANCH} && cd {{wt}}/examples/demo && "
        f"{sys.executable} -m pytest -q -p no:cacheprovider "
        f"$({ROOT}/target/release/blast --diff main..HEAD --format pytest-args); "
        f"cd {ROOT} && git worktree remove --force {{wt}}",
    ),
]

W, H = 980, 560
PAD = 22
LINE = 22
BG = (24, 26, 33)
FG = (220, 223, 228)
DIM = (130, 137, 150)
PROMPT = (126, 200, 140)
ACCENT = (240, 180, 90)
TITLE_BG = (38, 41, 50)


def font(size: int) -> ImageFont.FreeTypeFont:
    for path in ["/System/Library/Fonts/Menlo.ttc", "/System/Library/Fonts/SFNSMono.ttf",
                 "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"]:
        if os.path.exists(path):
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


FONT = font(15)


def run(cmd: str) -> list[str]:
    wt = Path(os.environ.get("TMPDIR", "/tmp")) / "blast-demo-wt"
    cmd = cmd.replace("{wt}", str(wt))
    out = subprocess.run(cmd, shell=True, cwd=ROOT, capture_output=True, text=True)
    text = (out.stdout + out.stderr).rstrip("\n")
    return [line for line in text.splitlines() if "ResourceWarning" not in line]


def colour(line: str):
    if line.startswith("running") or "passed" in line:
        return ACCENT
    if line.startswith("  because") or line.startswith("  "):
        return FG
    return DIM if "|" in line or "changed" in line else FG


def frame(lines: list[tuple[str, tuple]], cursor: bool) -> Image.Image:
    img = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(img)
    d.rectangle([0, 0, W, 34], fill=TITLE_BG)
    for i, c in enumerate([(237, 106, 94), (245, 191, 79), (98, 197, 84)]):
        d.ellipse([16 + i * 22, 11, 28 + i * 22, 23], fill=c)
    d.text((W // 2 - 40, 9), "blast demo", font=FONT, fill=DIM)

    max_lines = (H - 34 - PAD * 2) // LINE
    visible = lines[-max_lines:]
    y = 34 + PAD
    for text, col in visible:
        if text.startswith("$ "):
            d.text((PAD, y), "$", font=FONT, fill=PROMPT)
            d.text((PAD + 18, y), text[2:], font=FONT, fill=FG)
        else:
            d.text((PAD, y), text, font=FONT, fill=col)
        y += LINE
    if cursor and visible:
        last = visible[-1][0]
        x = PAD + FONT.getlength(last if not last.startswith("$ ") else "$ " + last[2:]) + 2
        d.rectangle([x, y - LINE + 3, x + 8, y - 3], fill=FG)
    return img


def main() -> int:
    frames: list[Image.Image] = []
    durations: list[int] = []
    screen: list[tuple[str, tuple]] = []

    def push(cursor=True, ms=60):
        frames.append(frame(screen, cursor))
        durations.append(ms)

    push(ms=600)
    for shown, actual in SCRIPT:
        output = run(actual)
        screen.append(("$ ", FG))
        for i in range(0, len(shown) + 1, 2):
            screen[-1] = ("$ " + shown[:i], FG)
            push(ms=45)
        screen[-1] = ("$ " + shown, FG)
        push(ms=400)
        for line in output:
            screen.append((line, colour(line)))
        push(cursor=False, ms=2600)
        screen.append(("", FG))
    push(cursor=False, ms=3500)

    # keep the palette small so the gif stays light
    pal = [f.convert("P", palette=Image.ADAPTIVE, colors=32) for f in frames]
    pal[0].save(OUT, save_all=True, append_images=pal[1:], duration=durations, loop=0, optimize=True)
    print(f"wrote {OUT} ({OUT.stat().st_size // 1024} KB, {len(frames)} frames)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
