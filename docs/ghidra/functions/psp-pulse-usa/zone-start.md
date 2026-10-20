# Zone's start, and the track definition's `+0x16e`

Pulse PSP (USA) `BOOT.BIN`, 2026-10-02, lane `pulse-zone-start`. Headless decompile plus a live PPSSPP 1.20.4
run.

## `Ship_UpdateEngine` (`0x0884c5c8`): Zone's auto-speed has no state-0 branch of its own

The brief's address `0x0884c8c8` is inside the function, in its tail: the only `craft+0x2a4` read there is the turbo
add (`(flags & 0x200 || flags & 0x400) && craft+0x2a4 == 1`). Zone (`g_game_mode == 6`, `g_debug_mode_override == 0`)
replaces the throttle target with

```text
local_10 = ((flags & 1) && !(flags & 2)) ? g_autospeed_base + g_autospeed_step * craft+0x28c : 0.0
...
local_10 = local_10 * craft+0x294 * 2.0
```

Bit 1 of `craft+0x1c0` is the grid state (`Craft_EnterGridState`, `0x088486d4`, `|= 2`; see
[grid.md](grid.md)). So the countdown writes `0.0` and the craft stands, and the launch multiplier `craft+0x294` is
in the same tail. Confidence **88**: read, and watched live.

**Live** (g_game_mode written to 6 at the first racing frame of a restarted race; `scripts/psp-launch-boost.py
--game-mode 6`, rows from `Weapons_DispatchFire`). `g_autospeed_base` = 34.0, `g_autospeed_step` = 4.4,
`craft+0x28c` = 0 at the start.

| | coasting | accelerate held |
| --- | --- | --- |
| flags, state 0 | `0x3`, speed 0.02 units/s for 1800 frames | `0x3`, 0.02 |
| first state-1 row | `0x1`, mul `1.4`, thrust 0 (the engine has not run) | same |
| next row | thrust `95.2` = 34 * 1.4 * 2, grade 0 | edge, grade 1, mul `1.2`, then thrust `81.6` |
| mul after 60 frames | `1.0`, thrust `68.0` = 34 * 2 | `1.0`, `68.0` |
| speed, 28 frames after state 1 | 42.11 | 36.34 (ratio 1.159) |

Caveat: the mode was patched on a Single Race grid, not a native Zone race (a fresh profile cannot pick Zone, and a
restart that loads under mode 6 never finished loading). `Race_UpdateLaunchGrade` (`0x0882773c`) and
`Ship_UpdateStartBoost` (`0x0883fdec`) read no game mode, so the grades should hold in a native race.

## `0x088c3f7c` (`FUN_088c3f7c`): `TrackDefinition_ReadXml`

Reads a `<Track>` element of `Data\Plugins\PI001\Definition.xml` into a definition object: `Location` to `+0x94`,
`Reversed` to `+0x16d`, **`availableInZone` to `+0x16e`** (`Xml_AttributeAsBool` at `0x088c40a8`-ish), `collisionCageEnabled`
to `+0x16f`, `SoundRegister` to `+0x160`. Confidence **78** for the name, 90 for the three byte offsets.

## `TrackSelection_PopulateList` (`0x088edf3c`): the `Mode == 6` byte is `availableInZone`

The cached `Mode` (`param_1 + 0xd8`) is `GameMode_FromName(&0x08b30f90, <front-end "Mode" global>)`, an index into
`g_game_mode_names`, where 6 is `Zone` ([state-machine.md](state-machine.md)). The list admits a circuit when
`Definition_IsUnlocked(def, 1)` and, in mode 6, `def->+0x16e != 0`, which `TrackDefinition_ReadXml` fills from
`availableInZone`. So the gate is the sixteen `availableInZone="true"` circuits, which this port already filters
(`shell.tracks_for(Mode::Zone)`). Confidence **85** (static, two independent sites:
`TrackDefinition_EnterScreenState` and `TrackSelection_ApplySelection` test the same byte with the same mode). Not
run live: Zone is greyed on a fresh profile. The earlier claim "the byte is on exactly three circuits" is not
supported by anything read here. `+0x99` (always-hidden) was not touched.

## The Autopilot's speedup reaches the auto-speed too (2026-10-20)

