#!/usr/bin/env python3
"""Build the seed manifest for `just compare-page` from the reference sheets under data/reference/.

    scripts/compare-seed.py            # writes data/reference/compare-page/seed.toml and its cells

Splits the composite sheets the HD lanes kept (3x3 grids of 640x400 cells: row 0 the
original, row 1 ours before, row 2 ours after) into one entry per column, and adds
the pairs that already exist as separate files. Nothing here is committed: data/ is
gitignored. Sheets not used, and why:

- data/reference/hd-craft-ibl-probe: raw texture dumps and poses, no screenshot of either side.
- the bottom row of hull-gap-sheet.png: texture dumps, not a frame.

The Omega and 2048 before/after pairs (data/reference/compare-page/seed/*-before.png,
*-after.png) have no original: the lane that made them recorded that none was captured,
so those entries say so and never put one of ours in the original slot.
"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REF = ROOT / "data" / "reference"
SEED = REF / "compare-page" / "seed"
CELL_W, CELL_H, LABEL = 640, 400, 14

HD = "Wipeout HD (EU)"
TALON = "Talon's Junction (Time Trial)"
HARIMAU = "Harimau classic"


def magick(*a):
    subprocess.run(["magick", *map(str, a)], check=True)


def cell(sheet: Path, row: int, col: int, out: Path):
    # The label strip at the top of each cell is baked into the sheet; cut it so it
    # does not show up in the difference.
    magick(sheet, "-crop", f"{CELL_W}x{CELL_H - LABEL}+{col * CELL_W}+{row * CELL_H + LABEL}", "+repage", out)


def toml_str(s: str) -> str:
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


PLATFORMS = {"Wipeout HD (EU)": "PS3", "Wipeout Omega Collection": "PS4", "Wipeout 2048 (DLC)": "Vita"}


def entry(**kv) -> str:
    kv = {**{k: v for k, v in kv.items() if k == "id"}, "platform": PLATFORMS[kv["game"]], **{k: v for k, v in kv.items() if k != "id"}}
    return "[[entry]]\n" + "".join(f"{k} = {toml_str(v)}\n" for k, v in kv.items()) + "\n"


def main():
    SEED.mkdir(parents=True, exist_ok=True)
    out = ['[page]\ntitle = "OpenAntiGrav: original vs ours"\n'
           'intro = "Seeded from the lanes\' reference sheets. Crops are chosen by eye, not pixel-registered."\n\n']

    flare = REF / "hd-flare-material"
    for sheet, name, state, ev in (
        ("rest-sheet.png", "rest", "at rest, 0 km/h, grid / just after GO (rest glare removed in 'ours')",
         "engine-trail.md, 'The flare's draw state, read live'; hd-ship-materials.md Finding 4"),
        ("thrust-sheet.png", "thrust", "under thrust and coasting at 177 km/h (flame draws present)",
         "engine-trail.md, 'The flare's draw state, read live'"),
    ):
        src = flare / sheet
        if not src.is_file():
            print(f"skip {sheet}: missing", file=sys.stderr)
            continue
        for col in range(3):
            base = f"flare-{name}-{col + 1}"
            for row, slot in enumerate(("original", "before", "ours")):
                cell(src, row, col, SEED / f"{base}-{slot}.png")
            out.append(entry(
                id=base, game=HD, title=f"Engine flare, {name}, crop {col + 1}",
                original=f"seed/{base}-original.png", before=f"seed/{base}-before.png", ours=f"seed/{base}-ours.png",
                track=TALON, craft=HARIMAU, state=state,
                setting="original on RPCS3 (Write Color Buffers on); ours at the hd-flare-material lane's before and after",
                note=f"Hull crop of the same framing, not pixel registered. Evidence: {ev}."))

    hull = REF / "hd-hull-reflection"
    sheet = hull / "hull-gap-sheet.png"
    ours = hull / "harimau-ours-before-1280x720.png"
    if sheet.is_file() and ours.is_file():
        magick(sheet, "-crop", "564x316+0+14", "+repage", SEED / "hull-gap-original.png")
        out.append(entry(
            id="hull-gap", game=HD, title="Hull lighting at rest (before the IBL change)",
            original="seed/hull-gap-original.png", ours="../hd-hull-reflection/harimau-ours-before-1280x720.png",
            track=TALON, craft=HARIMAU,
            state="at rest on the grid, GO banner overhead (original from RPCS3, ours the frame before the hull-reflection change)",
            setting="ours is BEFORE the hull-reflection change, not today's build; original is a window capture, low resolution",
            note="Framing differs between the two (different camera); read it for hull colour, not registration. "
                 "Evidence: hd-ship-materials.md Finding 4."))

    omega_2048 = (
        ("omega-01_vineta_k", "Wipeout Omega Collection", "Vineta K", "red/white craft"),
        ("omega-02_metropia", "Wipeout Omega Collection", "Metropia", "red/white craft"),
        ("omega-05_ubermall", "Wipeout Omega Collection", "Ubermall", "red/white craft"),
        ("Anulpha_Pass", "Wipeout 2048 (DLC)", "Anulpha Pass", "green FEISAR craft"),
        ("Moa_Therma", "Wipeout 2048 (DLC)", "Moa Therma", "yellow FEISAR craft"),
    )
    for stem, game, track, craft in omega_2048:
        if not (SEED / f"{stem}-before.png").is_file():
            print(f"skip {stem}: missing", file=sys.stderr)
            continue
        out.append(entry(
            id=stem.lower().replace("_", "-"), game=game, title=f"{track}: purple bands, before / after",
            before=f"seed/{stem}-before.png", ours=f"seed/{stem}-after.png",
            track=track, craft=craft, state="grid, 0 km/h, our race start",
            setting="normal-maps-as-colour lane: before = main-checkout binary of 2026-10-18, after = the lane's branch",
            not_captured="no original frame from the emulator was taken for this lane; this pair is ours against ours"))

    (REF / "compare-page" / "seed.toml").write_text("".join(out))
    print(f"wrote {REF / 'compare-page' / 'seed.toml'}")


if __name__ == "__main__":
    main()
