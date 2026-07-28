# PSP

Findings specific to the PlayStation Portable releases.

| Document | Covers |
| --- | --- |
| [Allegrex and the VFPU](allegrex-vfpu.md) | **Read first.** Stock Ghidra mis-decodes PSP vector code |
| [Frame pacing](frame-pacing.md) | The original's timestep. It is **not** fixed. |
| [Pulse disc layout](pulse-disc-layout.md) | Contents of the Pulse UMD, with findings |

## Key facts

- **`PSP_GAME/SYSDIR/BOOT.BIN` is an unencrypted ELF.** No decryption step is
  needed to start reverse engineering. `EBOOT.BIN` is the encrypted variant of
  the same program and can be ignored.
- UMDs use ISO 9660 with 2048-byte sectors.
- `UMD_DATA.BIN` in the root holds the disc serial as its first field.
- The VFPU is not IEEE-conformant for reciprocal and reciprocal square root,
  which is why bit-exactness with the original is a non-goal. See
  [ADR-0002](../architecture/adr/0002-determinism-model.md).

## Related

- [Pure](../formats/pure-status.md) uses the same disc structure and the same
  archive format, and is often the easier place to understand an ambiguous
  Pulse format. That page has its UMD's layout, and measures exactly how much
  of the Pulse format layer reads it unchanged.