Lane `zone-05-shield`. The four-corner branch (`0x0884c834`-`0x0884c868`: `lw a1,0x28c(a0)`,
`g_autospeed_step * zone + g_autospeed_base`) ends in the branch-delay `add.s` and falls into
`0x0884c86c`, the same `swc1 f12,0x0(sp)` the throttle path stores to. Everything after it is
shared: `flags & 4` -> `T *= craft+0x2a0` (`0x0884c874`-`0x0884c894`), the turbo/roll-payout add
on `flags & 0x200 || flags & 0x400` with `craft+0x2a4 == 1` (`0x0884c898`-`0x0884c914`), then the
launch multiplier at `0x0884c918`. Read off the disassembly (Ghidra, `/pulse/BOOT-psp-pulse-usa.BIN`).

**Live**, PPSSPP 1.20.4 software renderer, `time-trial-de-konstruct-black-venom-grid.ppst`
(forward `05_Track`), `g_game_mode` written to 6 and the Autopilot armed one frame after GO
(`scripts/psp-zone-autopilot.py`): `craft+0x328` reads **`81.6` = `34 * 1.2 * 2`** on 1,499 of
1,500 logged frames, with `craft+0x294` at `1.0` and the zone counter `craft+0x28c` at `0`. The one
`0.0` (frame 448) is one of the 34 half-grounded frames at the ramp's lip, the branch's own
`(flags & 1)` ground gate. The `1.2` is not a held launch grade: that one lives in `craft+0x294`,
which reads `1.0` here. So the x1.2 Autopilot speedup ([autopilot.md](autopilot.md)) multiplies the Zone target.
Confidence **88** (two sources: the control flow and the live word).

Ported for that one term (`oag_physics::engine::engine`, the auto-speed branch applies
`EngineFlags::speedup`). The speed it gives, the same autopilot on the same circuit at zone 0:

| x along the run-up | original | ours before | ours after |
| --- | ---: | ---: | ---: |
| about -370 | 123.5 | 113.6 | 124.7 |
| about -490 | 125.2 | 115.0 | 126.2 |

The turbo/roll-payout add on the same tail is **not ported** to the Zone branch: a static read only,
and nothing fires a Turbo in Zone; a Zone barrel roll's payout would reach it. Open.

Cross-title: HD's Zone engine (`Craft_UpdateThrottle`, `0x000efb50`) is not this shape (its Zone mask
only ramps `craft+0x2d4`, and it reads no `+0x2f0`), so HD's Zone keeps flying Pulse's auto-speed with
the speedup **inherited from Pulse**; it reaches only an Autopilot in Zone, which Zone never hands out
(the `--autopilot` aid alone): **checked, differs** (HD's engine), HD's Zone inherits Pulse's.
**Pure: checked, applies** (static): its `Ship_UpdateEngine` (`0x0892f1f0`) has the same mode-6
branch (`g_autospeed` base plus step times `craft+0x244`) falling into the same `flags & 4` multiply,
by `craft+0x258` there. Pure's mode 6 being Zone is the zone-hover thread's open question, so this is
static only. **2048 and Omega: not checkable here**: no Zone race of either runs in this engine yet.

**The fork re-commit, per title** (the driver half of this lane, `docs/gameplay/ai.md`). Route counts
of every Zone circuit as loaded here: HD `zone_1`, `zone_2`, `zone_4`, Vineta K 0, `zone_3` 1;
Pure `01_Zone`, `02_Zone`, `04_Zone` 0, `03_Zone` 1. Where there are no routes the failure cannot
happen: checked, not reachable. HD `zone_3` under the original's driver ends 7,200 ticks at shield 100.
Pure has no original driver (`original_driver` unset), so its Autopilot is this project's driver and
the re-commit stays plan-gated there; `03_Zone` ended at 94.6 to 98.2, no deck failure seen.
**Pure's Zone drains shield on every circuit** (02_Zone 93.9 before this lane's speedup and 77.4 after,
01_Zone 91.6 and 81.3), gradually and on circuits with no fork: a different class, open.

## On 05_Track a Zone craft falls short of the first jump in the original too (2026-10-20)

Same capture, with the player's driver's fork decision written to the ring (`Ai_ChooseBranch`'s
`ai+0x58 = 1`, excluded path `ai+0x60 = 1`, [ai-branch-choice.md](ai-branch-choice.md); the
driver's own coin took the lower deck on every unpatched run, so on this fork the junction's
primary successor is the lower deck): the craft reaches the ring's ramp at 121-125 units/s,
leaves it half-grounded at y 41 (`craft+0x2b0` 0.25-0.5 for 34 frames), lands on the lower deck
and follows it, **0 contact frames and the shield unchanged over 1,500 frames**. A patch of the
coin instruction itself (`0x08854a24`) changed nothing: PPSSPP keeps the compiled block.
