#!/usr/bin/env python3
"""Per-region tone/bloom/fog comparison on a matched-camera HD frame pair.

Consumes `data/reference/hd-capture/talons-matched/NN.{png,json}` - the
camera-pick pairs `docs/reverse-engineering/rpcs3-capture.md`'s "The pick is
fixed, and a rendered overlay confirms it" section produced (00, 01, 03; 02's
camera pick was refused and has no pair). For each pair it renders
`oag-game` at the recovered pose with this project's stated "comparison
setting" (`RenderProfile::default()`: native size, no MSAA, no motion blur,
no shadows, no upscaler) and reports per-region statistics a human can act
on, instead of the single-number aggregates-at-a-distance the pre-camera-pick
brightness threads were stuck with.

    scripts/hd-frame-compare.py                       # all three pairs
    scripts/hd-frame-compare.py --pose 01
    scripts/hd-frame-compare.py --game target/release/oag-game

# What this measures, and what it deliberately does not

Whole frame excluding the HUD, the sky, the road surface and a distant-
geometry band, each as a luminance mean/histogram, per-channel mean, clipped-
white share (R,G,B all >= 250, `scripts/clipped-white.py`'s own threshold)
and a bloom-halo ring profile around the brightest source pixels. A fifth
output regresses aligned, non-HUD, non-craft, non-clipped midtone luminance
between the two images against both an affine fit and a power-law fit - the
affine slope/intercept names a magnitude or exposure error, the power-law
exponent, if it fits distinctly better, names an encoding mismatch and its
sign says which way (~2.2 means ours is linear where the reference is
gamma-encoded, ~0.45 the reverse). That is a direct, per-pixel probe of
renderer.md's "whether the 8-bit surfaces hold linear or gamma light",
confidence 70 there - this script can move that number, not invent one.

**One asymmetry every region below has to account for**: at `--ticks 0` (no
`--pose`, only `--camera-pose`) our own craft is wherever the race's normal
spawn puts it, which the matched camera was never aimed at - `main.rs`'s own
comment: "an RPCS3 capture gives the camera exactly and the craft only as
whatever the frame shows." Measured directly: all three of our renders draw
*no craft at all* in frame, while all three reference frames do. So the
`craft` region below is excluded from every other region's mask on both
sides - not because our craft needs hiding (it draws nothing there), but
because the reference's craft is real geometry with its own material
(`lane-ship-hull`'s territory, out of scope here) and would otherwise read as
an environment-tone discrepancy that is actually a missing-craft discrepancy.
It is reported on its own so the gap stays visible rather than quietly
masked away.

# The regions are chosen, not measured

Five rectangles in normalized frame coordinates, picked by eye against the
three pairs and shared across them (Talon's Junction's matched poses are all
forward-facing tunnel/corridor framings, close enough in composition that one
set of boxes is a legitimate shared window rather than a per-pose accident).
**No confidence score** - this is a comparison instrument, not an RE claim.
`--dump-regions` writes a visible overlay so the boxes can be checked by eye
instead of trusted blind.

# Needs

Pillow and numpy (both already present in this environment; nothing here
needs ImageMagick). A built `oag-game` - `--game` defaults to the debug
binary since that is what a plain `cargo build -p oag-game` produces; pass
`--game target/release/oag-game` after a release build. The decrypted PS3
image under `data/images/hdfury-ps3-eu-dec.iso`.

`data/` is gitignored, so `rg`/`fd` return nothing under it with exit code 0
- use `ls`/`find`, or `--no-ignore`.
"""

import argparse
import json
import os
import pathlib
import subprocess
import tempfile

import numpy as np
from PIL import Image, ImageDraw

ROOT = pathlib.Path(__file__).resolve().parent.parent
PAIR_DIR = ROOT / "data" / "reference" / "hd-capture" / "talons-matched"
OUT_DIR = ROOT / "data" / "reference" / "hd-capture" / "talons-matched-compare"
IMAGE = ROOT / "data" / "images" / "hdfury-ps3-eu-dec.iso"
TRACK = "/data/environments/talons_junction/track.vex"
TEAM = "feisar_c1"

