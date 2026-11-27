#!/usr/bin/env python3
"""How many mip levels every texture on the PS2 Pulse disc authors.

    for w in WADS2 WADSP; do
      just unpack extract data/images/pulse-ps2-eu.chd 54748/$w.WAD --out <dir>/disc
      just wad extract <dir>/disc/54748/$w.WAD --out <dir>/blobs/$w
    done
    scripts/ps2-texture-level-census.py <dir>/blobs target/debug/oag-wad

A texture object is found at any offset by the `A+D` address words of its first upload packet (`TRXPOS` 0x51,
`TRXREG` 0x52, `TRXDIR` 0x53 at +0x95/+0xa5/+0xb5 from its header), which sit there whatever its level count,
so an object with levels or one embedded in a larger resource is not skipped. Nested WADs are expanded first.
The header fields are the ones `Texture_ReadFromStream` reads
(`docs/ghidra/functions/ps2-pulse-eu/texture-level.md`), and each is checked against the dimensions.
`docs/rendering/texture-level-per-title.md`, "Pulse PS2 in a race", records the result.
"""
import argparse
import collections
import struct
import subprocess
import sys
from pathlib import Path

HEADER_BYTES = 0x8D
FIRST_UPLOAD = ((0x95, 0x51), (0xA5, 0x52), (0xB5, 0x53))


def is_wad(path):
    head = path.open("rb").read(8)
    if len(head) < 8:
        return False
    version, count = struct.unpack("<II", head)
    return version == 1 and 0 < count < 20000 and 8 + 16 * count <= path.stat().st_size


def expand(root, oag_wad, counts):
    for blob in sorted(root.iterdir()):
        if blob.is_file() and is_wad(blob):
            out = blob.with_suffix(".d")
            if not out.exists():
                result = subprocess.run([oag_wad, "extract", str(blob), "--out", str(out)], capture_output=True)
                if result.returncode:
                    counts["nested WADs that failed to extract"] += 1
                    continue
            counts["nested WADs"] += 1
            expand(out, oag_wad, counts)


def texture_objects(data):
    """Offsets of every serialized texture header in `data`."""
    first = FIRST_UPLOAD[0][1].to_bytes(8, "little")
    at = data.find(first)
    while at != -1:
        p = at - FIRST_UPLOAD[0][0]
        if p >= 0 and all(data[p + off:p + off + 8] == reg.to_bytes(8, "little") for off, reg in FIRST_UPLOAD):
            packed, bpp = data[p], data[p + 1]
            height, width = struct.unpack_from("<HH", data, p + 4)
            # the CLUT packet carries the same three words, so require the header to agree with itself
            if bpp in (4, 8) and (1 << (packed >> 4), 1 << (packed & 15)) == (height, width):
                yield p
        at = data.find(first, at + 1)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("blobs", type=Path, help="directory `oag-wad extract` wrote the top-level blobs under")
    ap.add_argument("oag_wad", help="path of the oag-wad binary, for the nested WADs")
    args = ap.parse_args()

    counts = collections.Counter()
    for top in sorted(p for p in args.blobs.iterdir() if p.is_dir()):
        expand(top, args.oag_wad, counts)
    levels, flags_seen, failed = collections.Counter(), collections.Counter(), collections.Counter()
    for path in (p for p in args.blobs.rglob("*") if p.is_file()):
        data = path.read_bytes()
        counts["files"] += 1
        for p in texture_objects(data):
            _, bpp, flags, clut_bits = data[p:p + 4]
            height, width = struct.unpack_from("<HH", data, p + 4)
            counts["texture objects"] += 1
            counts["texture objects at offset 0 of a file" if p == 0 else "texture objects inside a larger file"] += 1
            levels[flags & 0x1F] += 1
            flags_seen[(flags, clut_bits)] += 1

            blocks = struct.unpack_from("<9H", data, p + 0x08)
            sizes = struct.unpack_from("<9I", data, p + 0x69)
            texels, palette = width * height * bpp // 8, (1 << bpp) * 4
            base = max(1, width * height * bpp >> 11)
            clut = clut_bits >> 3 if bpp == 8 else 1
            checks = {
                "level block offsets are (0, base, base + CLUT, 0...)":
                    blocks == (0, base, base + clut) + (0,) * 6,
                "packet sizes are (4 + texels / 16, 4 + palette / 16, 0...)":
                    sizes == (4 + texels // 16, 4 + palette // 16) + (0,) * 7,
            }
            for name, ok in checks.items():
                failed[name] += not ok
            if p == 0:
                whole = HEADER_BYTES + 128 + max(256, texels) + max(256, palette)
                counts["files that are one texture exactly" if len(data) == whole else
                       "files that start with a texture and are longer"] += 1

    print(dict(counts))
    print("(flags byte +2, CLUT bits +3):", {"%#04x,%d" % k: v for k, v in sorted(flags_seen.items())})
    print("mip levels beyond the base (flags & 0x1f):", dict(levels))
    for name, n in failed.items():
        print("objects failing '%s': %d" % (name, n))


if __name__ == "__main__":
    sys.exit(main())
