# Detonator placement on Omega: HD's law, line for line

The law, its evidence and every field are on HD's page,
[ps3-hdfury-eu/detonator-placement.md](../ps3-hdfury-eu/detonator-placement.md). This page
records what was read on the base (v1.00) `eboot.bin`, 2026-11-22, static only. The SELF's
segment 0 sits at file offset `0x50f0` (its entry table at `0x20`, readable as shipped in this
dump); `objdump -D -b binary -m i386:x86-64 --adjust-vma=0x1000000` over that segment
disassembles it at image base `0x01000000`.

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x012a2300` | `Detonator_PlaceStageObjects` | 75 |
| `0x012a3aa0` | `Detonator_SpawnObject` | 70 |

**Read line by line, `0x012a3720`-`0x012a3a97`** (HD's steps 2 to 5): the eligibility gate
(`stageToAppearFrom - 1 > stage` skips, `0x012a3747`), the `numOfObjects` sum and `idiv` by the
eligible count, the stage count `*(0x0193fc10 + stage*4)` less the mine manager's live count
(`*(*(0x01a15ee0)+0xa8)+0x718`) when the flag byte `0x0193fc00` is set, else less the
placed-this-call counter `rm+0x9b4f4`, the `chunk = total / n_groups` placement with
`rand() % group_count` and the `pos > total - 1` stop, `type == 0` gating the spawn, the top-up
loop (entry `rand() % total`, lane `rand() % lanes - lanes/2`, accepted only when
`0 <= lane < lanes` by one unsigned compare, the entry usable and `laneArray[lane] == 3`, spawn
with count `1`, spacing `5.0` at `0x01801004`, kinds `{0}`), and the one bomb (same draw without
the bounds test, kinds `{1}`). The same lane quirk as HD's follows.

Layout changes from HD, all pointer width: group stride `0xd68` (group count `rm+0x9b4d0`,
clusters still `0x44`), track entries `0x40` bytes (`*(rm+0x9b500)`, count `rm+0x9b4f8`; lane
count `+0x00`, lane array `+0x08`, usable `+0x10`). The start-zone radius is `150.0`
(`0x01801000`, compared at `0x012a370a`). Rand is `0x017a59f0`. Four calls to `0x01257f00`
match HD's four survey probes.

Two differences in signature and head: the function takes the stage as a second argument
(`esi`, kept in `r12d`/`r15d`), which Ghidra's `FUN_012a2300(rm)` drops; and its head, gated by
the byte `0x01a05a18`, sets `+0x74 |= 4` on every live mine and puts every live bomb's lane back
to `3` (`0x012a2338`-`0x012a23e0`), the `RemoveMinesEachStage` behaviour HD hides in
`0x0012ae38`. The disc's `GamePlayValues.xml` (in `data00.psarc`, byte-identical to HD's) has
`RemoveMinesEachStage="0"`; that the byte is that attribute is not read here.

**Checked, applies, not wired.** The loaders and the pools are not read on this binary; the
strings and both files are the same as HD's.
