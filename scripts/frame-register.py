#!/usr/bin/env python3
"""Measure the screen-space scale between two frames, without any threshold.

This is the instrument for comparing our render against a frame of the original.
Every threshold, hue and saturation rule tried before it failed, each for its own
reason, because our frame is darker with no bright pass and the two renderers'
bloom differs at low frequency - so no global mask rule separates a craft from
the track in both frames at once.

What works instead: the same mesh with the same texture carries the same
*internal* pattern in both frames - panel lines, intake grilles, a canopy edge.
High-pass it (a bloom difference is low-frequency; panel lines are not), take the
gradient magnitude (so a patch that is dark-on-light in one frame and
light-on-dark in the other still matches), and find the uniform zoom about the
image centre that maximises masked normalised cross-correlation. There is **no
brightness threshold anywhere in it**.

Pointed at a box containing only background, the same routine says whether the
*background* scales - which is how "our mesh is the wrong size" is told apart
from "our projection is the wrong shape". Far scenery is at effectively infinite
distance, so no mesh scale can move it: it is a free control, and running it is
not optional. See `docs/rendering/projection-vs-the-original.md`, which records
what happened when nobody did.

Validated to 0.26 % against known resize factors over a 1.8x range, and to 0.39 %
for linearity over a 1.43x range induced with `oag-game --camera-fov`.

Three things will silently give you a wrong number, and each has a cheap check:

1. **A box containing the HUD.** The HUD cannot scale, so it drags the answer
   toward 1.000 - the answer that looks like "no problem here". One 35-pixel
   strip of text once moved a 380x230 box from 1.14 to 1.01. `--exclude` it.
2. **A peak at an end of the sweep.** Guarded: this script exits rather than
   returning it. See `check_not_clamped`.
3. **A band with no structure to correlate.** Run the same box at two `--range`
   steps (0.01 and 0.005). A clean band moves by ~0.1 %; a band that moves by
   more than the 0.26 % validation figure is telling you it is not measurable,
   and no amount of re-running will sharpen it. Step sensitivity is peak
   sharpness, and it costs one extra invocation.

And when a band disagrees with its neighbours, **crop both frames and look at
them**. All three of the above were invisible in the correlation curve and
obvious in the picture.

Usage:
    frame-register.py REF MOV --box X0,Y0,X1,Y1 [--exclude X0,Y0,X1,Y1 ...]

Needs numpy and pillow only. Run it as
`uv run --with numpy --with pillow scripts/frame-register.py ...` if they are not
importable.
"""

import argparse
import sys

import numpy as np
from PIL import Image


def luma(path):
    im = Image.open(path).convert("RGB")
    a = np.asarray(im, dtype=np.float32)
    return 0.299 * a[..., 0] + 0.587 * a[..., 1] + 0.114 * a[..., 2]


def gaussian(a, sigma):
    """Separable Gaussian blur, edge-replicated.

    Written out rather than taken from PIL because PIL's filter wants 8-bit and
    the quantisation shows up in the correlation peak.
    """
    r = max(1, int(3 * sigma))
    x = np.arange(-r, r + 1, dtype=np.float32)
    k = np.exp(-0.5 * (x / sigma) ** 2)
    k /= k.sum()
    pad = np.pad(a, ((0, 0), (r, r)), mode="edge")
    out = np.apply_along_axis(lambda m: np.convolve(m, k, mode="valid"), 1, pad)
    pad = np.pad(out, ((r, r), (0, 0)), mode="edge")
    return np.apply_along_axis(lambda m: np.convolve(m, k, mode="valid"), 0, pad)


def feature(lum, sigma=6.0, kind="grad"):
    """Brightness-insensitive feature image."""
    hp = lum - gaussian(lum, sigma)
    if kind == "hp":
        return hp
    gy, gx = np.gradient(hp)
    return np.sqrt(gx * gx + gy * gy)


def resample(lum, s, center):
    """Zoom `lum` by `s` about `center`, same output size - a pure screen zoom.

    `s` may be a pair `(sx, sy)`, which is how an anisotropic (aspect) error is
    told apart from an isotropic (field-of-view) one.
    """
    sx, sy = s if isinstance(s, (tuple, list)) else (s, s)
    h, w = lum.shape
    cx, cy = center
    im = Image.fromarray(lum)
    # An affine mapping output (x,y) -> input (cx + (x-cx)/s, cy + (y-cy)/s).
    coeffs = (1.0 / sx, 0.0, cx - cx / sx, 0.0, 1.0 / sy, cy - cy / sy)
    out = im.transform((w, h), Image.AFFINE, coeffs, resample=Image.BICUBIC)
    return np.asarray(out, dtype=np.float32)


def ncc_map(template, tmask, image):
    """Masked normalised cross-correlation of `template` over `image`, via FFT."""
    shape = image.shape
    n = tmask.sum()
    tmean = (template * tmask).sum() / n
    t = (template - tmean) * tmask
    tnorm = np.sqrt((t * t).sum())

    f_image = np.fft.rfft2(image, s=shape)
    f_image2 = np.fft.rfft2(image * image, s=shape)
    f_t = np.fft.rfft2(t[::-1, ::-1], s=shape)
    f_mask = np.fft.rfft2(tmask[::-1, ::-1], s=shape)

    num = np.fft.irfft2(f_image * f_t, s=shape)
    sm = np.fft.irfft2(f_image * f_mask, s=shape)
    sm2 = np.fft.irfft2(f_image2 * f_mask, s=shape)
    var = sm2 - sm * sm / n
    var[var < 1e-6] = 1e-6
    return num / (np.sqrt(var) * tnorm)


