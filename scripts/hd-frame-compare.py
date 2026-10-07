#!/usr/bin/env python3
"""Per-region tone/bloom/fog comparison on a matched-camera HD frame pair.

Consumes `data/reference/hd-capture/<pair-dir>/NN.{png,json}` - the
camera-pick pairs `docs/reverse-engineering/rpcs3-capture.md`'s "The pick is
fixed, and a rendered overlay confirms it" section produced. Default
`--pair-dir` is `talons-matched` (00, 01, 03; 02's camera pick was refused
and has no pair). For each pair it renders `oag-game` at the recovered pose,
on the track the pair's own JSON names (`track_arg` - never the hardcoded
constant an earlier version of this script carried, which would have
silently rendered Talon's Junction against any other circuit's pair), with
this project's stated "comparison setting" (`RenderProfile::default()`:
native size, no MSAA, no motion blur, no shadows, no upscaler) and reports
per-region statistics a human can act on, instead of the single-number
aggregates-at-a-distance the pre-camera-pick brightness threads were stuck
with.

    scripts/hd-frame-compare.py                       # all three talons-matched pairs
    scripts/hd-frame-compare.py --pose 01
    scripts/hd-frame-compare.py --game target/release/oag-game
    scripts/hd-frame-compare.py --pair-dir data/reference/hd-capture/sol2-matched \
        --pose 00 --pose 01 --dump-regions

# What this measures, and what it deliberately does not

Whole frame excluding the HUD, the sky, the road surface and a distant-
geometry band, each as a luminance mean/histogram, per-channel mean, mean
HSV saturation, a saturation-weighted circular mean hue (`lane/hd-track-
lighting`'s own addition - luma alone missed the 2026-08-20 brightness
defect and the maintainer's own report named "colors", not only
brightness), clipped-white share (R,G,B all >= 250,
`scripts/clipped-white.py`'s own threshold) and a bloom-halo ring profile
around the brightest source pixels. A fifth
output regresses aligned, non-HUD, non-craft, non-clipped midtone luminance
between the two images against both an affine fit and a power-law fit - the
affine slope/intercept names a magnitude or exposure error, the power-law
exponent, if it fits distinctly better, names an encoding mismatch and its
sign says which way (~2.2 means ours is linear where the reference is
gamma-encoded, ~0.45 the reverse). That is a direct, per-pixel probe of
renderer.md's "whether the 8-bit surfaces hold linear or gamma light",
confidence 70 there - this script can move that number, not invent one.
**That per-pixel fit needs correspondence it does not reliably have** (the
pose is matched, not the geometry - sub-pixel misalignment on high-frequency
texture, our own missing craft, the known flat-black floor panel at pose 03)
and its R^2 says so directly (0.01-0.26 across the three poses here). A sixth
output, the quantile-quantile table, compares each side's own rank-sorted
luminance distribution instead - no pixel correspondence needed at all - so
it survives exactly the misalignment the per-pixel fit cannot: a
near-constant `ref - ours` gap across ranks says an offset, a gap growing
in proportion to the value says a scale, and a gap concentrated in the top
few quantiles with low/mid ranks close says the top end is compressed - a
curve, not a magnitude.

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

**A second asymmetry, poses `01` and `03` only: the reference frame was
captured in motion (529 km/h and 431 km/h, its own HUD speed readout says)
and ours is posed statically at `--ticks 0`.** `COMPARISON_ARGS` forces
`--motion-blur off` for a like-for-like *setting*, but the reference capture
is a real frame with the original's own speed-driven streak/blur baked into
it by construction - not a setting a re-render can turn off. At pose `01`
this reads as broad white streaks covering roughly the lower half of the
frame; no static render at any lighting value reproduces that. **Treat pose
`00` (grid, 74 km/h per its own HUD) as the only pose that isolates the lit
material path** from a speed-effect confound; `01`/`03` corroborate the
*direction* of the gap but not its magnitude, and their per-pixel and
quantile fits should be read as noisier for exactly this reason, on top of
pose `03`'s already-documented flat-black floor panel.

# The regions are chosen, not measured

Five rectangles in normalized frame coordinates, picked by eye against the
three `talons-matched` pairs and shared across them (Talon's Junction's
matched poses are all forward-facing tunnel/corridor framings, close enough
in composition that one set of boxes is a legitimate shared window rather
than a per-pose accident). **No confidence score** - this is a comparison
instrument, not an RE claim. `--dump-regions` writes a visible overlay so the
boxes can be checked by eye instead of trusted blind - **mandatory on a
`--pair-dir` other than the default**, since nothing here re-derives the
boxes per circuit and a box that lands on sky or wall on a differently
framed pose would otherwise report a plausible-looking number under the
wrong region name.

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
DEFAULT_PAIR_DIR = ROOT / "data" / "reference" / "hd-capture" / "talons-matched"
IMAGE = ROOT / "data" / "images" / "hdfury-ps3-eu-dec.iso"
TEAM = "feisar_c1"


def track_arg(meta_track):
    """`meta['track']` (`Data\\Environments\\Name\\track.rcsmodel`, the disc's
    own `TTY.log` spelling) to the `--track` this project's own loader wants
    (`/data/environments/name/track.vex`).

    **Derived per pair, not hardcoded**, because a pair off any circuit other
    than the one this script shipped for (`talons_junction`) would otherwise
    render the wrong track at the right camera - a silently plausible-looking
    wrong comparison, not an obviously broken one. `TTY.log`'s own line is
    what a capture already records `track` from
    (`scripts/rpcs3-drive.py`'s `track_name()`), so this is a format
    conversion, not a new read.
    """
    name = meta_track.replace("\\", "/").split("/")[-2]
    return f"/data/environments/{name.lower()}/track.vex"

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


# Added for the `lane/hd-track-lighting` sign-flip re-check: mean luma
# matched the reference throughout the *original* 2026-08-20 brightness
# investigation and was exactly why that defect survived review (see
# renderer.md/hds-frame-was-too-bright-and-too-bloomy.md) - clipped-white
# share was what discriminated there. The maintainer's own from-play report
# this time named "lighting/illumination/**colors**", and the fixed
# Amphiseum wall panel's own residual is described as reading "flatter/
# greyer than the reference's cyan-white" - a saturation/hue complaint, not
# a luminance one. So hue and saturation are measured per region alongside
# luma rather than assumed to track it.
def saturation(arr):
    """HSV saturation, 0..1, vectorised."""
    px = arr.astype(np.float64)
    maxc = px.max(axis=-1)
    minc = px.min(axis=-1)
    return np.where(maxc > 0, (maxc - minc) / np.maximum(maxc, 1e-9), 0.0)


def hue_deg(arr):
    """HSV hue in degrees [0, 360), vectorised. Undefined (0) where sat is 0."""
    px = arr.astype(np.float64)
    r, g, b = px[..., 0], px[..., 1], px[..., 2]
    maxc = px.max(axis=-1)
    minc = px.min(axis=-1)
    delta = np.maximum(maxc - minc, 1e-9)
    hue_r = (60.0 * (((g - b) / delta) % 6.0))
    hue_g = (60.0 * (((b - r) / delta) + 2.0))
    hue_b = (60.0 * (((r - g) / delta) + 4.0))
    hue = np.select([maxc == r, maxc == g], [hue_r, hue_g], default=hue_b)
    return hue % 360.0


def circular_mean_hue_deg(hue, weight):
    """Saturation-weighted circular mean of a hue sample, or `None` if empty.

    A plain arithmetic mean of an angle is wrong at the wrap (0/360 average
    to 180, the opposite of either) - this averages the unit vectors
    instead. Weighted by saturation so a low-saturation (near-grey) pixel,
    whose hue angle is close to meaningless, does not out-vote a strongly
    coloured one.
    """
    if weight.sum() <= 0:
        return None
    rad = np.deg2rad(hue)
    x = float(np.sum(weight * np.cos(rad)))
    y = float(np.sum(weight * np.sin(rad)))
    if x == 0.0 and y == 0.0:
        return None
    return float(np.degrees(np.arctan2(y, x)) % 360.0)


def region_stats(arr, luma, mask):
    count = int(mask.sum())
    if count == 0:
        return None
    px = arr[mask].astype(np.float64)
    lum = luma[mask]
    sat = saturation(arr)[mask]
    hue = hue_deg(arr)[mask]
    clipped = np.all(px >= CLIP, axis=-1)
    zero = np.all(px == 0, axis=-1)
    hist, edges = np.histogram(lum, bins=8, range=(0.0, 1.0))
    # Hue is only meaningful for a pixel with some colour to it; a sat > 0.08
    # floor keeps HUD anti-aliasing and near-grey concrete from dominating
    # the circular mean with an arbitrary angle.
    coloured = sat > 0.08
    return {
        "count": count,
        "mean_rgb": px.mean(axis=0),
        "mean_luma": float(lum.mean()),
        "mean_sat": float(sat.mean()),
        "coloured_pct": 100.0 * float(coloured.sum()) / count,
        "mean_hue_deg": circular_mean_hue_deg(hue[coloured], sat[coloured]),
        "clipped_pct": 100.0 * clipped.sum() / count,
        "zero_pct": 100.0 * zero.sum() / count,
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


HALO_BANDS = ((1, 3), (3, 6), (6, 12), (12, 20), (20, 32), (32, 48), (48, 72), (72, 100))


def halo_shape(ours_luma, ref_luma, valid_mask):
    """Ring profile around cores both frames clip, brightness-normalised.

    `bloom_halo` compares absolute ring levels, and a frame that is simply
    darker overall (HD's accuracy-pass item 1) reads as a weaker halo at every
    radius. Here our luminance is rank-matched onto the reference's own
    distribution over `valid_mask` first (the q-q map below), so a pure
    exposure gap cancels and what is left in `ref - qq` is spatial: negative
    near the core and positive far out says our glow is tighter, a flat zero
    says the chain's reach is right. Sources are the pixels *both* frames clip
    (a bright thing only one frame has is a scene difference, not a halo), and
    each band excludes any pixel either frame clips.
    """
    ref_sorted = np.sort(ref_luma[valid_mask])
    ours_sorted = np.sort(ours_luma[valid_mask])
    if ref_sorted.size == 0:
        return None
    mapped = np.interp(ours_luma, ours_sorted, ref_sorted)
    thr = CLIP / 255.0
    core = (ours_luma >= thr) & (ref_luma >= thr) & valid_mask
    if not core.any():
        return None
    either = (ours_luma >= thr) | (ref_luma >= thr)
    grown = {0: core}
    cur = core
    for step in range(1, HALO_BANDS[-1][1] + 1):
        cur = dilate(cur, 1)
        grown[step] = cur
    rows = []
    for lo, hi in HALO_BANDS:
        band = grown[hi] & ~grown[lo - 1] & ~either & valid_mask
        n = int(band.sum())
        if n < 300:
            rows.append((lo, hi, n, None))
            continue
        rows.append((lo, hi, n, (float(ours_luma[band].mean()),
                                 float(mapped[band].mean()),
                                 float(ref_luma[band].mean()))))
    return 100.0 * float(core.sum()) / float(valid_mask.sum()), rows


QUANTILES = (1, 5, 10, 25, 50, 75, 90, 95, 99)


def quantile_probe(ours_luma, ref_luma, mask):
    """Quantile-quantile comparison of a region's luminance distribution.

    Unlike `encoding_probe`, this needs no pixel correspondence at all - each
    side's own quantiles are compared by rank, not by matching (x, y)
    screen positions - so it survives the sub-pixel misalignment, our own
    missing craft, and the known flat-black floor panel that make the affine/
    power-law fit at pose 03 read as noise (R^2 well under 0.03 there). A
    near-constant `ref - ours` gap across quantiles says an offset (ambient);
    a gap that grows with the quantile (both roughly a fixed ratio apart)
    says a scale (a light magnitude or a texture decode); a gap concentrated
    at the top few quantiles with the low/mid ones close says the top end is
    compressed - a saturate-and-encode curve rather than a magnitude, because
    a magnitude or offset error moves the whole distribution, not just its
    tail.
    """
    ours = np.sort(ours_luma[mask])
    ref = np.sort(ref_luma[mask])
    if ours.size < 100 or ref.size < 100:
        return None
    ours_q = np.percentile(ours, QUANTILES)
    ref_q = np.percentile(ref, QUANTILES)
    return list(zip(QUANTILES, ours_q, ref_q, ref_q - ours_q))


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


def render(game, camera, track, out_path, bloom, cfg_root):
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

    **`display.aspect` is pinned to `"wide"` (16:9), not left at the
    project's own default of `"psp"` (30:17).** `--size 1280x720` is 16:9 and
    `oag_display::display::Aspect::Wide`'s own doc names it "Wipeout HD's own
    1920x1080" - the PSP default is 0.7% narrower, close enough to read as a
    rounding error but not one: at this canvas it letterboxes to a
    1270.6-wide fitted rectangle with an unwritten margin either side. Found
    by this script's own `zero_pct` column reading a 100%-black, full-height,
    10-column strip at the frame's right edge on every pose, present with
    `--bloom` on or off, absent on a non-HD (Pulse) capture at the same size -
    `oag_post::hd_bloom::Chain::run`'s caller
    (`crates/raceplay/src/scene/frame.rs`) passes the *fitted* width/height
    to the chain but not the *offset* `oag_display::display::viewport`
    computes alongside it, so the encode pass's own `set_viewport(0.0, 0.0,
    ...)` writes a rectangle anchored at the canvas origin while the scene was
    actually drawn centred - the gap between the two is the strip. Pinning
    `aspect` to the canvas's own ratio makes the offset zero and reproduces
    byte-for-byte what a corrected caller would draw; confirmed by rendering
    pose `00` both ways (`zero_pct` 2.44% -> 0.00% in `road surface`, the
    region the strip's right-hand box lands in). **Filed, not fixed** at the
    `hd_bloom`/`frame.rs` level - see renderer.md - because this script does
    not need the general case and patching the call site is `lane-ship-hull`
    territory, not a comparison harness's.
    """
    cfg = cfg_root / f"cfg-{'on' if bloom else 'off'}"
    (cfg / "oag").mkdir(parents=True, exist_ok=True)
    (cfg / "oag" / "settings.toml").write_text(
        f'[graphics]\nbloom = {"true" if bloom else "false"}\n\n'
        f'[display]\naspect = "wide"\n',
        encoding="utf-8",
    )
    cmd = [
        str(game),
        str(IMAGE),
        "--race",
        "--track", track,
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


def compare_one(game, pair_dir, out_dir, pose, dump_regions, bloom, cfg_root, tiles=None):
    json_path = pair_dir / f"{pose}.json"
    png_path = pair_dir / f"{pose}.png"
    if not json_path.exists() or not png_path.exists():
        raise SystemExit(f"missing pair for pose {pose}: {json_path}")
    meta = json.loads(json_path.read_text())
    camera = meta.get("camera")
    if not camera:
        raise SystemExit(
            f"{json_path}: camera pick was refused ({meta.get('camera_reason')})"
        )
    track = track_arg(meta["track"])

    out_dir.mkdir(parents=True, exist_ok=True)
    ours_path = out_dir / f"{pose}-ours-bloom-{'on' if bloom else 'off'}.png"
    render(game, camera, track, ours_path, bloom, cfg_root)

    ref_img = Image.open(png_path).convert("RGB")
    ours_img = Image.open(ours_path).convert("RGB")
    if ref_img.size != ours_img.size:
        margin = (abs(ref_img.width - ours_img.width),
                  abs(ref_img.height - ours_img.height))
        # `--size 1280x720` is our own canvas exactly; the reference went
        # through `screenshot(trim=True)`'s crop on exact #000000
        # (rpcs3-capture.md, "What is free, and what costs packets"), which
        # measured 1278x718 here - a 1px margin either side, not a real
        # framing difference. A center-crop of the larger side to match
        # loses at most that same 1px rim, negligible against any region
        # this script reports on; a gap past a few pixels is a real
        # mismatch and still fails loud rather than silently cropping a
        # meaningful chunk of frame away.
        if margin[0] <= 8 and margin[1] <= 8:
            w = min(ref_img.width, ours_img.width)
            h = min(ref_img.height, ours_img.height)

            def center_crop(img):
                left = (img.width - w) // 2
                top = (img.height - h) // 2
                return img.crop((left, top, left + w, top + h))

            print(f"pose {pose}: {margin[0]}x{margin[1]}px size margin "
                  f"(reference {ref_img.size} vs ours {ours_img.size}) - "
                  f"center-cropping both to {w}x{h}", flush=True)
            ref_img, ours_img = center_crop(ref_img), center_crop(ours_img)
        else:
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
        draw_regions(ref_img, masks, out_dir / f"{pose}-regions.png")

    print(f"\n=== pose {pose} ({meta['track']}, fov {camera['fov_y_deg']:.2f} deg) ===")
    print(f"{'region':<28}{'side':<10}{'px':>9}{'mean R':>8}{'mean G':>8}"
          f"{'mean B':>8}{'luma':>7}{'sat':>6}{'hue':>6}{'colr%':>7}{'clip%':>8}{'zero%':>8}  histogram")
    for name, mask in masks.items():
        for side, arr, luma in (("ours", ours_arr, ours_luma), ("reference", ref_arr, ref_luma)):
            stats = region_stats(arr, luma, mask)
            if stats is None:
                print(f"{name:<28}{side:<10}{'(empty)':>9}")
                continue
            r, g, b = stats["mean_rgb"]
            hue_txt = ("%5.0f" % stats["mean_hue_deg"]
                       if stats["mean_hue_deg"] is not None else "    -")
            print(
                f"{name:<28}{side:<10}{stats['count']:>9}{r:>8.1f}{g:>8.1f}"
                f"{b:>8.1f}{stats['mean_luma']:>7.3f}{stats['mean_sat']:>6.3f}"
                f"{hue_txt}{stats['coloured_pct']:>6.1f}%{stats['clipped_pct']:>7.2f}%"
                f"{stats['zero_pct']:>7.2f}%  {format_hist(stats['hist'])}"
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

    shape = halo_shape(ours_luma, ref_luma, valid)
    print("\nbloom halo shape (rank-matched ours, rings around cores both frames clip):")
    if shape is None:
        print("  no shared clipped core")
    else:
        print(f"  shared core {shape[0]:.2f}%   cols: raw ours / rank-matched ours / ref")
        for lo, hi, n, v in shape[1]:
            if v is None:
                print(f"  {lo:>3}-{hi:<3}px n={n:>6}  (too few)")
                continue
            print(f"  {lo:>3}-{hi:<3}px n={n:>6}  ours={v[0]:.3f} qq={v[1]:.3f} "
                  f"ref={v[2]:.3f}  ref-qq={v[2] - v[1]:+.3f}")

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

    print("\nquantile-quantile (rank-matched, no pixel correspondence needed):")
    print(f"  {'region':<24}{'q%':>4}" + "".join(f"{q:>7}" for q in ("ours", "ref", "ref-ours")))
    for name in ("whole (excl HUD, craft)", "road surface", "distant geometry", "sky"):
        rows = quantile_probe(ours_luma, ref_luma, masks[name])
        if rows is None:
            continue
        for q, o, r, d in rows:
            print(f"  {name:<24}{q:>4}{o:>7.3f}{r:>7.3f}{d:>+7.3f}")

    if tiles:
        print_tile_grid(ours_arr, ref_arr, ours_luma, ref_luma, valid, tiles)

    return {
        "pose": pose,
        "masks": masks,
        "ours_arr": ours_arr,
        "ref_arr": ref_arr,
    }


def print_tile_grid(ours_arr, ref_arr, ours_luma, ref_luma, valid_mask, tiles):
    """A coarse, box-free spatial breakdown: `rows x cols` equal tiles over the
    frame, each restricted to `valid_mask` (HUD and craft already excluded).

    Exists because a hand-picked region (`sky`, `road surface`, ...) is a
    claim about what a rectangle contains, and Amphiseum's own composition
    keeps refuting that claim on sight (`sky` samples a ceiling, `road
    surface` samples a barrier wall). A tile needs no such claim - it reports
    where in the frame a gap sits without asserting what is drawn there, which
    is what a maintainer's own "both, and it varies by area" report needs
    checked before trusting any single rectangle's name. No confidence score:
    straight pixel arithmetic over a fixed grid, same footing as the rest of
    this script's per-region output.
    """
    rows, cols = tiles
    h, w = valid_mask.shape
    print(f"\ntile grid ({rows}x{cols}, valid region only - HUD and craft excluded):")
    print(f"  {'tile':<10}{'px':>8}{'ours luma':>10}{'ref luma':>9}{'ref-ours':>9}"
          f"{'ours hue':>9}{'ref hue':>9}{'hue gap':>8}")
    for r in range(rows):
        y0, y1 = int(h * r / rows), int(h * (r + 1) / rows)
        for c in range(cols):
            x0, x1 = int(w * c / cols), int(w * (c + 1) / cols)
            cell = np.zeros((h, w), dtype=bool)
            cell[y0:y1, x0:x1] = True
            cell &= valid_mask
            count = int(cell.sum())
            label = f"r{r}c{c}"
            if count < 200:
                print(f"  {label:<10}{count:>8}  (too few valid px)")
                continue
            ours_l = float(ours_luma[cell].mean())
            ref_l = float(ref_luma[cell].mean())
            ours_sat = saturation(ours_arr)[cell]
            ref_sat = saturation(ref_arr)[cell]
            ours_hue = hue_deg(ours_arr)[cell]
            ref_hue = hue_deg(ref_arr)[cell]
            ours_h = circular_mean_hue_deg(ours_hue[ours_sat > 0.08], ours_sat[ours_sat > 0.08])
            ref_h = circular_mean_hue_deg(ref_hue[ref_sat > 0.08], ref_sat[ref_sat > 0.08])
            ours_h_txt = "%6.0f" % ours_h if ours_h is not None else "     -"
            ref_h_txt = "%6.0f" % ref_h if ref_h is not None else "     -"
            if ours_h is not None and ref_h is not None:
                gap = abs(ours_h - ref_h)
                gap = min(gap, 360.0 - gap)
                gap_txt = "%6.0f" % gap
            else:
                gap_txt = "     -"
            sign = "brighter" if ref_l < ours_l else "darker" if ref_l > ours_l else "="
            print(f"  {label:<10}{count:>8}{ours_l:>10.3f}{ref_l:>9.3f}"
                  f"{ref_l - ours_l:>+9.3f}{ours_h_txt:>9}{ref_h_txt:>9}{gap_txt:>8}  "
                  f"ours {sign}")


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--pose", action="append", dest="poses",
                         help="a pose stem (e.g. 00/01/03) present in --pair-dir; "
                              "repeatable. Default: 00, 01, 03 (talons-matched's own set - "
                              "pass explicit --pose values for any other --pair-dir).")
    parser.add_argument("--pair-dir", default=str(DEFAULT_PAIR_DIR),
                         help="directory of NN.{png,json} capture pairs, e.g. "
                              "data/reference/hd-capture/sol2-matched. The track "
                              "rendered for comparison is read out of each pair's "
                              "own JSON (`track_arg`), never assumed - a pair from "
                              "any circuit is safe to point this at. Default: "
                              "talons-matched. NOTE: HUD_BOX/CRAFT_BOX/SKY_BOX/"
                              "ROAD_BOXES/DISTANT_BOX below are picked by eye "
                              "against Talon's Junction's own forward-facing-tunnel "
                              "framing - run --dump-regions on a pair from any other "
                              "circuit before trusting a region's numbers, since a "
                              "box may land on sky or wall there instead.")
    parser.add_argument("--game", default=str(ROOT / "target" / "debug" / "oag-game"))
    parser.add_argument("--dump-regions", action="store_true",
                         help="write an <NN>-regions.png mask overlay per pose")
    parser.add_argument("--bloom", choices=["on", "off"], default="on",
                         help="[graphics] bloom for our render; default on - "
                              "see render()'s own doc for why that, not the "
                              "player-facing default, is the faithful setting")
    parser.add_argument("--tiles", metavar="ROWSxCOLS",
                         help="print a box-free RxC tile-grid breakdown "
                              "(luma/hue per tile, valid region only) - for "
                              "a circuit where the hand-picked sky/road/"
                              "distant boxes are not trusted, e.g. "
                              "--tiles 4x6")
    args = parser.parse_args()

    game = pathlib.Path(args.game)
    if not game.exists():
        raise SystemExit(f"{game} is missing - cargo build -p oag-game")
    if not IMAGE.exists():
        raise SystemExit(f"{IMAGE} is missing - see data/README.md")

    tiles = None
    if args.tiles:
        try:
            rows, cols = (int(v) for v in args.tiles.lower().split("x"))
        except ValueError:
            raise SystemExit(f"--tiles wants ROWSxCOLS, e.g. 4x6, got {args.tiles!r}")
        tiles = (rows, cols)

    pair_dir = pathlib.Path(args.pair_dir).resolve()
    out_dir = pair_dir.parent / f"{pair_dir.name}-compare"
    poses = args.poses or ["00", "01", "03"]
    with tempfile.TemporaryDirectory() as tmp:
        cfg_root = pathlib.Path(tmp)
        for pose in poses:
            compare_one(game, pair_dir, out_dir, pose, args.dump_regions,
                        args.bloom == "on", cfg_root, tiles=tiles)


if __name__ == "__main__":
    main()
