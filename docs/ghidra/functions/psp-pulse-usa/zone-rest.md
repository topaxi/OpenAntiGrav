# Zone's four-corner hover epilogue, and which handling a Zone craft flies

Pulse PSP (USA) `BOOT.BIN`, 2026-10-02, lane `pulse-zone-rest`. Headless decompile and disassembly plus a live PPSSPP
1.20.4 run. No new function names: `Ship_HoverFourCorner` (`0x0884ae90`) and `Ship_UpdateHover` (`0x0884870c`) were
already named.

**The mode was patched, not chosen.** A fresh profile cannot pick Zone (greyed), so every live read below is a Time
Trial on `16_Track` (Talon's Junction, Feisar-class Venom, one craft) with `g_game_mode` (`0x08b31048`) written to `6`
while the CPU was stopped. `Ship_UpdateHover` re-tests it every frame, so the four-corner variant runs from the next
frame; a native Zone race was not watched.

## `Ship_UpdateHover` picks the variant every frame

```c
void Ship_UpdateHover(int craft) {
  craft->flags_1c0 &= ~1;
  if (g_debug_mode_override == 0 && g_game_mode == 6) Ship_HoverFourCorner(craft);
  else                                              Ship_HoverTwoPoint(craft);
  Ship_UpdateMagLock(craft);
}
```

Confidence **95**: the decompile, and the live run: a breakpoint inside `Ship_HoverFourCorner` (`0x0884b7b0`) hit on
every frame once `g_game_mode` was `6`, and that function is reached nowhere else.

## The epilogue's bank-to-yaw gain is `50.0`, not `30.0`

`Ship_HoverFourCorner`, `0x0884b760 - 0x0884b7ac`:

| Address | Instruction | |
| --- | --- | --- |
| `0x0884b760` | `lui a0,0x8a9` / `lv.q C400,0xa00(a0)` | the addend vector starts as the constant at `0x08a90a00`: read `(0, 0, 0, 1.0)` |
| `0x0884b76c` | `lw a0,0x2a4(s0)` / `beq a0,zero,0x0884b798` | grid state (`craft+0x2a4 == 0`): skip the term, add the zero vector |
| `0x0884b77c` | `lui a0,0x4248` | `0x42480000` = **50.0** |
| `0x0884b778 - 0x0884b794` | `f12 = craft[0x174] * 50.0 * (1.0 - craft[0x280])` | stored to the addend's `y` |
| `0x0884b7a8` | `vadd.t C230,C230,C600` | `craft+0x340 += addend` (the body-local angular accumulator) |

`craft[0x174]` is `right.y` and `craft[0x280]` is `magLockBlend`. The two-point law (`Ship_HoverTwoPoint`,
`0x0884ad2c`) has the same shape with `30.0`, with one difference in structure: there the whole `vadd` sits inside the
`!= 0` branch, here the `vadd` runs every frame and adds the zero vector on the grid. The result is the same.

**Live**, 30 frames on a banked start (`right.y = -0.01934 ... -0.01816`, `magLockBlend = 0`, state 1): the addend's
`y`, read from the stack at `sp+0x170` at `0x0884b7b0`, equals `50 * right.y * (1 - magLockBlend)` to `1e-7` on every
frame (`-0.96689` against `-0.96689`; `-0.90815` against `-0.90815`); 30.0 would have read `-0.58`. A second run, 12
frames after the craft had been disturbed (`right.y` up to `0.101`), agrees to `1e-6` (`5.0503` against `5.0503`).
The grid branch (state 0) was **not** watched: writing `craft+0x2a4 = 0` over a running craft did not stick (the state
word reads `1` on the next frame), so the skip rests on the instruction read and the zero vector alone.

Confidence **90** for the gain and the formula (instruction read, live delta on two runs); **78** for the grid-state
skip (instruction read only, the same guard watched live on the two-point law, `docs/physics/grid-state.md`).

Ported: `oag_physics::hover::BANK_TO_YAW_GAIN_FOUR_CORNER`, selected by `ShipState::four_corner`, which the race writes
from the mode each tick. The effect is `50/30` on the same torque: a banked Zone craft yaws 1.67 times as hard.

## Not ported: the other differences in the four-corner epilogue

Read in the same disassembly (`0x0884b7b0 - 0x0884b81c`), unported because this engine has no four-probe layout to
hang them on and the difference is small where all four probes touch:

- **Downforce**: `-(track_gravity * mass)` along the contact-normal vector, with **no groundedness factor and no
  `(1 - magLockBlend)`**. The two-point law multiplies both. `track_gravity` is read as
  `*(craft+0x6c) + class * 0x80 + 0x100` here (`class` = `DAT_08b31040`) where the two-point law reads
  `*(craft+0x70) + 0x6c`. They are the same float: `Ship_InitCraft` (`0x08849354`) sets
  `craft+0x70 = *(craft+0x6c) + class * 0x80 + 0x94`. Live: `80.0` through both paths, `mass = 1.0`, class `0`.
  Groundedness is `0.25` a probe in the four-corner function (`craft+0x2b0 += 0.25` per contacting probe in the
  function's probe loop), so four contacts make `1.0` and the omission only matters with fewer than four.
- The probe layout, the alignment torque and the rest of the 2000-line function were not compared.

## Which handling does a Zone craft fly? The player's own team and class

`docs/gameplay/race-modes.md` and `oag_title::race::ZoneCraft` recorded that every title ships a `Data\Ships\Zone_01`
handling file (`<Stats team="ZoneMode">`, no `<Class>` block) that this engine does not read, and asked which block
Zone is meant to use. **On Pulse that file is not used by anything.**

| Read | Result |
| --- | --- |
| `Zone_01` and `ZoneMode` as byte strings in `BOOT.BIN` (headless `findBytes`) | **no hit** for either. The file is reachable only by a composed name, and no composition produces it, see the next two rows |
| `Data\Plugins\PI001\Definition.xml` (Pulse) | eight `PI_Team`s, all `type="Race"` (the `Values` default); a **`PI_TeamModel name="Zone"`** with `location="zone01"` per team, no `PI_Team` for `Zone_01`, no team of `type="Zone"` |
| `FUN_088c2a38` (the `PI_Team`/`PI_Track` `Values` parser) | `type` is matched against a two-entry name table at `0x08ab1ad0` (the second entry's string is `"Zone"`, `0x08a81018`) into `team+0xa0`, the attribute Pure uses to mark `Zone_01`. No Pulse team carries `type="Zone"` |
| `Ship_LoadHandlingStats` (`0x088c291c`) | the stats path is `<team+0x94>\handlingstats.xml`, the team's own `Location`; one parse per team, once |
| `Ship_LoadModel` (`0x08843258`) case 6 | builds `%s\Zone.vex` from the **same** `team+0x94`: the player's team directory again |
| `Ship_InitCraft` (`0x08849354`, line `0x70`) | `craft+0x70 = *(craft+0x6c) + DAT_08b31040 * 0x80 + 0x94`: the player's class block, in Zone as in every mode (no `g_game_mode` test on this line) |
| Live, mode patched to `6` | `craft+0x6c = 0x08f4c784`, `craft+0x70 = 0x08f4c818` = `0x08f4c784 + 0*0x80 + 0x94`; class `0`; `track_gravity` `80.0` |

So a Pulse Zone craft flies **its team's own `<Class>` block for the race's speed class**, which is what this engine
already does. `Zone_01` is data from the Pure branch of the engine that Pulse's disc still carries. Zone replaces three
things *inside* that handling and takes nothing from a Zone file: the engine (the auto-speed law), the brakes
(disabled) and the hover variant (the four-corner function above). Confidence **82**: the executable is read, the
definition is read, and the live craft matches; not higher because a native Zone race was not watched and the mode
factory's team choice was not traced end to end.

Two limits worth keeping: this is **Pulse only**. Pure's definition marks `Zone_01` as `type="Zone"` and a Pure Zone
race presumably reads it; HD's `/data/ships/zone` is the same shape. Neither executable was read.

## What sounds an opponent's destruction makes (answer to the `_BLOWUP` open question)

`_BLOWUP` (`~BLOWUP`) is played through `FUN_0883e9b0`, a wrapper that tests `craft+0x368 == 0` (and that the race
manager is in state 2) before `Sound_PlayNamedInSlot`, so an opponent plays nothing there: confirmed by reading the
wrapper (`0x0883e9b0`, a clean decompile). The sounds of the *sequence* an opponent goes through were logged live on a
Single Race grid (`16_Track` start area, a neighbouring track's pose, Venom, 8 craft; `Ship_SetState(entity, 4)` injected
on a grid opponent at a `Ship_UpdateCraft` stop, as `scripts/psp-wreck-capture.py` does, with a breakpoint on
`Scream_PlaySoundByName` `0x08991b20` and, in a second run, on `Sound_Play` `0x089392b0`; the opponent was also moved
15 units from the player in two of the runs, to rule out a distance cull):

| Frames after the injection | What | Path | Opponent | Local player |
| --- | --- | --- | --- | --- |
| 0 | `~BLOWUP` | `FUN_0883e9b0`, gated on `craft+0x368 == 0` | **nothing** (not in the log window; by the wrapper) | held loop |
| 29 (state 5 entry, 0.5 s) | `EXPLSMALL` | `FUN_0883e064` `0x0883e150`: `craft+0x368 == 0` ? dry `EXPLSMALL_PC` : `Sound_Play(emitter = entity+0x50, "EXPLSMALL")` | `Sound_Play` entered, emitter flags `0`, an instance is queued; **no `Scream_PlaySoundByName` followed in 6 s, in three runs, 15 units away included** | `EXPLSMALL_PC` dry, logged at 29 |
| 119 (state 6 entry, 1.5 s later) | `EXPLBIG` | `FUN_088407b0` `0x0884082c / 0x088408e4`: dry `EXPLBIG_PC` or `Sound_Play(..."EXPLBIG")` | `Sound_Play` entered with the emitter's `+0x5c` bit 0 **set** and `param_6 == 0`, so it returns without queuing (`Sound_Play`'s first test); nothing started | `EXPLBIG_PC` dry, logged at 119 |
| 167-168 (state 6 expiry) | **`cont_elim`** | `FUN_08840500` (`0x08840590`), `Sound_PlayNamedInSlot` dry at `0x400` | **plays**, in the `speech.bnk` the mode opened, in modes other than 2, 8 and 18 | plays (logged at 239 for the player, whose state 6 runs longer) |

So **the audible sound of an opponent's destruction in a Single Race is the announcer line `cont_elim` and nothing else
that was seen**: the explosion sounds are positional through an emitter, and on this evidence they never start. The
opponent's `EXPLSMALL` absence is the weak half (confidence **65**: three runs and a mechanism not read - the queued
instance's start is deferred to an emitter update nobody traced); the `EXPLBIG` drop is read in `Sound_Play`'s first
`if` and watched once (**80**); `cont_elim` is three live logs and a decompile (**90**). `cont_elim` exists in both
`speech.bnk` and `speech_elim.bnk`; the frame (167-168 from the injection) is `0.5 + 1.5 + 0.8 = 2.8 s` of states 4, 5
and 6, to the frame.

**Wired 2026-10-02 (`pulse-elim-credit`).** `Cue::ContElim` (`"cont_elim"`) loads with `Ready`/`Go` from the
mode's speech bank (`Cue::COUNTDOWN`), and `Race::tick_wreck_voice` (called from `tick_destroyed_craft`) raises it
2.3 s (`WRECK_VOICE_DELAY`) after a non-player craft is first seen `Eliminated`, once per wreck, in every mode
except `Mode::Eliminator`. The mode gate: game modes 2 (`Demo`), 8 (`Elimination`) and 18 (`Multiplayer
Elimination`) are the exclusions, by `g_game_mode_names` in [state-machine.md](state-machine.md); only 8 exists
here (it is `Mode::Eliminator`). No `EXPLSMALL`/`EXPLBIG` for an opponent. Pinned by
`race::tests::wreck_voice` (tick, once per wreck, not Eliminator, not the player, not an unvoiced title) and, on
the disc, by `wreck_voice_ground_truth` (Single Race, an opponent wrecked at tick 300: one cue at tick 438, and the
third run of speech in the speech-only WAV (not kept in the repository) starts at tick 438, 1.5 s long).
The 2.3 s is the live 2.8 s less the 0.5 s this build's `Eliminated` has already spent in state 4.