# The comparison setting every HD/Fury frame comparison in this project uses
# (`RenderProfile::default()`'s own doc comment names this combination):
# native size, no dynamic-resolution/FSR reconstruction, no MSAA, no motion
# blur, no shadow tier.
COMPARISON_ARGS = [
    "--size", "1280x720",
    "--render-scale", "100",
    "--msaa", "off",
    "--motion-blur", "off",
    "--shadows", "off",
    "--reconstruction", "off",
]

CLIP = 250  # clipped-white.py's own threshold, kept identical on purpose.

# Regions, chosen by eye, shared across all three poses. (x0, y0, x1, y1) in
# [0, 1] frame fractions, x left-to-right, y top-to-bottom.
HUD_BOXES = [
    (0.00, 0.00, 0.15, 0.20),  # position/lap hexagon, top-left
    (0.27, 0.00, 0.73, 0.23),  # shield/percentage bar, top-centre
    (0.81, 0.00, 1.00, 0.20),  # race-position box, top-right
    (0.00, 0.77, 0.29, 1.00),  # best-time / lap-timer box, bottom-left
    (0.70, 0.73, 1.00, 1.00),  # speed / absorb box, bottom-right
]
CRAFT_BOX = (0.28, 0.42, 0.68, 1.00)
SKY_BOX = (0.42, 0.08, 0.58, 0.30)
DISTANT_BOX = (0.10, 0.22, 0.90, 0.42)
# Road: the bottom band split around the craft box rather than under it.
ROAD_BOXES = [
    (0.00, 0.55, 0.28, 0.85),
    (0.68, 0.55, 1.00, 0.85),
]


def box_mask(shape, box):
    h, w = shape
    x0, y0, x1, y1 = box
    mask = np.zeros((h, w), dtype=bool)
    mask[int(y0 * h):int(y1 * h), int(x0 * w):int(x1 * w)] = True
    return mask


def union(shape, boxes):
    h, w = shape
    out = np.zeros((h, w), dtype=bool)
    for box in boxes:
        out |= box_mask(shape, box)
    return out


def build_masks(shape):
    hud = union(shape, HUD_BOXES)
    craft = box_mask(shape, CRAFT_BOX)
    exclude = hud | craft
    return {
        "whole (excl HUD)": ~hud,
        "whole (excl HUD, craft)": ~exclude,
        "sky": box_mask(shape, SKY_BOX) & ~exclude,
        "road surface": union(shape, ROAD_BOXES) & ~exclude,
        "distant geometry": box_mask(shape, DISTANT_BOX) & ~exclude,
        "craft (reference only)": craft & ~hud,
    }, hud, craft


def luminance(arr):
    """Rec. 709 luma, 0..1 - the same weights `clipped-white.py` uses."""
    r, g, b = arr[..., 0], arr[..., 1], arr[..., 2]
    return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0


def region_stats(arr, luma, mask):
    count = int(mask.sum())
    if count == 0:
        return None
    px = arr[mask].astype(np.float64)
    lum = luma[mask]
    clipped = np.all(px >= CLIP, axis=-1)
    hist, edges = np.histogram(lum, bins=8, range=(0.0, 1.0))
    return {
        "count": count,
        "mean_rgb": px.mean(axis=0),
        "mean_luma": float(lum.mean()),
        "clipped_pct": 100.0 * clipped.sum() / count,
        "hist": hist,
        "hist_edges": edges,
    }


def dilate(mask, steps):
    """Grows `mask` by `steps` 4-connected pixels, without scipy."""
    out = mask.copy()
    for _ in range(steps):
        grown = out.copy()
        grown[1:, :] |= out[:-1, :]
        grown[:-1, :] |= out[1:, :]
        grown[:, 1:] |= out[:, :-1]
        grown[:, :-1] |= out[:, 1:]
        out = grown
    return out


