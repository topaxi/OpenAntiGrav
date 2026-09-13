#!/usr/bin/env python3
"""Per-material tone probe on the matched-camera pose 00 HD frame pair.

`scripts/hd-frame-compare.py` measured that ours is darker than the original
by -0.13 to -0.24 mean luma per region, and that the gap is not one global
offset, scale or curve (its rank-matched quantile probe, confidence 80 -
`renderer.md`'s "The matched-camera comparison"). The remaining lead is
per-material: 275 of Talon's Junction's 301 drawn materials take a lit
formula whose transfer behaviour was never checked material by material. This
script is that check: it segments pose 00 by material slot and reports each
slot's own ours-vs-original luma statistics, so a gap that clusters by
formula names the term instead of the whole frame naming only its average.

    scripts/hd-material-probe.py                    # renders and reports
    scripts/hd-material-probe.py --out table.tsv     # also writes the table
    scripts/hd-material-probe.py --game target/release/oag-game

# Only pose 00

The other two matched poses (`01`, `03`) were captured at 431-529 km/h and
carry the original's own speed-streak brightness ours cannot match - see
`hd-frame-compare.py`'s own module doc. Pose 00 (grid, stopped) is the only
one that isolates the lit material path from that confound, so it is the
only one this script reads.

# How a pixel is attributed to a material slot

`OAG_TINT_MATERIALS=1` (`crates/render/src/mesh/rcs/isolate.rs`) replaces
every material's picture with a flat, **unlit** colour keyed by its slot
ordinal (`isolate::tint`) - `mesh/rcs.rs` clears `GpuVertex::lit` for it, so
the palette entry would reach the frame byte-for-byte rather than shaded,
*if nothing else touched it* - see [`classify_pixels`] for why something
does. Paired with `OAG_OPAQUE_ONLY=1`, which drops every chunk whose material
blends or is cutout-tested, the tint render contains **only** opaque
geometry - each pixel is either background or one known material's colour.
That sidesteps two failure modes a plain tint render would not: a cutout
material's holes close under a flat opaque 1x1 texture, and a blended
material goes fully opaque, so either would claim pixels the real frame
shows background or another surface through. This script therefore only
ever attributes pixels to **opaque** material slots - `cargo run --example
hd_material_probe_dump` on its own prints the `cutout`/`blended` population
this leaves out too.

**Fog reaches the tint diagnostic.** `mesh.wgsl`'s `fs_main` calls
`fogged()` unconditionally, including on the unlit stand-in path
`OAG_TINT_MATERIALS` takes - so a tinted pixel is not its slot's flat colour,
it is that colour mixed toward the circuit's authored fog colour by however
fogged the pixel's own distance makes it. See [`classify_pixels`] for the
consequence and the fix.

`crates/render/examples/hd_material_probe_dump.rs` is the join key: for every
drawn slot, which list it draws in, its material's own archive path, and the
role bits `mesh::rcs::skin::roles` packed - lightmap-lit, ambient-fed or not,
sun-fed or not (folded into `emissive` where neither), and whether it adds a
second-texture glow. Grouping this script's per-slot deltas by those roles is
what can tell "one material is dark" from "every lightmapped material is
dark by the same amount".

# What this does NOT do

**No histogram fitting into the shader.** If a family's gap fits a curve,
that curve is a lead to verify against the disc's own `.rcsmaterial`
microcode or `.envsettings` table, never a constant to carry into
`mesh.wgsl` on its own say-so - see `CLAUDE.md`'s "Never invent what the
assets already author".

The 500-pixel floor, the fog-segment tolerance and the coverage threshold
below are chosen, not measured - they carry no confidence score, the same
rule `hd-frame-compare.py`'s own region boxes state for themselves.
"""

import argparse
import csv
import json
import os
import pathlib
import subprocess
import sys
import tempfile

import numpy as np
from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parent.parent
PAIR_DIR = ROOT / "data" / "reference" / "hd-capture" / "talons-matched"
IMAGE = ROOT / "data" / "images" / "hdfury-ps3-eu-dec.iso"
ARCHIVE = "PS3_GAME/USRDIR/DATA00.PSARC"
TRACK = "/data/environments/talons_junction/track.vex"
TEAM = "feisar_c1"

