#!/usr/bin/env python3
"""Appends an alpha readout to a PCSX2 GS dump, so replaying it shows the frame's alpha.

    scripts/pcsx2-gsdump-alpha.py frame.gs --after-group 290 --fbp 0 out.gs
    pcsx2-qt -batch -nogui -- out.gs     # then F8 (screenshot): grey = alpha * 2

A GS dump holds the VRAM as it was when the dump began and the packets that
follow, so the alpha of the frame being drawn is in neither - it exists only
while the packets replay. This keeps the first field's packets up to the end of
state group `--after-group` (`scripts/pcsx2-gsdump.py`'s numbering), then adds
one full-frame sprite that blends white onto the frame buffer `--fbp` with
`ALPHA = (Cs - 0) * Ad`, colour writes on and alpha masked (`FBMSK` =
`0xff000000`). The frame's colour becomes `255 * alpha / 128`, so a replay of
the result is a picture of the alpha channel: `0x80`, the PS2's `1.0`, reads
`255`. Pass `--plain` to cut the dump at the same place without the readout,
which is how a before and an after of one pass are compared.

The first field's frame buffer is `FBP` 0 or 140 depending on the dump; the
frame is the first state group's `FRAME_1`. `docs/rendering/ps2-bloom.md` is
the measurement this was written for, and the traps: PCSX2 must run the
OpenGL renderer under Xvfb (Vulkan finds no present queue there, software
included), and a packet that starts a state group may also finish the one
before, so `--after-group` is found by decoding, not by counting packets.
"""
import argparse
import importlib.util
import os
import struct
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("gsdump", os.path.join(HERE, "pcsx2-gsdump.py"))
gsdump = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gsdump)

# GS register addresses used by the readout draw.
FRAME_1, ZBUF_1, TEST_1, ALPHA_1, COLCLAMP, DTHE, TEX0_1, PRIM, RGBAQ, XYZ2 = (
    0x4C, 0x4E, 0x47, 0x42, 0x46, 0x45, 0x06, 0x00, 0x01, 0x05)
# `ALPHA`: A = Cs (0), B = zero (2), C = Ad (1), D = zero (2).
ALPHA_SRC_TIMES_DEST_ALPHA = 0 | (2 << 2) | (1 << 4) | (2 << 6)
SPRITE_WITH_BLEND = 0x06 | 0x40


def packets(data, start):
    out = []
    i = start
    while i < len(data):
        kind = data[i]
        if kind == 0:
            size = struct.unpack_from("<I", data, i + 2)[0]
            out.append((0, i, i + 6 + size))
        elif kind == 1:
            out.append((1, i, i + 2))
        elif kind == 2:
            out.append((2, i, i + 5))
        else:
            out.append((3, i, i + 8193))
        i = out[-1][2]
    return out


def readout_transfer(fbp, left, right):
    regs = [
        (FRAME_1, fbp | (8 << 16) | (0xFF000000 << 32)),
        (ZBUF_1, 0x13A000118),
        (TEST_1, 0x30000),
        (ALPHA_1, ALPHA_SRC_TIMES_DEST_ALPHA),
        (COLCLAMP, 1),
        (DTHE, 0),
        (TEX0_1, 0),
        (PRIM, SPRITE_WITH_BLEND),
        (RGBAQ, (0x80 << 24) | 0xFFFFFF),
        (XYZ2, left | (left << 16)),
        (XYZ2, right | (right << 16)),
    ]
    body = b"".join(struct.pack("<QQ", value, address) for address, value in regs)
    tag = len(regs) | (1 << 15) | (1 << 60)  # NLOOP, EOP, PACKED, NREG 1
    gif = struct.pack("<QQ", tag, 0xE) + body  # REGS = A+D
    return bytes([0, 3]) + struct.pack("<I", len(gif)) + gif


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("dump")
    ap.add_argument("out")
    ap.add_argument("--after-group", type=int, required=True)
    ap.add_argument("--fbp", type=int, default=0, help="the frame buffer's FBP (0 or 140)")
    ap.add_argument("--plain", action="store_true", help="cut only, append no readout")
    ap.add_argument("--span", type=int, default=512, help="the frame's width and height in pixels")
    args = ap.parse_args()

    data = open(args.dump, "rb").read()
    header = struct.unpack_from("<I", data, 4)[0]
    state = struct.unpack_from("<I", data, 12)[0]
    start = 8 + header + state + 8192
    pk = packets(data, start)
    vsyncs = [k for k, p in enumerate(pk) if p[0] == 1]
    field_end = vsyncs[1]
    transfers = [k for k in range(field_end) if pk[k][0] == 0]

    def decode(n):
        keep = [k for k in range(field_end) if pk[k][0] != 0 or k <= transfers[n - 1]]
        with tempfile.NamedTemporaryFile(suffix=".gs", delete=False) as f:
            f.write(data[:start] + b"".join(data[pk[k][1]:pk[k][2]] for k in keep))
        try:
            return gsdump.run(f.name)
        finally:
            os.unlink(f.name)

    lo, hi = 1, len(transfers)
    while lo < hi:
        mid = (lo + hi) // 2
        if len(decode(mid).draws) > args.after_group + 1:
            hi = mid
        else:
            lo = mid + 1
    n = lo - 1
    groups = len(decode(n).draws)
    if groups != args.after_group + 1:
        sys.exit("no packet boundary falls after group %d (got %d groups)" % (args.after_group, groups))
    keep = [k for k in range(field_end) if pk[k][0] != 0 or k <= transfers[n - 1]]
    body = b"".join(data[pk[k][1]:pk[k][2]] for k in keep)
    centre = 2048 * 16
    half = (args.span // 2) * 16
    transfer = b"" if args.plain else readout_transfer(args.fbp, centre - half, centre + half)
    # The registers packet before the field's closing vsync, then the vsync.
    tail = b"".join(data[pk[k][1]:pk[k][2]] for k in (field_end - 1, field_end))
    with open(args.out, "wb") as f:
        f.write(data[:start] + body + transfer + tail)
    print("wrote %s: field 1 up to group %d%s" % (
        args.out, args.after_group, "" if args.plain else ", then the alpha readout"))


if __name__ == "__main__":
    main()