def bloom_halo(arr, luma, valid_mask):
    """Ring-averaged luminance around clipped-white source pixels.

    A per-pixel measure of how far the glow spreads past its own clipped
    core, rather than a single aggregate: `[2, 4, 8, 16]`-pixel rings around
    the clipped-white mask, each ring's own mean luminance. A chain with a
    wide halo (the pre-fix `1.6e5`-argument gate renderer.md documents) reads
    as a slow fall-off across the rings; a tight one reads as a fast one.
    Restricted to `valid_mask` (HUD and craft already excluded) so HUD glyph
    edges and the reference craft's own bright trim never enter it.
    """
    core = np.all(arr >= CLIP, axis=-1) & valid_mask
    core_count = int(core.sum())
    if core_count == 0:
        return {"core_pct": 0.0, "rings": []}
    rings = []
    prev = core
    for radius in (2, 4, 8, 16):
        grown = dilate(core, radius) & valid_mask
        ring = grown & ~prev
        if ring.any():
            rings.append((radius, float(luma[ring].mean()), int(ring.sum())))
        else:
            rings.append((radius, None, 0))
        prev = grown
    return {
        "core_pct": 100.0 * core_count / int(valid_mask.sum()),
        "rings": rings,
    }


def encoding_probe(ours_luma, ref_luma, valid_mask, ours_arr, ref_arr):
    """Affine vs. power-law fit between matched midtone luminance.

    Sampled over `valid_mask` (HUD and craft already excluded) and further
    restricted to pixels clipped in neither image - a clipped pixel carries
    no information about the transfer curve, only about the gate. Affine
    fits a magnitude/exposure error; a power law that fits distinctly better
    names an encoding mismatch, and its exponent's distance from 1.0 says
    which direction (~2.2 vs ~0.45).
    """
    clipped_either = np.all(ours_arr >= CLIP, axis=-1) | np.all(
        ref_arr >= CLIP, axis=-1
    )
    sample = valid_mask & ~clipped_either
    x = ours_luma[sample].astype(np.float64)
    y = ref_luma[sample].astype(np.float64)
    if x.size < 100:
        return None
    # Affine fit, R^2 in linear space.
    a, b = np.polyfit(x, y, 1)
    pred = a * x + b
    ss_res = float(np.sum((y - pred) ** 2))
    ss_tot = float(np.sum((y - y.mean()) ** 2))
    r2_affine = 1.0 - ss_res / ss_tot if ss_tot > 0 else float("nan")
    # Power-law fit via a log-log regression, floored well above zero so a
    # near-black pixel does not dominate the log.
    floor = 1.0 / 255.0
    xf, yf = np.maximum(x, floor), np.maximum(y, floor)
    lx, ly = np.log(xf), np.log(yf)
    gamma, c = np.polyfit(lx, ly, 1)
    predl = gamma * lx + c
    ss_res_l = float(np.sum((ly - predl) ** 2))
    ss_tot_l = float(np.sum((ly - ly.mean()) ** 2))
    r2_gamma = 1.0 - ss_res_l / ss_tot_l if ss_tot_l > 0 else float("nan")
    return {
        "n": int(x.size),
        "affine_a": float(a),
        "affine_b": float(b),
        "affine_r2": r2_affine,
        "gamma": float(gamma),
        "gamma_r2": r2_gamma,
    }


def camera_pose_args(camera):
    eye = camera["eye"]
    fwd = camera["forward"]
    up = camera["up"]
    nums = ",".join(f"{v:.6f}" for v in (*eye, *fwd, *up))
    return [f"--camera-pose={nums}", f"--camera-fov={camera['fov_y_deg']:.6f}"]