# Duplicated from `hd-frame-compare.py` rather than imported: that script is
# another lane's tonight, and both are small enough that duplication is
# cheaper than a merge conflict. Keep in sync by eye if either changes.
COMPARISON_ARGS = [
    "--size", "1280x720",
    "--render-scale", "100",
    "--msaa", "off",
    "--motion-blur", "off",
    "--shadows", "off",
    "--reconstruction", "off",
]
CLIP = 250
HUD_BOXES = [
    (0.00, 0.00, 0.15, 0.20),
    (0.27, 0.00, 0.73, 0.23),
    (0.81, 0.00, 1.00, 0.20),
    (0.00, 0.77, 0.29, 1.00),
    (0.70, 0.73, 1.00, 1.00),
]
CRAFT_BOX = (0.28, 0.42, 0.68, 1.00)

MIN_PIXELS = 500  # chosen, not measured - the task's own floor.
COVERAGE_FLOOR = 0.90  # chosen - below this, stop and read why before trusting a row.
TINT_TOLERANCE = 10  # perpendicular distance to a slot's fog segment, in 8-bit
# units. Chosen, not measured, and swept by eye (`--tolerance`): 4 (the pre-fog
# exact-match tolerance) covers only 44% of the valid region because almost
# every non-trivial fraction of Talon's Junction's own geometry sits far
# enough from the camera for fog to move its tint colour off the exact match;
# coverage climbs to 64/80/86/89% at 6/8/10/12 while the number of distinct
# slots meeting the pixel floor stops growing past 6, which is why the ceiling
# reads as "distant geometry the fog model cannot arbitrate" rather than
# "misclassification getting worse" - see the report for the sweep.


def tint_rgb(slot: int) -> tuple[int, int, int]:
    """`isolate::tint`'s own golden-ratio hue walk, reproduced exactly."""
    hue = ((slot * 0.618034) % 1.0) * 6.0
    sector = int(hue)
    f = hue - sector
    q, t = 1.0 - f, f
    r, g, b = {
        0: (1.0, t, 0.0),
        1: (q, 1.0, 0.0),
        2: (0.0, 1.0, t),
        3: (0.0, q, 1.0),
        4: (t, 0.0, 1.0),
    }.get(sector, (1.0, 0.0, q))
    return (int(r * 255), int(g * 255), int(b * 255))


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


def luminance(arr):
    r, g, b = arr[..., 0], arr[..., 1], arr[..., 2]
    return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0


def camera_pose_args(camera):
    eye = camera["eye"]
    fwd = camera["forward"]
    up = camera["up"]
    nums = ",".join(f"{v:.6f}" for v in (*eye, *fwd, *up))
    return [f"--camera-pose={nums}", f"--camera-fov={camera['fov_y_deg']:.6f}"]


def render(game, camera, out_path, cfg_root, tag, extra_env, bloom):
    """Renders one frame with `extra_env` on top of the comparison setting.

    `aspect` stays pinned to `"wide"` for every render this script makes,
    tint pass included: a different letterbox would place the tint and the
    lit frame's geometry at different pixel offsets, which corrupts every
    per-material statistic before any of them is read - see this file's own
    module doc, gate 1.
    """
    cfg = cfg_root / f"cfg-{tag}"
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
        "--track", TRACK,
        "--team", TEAM,
        *COMPARISON_ARGS,
        *camera_pose_args(camera),
        "--no-audio",
        "--screenshot", str(out_path),
    ]
    env = dict(os.environ, XDG_CONFIG_HOME=str(cfg), **extra_env)
    done = subprocess.run(cmd, capture_output=True, text=True, env=env)
    if done.returncode != 0 or not out_path.exists():
        tail = (done.stderr or done.stdout).strip().splitlines()[-8:]
        raise SystemExit(f"render failed ({tag}): {' '.join(cmd)}\n" + "\n".join(tail))


