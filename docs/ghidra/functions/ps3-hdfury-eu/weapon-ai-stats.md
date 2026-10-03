# `WeaponAIStats` loader: where HD parses `weaponaistats.xml`

**Binary:** `ps3-hdfury-eu` `EBOOT.elf` (PowerPC, 32-bit pointers, TOC `r2 = 0x8ad4d8`).
Read 2026-10-03 for the `hd-eliminator-aistats` lane from the offline disassembly and a TOC-slot
scan of the ELF, no Ghidra database. **No function is renamed**: nothing here is named in
`names.tsv`, the addresses below are plain evidence.

| Address | What | Confidence |
| --- | --- | --- |
| `0x000fcbf8` | the `WeaponAIStats` loader: zeroes the 232-byte object, sets the two flips to `1.0` (`lis 0,0x3f80`), then matches each child element by name | 85 |
| `0x000fc698` | the `EliminatorAIStats` row parser: one `strcmp`-style match per attribute, `stfs` into the object | 85 |
| `0x000fc550` | the three-float row parser (`useAgainstPlayer`, `useAgainstAI`, `absorb`) shared by every weapon and by `AllWeapons` | 75 |
| `0x00056d24`, `0x0005d46c` | the two callers, both in the RaceManager constructor; the object is `RaceManager + 6240` (`addi 0,22,6240`) | 85 |
| `0x0042b640`, `0x0042a908` | a second pair of references to `EliminatorAIStats` and `normalFlip` in a different function; not read | n/a |

**TOC slots** (string addresses are `0x782998..0x782b98`, loaded as `lwz 4, off(2)`):
`normalFlip` `-16416`, `infrontFlip` `-16412`, `easyFlipScale` `-16408`, `mediumFlipScale`
`-16404`, `hardFlipScale` `-16400`, `easyAbsorbScale` `-16396`, `mediumAbsorbScale` `-16392`,
`hardAbsorbScale` `-16388`, `easyUseScale` `-16384`, `mediumUseScale` `-16380`, `hardUseScale`
`-16376`, `easyScoreScale` `-16372`, `mediumScoreScale` `-16368`, `hardScoreScale` `-16364`,
`AllWeapons` `-16296`, `EliminatorAIStats` `-16292`, `Template` `-16288`.

## The object

| Offset | Field | Evidence |
| --- | --- | --- |
| `+8 .. +163` | thirteen weapon rows, three floats each | `stw 0, 8(9)` ... zero fill; per-row stores at `fca44..fcb90` |
| `+164 .. +172` | `AllWeapons` row (three floats) | `bl 0xfc550` after the `AllWeapons` match at `0xfd0a4` |
| `+176` | `normalFlip` (default `1.0`) | `stfs 1, 176(30)` at `0xfc748` |
| `+180` | `infrontFlip` (default `1.0`) | `stfs 1, 180(30)` at `0xfc7a8` |
| `+184 / +188 / +192` | easy / medium / hard flip scale | `0xfc7d8`, `0xfc808`, `0xfc868` |
| `+196 / +200 / +204` | easy / medium / hard use scale | `0xfc8f8`, `0xfc928`, `0xfc958` |
| `+208 / +212 / +216` | easy / medium / hard absorb scale | `0xfc854`, `0xfc8c8`, `0xfc8b4` |
| `+220 / +224 / +228` | easy / medium / hard score scale | `0xfc988`, `0xfc9b8`, `0xfc9e8` |

Confidence 85 on the offset-to-attribute mapping: the order is the branch structure of
`0xfc698`, checked against the zero-fill defaults (`stw 0, 184..228`). The `+164` start of
`AllWeapons` is 70: the row parser writes `+168`/`+172` explicitly and `+164` by the same
function's first store, which was not isolated.

**Defaults matter.** A file without the row (`DATA02`'s base-game copy, Omega's unsuffixed one)
leaves every scale `0.0` and both flips `1.0`.

## What was not found

The reader of `+176 .. +228`. There is no `lfs` of `RaceManager + 6416 .. 6468` through the
RaceManager register, and a search for indexed (`slwi 2`) loads of the four array bases found
nothing; the object is reached through a computed pointer. The next step is a Ghidra data xref
on the RaceManager field `+6240`, or a write watchpoint under RPCS3 on `+6416`.
