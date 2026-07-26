# Pulse PSP functions

Functions from `PSP_GAME/SYSDIR/BOOT.BIN` (Wipeout Pulse, PSP, UCUS-98712),
image base `0x08804000`, language `Allegrex:LE:32:default`.

An unencrypted ELF, so no decryption step is needed. See
[workflow](../../workflow.md), and read
[Allegrex and the VFPU](../../../psp/allegrex-vfpu.md) first.

| Page | Covers |
| --- | --- |
| [WAD subsystem](wad-subsystem.md) | Name hash, lookup, mount, read, LZSS and zlib decoders |
| [Main loop](main-loop.md) | Frame pacing, timestep, state machine, input |

## Renames

**None applied yet.** The pages above carry the evidence and proposed names;
applying them is a separate step, and anything below 70 confidence keeps its
address name per
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md).

## Highest-confidence findings

| Address | Name | Conf | Why it matters |
| --- | --- | ---: | --- |
| `0x08940d0c` | `Wad_HashName` | 97 | Unblocked all asset work; entry names are recoverable |
| `0x08807244` | `Game_MainLoop` | 95 | Names itself via a profiler string |
| `0x08804978` | `Game_UpdateFrame` | 92 | Showed the timestep is variable, not fixed |
| `0x089411e8` | `Wad_Open` | 93 | Hash plus rotating linear scan |
