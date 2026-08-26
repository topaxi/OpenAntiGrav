#!/usr/bin/env python3
"""Decrypts a Vita retail SELF (`eboot.bin`/`.suprx`) to the plain ELF
VitaLoaderRedux actually opens. Replaces an earlier `strip-vita-self.py` that
only cut the SCE header off and left the code/data segments underneath still
NpDrm-encrypted (confirmed empirically: not zlib, ~8 bits/byte entropy - see
docs/reverse-engineering/toolchain.md#vita). A plaintext ELF header and phdrs
made `readelf` look satisfied; they say nothing about the segments.

Three layers, each verified against a real build of WipEout 2048
(`data/extracted/vita/PCSF00007/patch-v104/eboot.bin`), not assumed from the
struct definitions alone - readelf-clean headers, valid `PT_LOAD`/
`PT_SCE_VERSION` phdrs, and disassembled ARM Thumb-2 (`movw`/`movt` pairs,
sane `push`/`pop` prologues) at the resulting file's `0x81000000` load base:

1. **zRIF -> klicensee.** Pure zlib with a preset dictionary, no AES, no
   console-derived secret - see `scripts/zrif-to-klicensee.py` and
   `data/keys/README.md`. The klicensee is the only external secret this
   script needs; everything else below is public, compiled-in key material.
2. **The SELF's metadata block -> per-segment keys.** A chain of AES-128/256-CBC
   decrypts (klicensee -> NpDrm-wrapped intermediate key -> a title-independent
   "metadata key" selected by system-version range and key revision -> the
   actual per-segment AES-128-CTR keys and IVs), following
   `SceHeader.metadata_offset`. The compiled-in key table below is transcribed
   from [Vita3K/Vita3K](https://github.com/Vita3K/Vita3K)'s
   `vita3k/packages/src/sce_utils.cpp` `register_keys()` (the `type=1` case,
   the one `decrypt_fself` actually calls) - credited there to TeamMolecule's
   original `sceutils` work.
3. **AES-128-CTR per segment, then inflate.** Each ELF program header has a
   matching `segment_info` entry (`SelfHeader.segment_info_offset`, **not**
   the ELF's own `e_phoff` - see the `phdr_offset`/`pad[3]` finding in
   toolchain.md) naming where its ciphertext lives and whether it is
   `compressed`. Decrypt with the segment's own key/IV from step 2, then zlib
   `inflate` if `compressed`, and place the result at that phdr's own
   `p_offset` in the output file - segments are not necessarily stored in
   phdr order.

This produces the same `elf` this format's own `decrypt_fself` builds
in-memory before wrapping it back into a synthetic SELF for Vita3K's own
cache; VitaLoaderRedux wants the bare ELF, so this script stops there and
never reconstructs a wrapper.

Usage:
    scripts/vita-self-decrypt.py <eboot.bin|module.suprx> -k <klicensee-hex> [-o out.elf]
    scripts/vita-self-decrypt.py data/extracted/vita/PCSF00007/patch-v104/eboot.bin \\
        -k "$(python3 scripts/zrif-to-klicensee.py -f data/keys/vita-zrif.tsv | awk -F'\\t' '/PCSF00007/{print $2}')"
"""

from __future__ import annotations

import argparse
import struct
import sys
import zlib
from pathlib import Path

from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

SCE_MAGIC = 0x00454353
METADATA_INFO_SIZE = 64
METADATA_HEADER_SIZE = 32
METADATA_SECTION_SIZE = 48
ENCRYPTION_AES128CTR = 3
COMPRESSION_DEFLATE = 2
SECURE_BOOL_NO = 1  # SecureBool::NO - "not plaintext", i.e. still encrypted
SECURE_BOOL_YES = 2  # SecureBool::YES

# NpDrm unwrap key/IV, keyed by (key_revision >= 2). Transcribed from
# sce_utils.cpp register_keys(SCE_KEYS, 1)'s KeyType::NPDRM / SelfType::APP rows.
NPDRM_APP = {
    0: ("C10368BF3D2943BC6E5BD05E46A9A7B6", "00000000000000000000000000000000"[:32]),
    1: ("16419DD3BFBE8BDC596929B72CE237CD", "00000000000000000000000000000000"[:32]),
}