def dump_material_roles(game_dir: pathlib.Path) -> tuple[dict[int, dict], tuple[float, float, float] | None]:
    """Runs `hd_material_probe_dump` and parses its TSV into `{slot: row}`,
    plus the circuit's authored fog colour off the same `# fog_colour_linear`
    comment line, or `None` where the circuit authors no usable fog.

    Built beside `game` rather than assumed present: `cargo run --example`
    builds it on first use, same as any other `hd_*` scratch probe in
    `crates/render/examples/`.
    """
    example = game_dir / "examples" / "hd_material_probe_dump"
    if "release" in str(game_dir):
        cmd = ["cargo", "build", "--release", "-p", "oag-render", "--example", "hd_material_probe_dump"]
    else:
        cmd = ["cargo", "build", "-p", "oag-render", "--example", "hd_material_probe_dump"]
    subprocess.run(cmd, cwd=ROOT, check=True, capture_output=True, text=True)
    done = subprocess.run(
        [str(example), f"{IMAGE}:{ARCHIVE}", TRACK],
        cwd=ROOT, capture_output=True, text=True, check=True,
    )
    lines = done.stdout.splitlines()
    fog = None
    body = []
    for line in lines:
        if line.startswith("# fog_colour_linear"):
            parts = line.split("\t")[1:]
            if len(parts) == 3:
                fog = tuple(float(p) for p in parts)
        else:
            body.append(line)
    rows = {}
    reader = csv.DictReader(body, delimiter="\t")
    for row in reader:
        rows[int(row["slot"])] = row
    return rows, fog


def build_slot_palette(roles: dict[int, dict]) -> dict[tuple[int, int, int], int]:
    """`{tint RGB: slot}` for the **opaque** population only - see module doc.

    A collision (two opaque slots landing on the same 8-bit tint colour) drops
    both rather than guessing; `isolate::tint`'s own doc admits it is a hue
    walk, not a bijection, so this is checked rather than assumed clean.
    """
    by_colour: dict[tuple[int, int, int], list[int]] = {}
    for slot, row in roles.items():
        if row["list"] != "opaque":
            continue
        by_colour.setdefault(tint_rgb(slot), []).append(slot)
    collisions = {c: s for c, s in by_colour.items() if len(s) > 1}
    if collisions:
        print(
            f"warning: {len(collisions)} tint colour(s) shared by more than one "
            f"opaque slot, dropped from the palette: {collisions}",
            file=sys.stderr,
        )
    return {c: s[0] for c, s in by_colour.items() if len(s) == 1}


def fog_byte_anchor(fog_linear):
    """The circuit's fog colour, gamma-encoded to the byte domain a screenshot
    actually holds.

    **`fogged()` mixes in linear space** (`mesh_render::uniforms::Fog::colour`
    is documented `linear`), and the tinted stand-in path writes into an HD
    circuit's linear scene target, so a tinted pixel at fog weight `w` is
    `pow(clamp(k * mix(fog_linear, tint_linear, w), 0, 1), 1/2.2)` for some
    frame-wide exposure scalar `k` this script does not solve for -
    **`k = 1` is assumed**, calibrated once by eye against a near-camera,
    unclipped, non-zero channel (slot 317's blue channel matched its exact
    tint value at `k = 1`, pose 00 - see the report). That assumption is
    carried, not measured, hence no confidence score.
    """
    return tuple(255.0 * (max(0.0, min(1.0, c)) ** (1.0 / 2.2)) for c in fog_linear)


