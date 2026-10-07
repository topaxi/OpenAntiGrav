#!/usr/bin/env python3
"""Inspect / decrypt a PS3 BD-ROM image (redump-style .iso).

Layer 1 of PS3 disc protection only: the per-sector AES-128-CBC encryption of
the disc's "encrypted regions". It does not touch SELF/SPRX/EDAT (layer 2), so
an `EBOOT.BIN` this writes out is a readable SELF container and still encrypted
code. See docs/formats/ps3-disc.md.

  ps3iso.py map     <iso>                 region table + which file lands where
  ps3iso.py extract <iso> <outdir>        pull the files out (see below)
  ps3iso.py oracle  <iso> <keyhex>        test a candidate key against known magic
  ps3iso.py decrypt <iso> <keyhex> <out>  whole-image decrypt (needs a valid key)

`extract` writes every file if the image is already decrypted, and only the
files lying wholly in plain regions if it is not - the region table survives
decryption unchanged, so the two cases are told apart by the oracle rather than
by the table.

<keyhex> is 32 hex chars. It is tried both as the sector key directly and as
`data1` run through the documented secret, whichever the oracle accepts; a
redump `.dkey` is the former. No key is stored in this repository.
"""

import argparse
import os
import struct
import sys

try:
    from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes
except ImportError:
    sys.exit(
        "ps3iso.py needs the Python package `cryptography`. Either run it as\n"
        "  uv run --with cryptography python3 scripts/ps3iso.py ...\n"
        "or install it first: `pip install cryptography`."
    )

SECTOR = 2048
DATA1_SECRET = bytes.fromhex("380bcf0b53455b3c7817ab4fa3ba90ed")

# Magic bytes expected at the head of a file, keyed by extension. The oracle
# turns "is this key right" into one AES call against a sector we can predict.
MAGIC_BY_EXT = {
    ".PSARC": b"PSAR",
    ".PNG": b"\x89PNG",
    ".SPRX": b"SCE\x00",
    ".BIN": b"SCE\x00",
    ".PUP": b"SCEUF",
}


def read_regions(f):
    """Region table in sector 0. Returns [(first_lba, last_lba, encrypted)]."""
    f.seek(0)
    head = f.read(SECTOR)
    n_plain = struct.unpack(">I", head[0:4])[0]
    n_bounds = n_plain * 2
    bounds = list(struct.unpack(">%dI" % n_bounds, head[8 : 8 + 4 * n_bounds]))
    # bounds alternate: plain0_start, plain0_end, plain1_start, plain1_end, ...
    total = os.path.getsize(f.name) // SECTOR
    assert bounds == sorted(bounds) and len(set(bounds)) == len(bounds), (
        "region bounds not strictly increasing: %s" % [hex(b) for b in bounds]
    )
    assert bounds[0] == 0, "first plain region does not start at sector 0"
    assert bounds[-1] == total - 1, (
        "last region ends at %s but the image holds %s sectors - the bound count "
        "read from sector 0 is wrong for this image" % (hex(bounds[-1]), hex(total))
    )
    plains = [(bounds[i], bounds[i + 1]) for i in range(0, len(bounds), 2)]
    regions = []
    lba = 0
    for start, end in plains:
        if lba < start:
            regions.append((lba, start - 1, True))
        regions.append((start, end, False))
        lba = end + 1
    return regions


def region_of(regions, lba):
    for start, end, enc in regions:
        if start <= lba <= end:
            return (start, end, enc)
    return None


def decrypt_sector(key, lba, data):
    iv = b"\x00" * 12 + struct.pack(">I", lba)
    dec = Cipher(algorithms.AES(key), modes.CBC(iv)).decryptor()
    return dec.update(data) + dec.finalize()


def derive_from_data1(data1):
    """disc key = AES-128-CBC(data1) under the documented secret, zero IV."""
    enc = Cipher(algorithms.AES(DATA1_SECRET), modes.CBC(b"\x00" * 16)).encryptor()
    return enc.update(data1) + enc.finalize()


