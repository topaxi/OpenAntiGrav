# 2048 mounts five PSARCs at one mount point, in a fixed order

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`, Ghidra program `/2048/eboot-vita-2048-eu-v104.elf`. Read
2026-10-07 (`title-patches` lane). The base v1.00 executable carries the string
`data.psarc` alone (checked with `strings`); `data1.psarc` and `data2.psarc` are
in the v1.04 binary only.

## `FileSystemFios_Construct` - `0x8101472a`

**Confidence: 80** (static decompile; the open order is also seen live, below).

Tagged `Filesystem/FileSystem_Fios.cpp` (line 0x79, in its `"fios cache"`
allocation), vtable `PTR_FUN_81014b1e_1_815084f4`. It initialises Sony's FIOS2
(`SceFios2_774C2C05`), copies its `param_2` into `DAT_815780e8` flipping `\` to
`/` - the mount point, `app0:PSP2/` as `Game_Main` passes it (string at
`0x8141a918`, xref `0x81003e0a`) - then builds five archive paths with
`FUN_8101440c` (mount point + name, unnamed: below 70) in this order:

| # | path |
| --- | --- |
| 1 | `app0:PSP2/data.psarc` |
| 2 | `app0:PSP2/data1.psarc` |
| 3 | `app0:PSP2/data2.psarc` |
| 4 | `addcont0:DLC1W2048PACKAGE/PSP2/dlc1.psarc` |
| 5 | `addcont0:DLC2W2048PACKAGE/PSP2/dlc2.psarc` |

It then asks `SceFios2_DF3352FC` (mount buffer size) for each and, for every one
that exists, `SceFios2_C4822276` (`sceFiosArchiveMountSync`, the NID Omega's
[`psarc-mount.md`](../ps4-omega-eu/psarc-mount.md) names) with the **same**
mount-point buffer `DAT_815780e8`, in the same order 1 to 5. `SceFios2_B309E327`
(`sceFiosIsValidHandle`) follows each. A missing archive (no DLC installed) is
skipped without a trace of its own.

`0x81014308` (`FileSystemFios_CreateOnce`, confidence 70) allocates 0x68 bytes,
calls this once behind a `DAT_815192f4 == 0` guard and sets `DAT_81578168`.

The mount order is **seen live too**: Vita3K, which loads the game's own
`libfios2.suprx`, logs `sceIoOpen` of `app0:/PSP2/data.psarc`, `data1.psarc`,
`data2.psarc`, then `addcont0:/DLC1W2048PACKAGE/PSP2/dlc1.psarc` and the DLC2 one,
in that order (`data/scratch/title-patches/data2-corrupt.log`).

## Which copy answers a path all of them carry

The game does not decide this: all five share one FIOS2 mount point, so the
lookup is `libfios2`'s. Observed on Vita3K (EU v1.04, 2026-10-07; the same
`libfios2.suprx` is shipped in the package) with a copy of the installed game
whose `Data/plugins/frontend/NEWGUI/Definition.xml` - carried by all three of
`data`, `data1`, `data2`, with 30,227, 31,974 and 34,908 bytes - had its first
48 compressed bytes overwritten in **one** archive at a time:

| archive corrupted | result at 50 s |
| --- | --- |
| none (symlinks) | Game Mode screen |
| none (the archive copied, not corrupted) | Game Mode screen (control: a copy alone is harmless) |
| `data.psarc` | Game Mode screen, unchanged |
| `data1.psarc` | stuck on the first logo |
| `data2.psarc` | stuck on the first logo |

**Measured (confidence 85): the base archive is shadowed for that path by the
patch**, the shadowed copy is never read. **Not resolved: `data1` against
`data2`.** Corrupting either stops the game, so both copies are read for this
file - by FIOS2 itself or by something in the game that opens both; the
experiment cannot say which wins a collision. The dependence is a fact about
`Definition.xml`, not shown for another path.

What is chosen, not measured: `data2` ahead of `data1`, because it is the later
mount and the later revision (the three-way text paths grow with every
archive). The DLC packs are mounted last, so by the same reading they would also
shadow the patch; this project keeps them behind the base, as it did before,
because their order against the patch is untested and the four circuits both
carry are byte-different in 290 of 312 shared paths.

The numbers behind it, from `data/scratch/title-patches/overlap.txt` (SHA-1 of
the inflated entry): `data1` and `data` share 753 paths and 734 differ; `data2`
and `data` 1,132 and 1,122; `data1` and `data2` 570 and 552; `dlc1` and `data`
312 and 290; `dlc1` and `data1` 172 and 172; `dlc2` and `data2` 16 and 0.

## Omega

Checked, not determinable: Omega's `PsarcArchive_WaitAndMountAll` mounts
`data%02d.psarc` per PlayGo chunk, and whether its patch archives overlay the
base is `libSceFios2.prx`'s rule, with no PS4 emulator to observe it
(`oag_omega::EXTRA_CANDIDATES` remains **chosen, not measured**). The same
`SceFios2` pattern (one mount point, `sceFiosArchiveMountSync` per archive) is
what 2048 shows, which is the reason to read both as one rule, not a measurement.
