#!/usr/bin/env python3
"""Decode mip 0 of a raw DXT5 (RSX format 0x88) texture to RGBA PNGs and print alpha statistics.

    dxt5-decode.py <raw.bin> <width> <height> <out-prefix>

Writes <prefix>-rgb.png and <prefix>-a.png. Needs numpy and Pillow (`uv run --with numpy --with pillow`).
"""
import sys
import numpy as np
from PIL import Image

raw = np.frombuffer(open(sys.argv[1], "rb").read(), dtype=np.uint8)
w, h = int(sys.argv[2]), int(sys.argv[3])
bw, bh = w // 4, h // 4
blocks = raw[: bw * bh * 16].reshape(bh, bw, 16)

a0 = blocks[..., 0].astype(np.int32)
a1 = blocks[..., 1].astype(np.int32)
bits = np.zeros((bh, bw), dtype=np.uint64)
for i in range(6):
    bits |= blocks[..., 2 + i].astype(np.uint64) << np.uint64(8 * i)
pal = np.zeros((bh, bw, 8), dtype=np.int32)
pal[..., 0] = a0
pal[..., 1] = a1
gt = a0 > a1
for k in range(2, 8):
    pal[..., k] = np.where(gt, ((8 - k) * a0 + (k - 1) * a1) // 7,
                           np.where(k < 6, ((6 - k) * a0 + (k - 1) * a1) // 5, 0 if k == 6 else 255))
alpha = np.zeros((h, w), dtype=np.uint8)
for p in range(16):
    idx = ((bits >> np.uint64(3 * p)) & np.uint64(7)).astype(np.int64)
    val = np.take_along_axis(pal, idx[..., None], axis=2)[..., 0]
    py, px = divmod(p, 4)
    alpha[py::4, px::4] = val.astype(np.uint8)

def rgb565(v):
    r = ((v >> 11) & 31) * 255 // 31
    g = ((v >> 5) & 63) * 255 // 63
    b = (v & 31) * 255 // 31
    return np.stack([r, g, b], axis=-1).astype(np.int32)

c0 = blocks[..., 8].astype(np.int32) | (blocks[..., 9].astype(np.int32) << 8)
c1 = blocks[..., 10].astype(np.int32) | (blocks[..., 11].astype(np.int32) << 8)
rgb0, rgb1 = rgb565(c0), rgb565(c1)
cols = np.stack([rgb0, rgb1, (2 * rgb0 + rgb1) // 3, (rgb0 + 2 * rgb1) // 3], axis=2)  # DXT5 always 4-colour
cb = np.zeros((bh, bw), dtype=np.uint32)
for i in range(4):
    cb |= blocks[..., 12 + i].astype(np.uint32) << np.uint32(8 * i)
rgb = np.zeros((h, w, 3), dtype=np.uint8)
for p in range(16):
    idx = ((cb >> np.uint32(2 * p)) & 3).astype(np.int64)
    val = np.take_along_axis(cols, idx[..., None, None], axis=2)[:, :, 0, :]
    py, px = divmod(p, 4)
    rgb[py::4, px::4] = val.astype(np.uint8)
np.save(sys.argv[4] + "-a.npy", alpha)
np.save(sys.argv[4] + "-rgb.npy", rgb)
Image.fromarray(rgb).save(sys.argv[4] + "-rgb.png")
Image.fromarray(alpha).save(sys.argv[4] + "-a.png")
print("alpha mean %.1f min %d max %d; share <16: %.3f, share >240: %.3f" % (
    alpha.mean(), alpha.min(), alpha.max(), (alpha < 16).mean(), (alpha > 240).mean()))