def walk_iso9660(f):
    """Yield (path, lba, size) for every file, straight off the PVD root."""
    f.seek(16 * SECTOR)
    pvd = f.read(SECTOR)
    assert pvd[1:6] == b"CD001", "no ISO 9660 primary volume descriptor"
    root = pvd[156 : 156 + 34]
    stack = [("", struct.unpack("<I", root[2:6])[0], struct.unpack("<I", root[10:14])[0])]
    while stack:
        path, lba, size = stack.pop()
        f.seek(lba * SECTOR)
        blob = f.read(size)
        off = 0
        while off < len(blob):
            rec_len = blob[off]
            if rec_len == 0:
                off = (off // SECTOR + 1) * SECTOR
                continue
            rec = blob[off : off + rec_len]
            child_lba = struct.unpack("<I", rec[2:6])[0]
            child_size = struct.unpack("<I", rec[10:14])[0]
            flags = rec[25]
            name_len = rec[32]
            name = rec[33 : 33 + name_len]
            off += rec_len
            if name in (b"\x00", b"\x01"):
                continue
            name = name.split(b";")[0].decode("ascii")
            full = path + "/" + name if path else name
            if flags & 0x02:
                stack.append((full, child_lba, child_size))
            else:
                yield (full, child_lba, child_size)


def cmd_map(iso):
    with open(iso, "rb") as f:
        size = os.path.getsize(iso)
        regions = read_regions(f)
        print("image %s  %d bytes  %d sectors" % (iso, size, size // SECTOR))
        f.seek(SECTOR)
        sfb = f.read(64)
        print("disc id: %s" % sfb[0x10:0x1E].decode("ascii", "replace"))
        print()
        print("%-9s %-9s %-12s %-12s %s" % ("lba_start", "lba_end", "off_start", "bytes", "state"))
        for start, end, enc in regions:
            print(
                "%-9s %-9s %-12s %-12d %s"
                % (
                    hex(start),
                    hex(end),
                    hex(start * SECTOR),
                    (end - start + 1) * SECTOR,
                    "ENCRYPTED" if enc else "plain",
                )
            )
        print()
        if already_decrypted(f, regions):
            print("NOTE: this image is already decrypted - the table above still")
            print("      describes which spans *were* encrypted on the disc.")
            print()
        print("%-38s %-9s %-12s %s" % ("file", "lba", "size", "state"))
        for path, lba, fsize in sorted(walk_iso9660(f), key=lambda t: t[1]):
            last = lba + max(0, (fsize - 1)) // SECTOR
            r0, r1 = region_of(regions, lba), region_of(regions, last)
            if r0 == r1:
                state = "ENCRYPTED" if r0[2] else "plain"
            else:
                state = "SPLIT %s..%s" % (
                    "enc" if r0[2] else "plain",
                    "enc" if r1[2] else "plain",
                )
            print("%-38s %-9s %-12d %s" % (path, hex(lba), fsize, state))


def already_decrypted(f, regions):
    """True if the encrypted regions read as plaintext already.

    A decrypted image keeps sector 0's region table verbatim - it still says
    which spans *were* encrypted - so the table alone cannot tell the two apart.
    The oracle can: if the known-magic sectors already read correctly, the
    ciphertext is gone.

    All-or-nothing on purpose. A half-decrypted or truncated image fails here and
    is treated as encrypted, so `extract` writes only its plain regions rather
    than a mix of real files and noise.
    """
    targets = oracle_targets(f, regions)
    if not targets:
        return False
    for path, lba, expect in targets:
        f.seek(lba * SECTOR)
        if f.read(SECTOR)[: len(expect)] != expect:
            return False
    return True


def cmd_extract(iso, outdir):
    with open(iso, "rb") as f:
        regions = read_regions(f)
        plain_image = already_decrypted(f, regions)
        if plain_image:
            print("image already decrypted (oracle passes on raw sectors)")
        for path, lba, fsize in walk_iso9660(f):
            last = lba + max(0, (fsize - 1)) // SECTOR
            if not plain_image and (
                region_of(regions, lba)[2] or region_of(regions, last)[2]
            ):
                print("skip  %-34s (encrypted)" % path)
                continue
            dest = os.path.join(outdir, path)
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            f.seek(lba * SECTOR)
            with open(dest, "wb") as out:
                left = fsize
                while left:
                    chunk = f.read(min(left, 1 << 22))
                    out.write(chunk)
                    left -= len(chunk)
            print("write %-34s %d bytes" % (path, fsize))


def candidate_keys(keyhex):
    raw = bytes.fromhex(keyhex)
    assert len(raw) == 16, "key must be 16 bytes / 32 hex chars"
    return [("as-is (disc key)", raw), ("derived from data1", derive_from_data1(raw))]


def oracle_targets(f, regions):
    """Files starting in an encrypted region whose first bytes we can predict.

    Two sources of expected plaintext, strongest first: this disc ships three
    files twice - once in PS3_GAME (plain region) and once in PS3_GAME/USRDIR
    (encrypted region) - so the plain twin gives a full 2048-byte crib. Anything
    else falls back to its format's magic.

    The twin crib assumes same-name same-size copies are byte-identical, which
    is unverified (the second copy is the encrypted one). If that assumption is
    wrong the oracle rejects a correct key; it can never accept a wrong one.
    """
    files = list(walk_iso9660(f))
    plain_by_name = {}
    for path, lba, fsize in files:
        if not region_of(regions, lba)[2]:
            plain_by_name.setdefault((os.path.basename(path).upper(), fsize), lba)

    out = []
    for path, lba, fsize in files:
        if not region_of(regions, lba)[2]:
            continue
        twin = plain_by_name.get((os.path.basename(path).upper(), fsize))
        if twin is not None:
            f.seek(twin * SECTOR)
            out.append((path + " (twin crib)", lba, f.read(SECTOR)))
            continue
        ext = os.path.splitext(path.upper())[1]
        if ext in MAGIC_BY_EXT:
            out.append((path, lba, MAGIC_BY_EXT[ext]))
    return out


def cmd_oracle(iso, keyhex):
    with open(iso, "rb") as f:
        regions = read_regions(f)
        targets = oracle_targets(f, regions)
        if not targets:
            print("no known-magic file starts inside an encrypted region")
            return 2
        ok = False
        for label, key in candidate_keys(keyhex):
            hits = 0
            for path, lba, magic in targets:
                f.seek(lba * SECTOR)
                plain = decrypt_sector(key, lba, f.read(SECTOR))
                good = plain[: len(magic)] == magic
                hits += good
                print(
                    "  %-20s %-40s lba=%-9s got %-14r %s"
                    % (label, path, hex(lba), plain[:8], "OK" if good else "no")
                )
            if hits == len(targets):
                print("\nKEY ACCEPTED (%s): %s" % (label, key.hex()))
                ok = True
        return 0 if ok else 1


def cmd_decrypt(iso, keyhex, out):
    with open(iso, "rb") as f:
        regions = read_regions(f)
        targets = oracle_targets(f, regions)
        key = chosen = None
        for label, cand in candidate_keys(keyhex):
            good = True
            for path, lba, magic in targets:
                f.seek(lba * SECTOR)
                if decrypt_sector(cand, lba, f.read(SECTOR))[: len(magic)] != magic:
                    good = False
                    break
            if good:
                key, chosen = cand, label
                break
        if key is None:
            print("key rejected by the oracle; refusing to write garbage")
            return 1
        print("using key %s (%s)" % (key.hex(), chosen))
        total = os.path.getsize(iso) // SECTOR
        with open(out, "wb") as o:
            for start, end, enc in regions:
                f.seek(start * SECTOR)
                for lba in range(start, min(end, total - 1) + 1):
                    data = f.read(SECTOR)
                    o.write(decrypt_sector(key, lba, data) if enc else data)
                    if lba % 0x10000 == 0:
                        print("  %d / %d sectors" % (lba, total), flush=True)
        print("wrote %s" % out)
        return 0


def main(argv):
    parser = argparse.ArgumentParser(
        prog="ps3iso.py",
        description=__doc__.split("\n\n")[0],
        epilog="Needs the Python package `cryptography` "
        "(`uv run --with cryptography python3 scripts/ps3iso.py ...`).",
    )
    commands = parser.add_subparsers(dest="cmd", required=True, metavar="command")
    for name, handler, arguments, help_text in (
        ("map", cmd_map, ["iso"], "print the region table and which file lands where"),
        ("extract", cmd_extract, ["iso", "outdir"], "pull the readable files out"),
        ("oracle", cmd_oracle, ["iso", "keyhex"], "test a candidate key"),
        (
            "decrypt",
            cmd_decrypt,
            ["iso", "keyhex", "out"],
            "write a whole-image decrypted copy (keyhex: 32 hex chars, your disc's own key)",
        ),
    ):
        sub = commands.add_parser(name, help=help_text, description=help_text)
        for argument in arguments:
            sub.add_argument(argument)
        sub.set_defaults(handler=handler, arguments=arguments)
    args = parser.parse_args(argv[1:])
    return args.handler(*[getattr(args, a) for a in args.arguments])


if __name__ == "__main__":
    sys.exit(main(sys.argv) or 0)