# (minver, maxver, key_revision) -> (key, iv). KeyType::METADATA / SelfType::APP
# rows from the same function and the same `type=1` case.
METADATA_APP = [
    (0x00000000000, 0x16920000000, 0, "AAA508FA5E85EAEE597ED2B27804D22287CFADF1DF32EDC7A7C58E8C9AA8BB36", "CD1BD3A59200CC67A3B804808DC2AE73"),
    (0x18000000000, 0xFFFFFFFFFFFFFFFF, 0, "5661E5FB20CFD1D1DFF50C1E59A6EA977D0AA5C5770F53B9CDD4E9451FFF55CB", "23D02FF79BF430E2D123869BF0CACAA0"),
    (0x00000000000, 0xFFFFFFFFFFFFFFFF, 1, "4181B2DF5F5D94D3C80B7D86EACF1928533A49BA58EDE2B43CDEE7E572568BD4", "B1678C0543B6C1997B63A6F4F3C8FD33"),
    (0x00000000000, 0xFFFFFFFFFFFFFFFF, 2, "5282582F17F068F89A260AAFB71C58928F45A8D08C681376B07FF9EAB1114226", "29672DF43E426F41AF46D42E8437D449"),
    (0x00000000000, 0xFFFFFFFFFFFFFFFF, 3, "270CBA370061B87077672ADB5142D18844AAED352A9CCEE63602B0D740594334", "1CF2454FBF47D76221B91AFC3B608C28"),
    (0x00000000000, 0xFFFFFFFFFFFFFFFF, 4, "A782BC5A9EDDFC49A513FF3E592C4677A8C8920F23C9F11F2558FB9D99A43868", "559B5E658559EB65EBF892C274E098A9"),
    (0x00000000000, 0xFFFFFFFFFFFFFFFF, 5, "12D64D0172495226010A687DE245A73DE028B3561E25E69BABC325636F3CAE0A", "F149EED1757E5A915B24309795BFC380"),
]

SELF_TYPE_APP = 8


def _aes_cbc_decrypt(key: bytes, iv: bytes, data: bytes) -> bytes:
    d = Cipher(algorithms.AES(key), modes.CBC(iv)).decryptor()
    return d.update(data) + d.finalize()


def _aes_ctr_decrypt(key: bytes, iv: bytes, data: bytes) -> bytes:
    d = Cipher(algorithms.AES(key), modes.CTR(iv)).decryptor()
    return d.update(data) + d.finalize()


def _metadata_key(sys_version: int, key_revision: int) -> tuple[bytes, bytes]:
    for minver, maxver, kr, key, iv in METADATA_APP:
        if minver <= sys_version <= maxver and kr == key_revision:
            return bytes.fromhex(key), bytes.fromhex(iv)
    raise ValueError(f"no metadata key for sys_version=0x{sys_version:x} key_revision={key_revision}")


