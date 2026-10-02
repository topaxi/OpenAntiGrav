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
