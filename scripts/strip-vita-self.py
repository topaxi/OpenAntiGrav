#!/usr/bin/env python3
"""Strips a decrypted Vita SELF's SCE header to the plain ELF-PRX underneath.

`scripts/build-psvpfstools.sh` decrypts a PKG's PFS layer, which recovers a
plaintext `eboot.bin`/`.suprx` - but that file is still an SCE-wrapped SELF,
not a bare ELF: it opens with the 4-byte magic `SCE\0`, and the ELF payload
begins partway in, at an offset the header itself gives. VitaLoaderRedux (the
Ghidra loader `scripts/build-vita-loader-redux.sh` builds) parses an ELF
header directly from byte 0 of whatever file it is given - confirmed by
reading `ArmElfPrxLoader.findSupportedLoadSpecs`, which does `new
ElfEhdr(provider)` with no SCE-magic handling at all - so it cannot open the
SELF directly. This script does the one step in between.

The struct below is `SCE_header` as used by
CelesteBlue-dev/PSVita-RE-tools's `vita-unmake-fself` (and matches
vitasdk/vita-toolchain's `sce-elf.h`); `elf_offset` is the field that matters,
read rather than assumed constant - it happened to be 0xa0 for every file
checked while writing this (WipEout 2048's eboot.bin and every sce_module/
.suprx, base and patch, both regions), which is a property of this title's
SDK build, not a promise a different one has to keep.

Everything from `elf_offset` to end of file is written out as-is. That
includes Vita-specific program header types (`PT_SCE_RELA` etc, which
`readelf` reports as `<unknown>`) and possibly per-segment compression -
VitaLoaderRedux is what actually interprets those; this script only locates
the boundary, the same division of labour as `rpcs3 --decrypt` decrypting a
PS3 SELF and Ps3GhidraScripts interpreting the result (see
`scripts/import-ps3-eboot.sh`).

Usage:
    scripts/strip-vita-self.py <eboot.bin|module.suprx> [-o out.elf]
    scripts/strip-vita-self.py data/extracted/vita/PCSF00007/patch-v104/eboot.bin
        # writes eboot.elf beside it, PS3 EBOOT.elf-style
"""

from __future__ import annotations

import argparse
import struct
import sys
from pathlib import Path

MAGIC = b"SCE\0"

# Offsets into the fixed-size SCE_header prefix. Every field up to and
# including elf_offset; nothing past it is needed here.
_HDR = struct.Struct("<4sIHHI7Q")
_FIELDS = (
    "magic",
    "version",
    "sdk_type",
    "header_type",
    "metadata_offset",
    "header_len",
    "elf_filesize",
    "self_filesize",
    "unknown",
    "self_offset",
    "appinfo_offset",
    "elf_offset",
)


def parse_header(data: bytes) -> dict:
    if len(data) < _HDR.size:
        raise ValueError(f"file is only {len(data)} bytes, shorter than the SCE header")
    fields = dict(zip(_FIELDS, _HDR.unpack_from(data, 0)))
    if fields["magic"] != MAGIC:
        raise ValueError(f"no SCE\\0 magic - starts with {fields['magic']!r}, already a plain ELF?")
    return fields


def strip(in_path: Path, out_path: Path) -> dict:
    data = in_path.read_bytes()
    header = parse_header(data)
    elf = data[header["elf_offset"] :]
    if elf[:4] != b"\x7fELF":
        raise ValueError(
            f"elf_offset 0x{header['elf_offset']:x} does not point at an ELF magic "
            f"({elf[:4]!r}) - header layout may differ for this file"
        )
    out_path.write_bytes(elf)
    return header


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("input", type=Path, help="decrypted eboot.bin or .suprx (SCE-wrapped SELF)")
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        default=None,
        help="output .elf path (default: input with .bin/.suprx/.self replaced by .elf)",
    )
    args = parser.parse_args()

    if not args.input.is_file():
        print(f"error: {args.input} not found", file=sys.stderr)
        return 1

    out_path = args.output
    if out_path is None:
        stem = args.input.name
        for suffix in (".bin", ".suprx", ".skprx", ".self"):
            if stem.lower().endswith(suffix):
                stem = stem[: -len(suffix)]
                break
        out_path = args.input.with_name(stem + ".elf")

    try:
        header = strip(args.input, out_path)
    except ValueError as e:
        print(f"error: {args.input}: {e}", file=sys.stderr)
        return 1

    print(f"wrote {out_path} ({out_path.stat().st_size} bytes)")
    print(f"  elf_offset=0x{header['elf_offset']:x}  sdk_type=0x{header['sdk_type']:x}  version={header['version']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
