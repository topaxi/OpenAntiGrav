#!/usr/bin/env python3
"""Decompose a compare pair's error by what ours draws there.

    uv run --with numpy --with pillow --with scipy python3 scripts/compare-decompose.py \
        --pair data/reference/compare-page/omega/sol-2048 --work <scratch dir> \
        --binary target/debug/oag-game --road road_alphaspec [--needle crowd ...] [--tag NAME --env K=V ...]

The pair is copied into `<work>/<tag>/<pair>/` and re-rendered there with `compare-rerender-ours.rerender` (the
shared pair is never written). Each `--needle` (and `--road`, `sky`, the own hull `feisar2048`, `crowd`, `flag`) is a
render with `OAG_SKIP_MATERIAL=<needle>`; the pixels that change by more than 25 grey, after taking out the median
frame gain the adaptive exposure adds when a surface goes, are what that needle owns in ours.

Regions: `dynamic` (own hull, crowd, flags, dilated 6 px, plus the HUD boxes) is left out of every score, since
the livery and crowd vary boot to boot; `road` is the road family's pixels, `sky` the sky's, `scenery` the rest.
Per region it prints the area, `LF` (mean |blur12(ours) - blur12(theirs)| in grey, the low-frequency error a light,
tone or emissive cause would close), `bias` (mean ours - theirs) and `G` (Sobel gradient energy ours / theirs,
the detail axis: filtering, MIP choice, a missing detail layer). The raw mean grey is printed beside it, because
on these pairs it is dominated by pixel decorrelation (edge correlation about 0): correct detail a few pixels
out of phase scores worse there. `docs/formats/omega-status.md`, "Where the WipEout 2048 mode pairs' error is".
"""
import argparse
import importlib.util
import json
import os
import shutil
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy.ndimage import binary_dilation, gaussian_filter, sobel

ROOT = Path(__file__).resolve().parent.parent


def rerender_module():
    sys.path.insert(0, str(ROOT / "scripts"))
    spec = importlib.util.spec_from_file_location("compare_rerender_ours", ROOT / "scripts/compare-rerender-ours.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def render(module, pair_dir, work, tag, binary, env):
    dst = work / tag / pair_dir.name
    dst.mkdir(parents=True, exist_ok=True)
    for name in ("pair.json", "theirs.png"):
        shutil.copy(pair_dir / name, dst / name)
    saved = {k: os.environ.get(k) for k in env}
    os.environ.update(env)
    try:
        result = module.rerender(dst / "pair.json", binary)
    finally:
        for k, v in saved.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v
    png = dst / "ours.png"
    return (png if result.startswith("ok") and png.exists() else None), result


def grey(path):
    a = np.asarray(Image.open(path).convert("RGB"), dtype=np.float32)
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


def owned(base, skipped):
    """Pixels the skipped needle drew in `base`, with the frame-wide exposure change divided out."""
    k = grey(skipped)
    sel = (base > 20) & (base < 235)
    gain = np.median(k[sel] / base[sel])
    return np.abs(k - gain * base) > 25


def hud_boxes(h, w):
    m = np.zeros((h, w), bool)
    m[0:int(h * 0.22), 0:int(w * 0.26)] = True
    m[0:int(h * 0.20), int(w * 0.88):] = True
    m[int(h * 0.70):, 0:int(w * 0.09)] = True
    m[int(h * 0.85):, int(w * 0.68):] = True
    return m


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--pair", required=True, type=Path)
    ap.add_argument("--work", required=True, type=Path)
    ap.add_argument("--binary", required=True, type=Path)
    ap.add_argument("--road", required=True, help="OAG_SKIP_MATERIAL needle of the road family")
    ap.add_argument("--hull", default="feisar2048")
    ap.add_argument("--needle", action="append", default=[])
    ap.add_argument("--tag", default=None, help="score one more render under this tag, with --env set")
    ap.add_argument("--env", action="append", default=[], help="K=V for the --tag render")
    args = ap.parse_args()
    module = rerender_module()
    pair, work, binary = args.pair.resolve(), args.work.resolve(), args.binary.resolve()
    base_png, result = render(module, pair, work, "base", binary, {})
    if base_png is None:
        sys.exit("base render failed: " + result)
    base = grey(base_png)
    theirs = grey(pair / "theirs.png")
    h, w = base.shape
    masks = {}
    for needle in {args.road, "sky", args.hull, "crowd", "flag", *args.needle}:
        png, result = render(module, pair, work, "skip-" + needle.replace("/", "_"), binary, {"OAG_SKIP_MATERIAL": needle})
        masks[needle] = owned(base, png) if png else np.zeros((h, w), bool)
        if png is None:
            print(f"  {needle}: render failed ({result.splitlines()[0][:80]}); owns nothing here")
    dynamic = binary_dilation(masks[args.hull] | masks["crowd"] | masks["flag"], iterations=6) | hud_boxes(h, w)
    regions = {"road": masks[args.road] & ~dynamic, "sky": masks["sky"] & ~dynamic}
    regions["scenery"] = ~dynamic & ~regions["road"] & ~regions["sky"]
    regions["static"] = ~dynamic
    for needle in args.needle:
        regions[needle] = masks[needle] & ~dynamic
    renders = [("base", base_png)]
    if args.tag:
        env = dict(kv.split("=", 1) for kv in args.env)
        png, result = render(module, pair, work, args.tag, binary, env)
        if png is None:
            sys.exit(f"{args.tag} render failed: {result}")
        renders.append((args.tag, png))
    t_grad = np.hypot(sobel(theirs, 0), sobel(theirs, 1))
    t_low = gaussian_filter(theirs, 12)
    for tag, png in renders:
        ours = grey(png)
        o_grad = np.hypot(sobel(ours, 0), sobel(ours, 1))
        o_low = gaussian_filter(ours, 12)
        overlap = json.loads((png.parent / "pair.json").read_text()).get("overlap", {})
        print(f"{pair.name} {tag}: raw mean grey {overlap.get('mean_grey', float('nan')):.2f}, rmse {overlap.get('rmse', float('nan')):.3f}")
        static_lf = np.abs(o_low - t_low)[regions["static"]].sum()
        for name, m in regions.items():
            if not m.any():
                continue
            lf = np.abs(o_low - t_low)[m]
            print(f"  {name:10s} area {100 * m.mean():5.1f}%  LF {lf.mean():5.1f} ({100 * lf.sum() / static_lf:4.0f}% of static LF)"
                  f"  bias {(ours - theirs)[m].mean():+6.1f}  G {o_grad[m].mean() / t_grad[m].mean():4.2f}")


if __name__ == "__main__":
    main()
