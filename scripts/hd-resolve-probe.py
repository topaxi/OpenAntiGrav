#!/usr/bin/env python3
"""Offline check: does the read exposure resolve close the tone gap?

`lane-hd-resolve-fill`'s own experiment for `renderer.md`'s "The exposure is
read" thread. It does **not** touch the renderer - `crates/render/` is another
lane's tonight - it takes an *already-rendered* `ours` frame from a
`hd-frame-compare.py --pair-dir` run and multiplies it, in Python, by the
circuit's own read exposure scale:

    scale = <Tone maximum brightness> - min(adapted * <Tone adaption boost>,
                                             <Tone darkening clamp>)

the same formula `oag_post::hd_bloom` already implements for a
circuit whose `.envsettings` authors the whole `HDR and Bloom` block. **Sol
2's own file does not** - it authors `Tone adaption boost` alone, so
`crates/raceplay/src/load/environment.rs::envsettings_bloom` returns `None`
and the whole chain (gate, blur, resolve) is skipped for that circuit and the
seven other Fury/DLC circuits sharing the same partial file (see `renderer.md`,
"The exposure resolve's Fury circuits carry the front end's own Tone triple").
A live read during a Sol 2 race (this session, `2026-09-13`) found the
settings singleton's Tone triple holding `(20.0, 3.0, 4.0)` regardless - the
disc's own front-end `.envsettings` (`fe.track.envsettings`,
`fe.fury.track.envsettings`) authors exactly that triple, and the registrar is
a persistent singleton (`renderer.md`'s own reading, confidence 82) that a
per-circuit file only overrides the keys it declares in - so the real engine
runs the exposure resolve on Sol 2 at the same coefficients Talon's Junction
authors explicitly, carried over rather than reset.

This script asks: if `oag_render` ran that same resolve on Sol 2 - instead of
skipping the chain outright - does the read scale move the frame toward the
reference?

**What this is not.** `ours`'s PNG is already whatever `caller_format` the
scene fell back to with `hd_bloom` absent - not a linear HDR buffer, so
"decode gamma, multiply, re-encode" is an approximation of the skipped
resolve's `saturate(scene * scale)` term, not a replay of it. The gamma
assumed (2.2, a plain power law, not sRGB's piecewise curve) is **chosen, not
measured, and carries no confidence score** - `renderer.md`'s own
`encoding_probe` leaves the domain question open on this project's existing
material-lighting gap, and this script does not re-litigate it. `adapted`
(the luminance-adaptation state the real chain converges to over several
frames of readback) is approximated here as the frame's own mean linear luma
over the non-HUD, non-craft region - a single-frame proxy for a multi-frame
lerp, also chosen rather than measured.

Usage:

    python3 scripts/hd-resolve-probe.py --pair-dir data/reference/hd-capture/sol2-matched 00 \\
        --boost 20 --clamp 3 --max 4
"""

import argparse
import importlib.util
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent


