#!/usr/bin/env python3
"""Read a PS3 `.psarc` archive, in place inside a decrypted PS3 disc image.

  psarc.py info    <image>[:<archive>]           header, and a census by extension
  psarc.py list    <image>[:<archive>]           one line per entry: size, path
  psarc.py verify  <image>[:<archive>]           check the directory against itself
  psarc.py cat     <image>:<archive> <path>      one entry to stdout
  psarc.py extract <image>:<archive> <outdir> [substring ...]

`<archive>` is a path inside the image (`PS3_GAME/USRDIR/DATA00.PSARC`) or a
loose `.psarc` file on disk. Omit it and every `.psarc` on the image is read,
which is what produces the census the format page quotes.

Nothing is extracted to read a table of contents: the archive is decompressed
block by block straight out of the image, at the LBA the ISO 9660 walk reports.
The image must already be layer-1 decrypted - see scripts/ps3iso.py and
docs/formats/ps3-disc.md - or the header read will not find `PSAR`.
"""

import hashlib
import os
import struct
import sys
import zlib

SECTOR = 2048
HEADER_LEN = 32
ENTRY_LEN = 30

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ps3iso import walk_iso9660  # noqa: E402


class Reader:
    """One `.psarc`, read through a file object at a byte offset."""

    def __init__(self, f, base, label):
        self.f = f
        self.base = base
        self.label = label
        f.seek(base)
        head = f.read(HEADER_LEN)
        if head[:4] != b"PSAR":
            raise ValueError(f"{label}: not a PSARC (magic {head[:4]!r})")
        self.version = (
            struct.unpack(">H", head[4:6])[0],
            struct.unpack(">H", head[6:8])[0],
        )
        self.compression = head[8:12].decode("ascii")
        toc_len, entry_len, count, self.block_size, self.flags = struct.unpack(
            ">IIIII", head[12:32]
        )
        if entry_len != ENTRY_LEN:
            raise ValueError(f"{label}: entry size {entry_len}, expected {ENTRY_LEN}")
        toc = f.read(toc_len - HEADER_LEN)
        self.entries = []
        for i in range(count):
            e = toc[i * entry_len : (i + 1) * entry_len]
            self.entries.append(
                {
                    "md5": e[0:16],
                    "block": int.from_bytes(e[16:20], "big"),
                    "size": int.from_bytes(e[20:25], "big"),
                    "offset": int.from_bytes(e[25:30], "big"),
                }
            )
        self.blocks = self._read_block_table(toc[count * entry_len :])
        self.paths = self._read_manifest()

    def _read_block_table(self, rest):
        """Block sizes, whose width is whatever divides the table evenly."""
        needed = max(e["block"] for e in self.entries) if self.entries else 0
        for width in (2, 3, 4):
            if len(rest) % width == 0 and len(rest) // width >= needed:
                return [
                    int.from_bytes(rest[i : i + width], "big")
                    for i in range(0, len(rest), width)
                ]
        raise ValueError(f"{self.label}: no block-size width divides the table")

    def read(self, index):
        """Entry `index` inflated. Index 0 is the manifest, not a file."""
        e = self.entries[index]
        out = bytearray()
        self.f.seek(self.base + e["offset"])
        block = e["block"]
        while len(out) < e["size"]:
            n = self.blocks[block]
            block += 1
            if n == 0:
                out += self.f.read(self.block_size)
                continue
            chunk = self.f.read(n)
            # A block is stored raw whenever deflating it would not have paid.
            out += zlib.decompress(chunk) if chunk[:1] == b"\x78" else chunk
        return bytes(out[: e["size"]])

    def _read_manifest(self):
        text = self.read(0).decode("utf-8", "replace")
        return [line for line in text.replace("\r", "").replace("\0", "\n").split("\n") if line.strip()]

    def files(self):
        """(path, size, index) per real entry, the manifest excluded."""
        for i, path in enumerate(self.paths):
            yield path, self.entries[i + 1]["size"], i + 1

    def find(self, path):
        want = path.lstrip("/").lower()
        for p, _size, index in self.files():
            if p.lstrip("/").lower() == want:
                return index
        return None