def peak_shift(corr, search):
    """Best shift within +-`search` px, folding the circular correlation."""
    h, w = corr.shape
    ys = np.concatenate([np.arange(0, search + 1), np.arange(h - search, h)])
    xs = np.concatenate([np.arange(0, search + 1), np.arange(w - search, w)])
    sub = corr[np.ix_(ys, xs)]
    k = np.unravel_index(np.argmax(sub), sub.shape)
    dy = ys[k[0]] - (h if ys[k[0]] > search else 0)
    dx = xs[k[1]] - (w if xs[k[1]] > search else 0)
    return float(sub[k]), int(dx), int(dy)


def refine(xs, ys):
    """Sub-step peak, by fitting a parabola to the three points around the max."""
    i = int(np.argmax(ys))
    if i in (0, len(ys) - 1):
        return xs[i]
    d = ys[i - 1] - 2 * ys[i] + ys[i + 1]
    if abs(d) < 1e-12:
        return xs[i]
    step = xs[i + 1] - xs[i]
    return xs[i] - 0.5 * step * (ys[i + 1] - ys[i - 1]) / d


def run(ref_path, mov_path, box, excludes, scales, center, search, kind, quiet=False):
    ref_f = feature(luma(ref_path), kind=kind)
    mov_l = luma(mov_path)

    mask = np.zeros_like(ref_f)
    x0, y0, x1, y1 = box
    mask[y0:y1, x0:x1] = 1.0
    for a, b, c, d in excludes:
        mask[b:d, a:c] = 0.0
    if mask.sum() == 0:
        sys.exit("the mask is empty: --box and --exclude leave no pixels")

    peaks = []
    for s in scales:
        corr = ncc_map(ref_f, mask, feature(resample(mov_l, s, center), kind=kind))
        ncc, dx, dy = peak_shift(corr, search)
        peaks.append((float(s), ncc, dx, dy))
        if not quiet:
            print(f"  s={s:6.3f}  ncc={ncc:.4f}  dx={dx:+4d} dy={dy:+4d}")

    ss = np.array([p[0] for p in peaks])
    nn = np.array([p[1] for p in peaks])
    return refine(ss, nn), peaks


def check_not_clamped(peaks):
    """Refuse a peak sitting at an end of the sweep.

    This is not a nicety. A sweep capped at 1.25 once returned exactly `1.2500`
    for the three fastest ticks of a capture and the numbers were written up as
    measurements - they were the range's own edge. A clamped peak is not a
    measurement, so it is an error rather than a warning.
    """
    nn = [p[1] for p in peaks]
    i = int(np.argmax(nn))
    if len(peaks) > 1 and i in (0, len(peaks) - 1):
        lo, hi = peaks[0][0], peaks[-1][0]
        sys.exit(
            f"peak is at the {'low' if i == 0 else 'high'} end of the sweep "
            f"({peaks[i][0]:.4f} in [{lo:.4f}, {hi:.4f}]) - the answer is outside "
            f"the range and this number would be the range's edge, not a "
            f"measurement. Widen --range and re-run."
        )


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("ref", help="the reference frame - the original's")
    ap.add_argument("mov", help="the frame to measure - ours")
    ap.add_argument("--box", required=True, help="X0,Y0,X1,Y1 region to register")
    ap.add_argument("--exclude", action="append", default=[], help="X0,Y0,X1,Y1")
    ap.add_argument("--range", default="0.60,1.40,0.02", help="LO,HI,STEP zoom sweep")
    ap.add_argument("--center", default="", help="CX,CY; defaults to the image centre")
    ap.add_argument("--search", type=int, default=60, help="max shift, px")
    ap.add_argument("--kind", default="grad", choices=("grad", "hp"))
    ap.add_argument("--label", default="")
    ap.add_argument(
        "--allow-clamped",
        action="store_true",
        help="do not fail when the peak is at an end of the sweep (it is not a "
        "measurement; this exists only for deliberate one-sided probes)",
    )
    a = ap.parse_args()

    box = tuple(int(v) for v in a.box.split(","))
    ex = [tuple(int(v) for v in e.split(",")) for e in a.exclude]
    lo, hi, step = (float(v) for v in a.range.split(","))
    scales = np.arange(lo, hi + 1e-9, step)
    if a.center:
        center = tuple(float(v) for v in a.center.split(","))
    else:
        h, w = luma(a.ref).shape
        center = (w / 2.0, h / 2.0)

    print(f"== {a.label or a.mov} onto {a.ref}  box={box} kind={a.kind}")
    s, peaks = run(a.ref, a.mov, box, ex, scales, center, a.search, a.kind)
    if not a.allow_clamped:
        check_not_clamped(peaks)
    print(f"  peak zoom applied to MOV: {s:.4f}   =>  MOV/REF size ratio {1 / s:.4f}")


if __name__ == "__main__":
    main()