def render(game, camera, out_path, bloom, cfg_root):
    """Renders one frame.

    `[graphics] bloom` has no CLI flag - it is a `[graphics]` settings key,
    not a `RenderProfile` field the pose flags reach - so this writes its own
    `settings.toml` into a scratch `XDG_CONFIG_HOME`, the same trick
    `hd-glow-sweep.py` uses, rather than touching the machine's own
    configuration. **Default here is bloom on**, not the project's own
    player-facing default of off: the original hardware has no such toggle
    at all, its chain is always live (renderer.md, "The chain was
    unswitchable"), so the faithful comparison to a fixed-function original
    is with the switch on. `--bloom off` exists to isolate the chain's own
    contribution when a discrepancy needs it.
    """
    cfg = cfg_root / f"cfg-{'on' if bloom else 'off'}"
    (cfg / "oag").mkdir(parents=True, exist_ok=True)
    (cfg / "oag" / "settings.toml").write_text(
        f"[graphics]\nbloom = {'true' if bloom else 'false'}\n", encoding="utf-8"
    )
    cmd = [
        str(game),
        str(IMAGE),
        "--race",
        "--track", TRACK,
        "--team", TEAM,
        *COMPARISON_ARGS,
        *camera_pose_args(camera),
        "--no-audio",
        "--screenshot", str(out_path),
    ]
    env = dict(os.environ, XDG_CONFIG_HOME=str(cfg))
    done = subprocess.run(cmd, capture_output=True, text=True, env=env)
    if done.returncode != 0 or not out_path.exists():
        tail = (done.stderr or done.stdout).strip().splitlines()[-5:]
        raise SystemExit(f"render failed: {' '.join(cmd)}\n" + "\n".join(tail))


def draw_regions(ref_img, masks, out_path):
    """A visible overlay of every mask, so the boxes are checked, not trusted."""
    colours = {
        "whole (excl HUD)": None,  # too broad to usefully paint
        "whole (excl HUD, craft)": None,
        "sky": (80, 220, 255, 90),
        "road surface": (80, 255, 120, 90),
        "distant geometry": (255, 220, 60, 90),
        "craft (reference only)": (255, 60, 220, 90),
    }
    overlay = Image.new("RGBA", ref_img.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay)
    for name, colour in colours.items():
        if colour is None:
            continue
        mask = masks[name]
        rgba = np.zeros((*mask.shape, 4), dtype=np.uint8)
        rgba[mask] = colour
        overlay = Image.alpha_composite(overlay, Image.fromarray(rgba, "RGBA"))
    hud = union((ref_img.height, ref_img.width), HUD_BOXES)
    rgba = np.zeros((*hud.shape, 4), dtype=np.uint8)
    rgba[hud] = (255, 0, 0, 90)
    overlay = Image.alpha_composite(overlay, Image.fromarray(rgba, "RGBA"))
    base = ref_img.convert("RGBA")
    Image.alpha_composite(base, overlay).save(out_path)


def format_hist(hist):
    total = hist.sum()
    if total == 0:
        return "-"
    blocks = " .:-=+*#%@"
    scale = hist.max() if hist.max() > 0 else 1
    return "".join(blocks[min(len(blocks) - 1, int(9 * v / scale))] for v in hist)