def _load_compare_module():
    """Import `hd-frame-compare.py` by path - its name is not a valid module."""
    spec = importlib.util.spec_from_file_location(
        "hd_frame_compare", ROOT / "scripts" / "hd-frame-compare.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def decode_gamma(arr_u8, gamma):
    """`arr_u8` (0..255) to an approximate linear float in 0..1."""
    return (arr_u8.astype(np.float64) / 255.0) ** gamma


def encode_gamma(arr_linear, gamma):
    """The inverse of `decode_gamma`, saturated and back to 0..255 uint8."""
    clipped = np.clip(arr_linear, 0.0, 1.0)
    return np.clip((clipped ** (1.0 / gamma)) * 255.0, 0, 255).astype(np.uint8)


def apply_resolve_scale(ours_arr, valid_mask, gamma, boost, clamp, max_brightness):
    """`saturate(scene * scale)`, `scale` from the read Tone triple.

    `adapted` is this frame's own mean linear luma over `valid_mask` - see the
    module docstring for why that is a proxy, not a measurement.
    """
    linear = decode_gamma(ours_arr, gamma)
    luma = 0.2126 * linear[..., 0] + 0.7152 * linear[..., 1] + 0.0722 * linear[..., 2]
    adapted = float(luma[valid_mask].mean())
    scale = max_brightness - min(boost * adapted, clamp)
    scaled = linear * scale
    return encode_gamma(scaled, gamma), adapted, scale


def report_region(name, mask, ours_arr, scaled_arr, ref_arr, luminance, region_stats):
    print(f"\n-- {name} --")
    for label, arr in (("ours (shipped)", ours_arr), ("ours (scale applied)", scaled_arr),
                        ("reference", ref_arr)):
        luma = luminance(arr)
        stats = region_stats(arr, luma, mask)
        if stats is None:
            print(f"  {label:<24} (empty)")
            continue
        print(
            f"  {label:<24} mean_luma={stats['mean_luma']:.3f}  "
            f"clipped={stats['clipped_pct']:.2f}%"
        )


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("pose", help="pose stem, e.g. 00")
    parser.add_argument("--pair-dir", required=True,
                         help="e.g. data/reference/hd-capture/sol2-matched")
    parser.add_argument("--compare-dir", default=None,
                         help="where the *-ours-bloom-*.png renders live; "
                              "default: <pair-dir>-compare")
    parser.add_argument("--bloom", choices=("on", "off"), default="on",
                         help="which existing ours render to read")
    parser.add_argument("--gamma", type=float, default=2.2,
                         help="chosen, not measured - see module docstring")
    parser.add_argument("--boost", type=float, required=True, help="Tone adaption boost")
    parser.add_argument("--clamp", type=float, required=True, help="Tone darkening clamp")
    parser.add_argument("--max", type=float, required=True, dest="max_brightness",
                         help="Tone maximum brightness")
    args = parser.parse_args()

    compare = _load_compare_module()

    pair_dir = Path(args.pair_dir)
    compare_dir = Path(args.compare_dir) if args.compare_dir else Path(
        str(pair_dir) + "-compare"
    )
    ref_path = pair_dir / f"{args.pose}.png"
    ours_path = compare_dir / f"{args.pose}-ours-bloom-{args.bloom}.png"
    if not ref_path.exists() or not ours_path.exists():
        sys.exit(f"missing pair: {ref_path} / {ours_path} - run hd-frame-compare.py first")

    ref_img = Image.open(ref_path).convert("RGB")
    ours_img = Image.open(ours_path).convert("RGB")
    if ref_img.size != ours_img.size:
        sys.exit(f"size mismatch: reference {ref_img.size} vs ours {ours_img.size}")

    ref_arr = np.asarray(ref_img)
    ours_arr = np.asarray(ours_img)
    shape = ref_arr.shape[:2]
    masks, _hud, _craft = compare.build_masks(shape)
    valid = masks["whole (excl HUD, craft)"]

    scaled_arr, adapted, scale = apply_resolve_scale(
        ours_arr, valid, args.gamma, args.boost, args.clamp, args.max_brightness
    )

    print(f"pair {pair_dir}, pose {args.pose}, bloom {args.bloom}")
    print(
        f"adapted (proxy, mean linear luma over 'whole excl HUD, craft') = {adapted:.4f}\n"
        f"scale = {args.max_brightness} - min({args.boost} * {adapted:.4f}, {args.clamp}) "
        f"= {scale:.4f}"
    )
    if scale == 1.0:
        print("scale is exactly the floor (max - clamp) - saturating, same as Talon's "
              "Junction's own pose-00 reading; applying it changes nothing.")

    for name in ("whole (excl HUD, craft)", "sky", "road surface", "distant geometry"):
        report_region(name, masks[name], ours_arr, scaled_arr, ref_arr,
                      compare.luminance, compare.region_stats)


if __name__ == "__main__":
    main()
