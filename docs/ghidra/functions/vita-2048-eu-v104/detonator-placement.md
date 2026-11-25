# Detonator placement on 2048: HD's law, checked structurally

The law, its evidence and every field are on HD's page,
[ps3-hdfury-eu/detonator-placement.md](../ps3-hdfury-eu/detonator-placement.md). This page
records what was checked on `eboot.elf` (`PCSF00007`, v1.04), 2026-11-22, static only, with
`llvm-objdump -d --triple=thumbv7-none-eabi --start-address=0x8117cf22 --stop-address=0x8117e9c2`
(Ghidra not used).

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x8117cf22` | `Detonator_PlaceStageObjects` | 70 |
| `0x8117cb9c` | `Detonator_SpawnObject` | 60 |

`0x8117cf22` is the placer `FUN_8117e9c2` (the Detonator update,
[ps3-hdfury-eu/detonator-mode.md](../ps3-hdfury-eu/detonator-mode.md)) calls with the old stage.
What matches HD's `0x000654e0`:

- the placed-this-call counter cleared on entry: `str r4(=0), [r0, #0x9626c]` at `0x8117cf44`
  (HD `rm+0x6de48`);
- the top-up and bomb spacing `5.0` (`vmov.f32 s0, #5.0`, `0x8117cf2c`);
- group stride `0xd60` (`0x8117d414`-`0x8117d420`, `mla` by `0xd60` at `0x8117d4b4`), cluster
  stride `0x44` (`0x8117d53c`-`0x8117d548`), track-entry stride `0x30`;
- the free-lane value `3` tested on the top-up and bomb draws (`cmp r0, #3`, `0x8117e0e0`,
  `0x8117e186`) with `movs r2, #3` also at `0x8117d08c` and `0x8117d6fa` (not traced to the survey's lane fill);
- four calls to `Collision_SweepSegment` (`0x8118b8a0`, [own-craft-box.md](own-craft-box.md))
  as on HD's survey (two width probes, the centre probe, the per-lane probe);
- three calls to one helper, `0x8117cb9c`, where HD calls `Detonator_SpawnObject` three times.

The executable carries HD's loader strings (`data\xml\DetonatorLayouts\PatternFile.xml`,
`GamePlayValues.xml`, `%s.xml`, `MineCountStage%i`, `stageToAppearFrom`,
`percentageAcrossTrack`, `NumStepsToIgnoreAfterJump`), and `PSP2/data.psarc` ships both files:
`GamePlayValues.xml` byte-identical to HD's, `PatternFile.xml` the same content with CRLF line
endings.

**Checked, applies, not wired.** Structural (strides, constants, call shape), not read line by
line: the loaders, the spawn's body and the pool sizes are not read here. The `_q` on the spawn
follows from its confidence: it is named from the call count alone.

## The fork walk (2026-11-25, `det-mine-branch-placement`)

The step `0x81214f6a` is called 12 times; selector `2` at `0x8117d058`, `3` at `0x8117d6fa` and
`1` at `0x8117d366` and `0x8117d9c8` match HD's fork walk
([ps3-hdfury-eu/detonator-placement.md](../ps3-hdfury-eu/detonator-placement.md#the-fork-walk)).
The filter between them was not read line by line. **Ported**: 2048's Detonator races run the
same survey builder, `tests::mine_branch::a_2048_forked_circuit_lays_mines_on_its_branch`
(DLC1 `zone_3`). Confidence 65.