def classify_pixels(tint_arr, palette, valid_mask, fog_byte, tolerance=TINT_TOLERANCE):
    """Slot id per pixel, `-1` where no candidate segment comes close enough.

    **Fog reaches the tint diagnostic.** `fs_main` calls `fogged()`
    unconditionally, on every path including the unlit stand-in `lit == 0.0`
    takes under `OAG_TINT_MATERIALS` - confirmed by the pose-00 tint render
    itself: a material's flat colour drifts smoothly toward a single
    frame-wide colour as its distance from the camera grows (checked by eye
    against a vertical scan of the render), and that colour matches
    `fog_byte` (`fog_byte_anchor` above) at the frame's most-fogged points to
    within a few 8-bit units - see the report for the comparison. A plain
    exact-match or small-tolerance-to-nearest-colour classifier (this
    function's first version) therefore only ever classified the closest 30%
    of the frame; every material's own visible extent runs from the camera to
    the horizon, so that silently dropped exactly the distant, structural
    geometry the per-material investigation is most about.

    So a pixel is matched against the **segment** from a slot's own tint
    colour (`w = 1`, no fog) to `fog_byte` (`w = 0`, fully fogged) rather
    than against the tint colour alone - the true path bows slightly off that
    straight line (the mix is linear, the encode that follows it is not), so
    this is an approximation of the true curve, chosen for being one `argmin`
    over precomputed geometry rather than a second render pass or a per-pixel
    depth read. **Never snapped past `TINT_TOLERANCE`**: a pixel farther than
    that from every candidate segment stays unclassified.
    """
    h, w, _ = tint_arr.shape
    slot_map = np.full((h, w), -1, dtype=np.int32)
    slots = np.array(list(palette.values()), dtype=np.int32)
    a = np.array(list(palette.keys()), dtype=np.float64)  # (n, 3): w=1 endpoint
    b = np.array(fog_byte, dtype=np.float64)  # (3,): w=0 endpoint, shared
    seg = b[None, :] - a  # (n, 3)
    seg_len_sq = np.sum(seg * seg, axis=-1)
    seg_len_sq = np.where(seg_len_sq == 0, 1.0, seg_len_sq)  # a slot whose tint is fog_byte itself

    flat = tint_arr[valid_mask].astype(np.float64)  # (m, 3)
    best_dist = np.full(flat.shape[0], np.inf)
    best_slot = np.full(flat.shape[0], -1, dtype=np.int32)
    # Per candidate slot rather than one (m, n, 3) tensor: m is the whole
    # frame (up to ~900k valid pixels) and n is every opaque slot (up to a
    # few hundred), and the product is too large to hold at once. This is
    # `n` passes over `m` instead - the same total work, bounded memory.
    for i in range(a.shape[0]):
        delta = flat - a[i]  # (m, 3)
        t = np.clip(np.dot(delta, seg[i]) / seg_len_sq[i], 0.0, 1.0)
        nearest = a[i] + t[:, None] * seg[i]
        dist = np.linalg.norm(flat - nearest, axis=-1)
        better = dist < best_dist
        best_dist = np.where(better, dist, best_dist)
        best_slot = np.where(better, slots[i], best_slot)
    assigned = np.where(best_dist <= tolerance, best_slot, -1)
    slot_map[valid_mask] = assigned
    return slot_map


