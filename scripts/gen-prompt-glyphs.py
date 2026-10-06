#!/usr/bin/env python3
"""Rasterises the PromptFont glyphs the front end can substitute for the
disc's own button glyphs, into assets/ui/prompts/.

PromptFont by Shinmera, SIL Open Font Licence 1.1 (licences/PromptFont-OFL.txt),
https://codeberg.org/shinmera/promptfont . Not game content.

Usage: python3 -I scripts/gen-prompt-glyphs.py <promptfont.ttf> <glyphs.json>

The font file is pinned by hash: a different release may move glyphs, and the
table in crates/ui/src/prompt.rs names them by code-name, not by codepoint.
Output:
  promptfont-subset.a8   N cells of CELL x CELL alpha bytes, row-major, the
                         glyph's ink fitted and centred, in index order
  promptfont-subset.idx  one line per cell: <codepoint hex> <code-name>
"""
import hashlib
import json
import sys

from PIL import Image, ImageDraw, ImageFont

TTF_SHA256 = "a5fe3498acda95c12694ddb5c1b6eccd56a73826977bdda85756b24c7cf0eeb3"
CELL = 48
NAMES = (
    "sony-a sony-b sony-x sony-y sony-left-shoulder sony-right-shoulder "
    "sony-options gamepad-select "
    "xbox-a xbox-b xbox-x xbox-y xbox-left-shoulder xbox-right-shoulder "
    "xbox-menu xbox-view "
    "nintendo-left-shoulder nintendo-right-shoulder nintendo-plus nintendo-minus "
    "dpad-up dpad-down dpad-left dpad-right "
    "keyboard-up keyboard-down keyboard-left keyboard-right keyboard-enter "
    "keyboard-backspace keyboard-space keyboard-tab keyboard-key "
).split() + [f"keyboard-{c}" for c in "abcdefghijklmnopqrstuvwxyz"]


def main(ttf_path, json_path, out_dir="assets/ui/prompts"):
    data = open(ttf_path, "rb").read()
    if hashlib.sha256(data).hexdigest() != TTF_SHA256:
        sys.exit("promptfont.ttf is not the pinned release")
    glyphs = {g["code-name"]: g for g in json.load(open(json_path))}
    font = ImageFont.truetype(ttf_path, CELL * 2)
    cells = bytearray()
    index = []
    for name in NAMES:
        glyph = glyphs[name]
        canvas = Image.new("L", (CELL * 5, CELL * 5), 0)
        ImageDraw.Draw(canvas).text((CELL, CELL), glyph["character"], font=font, fill=255)
        box = canvas.getbbox()
        ink = canvas.crop(box)
        scale = (CELL - 2) / max(ink.size)
        ink = ink.resize(
            (max(1, round(ink.width * scale)), max(1, round(ink.height * scale))),
            Image.LANCZOS,
        )
        cell = Image.new("L", (CELL, CELL), 0)
        cell.paste(ink, ((CELL - ink.width) // 2, (CELL - ink.height) // 2))
        cells += cell.tobytes()
        index.append(f"{glyph['codepoint']:x} {name}\n")
    with open(f"{out_dir}/promptfont-subset.a8", "wb") as f:
        f.write(cells)
    with open(f"{out_dir}/promptfont-subset.idx", "w") as f:
        f.writelines(index)
    print(len(index), "glyphs,", len(cells), "bytes")


if __name__ == "__main__":
    main(*sys.argv[1:])
