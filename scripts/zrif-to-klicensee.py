#!/usr/bin/env python3
"""Decodes a Vita zRIF string back to its 512-byte SceNpDrmLicense and pulls
out the klicensee - the 16-byte secret `scripts/vita-self-decrypt.py` needs to
decrypt a SELF's NpDrm-encrypted code/data segments. See
docs/reverse-engineering/toolchain.md#vita for what that encryption is and why
a plaintext SELF header and phdrs do not mean the segments are plaintext too.

A zRIF is `base64(zlib(license_bytes, preset_dictionary))` - no AES, no
console-derived secret, confirmed by reading
[KorewaWatchful/libzrif](https://github.com/KorewaWatchful/libzrif)'s
`keyflate.c` directly (`inflateKey`, windowBits 10, the fixed 1024-byte
`g_dict` below, transcribed byte-for-byte from that file, not retyped by
hand). This is a *different, lighter* layer than `psvpfsparser`'s own
`-f00d_url`/`-f00d_cache`, which derives the PFS *filesystem* key and does
need the F00D service - this script needs nothing but the zRIF string.

The `SceNpDrmLicense` field layout (content_id at 0x10, key at 0x50) is
Vita3K's `packages/include/packages/license.h`, cross-checked against the
decoded `content_id` matching the zRIF's own row in nopaystation.com's
`PSV_GAMES.tsv` export - not assumed from the struct alone.

Usage:
    scripts/zrif-to-klicensee.py <zrif-string>
    scripts/zrif-to-klicensee.py -f data/keys/vita-zrif.tsv
        # decodes every zrif column in a TSV with a header row
"""

from __future__ import annotations

import argparse
import base64
import sys
import zlib
from pathlib import Path

# The first 1024 bytes of libzrif/src/keyflate.c's g_dict[] (that file also
# declares g_dict_size = 1024; the C array literal has one trailing element
# past that, a zlib End-Of-Block marker byte that is not part of the
# dictionary). Kept as base64 rather than a 1024-entry Python list so this
# file stays readable.
_DICT_B64 = (
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADAwMDA5AAAAAAAAAAAAAAAwMDAwNjAwMDA3MDAwMDgA"
    "MDAwMDMwMDAwNDAwMDA1MF8wMC1BRERDT05UMDAwMDItUENTRzAwMDAwMDAwMDAxLVBDU0UwMDAt"
    "UENTRjAwMC1QQ1NDMDAwLVBDU0QwMDAtUENTQTAwMC1QQ1NCMDAwAAEAAQABAALvzauJZ0UjAQ=="
)
_DICT = base64.b64decode(_DICT_B64)
assert len(_DICT) == 1024, f"g_dict should be 1024 bytes, got {len(_DICT)}"

LICENSE_SIZE = 512
CONTENT_ID_OFFSET, CONTENT_ID_SIZE = 0x10, 0x30
KLICENSEE_OFFSET, KLICENSEE_SIZE = 0x50, 0x10


def zrif_to_license(zrif: str) -> bytes:
    raw = base64.b64decode(zrif.strip())
    d = zlib.decompressobj(wbits=10, zdict=_DICT)
    license_bytes = d.decompress(raw) + d.flush()
    if len(license_bytes) != LICENSE_SIZE:
        raise ValueError(f"decoded {len(license_bytes)} bytes, expected {LICENSE_SIZE} (SceNpDrmLicense)")
    return license_bytes


def license_fields(license_bytes: bytes) -> tuple[str, str]:
    content_id = license_bytes[CONTENT_ID_OFFSET : CONTENT_ID_OFFSET + CONTENT_ID_SIZE].split(b"\x00")[0].decode()
    klicensee = license_bytes[KLICENSEE_OFFSET : KLICENSEE_OFFSET + KLICENSEE_SIZE].hex()
    return content_id, klicensee


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("zrif", nargs="?", help="a single zRIF string")
    parser.add_argument("-f", "--tsv", type=Path, help="a TSV with a 'zrif' column and a header row")
    args = parser.parse_args()

    if not args.zrif and not args.tsv:
        parser.error("pass a zRIF string or -f/--tsv")

    rows: list[str] = []
    if args.zrif:
        rows = [args.zrif]
    else:
        lines = args.tsv.read_text().splitlines()
        header = lines[0].split("\t")
        zrif_col = header.index("zrif")
        rows = [line.split("\t")[zrif_col] for line in lines[1:] if line.strip()]

    ok = True
    for zrif in rows:
        try:
            content_id, klicensee = license_fields(zrif_to_license(zrif))
            print(f"{content_id}\t{klicensee}")
        except (ValueError, zlib.error) as e:
            print(f"error: {zrif[:24]}...: {e}", file=sys.stderr)
            ok = False
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