def fit_affine_and_power(ours, ref):
    """Same two fits as `hd-frame-compare.py`'s `encoding_probe`, unclipped."""
    if ours.size < 20:
        return None
    a, b = np.polyfit(ours, ref, 1)
    pred = a * ours + b
    ss_res = float(np.sum((ref - pred) ** 2))
    ss_tot = float(np.sum((ref - ref.mean()) ** 2))
    r2_affine = 1.0 - ss_res / ss_tot if ss_tot > 0 else float("nan")
    floor = 1.0 / 255.0
    lx, ly = np.log(np.maximum(ours, floor)), np.log(np.maximum(ref, floor))
    gamma, c = np.polyfit(lx, ly, 1)
    predl = gamma * lx + c
    ss_res_l = float(np.sum((ly - predl) ** 2))
    ss_tot_l = float(np.sum((ly - ly.mean()) ** 2))
    r2_gamma = 1.0 - ss_res_l / ss_tot_l if ss_tot_l > 0 else float("nan")
    return {
        "affine_a": float(a), "affine_b": float(b), "affine_r2": r2_affine,
        "gamma": float(gamma), "gamma_r2": r2_gamma,
    }


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--game", default=str(ROOT / "target" / "debug" / "oag-game"))
    parser.add_argument("--out", type=pathlib.Path, help="also write the per-slot table as TSV")
    parser.add_argument("--bloom", choices=["on", "off"], default="off",
                         help="[graphics] bloom for the lit render; default off - "
                              "bloom bleeds light across material boundaries, which "
                              "is exactly the cross-contamination this probe exists "
                              "to avoid. See hd-frame-compare.py for the on-by-default "
                              "whole-picture comparison instead.")
    parser.add_argument("--keep", type=pathlib.Path,
                         help="render into this directory instead of a scratch one, "
                              "and leave the tint/lit PNGs there afterwards - for "
                              "checking the segmentation by eye")
    parser.add_argument("--tolerance", type=float, default=TINT_TOLERANCE,
                         help="override the segment-distance tolerance, for probing "
                              "the coverage/precision trade-off")
    args = parser.parse_args()

    game = pathlib.Path(args.game)
    if not game.exists():
        raise SystemExit(f"{game} is missing - cargo build -p oag-game")
    if not IMAGE.exists():
        raise SystemExit(f"{IMAGE} is missing - see data/README.md")

    json_path = PAIR_DIR / "00.json"
    png_path = PAIR_DIR / "00.png"
    meta = json.loads(json_path.read_text())
    camera = meta["camera"]

    print("dumping material roles (cargo run --example hd_material_probe_dump)...", file=sys.stderr)
    roles, fog_linear = dump_material_roles(game.parent)
    palette = build_slot_palette(roles)
    print(f"{len(palette)} opaque slot(s) in the palette", file=sys.stderr)
    if fog_linear is None:
        raise SystemExit(
            "the circuit authors no usable fog colour - classify_pixels's "
            "segment model has no second endpoint to build from; fall back "
            "to plain nearest-colour matching (this script's first version, "
            "in git history) for a circuit like this"
        )
    fog_byte = fog_byte_anchor(fog_linear)
    print(
        f"fog colour: linear {tuple(round(c, 3) for c in fog_linear)}, "
        f"byte anchor (k=1) {tuple(round(c, 1) for c in fog_byte)}",
        file=sys.stderr,
    )

    def run(cfg_root: pathlib.Path):
        cfg_root.mkdir(parents=True, exist_ok=True)
        tint_path = cfg_root / "tint.png"
        lit_path = cfg_root / "lit.png"
        if tint_path.exists() and lit_path.exists() and args.keep:
            print(f"reusing renders already under {cfg_root} (--keep)", file=sys.stderr)
        else:
            print("rendering the tint pass (OAG_TINT_MATERIALS=1 OAG_OPAQUE_ONLY=1, bloom off)...",
                  file=sys.stderr)
            render(game, camera, tint_path, cfg_root, "tint",
                   {"OAG_TINT_MATERIALS": "1", "OAG_OPAQUE_ONLY": "1"}, bloom=False)
            print(f"rendering the lit pass (bloom {args.bloom})...", file=sys.stderr)
            render(game, camera, lit_path, cfg_root, "lit", {}, bloom=(args.bloom == "on"))
        return tint_path, lit_path

    if args.keep:
        tint_path, lit_path = run(args.keep)
        _classify_and_report(
            png_path, tint_path, lit_path, roles, palette, fog_byte, args
        )
    else:
        with tempfile.TemporaryDirectory() as tmp:
            tint_path, lit_path = run(pathlib.Path(tmp))
            _classify_and_report(
                png_path, tint_path, lit_path, roles, palette, fog_byte, args
            )