def open_archives(spec):
    """Yield Readers for `spec`, which names an image, an image:path, or a file."""
    image, _, inner = spec.partition(":")
    if not os.path.exists(image):
        raise SystemExit(f"no such image: {image}")
    if image.lower().endswith(".psarc"):
        f = open(image, "rb")
        yield Reader(f, 0, os.path.basename(image))
        return
    f = open(image, "rb")
    found = False
    for path, lba, _size in sorted(walk_iso9660(f)):
        if not path.lower().endswith(".psarc"):
            continue
        if inner and path.lower() != inner.lstrip("/").lower():
            continue
        found = True
        yield Reader(f, lba * SECTOR, path)
    if not found:
        raise SystemExit(f"no .psarc in {image}" + (f" matching {inner}" if inner else ""))


def cmd_info(spec):
    total = {}
    for arc in open_archives(spec):
        census = {}
        for path, size, _ in arc.files():
            ext = os.path.splitext(path)[1].lower() or "(none)"
            count, byts = census.get(ext, (0, 0))
            census[ext] = (count + 1, byts + size)
            count, byts = total.get(ext, (0, 0))
            total[ext] = (count + 1, byts + size)
        print(
            f"{arc.label}  PSAR {arc.version[0]}.{arc.version[1]}  "
            f"{arc.compression}  block {arc.block_size}  "
            f"flags {arc.flags}  {len(arc.paths)} entries"
        )
        for ext, (count, byts) in sorted(census.items(), key=lambda kv: -kv[1][0]):
            print(f"  {count:7}  {byts / 1048576:10.1f} MiB  {ext}")
    if len(total) and spec.find(":") < 0:
        print("\nwhole image")
        for ext, (count, byts) in sorted(total.items(), key=lambda kv: -kv[1][0]):
            print(f"  {count:7}  {byts / 1048576:10.1f} MiB  {ext}")


def cmd_list(spec):
    for arc in open_archives(spec):
        for path, size, _ in arc.files():
            print(f"{size:12}  {arc.label}  {path}")


def cmd_verify(spec):
    """The directory predicting itself: each entry's digest against its own path.

    This is the check the format page's confidence rests on. The 16-byte field
    is `MD5(path.upper())`, so it ties the manifest text, the entry ordering and
    the entry stride together - a parse one field out fails on entry one.
    """
    total = digests = archives = counted = 0
    for arc in open_archives(spec):
        archives += 1
        counted += len(arc.paths) == len(arc.entries) - 1
        for path, _size, index in arc.files():
            total += 1
            digests += (
                hashlib.md5(path.upper().encode()).digest() == arc.entries[index]["md5"]
            )
        print(
            f"{arc.label}: {len(arc.paths)} manifest paths for "
            f"{len(arc.entries) - 1} entries"
        )
    print(
        f"{counted} of {archives} archives name every entry but the manifest; "
        f"{digests} of {total} entries carry MD5(uppercased path)"
    )
    return 0 if digests == total and counted == archives else 1


def cmd_cat(spec, path):
    for arc in open_archives(spec):
        index = arc.find(path)
        if index is not None:
            sys.stdout.buffer.write(arc.read(index))
            return 0
    raise SystemExit(f"no entry named {path}")


def cmd_extract(spec, outdir, patterns):
    written = 0
    for arc in open_archives(spec):
        for path, _size, index in arc.files():
            if patterns and not any(p in path for p in patterns):
                continue
            dest = os.path.join(outdir, path.lstrip("/"))
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            data = arc.read(index)
            with open(dest, "wb") as out:
                out.write(data)
            print(f"{len(data):12}  {path}")
            written += 1
    print(f"{written} file(s) to {outdir}")


def main(argv):
    if len(argv) < 3:
        print(__doc__.strip())
        return 2
    cmd = argv[1]
    if cmd == "info":
        return cmd_info(argv[2])
    if cmd == "list":
        return cmd_list(argv[2])
    if cmd == "verify":
        return cmd_verify(argv[2])
    if cmd == "cat" and len(argv) >= 4:
        return cmd_cat(argv[2], argv[3])
    if cmd == "extract" and len(argv) >= 4:
        return cmd_extract(argv[2], argv[3], argv[4:])
    print(__doc__.strip())
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv) or 0)