def decrypt_fself(fself: bytes, klic: bytes) -> bytes:
    """Returns the plain ELF bytes - header, phdrs, and every segment
    decrypted/decompressed/placed at its real p_offset."""
    magic, _version = struct.unpack_from("<II", fself, 0)
    if magic != SCE_MAGIC:
        raise ValueError(f"not an SCE SELF (magic 0x{magic:x})")
    key_revision = fself[9]
    metadata_offset = struct.unpack_from("<I", fself, 12)[0]
    header_length = struct.unpack_from("<Q", fself, 16)[0]

    (_file_length, _field_8, _self_offset, appinfo_offset, elf_offset, phdr_offset,
     _shdr_offset, segment_info_offset, _sceversion_offset, _controlinfo_offset,
     _controlinfo_length) = struct.unpack_from("<11Q", fself, 32)

    _auth_id, _vendor_id, self_type, sys_version, _field_18 = struct.unpack_from("<QIIQQ", fself, appinfo_offset)
    is_app = self_type == SELF_TYPE_APP

    if is_app and klic == b"\x00" * 16:
        raise ValueError("no klicensee provided for an NpDrm-encrypted App SELF")

    elf_header_bytes = fself[elf_offset : elf_offset + 52]
    e_phnum = struct.unpack_from("<H", elf_header_bytes, 44)[0]

    phdrs = []
    segment_infos = []
    encrypted = False
    for i in range(e_phnum):
        phdr = struct.unpack_from("<8I", fself, phdr_offset + i * 32)
        phdrs.append(phdr)

        offset, size, compressed, _pad0, plaintext, _pad1 = struct.unpack_from(
            "<QQIIII", fself, segment_info_offset + i * 32
        )
        segment_infos.append((offset, size, compressed, plaintext))
        if plaintext == SECURE_BOOL_NO:
            encrypted = True

    scesegs = []
    if encrypted:
        keytype = 1 if key_revision >= 2 else 0
        np_key, np_iv = (bytes.fromhex(k) for k in NPDRM_APP[keytype])
        predec = _aes_cbc_decrypt(np_key, np_iv, klic)

        header_tail = fself[metadata_offset + 48 :]
        dec_in = _aes_cbc_decrypt(predec, np_iv, header_tail[:METADATA_INFO_SIZE])

        meta_key, meta_iv = _metadata_key(sys_version, key_revision)
        metadata_info = _aes_cbc_decrypt(meta_key, meta_iv, dec_in)
        seg_key, pad0, pad1, seg_iv, pad2, pad3 = (
            metadata_info[0:16],
            metadata_info[16:24],
            metadata_info[24:32],
            metadata_info[32:48],
            metadata_info[48:56],
            metadata_info[56:64],
        )
        if any(pad0) or any(pad1) or any(pad2) or any(pad3):
            print("warning: MetadataInfo padding is non-zero - decryption likely wrong", file=sys.stderr)

        rest_len = header_length - metadata_offset - 48 - METADATA_INFO_SIZE
        dec1 = _aes_cbc_decrypt(seg_key, seg_iv, header_tail[METADATA_INFO_SIZE : METADATA_INFO_SIZE + rest_len])

        _sig_len, _sig_type, section_count, key_count, _opt_size = struct.unpack_from("<QIIII", dec1, 0)
        vault_start = METADATA_HEADER_SIZE + section_count * METADATA_SECTION_SIZE
        vault = [dec1[vault_start + 16 * i : vault_start + 16 * (i + 1)] for i in range(key_count)]

        for i in range(section_count):
            base = METADATA_HEADER_SIZE + i * METADATA_SECTION_SIZE
            sec_offset, sec_size, _type, seg_idx, _hashtype, _hash_idx, enc, key_idx, iv_idx, comp = (
                struct.unpack_from("<QQIiIiIiiI", dec1, base)
            )
            if enc == ENCRYPTION_AES128CTR:
                scesegs.append((sec_offset, seg_idx, sec_size, comp == COMPRESSION_DEFLATE, vault[key_idx], vault[iv_idx]))

    elf = bytearray(elf_header_bytes)
    for phdr in phdrs:
        p_type, p_offset, p_vaddr, p_paddr, p_filesz, p_memsz, p_flags, p_align = phdr
        elf += struct.pack("<8I", p_type, p_offset, p_vaddr, p_paddr, p_filesz, p_memsz, p_flags, min(p_align, 0x1000))
    at = 52 + 32 * e_phnum

    for i in range(e_phnum):
        idx = scesegs[i][1] if scesegs else i
        p_offset, p_filesz = phdrs[idx][1], phdrs[idx][4]
        if p_filesz == 0:
            continue

        pad_len = p_offset - at
        if pad_len < 0:
            raise ValueError(f"segment {i}: ELF p_offset (0x{p_offset:x}) is before the current write position (0x{at:x})")
        elf += b"\x00" * pad_len
        at += pad_len

        offset, size, compressed, plaintext = segment_infos[idx]
        raw = fself[offset : offset + size]
        if plaintext == SECURE_BOOL_NO:
            _sec_offset, _seg_idx, _sec_size, _sec_compressed, key, iv = scesegs[i]
            decrypted = _aes_ctr_decrypt(key, iv, raw)
        else:
            decrypted = raw

        payload = zlib.decompress(decrypted) if compressed == SECURE_BOOL_YES else decrypted
        elf += payload
        at += len(payload)

    return bytes(elf)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("input", type=Path, help="decrypted (PFS layer only) eboot.bin or .suprx - still an SCE SELF")
    parser.add_argument("-k", "--klicensee", required=True, help="16-byte klicensee, hex - see scripts/zrif-to-klicensee.py")
    parser.add_argument("-o", "--output", type=Path, default=None, help="output .elf path (default: input with .bin/.suprx replaced by .elf)")
    args = parser.parse_args()

    if not args.input.is_file():
        print(f"error: {args.input} not found", file=sys.stderr)
        return 1

    klic = bytes.fromhex(args.klicensee)
    if len(klic) != 16:
        print(f"error: klicensee must be 16 bytes, got {len(klic)}", file=sys.stderr)
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
        elf = decrypt_fself(args.input.read_bytes(), klic)
    except ValueError as e:
        print(f"error: {args.input}: {e}", file=sys.stderr)
        return 1

    out_path.write_bytes(elf)
    print(f"wrote {out_path} ({len(elf)} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