def _classify_and_report(png_path, tint_path, lit_path, roles, palette, fog_byte, args):
    ref_img = Image.open(png_path).convert("RGB")
    tint_img = Image.open(tint_path).convert("RGB")
    lit_img = Image.open(lit_path).convert("RGB")
    if not (ref_img.size == tint_img.size == lit_img.size):
        raise SystemExit(
            f"size mismatch: ref {ref_img.size}, tint {tint_img.size}, "
            f"lit {lit_img.size}"
        )

    ref_arr = np.asarray(ref_img)
    tint_arr = np.asarray(tint_img)
    lit_arr = np.asarray(lit_img)
    ref_luma = luminance(ref_arr)
    lit_luma = luminance(lit_arr)
    shape = ref_arr.shape[:2]

    hud = union(shape, HUD_BOXES)
    craft = box_mask(shape, CRAFT_BOX)
    valid = ~(hud | craft)

    slot_map = classify_pixels(tint_arr, palette, valid, fog_byte, args.tolerance)
    classified = slot_map >= 0
    coverage = classified.sum() / valid.sum()
    print(f"palette coverage of the valid region: {100 * coverage:.1f}%", file=sys.stderr)
    if coverage < COVERAGE_FLOOR:
        print(
            f"WARNING: coverage below the {100 * COVERAGE_FLOOR:.0f}% floor - "
            "read the per-slot table with that in mind; the unclassified "
            "remainder is background, non-opaque geometry (by design), or a "
            "tint pixel that did not land on a known palette entry",
            file=sys.stderr,
        )

    rows = []
    for slot in np.unique(slot_map[classified]):
        slot = int(slot)
        mask = slot_map == slot
        count = int(mask.sum())
        if count < MIN_PIXELS:
            continue
        ours = lit_luma[mask]
        ref = ref_luma[mask]
        clipped = np.all(ref_arr[mask] >= CLIP, axis=-1) | np.all(lit_arr[mask] >= CLIP, axis=-1)
        fit_input_ours, fit_input_ref = ours[~clipped], ref[~clipped]
        fit = fit_affine_and_power(fit_input_ours, fit_input_ref)
        row = roles.get(slot, {})
        rows.append({
            "slot": slot,
            "material": row.get("material", "?"),
            "roles": row.get("roles", "?"),
            "specular_exponent": row.get("specular_exponent", "?"),
            "count": count,
            "ours_mean": float(ours.mean()),
            "ref_mean": float(ref.mean()),
            "delta": float(ref.mean() - ours.mean()),
            "affine_a": fit["affine_a"] if fit else None,
            "affine_b": fit["affine_b"] if fit else None,
            "affine_r2": fit["affine_r2"] if fit else None,
            "gamma": fit["gamma"] if fit else None,
            "gamma_r2": fit["gamma_r2"] if fit else None,
        })
    rows.sort(key=lambda r: r["delta"])

    header = ["slot", "material", "roles", "specular_exponent", "count",
              "ours_mean", "ref_mean", "delta", "affine_a", "affine_b",
              "affine_r2", "gamma", "gamma_r2"]
    print("\n" + "\t".join(header))
    for row in rows:
        print("\t".join(
            f"{row[k]:.4f}" if isinstance(row[k], float) else str(row[k])
            for k in header
        ))

    if args.out:
        with open(args.out, "w", newline="") as f:
            writer = csv.DictWriter(f, fieldnames=header, delimiter="\t")
            writer.writeheader()
            writer.writerows(rows)
        print(f"\nwrote {args.out}", file=sys.stderr)

    print(f"\n{len(rows)} slot(s) with >= {MIN_PIXELS} px", file=sys.stderr)

    # Grouped-by-role summary: pixel-weighted mean delta per role bucket, the
    # direct answer to "does the gap cluster by formula".
    by_role: dict[str, list[tuple[int, float]]] = {}
    for row in rows:
        by_role.setdefault(row["roles"], []).append((row["count"], row["delta"]))
    print("\nby role (pixel-weighted mean delta = ref - ours):")
    for role, pairs in sorted(by_role.items(), key=lambda kv: -sum(c for c, _ in kv[1])):
        total = sum(c for c, _ in pairs)
        weighted = sum(c * d for c, d in pairs) / total
        print(f"  {role:<30} {len(pairs):>3} slot(s) {total:>9} px  mean delta {weighted:+.4f}")

    # `track_surface`'s 15 chunks are the shader's own stated exception - see
    # `mesh.wgsl`'s "sun_diffuse is applied to every chunk" comment. If this
    # probe cannot separate them from the 275-strong majority, it is not
    # sensitive enough to trust on an unknown cluster - check this first.
    track_surface = [r for r in rows if "track_surface" in r["material"]]
    rest = [r for r in rows if "track_surface" not in r["material"]]
    if track_surface and rest:
        ts_mean = sum(r["count"] * r["delta"] for r in track_surface) / sum(r["count"] for r in track_surface)
        rest_mean = sum(r["count"] * r["delta"] for r in rest) / sum(r["count"] for r in rest)
        print(
            f"\ninstrument check: track_surface (no sun/N.L of its own, shader "
            f"comment's stated exception) mean delta {ts_mean:+.4f} over "
            f"{len(track_surface)} slot(s), against {rest_mean:+.4f} over the "
            f"other {len(rest)} - expect track_surface measurably less negative "
            "(or positive) if the probe is sensitive to a known-different population"
        )
    elif not track_surface:
        print("\ninstrument check: no track_surface slot met the pixel floor at pose 00", file=sys.stderr)


if __name__ == "__main__":
    main()
