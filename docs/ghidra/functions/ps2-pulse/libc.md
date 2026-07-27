# Recovered C library routines (PS2)

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`.

Unlike the PSP build, whose library calls go through NID-hashed import stubs
resolved by [`scripts/resolve-psp-imports.py`](../../../../scripts/resolve-psp-imports.py)
(see [psp-pulse/imports.md](../psp-pulse/imports.md)), the PS2 executable is
statically linked: the C library is compiled in and indistinguishable from game
code until it is identified by shape. These are the few routines that had to be
identified to read the subsystems documented in this directory, and they keep
their real library names rather than the project's `Subsystem_VerbNoun` scheme.

**The names below are applied**, from [names.tsv](names.tsv).

| Address | Name | Conf |
| --- | --- | ---: |
| `0x0010ee78` | `strlen` | 92 |
| `0x0011568c` | `strcmp` | 90 |
| `0x00111740` | `strcasecmp` | 88 |
| `0x00115540` | `tolower` | 90 |
| `0x0029f8c0` | `_ctype_` | 88 |

Six more are documented on [xml-reader.md](xml-reader.md), because they were
identified while reading that family and the evidence for each is its use
there: `strncpy` (`0x0010efc0`), `strcpy` (`0x0010ed60`), `memset`
(`0x0010eb78`), `strrchr` (`0x0025c528`), `atoi` (`0x00110028`) and `atof`
(`0x0025b9b0`).

`strlen` and `strcmp` are the Emotion Engine's SIMD versions: they check
alignment, then scan eight or sixteen bytes at a time with `psubb` / `pand`
against the `0x0101...` and `0x8080...` word constants - the classic
have-a-zero-byte trick, in the 128-bit form the EE core supports - and fall
back to a byte loop for the tail. Both were confirmed by use rather than by
shape alone: `strlen`'s result is what `Wad_HashName` iterates over, and
`strcmp`'s zero-means-equal result is what `Camera_UpdatePlayerView` tests
against the three `OPT_*` literals.

`tolower` is `c + 0x20` guarded by `_ctype_[c] & 0x01`, and `strcasecmp` is a
plain byte loop that pushes both operands through it. `strcasecmp` is what
`Input_ParseButtonName` and the front-end XML parsers compare with, which is
worth knowing: **button and element names in this game's XML are matched
case-insensitively**.

`_ctype_` is newlib's table, identified by reading it: one leading byte for the
`EOF` slot, then `0x20` (`_C`) for the control range, `0x28` (`_C|_S`) for
`\t\n\v\f\r`, `0x88` (`_S|_B`) for space, and `0x10` (`_N`) for the digits -
and by the fact that every consumer indexes it through a pointer to
`_ctype_ + 1`, which is exactly how newlib's `__ctype_ptr__` is defined. Bit
`0x01` is `_U`, which is what both `tolower` and `Wad_HashName` test.

Confidence is capped in the high 80s and low 90s because none of these was run;
each rests on a structural reading plus at least one consumer that only makes
sense if the reading is right.

`crc32` (`0x002068c8`) is also a library routine, but it is documented on
[wad-subsystem.md](wad-subsystem.md) instead, because the only reason to name it
was to keep it from being mistaken for the WAD name hash.

## Not determined

- Everything else. No systematic library identification (FLIRT-style signature
  matching or otherwise) has been attempted on this binary, and it would be
  worth doing before much more of the executable is read: 5,234 functions were
  found by auto-analysis and an unknown but large fraction of them are libc,
  libstdc++ and Sony middleware rather than game code. The eleven symbols named
  across this page and [xml-reader.md](xml-reader.md) were each identified
  because something being read needed them, which is not a strategy that scales.

## Cross-platform

| Function | PS2 (`SCES_547.48`) | PSP (`BOOT.BIN`) |
| --- | --- | --- |
| `strlen` | `0x0010ee78` | `0x0897349c` |
| `_ctype_` | `0x0029f8c0` | `0x08a90d20` |
| `strcmp` | `0x0011568c` | not located |
| `strcasecmp` | `0x00111740` | not located |
| `tolower` | `0x00115540` | not located |

## History

- 2026-07-27: first pass, five symbols, identified only as far as the WAD,
  handling, input and camera work required.