def compare_one(game, pose, dump_regions, bloom, cfg_root):
    json_path = PAIR_DIR / f"{pose}.json"
    png_path = PAIR_DIR / f"{pose}.png"
    if not json_path.exists() or not png_path.exists():
        raise SystemExit(f"missing pair for pose {pose}: {json_path}")
    meta = json.loads(json_path.read_text())
    camera = meta.get("camera")
    if not camera:
        raise SystemExit(
            f"{json_path}: camera pick was refused ({meta.get('camera_reason')})"
        )

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    ours_path = OUT_DIR / f"{pose}-ours-bloom-{'on' if bloom else 'off'}.png"
    render(game, camera, ours_path, bloom, cfg_root)

    ref_img = Image.open(png_path).convert("RGB")
    ours_img = Image.open(ours_path).convert("RGB")
    if ref_img.size != ours_img.size:
        raise SystemExit(
            f"pose {pose}: size mismatch, reference {ref_img.size} vs ours "
            f"{ours_img.size} - resample before trusting any region stat"
        )

    ref_arr = np.asarray(ref_img)
    ours_arr = np.asarray(ours_img)
    ref_luma = luminance(ref_arr)
    ours_luma = luminance(ours_arr)
    shape = ref_arr.shape[:2]
    masks, hud, craft = build_masks(shape)

    if dump_regions:
        draw_regions(ref_img, masks, OUT_DIR / f"{pose}-regions.png")

    print(f"\n=== pose {pose} ({meta['track']}, fov {camera['fov_y_deg']:.2f} deg) ===")
    print(f"{'region':<28}{'side':<10}{'px':>9}{'mean R':>8}{'mean G':>8}"
          f"{'mean B':>8}{'luma':>7}{'clip%':>8}  histogram")
    for name, mask in masks.items():
        for side, arr, luma in (("ours", ours_arr, ours_luma), ("reference", ref_arr, ref_luma)):
            stats = region_stats(arr, luma, mask)
            if stats is None:
                print(f"{name:<28}{side:<10}{'(empty)':>9}")
                continue
            r, g, b = stats["mean_rgb"]
            print(
                f"{name:<28}{side:<10}{stats['count']:>9}{r:>8.1f}{g:>8.1f}"
                f"{b:>8.1f}{stats['mean_luma']:>7.3f}{stats['clipped_pct']:>7.2f}%"
                f"  {format_hist(stats['hist'])}"
            )

    valid = masks["whole (excl HUD, craft)"]
    print("\nbloom halo (ring mean luma past the clipped-white core, valid region only):")
    for side, arr, luma in (("ours", ours_arr, ours_luma), ("reference", ref_arr, ref_luma)):
        halo = bloom_halo(arr, luma, valid)
        ring_txt = ", ".join(
            f"+{r}px={m:.3f}" if m is not None else f"+{r}px=(none)"
            for r, m, _ in halo["rings"]
        )
        print(f"  {side:<10} core {halo['core_pct']:.3f}%  {ring_txt}")

    probe = encoding_probe(ours_luma, ref_luma, valid, ours_arr, ref_arr)
    print("\nlinear-vs-gamma probe (ours -> reference luminance, non-HUD/craft/clipped midtones):")
    if probe is None:
        print("  too few sample pixels")
    else:
        print(
            f"  n={probe['n']}  affine y={probe['affine_a']:.3f}x+{probe['affine_b']:.3f} "
            f"R^2={probe['affine_r2']:.4f}  power gamma={probe['gamma']:.3f} "
            f"R^2={probe['gamma_r2']:.4f}"
        )
        better = "power" if probe["gamma_r2"] > probe["affine_r2"] + 0.01 else "affine"
        print(f"  better fit: {better}")

    return {
        "pose": pose,
        "masks": masks,
        "ours_arr": ours_arr,
        "ref_arr": ref_arr,
    }


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--pose", action="append", dest="poses",
                         help="one of 00/01/03; repeatable. Default: all three.")
    parser.add_argument("--game", default=str(ROOT / "target" / "debug" / "oag-game"))
    parser.add_argument("--dump-regions", action="store_true",
                         help="write an <NN>-regions.png mask overlay per pose")
    parser.add_argument("--bloom", choices=["on", "off"], default="on",
                         help="[graphics] bloom for our render; default on - "
                              "see render()'s own doc for why that, not the "
                              "player-facing default, is the faithful setting")
    args = parser.parse_args()

    game = pathlib.Path(args.game)
    if not game.exists():
        raise SystemExit(f"{game} is missing - cargo build -p oag-game")
    if not IMAGE.exists():
        raise SystemExit(f"{IMAGE} is missing - see data/README.md")

    poses = args.poses or ["00", "01", "03"]
    with tempfile.TemporaryDirectory() as tmp:
        cfg_root = pathlib.Path(tmp)
        for pose in poses:
            compare_one(game, pose, args.dump_regions, args.bloom == "on", cfg_root)


if __name__ == "__main__":
    main()
