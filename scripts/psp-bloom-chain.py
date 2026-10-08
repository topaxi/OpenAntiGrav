#!/usr/bin/env python3
"""Check Pulse PSP's bloom chain against the original's own scratch buffers.

`scripts/psp-trace.py --edram-every N --edram-dir DIR` writes, per recorded
tick, the two framebuffers (EDRAM `0x0` and `0x88000`, 512 x 272 x 4) and the
bloom's scratch buffers A (`0x110000`) and B (`0x132000`, 256 x 136 x 4). With
the composite poked off (`docs/ghidra/functions/psp-pulse-usa/bloom.md`, "Is
ours stronger than the original?" has the poke) the finished frame's
framebuffer is the pre-composite scene, its alpha the glow mask, and A and B
the chain's own output for that very frame. This script runs the recovered
chain on the framebuffer and reports how far it lands from A and B:

    uv run --with numpy scripts/psp-bloom-chain.py DIR/tick00240.edram [more ...]

Output is derived game data; write dumps under `data/`, never commit them.

The model is the one `crates/post/shaders/bloom.wesl` implements: the bright
pass is the mean of each 2 x 2 block's rgb times the mean of its alpha, the
blur is eleven taps along one axis, each `(v * (w + 1)) >> 8`, clamped at 255,
with zeros past the edge, and the composite adds `floor(bilinear(A) * 175 /
255)`.
"""

import sys

import numpy as np

WEIGHTS = [20, 30, 40, 50, 64, 64, 64, 50, 40, 30, 20]


def load(path):
    raw = open(path, "rb").read()
    big, small = 512 * 272 * 4, 256 * 136 * 4
    fb = lambda at: np.frombuffer(raw[at : at + big], np.uint8).reshape(272, 512, 4)[:, :480]
    scratch = lambda at: np.frombuffer(raw[at : at + small], np.uint8).reshape(136, 256, 4)[:, :240]
    return fb(0), fb(big), scratch(2 * big), scratch(2 * big + small)


def blur(x, axis):
    x = x.astype(np.int64)
    pad = [(5, 5) if i == axis else (0, 0) for i in range(x.ndim)]
    padded = np.pad(x, pad)
    n = x.shape[axis]
    total = 0
    for i, w in enumerate(WEIGHTS):
        total = np.minimum(total + ((np.take(padded, range(i, i + n), axis=axis) * (w + 1)) >> 8), 255)
    return total


def bright(rgb, alpha):
    rgb, alpha = rgb.astype(np.int64), alpha.astype(np.int64)[..., None]
    mean = lambda v: (v[0::2, 0::2] + v[1::2, 0::2] + v[0::2, 1::2] + v[1::2, 1::2]) // 4
    return (mean(rgb) * mean(alpha)) // 255


def main():
    for path in sys.argv[1:]:
        fb0, _, a, b = load(path)
        h = blur(bright(fb0[..., :3], fb0[..., 3]), 1)
        v = blur(h, 0)
        print(
            "%s: A mean original %.3f model %.3f; B mean original %.3f model %.3f; "
            "A exact %.1f %%"
            % (
                path,
                a[..., :3].mean(),
                v.mean(),
                b[..., :3].mean(),
                h.mean(),
                100 * (v == a[..., :3]).mean(),
            )
        )


if __name__ == "__main__":
    main()
