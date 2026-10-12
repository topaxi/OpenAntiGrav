# The opponent driver on 2048: HD's, with four laws of its own and two planner extensions

**Binary:** `vita-2048-eu-v104` `eboot.elf` (ARM Thumb-2, image base `0x81000000`,
Ghidra program `/2048/eboot-vita-2048-eu-v104.elf`). Read 2026-10-12 by the
`orig-ai-2048` lane, which flies 2048's original opponent in 2048's campaign. The
template is HD's map, [ps3-hdfury-eu/ai-controls.md](../ps3-hdfury-eu/ai-controls.md),
which this page compares against piece by piece; [ai-branch-choice.md](ai-branch-choice.md)
already holds `Ai_Construct` and `Ai_ChooseBranch`. Constants were read off the
instructions with a capstone pass over `0x81198000..0x811a2000` that resolves
`movw`/`movt` pairs and literal-pool loads; Ghidra's decompile of the NEON code was
used for structure only.

**The answer in one paragraph.** 2048's opponent is HD's: the same functions in the
same order, the same thrust law (finished spread `75`, wander to `8 - place`,
`WhenBehind` dropped in an Eliminator, the round robin flagging racer `1` first), the
same rail, pitch, lookahead, airborne term, steering constants and halved-lookahead
gate. What 2048 adds is keyed on **which lineage the game mode is**: `Ai_Construct`
stores `GameMode_IsHdLineage()` (mode id below `0x17`) at `driver+0x98`, and every
2048 event has a CRC mode id above it (`0x9b3bd491` read live), so in 2048's own
campaign (1) every opponent runs its thrust law, speed floor and lane choice **every
tick**, (2) the class tables come from the `*2048.xml` set, (3) the planner's tuning
comes from the event's `m_aiPadTrackingSkill` and `m_aiWeaponAvoidanceSkill`, (4) the
lane search follows a per-driver coin and a well-scored choice moves the preferred
lane, and (5) the planner adds **weapon-pad tracking** and **weapon avoidance**. The
first four are ported; the fifth is not.

Confidence is per row. **Measured** means read live on Vita3K (below).

## The cast

| Address | Name | Confidence | HD counterpart | Verdict |
| --- | --- | --- | --- | --- |
| `0x81198708` | `Ai_Construct` | 80 (its row and page are [ai-branch-choice.md](ai-branch-choice.md)'s; the additions below are read at 85) | `Ai_Construct` (`0x000fee30`) | HD's, plus `driver+0x98 = GameMode_IsHdLineage()`, the coin `driver+0xc8 = rand() % 2`, four `rand()`-derived values at `+0x2a0..+0x2ac` whose readers were not followed, and a tail call to `Ai_SetEventTuning`; see [ai-branch-choice.md](ai-branch-choice.md) |
| `0x8119853c` | `Ai_SetEventTuning` | 90 | none | new, [below](#the-event-tuning) |
| `0x8119fa02` | `AIManager_Update` | 85 | `AIManager_Update` (`0x000bb960`) | HD's: snapshot (`FUN_8119f254`, not read), the once-a-race shuffle, `mgr+0xa8` incremented then wrapped at `8` and that racer's `+0x988` set, `AI_UpdateCraft` on every driver, then the weapon AIs |
| `0x8119f0dc` | `AIManager_ShufflePack` | 85 | the shuffle block of `0x000bb960` | HD's: `numAIPositionSwaps` of `AIPackSwapping<level+1>` adjacent swaps by `rand() % (n - 1)`, `mgr+0x3c = -10000`; the record is `RM + 0x114dc` (2048 lineage) or `RM + 0x10e60` (HD lineage) plus `class * 0x118`, the level `g_GameState+0xe0` |
| `0x8119dda4` | `AI_UpdateCraft` | 88 | `AI_UpdateCraft` (`0x00102d70`) | HD's order, [below](#ai_updatecraft-0x8119dda4) |
| `0x8119a36a` | `AI_ComputeControls` | 82 | `AI_ComputeControls` (`0x00100658`) | HD's law, [below](#the-steering-controller) |
| `0x81199dc8` | `AI_ComputeOpponentThrust` | 94 | `AI_ComputeOpponentThrust` (`0x000fdf00`) | HD's law, measured, [below](#the-thrust-law) |
| `0x811995cc` | `AI_UpdateSpreadWander` | 82 | `AI_UpdateSpreadWander` (`0x000fdd18`) | HD's, finished branch included (`0x811995d6..0x811995f8`) |
| `0x8119fb5e` | `AI_ResolveSkillScale` | 90 | `AI_ResolveSkillScale` (`0x000babf8`) | a 2048 event's `m_skillScale`, else HD's per-circuit stats, [below](#the-skill) |
| `0x8119c700` | `Ai_PlanLane` | 72 | `Ai_PlanLane` (`0x00101b50`) | HD's skeleton, extended, [below](#the-planner) |
| `0x8119c0e0` | `Ai_TrackWeaponPads` | 65 | none | new: bumps the nine candidates toward weapon pads ahead |
| `0x81198d9c` | `Ai_LanePointOnRing` | 75 | `Ai_LanePointOnRing` (`0x000ff850`) | HD's, plus an avoidance sidestep under a flag argument |
| `0x81199172` | `Ai_SnapToLane` | 85 | `Ai_SnapToLane` (`0x000ff9b0`) | identical constants (`50`, `0.01`, `7`, `-1/7`, `0.7`, `1e-6`, `0.25`, `0.6`/`1.5`); calls the lane point with the flag clear |
| `0x81198c1c` | `Ai_ComputePitch` | 80 | `Ai_ComputePitch` (`0x000fe510`) | identical constants (`0.9`, `40`, `-2500`, `+/-100`) |
| `0x811a0b18` | `AiStats_Load` | 88 | `AiStats_SetDefaults` and the parser | header `950`, `80`, `0.002`, `23000`, `10`; five classes of `0x118`; the base set or the `*2048.xml` set by its flag |
| `0x8117053c` | `RaceManager_Construct` | 80 | - | loads both sets: `RM + 0x10e4c` (base) and `RM + 0x114c8` (`*2048.xml`) |
| `0x8100193c` | `GameMode_IsEliminatorFamily` | 65 | the mode test in `0x000fdf00` | mode `8` or `0x14`, or the current game mode object flagged `1` or `4` |
| `0x812b5890` | `GameMode_Current` | 85 | - | returns `g_CurrentGameMode` (`0x81965b58`), the event (`GameModeBase`) a race was launched from; `m_name` read `"2048 - Event 4"` through it live |
| `0x812afd9e` | `GameModeBase_Construct` | 75 | - | the defaults: `m_skillScale = 2.0` (`+0x1a8`), `m_aiPadTrackingSkill = m_aiWeaponAvoidanceSkill = 1.0` (`+0x1f4`/`+0x1f8`, `0x812b007c..0x812b008a`), `event+0x1a4 = 1` |
| `0x812b40de` | `GameModeBase_PostLoad` | 60 | - | clamps both AI skills to `[0, 1]` (`0x812b4152..0x812b41ba`) |
| `0x812b0d52` | `GameModeBase_Apply` | 80 | - | the mode id from `m_name`, the class from `m_speedClass` to `g_GameState+0xd8`, laps, and **`g_GameState+0xe0 = event+0x1a4`**: the AI level is `1` in every event |
| `0x81965b58` | `g_CurrentGameMode` (data) | 85 | - | read live |

Not named, hypotheses: `0x8119b7b6` (called by the planner after the pad tracking,
reads `driver+0xbc`; weapon avoidance, confidence 45); `0x81199800` (the thrust law
`AI_ComputeOpponentThrust` calls instead of its own when `GameMode_IsEliminatorFamily`
is set and the mode is not HD lineage, constants `200`, `-50`, `5.0`; an
Eliminator-family law of 2048's own, 40); `0x810006fc` (a mode test on ids
`0x10..0x16` or a game-mode flag `4`, which picks the several-humans reference craft;
40).

## `AI_UpdateCraft` (`0x8119dda4`)

Skipped (controls zeroed) under HD's four conditions. Then, in HD's order:

1. The ring advance (`(int)(racer+0x8e8 * 4000)`, the same wrap), inlined.
2. `dt * 60` into `0x8151fc14`.
3. First call: the race manager and the class record, **`RM + 0x114dc + class * 0x118`
   when the mode is not HD lineage**, `RM + 0x10e60 + ...` when it is.
4. **When `driver+0x98` is clear, `racer+0x988 = 1`** (`0x8119de7a`): the racer's turn
   flag, which `AI_ComputeControls` gates the thrust law on and `Ai_PlanLane` gates
   the lane choice on, is set every tick. HD has no such store.
5. `driver+0x288 = 1.0`; the player's flag `driver+0x1a4`.
6. The halved-lookahead state `driver+0x5bc` from `racer+0x88` and the two flags at
   `+0x5c0`/`+0x5c1`, HD's windows (`4600`/`250`, `5400`/`400`).
7. The excluded fork path to `craft+0x662c`.
8. `LookAheadSecs` and `SteerMul` from the record; in mode `0xd` (Duel) HD's override
   and rail gap.
9. The smoothed speed `0.95 * old + 0.05 * |v|` (`driver+0x26c`).
10. `Ai_PlanLane`, `Ai_ChooseBranch`, `AI_ComputeControls`.
11. Steer and both airbrakes times the authority (`driver+0x140`).
12. Thrust only when `racer+0x988` is set; HD's speed floor (`SplinePt+0x5e < 35`,
    `-0.002715 x^2 - 0.869027 x + 95.11588`), not for the craft at `RM+0x10e48`.
13. HD's Eliminator block, on `GameMode_IsEliminatorFamily` and `craft+0x58dc == 2`:
    a spin about the up axis at `pi` over `craft+0x5640 -> +0x80 -> +0x4b4`, started by
    `AI_ComputeControls` (below). Not ported, as on HD.
14. `Ai_SnapToLane` unless the Duel gap runs, then `Ai_ComputePitch`.
15. The 20-tick steer average.

Confidence 88: the order and every gate read; step 4 measured.

## The steering controller

`AI_ComputeControls` (`0x8119a36a`) takes the lookahead time in `s0` and the driver,
the output record and the steering state in `r0..r2`. Against HD's:

- the airborne term `clamp((|h| - 15) * 0.025, 0, 1)`; the own-lane error from the
  centre-shifted point (`racer+0x8f4`/`+0x8f8` less `+0x8fc`); the lookahead time
  `LookAheadSecs + air`, halved under `driver+0x5bc`; `d = max(t * smoothed, 15)`,
  `p = own + d / lap * 1.5`, walked `0x95` steps at most; the aim shift `0.7`/`0.3`
  times `xtrackMul` (`record+0x0c`); the velocity lead `0.0676`; the yaw prediction
  `-0.0206`; `steer = (err + derr * SteerDamp) * 950 * SteerMul`; the airbrakes
  `(|steer| - 100) * 80 - 10`. **All HD's.**
- **The steering gain also carries `1 + 2 k^2`**, `k = driver+0x160`, the weapon
  avoidance's 3-second decay (below). With no avoidance `k` stays `0` and the gain is
  HD's.
- The lane point is read with a flag argument that is set unless the chase byte
  `driver+0xcc` is: under it, and while `k > 0`, the lane is pulled toward the craft's
  own lateral position less `driver+0x15c * 12` units, weighted `1 - (1 - k)^2`
  (`Ai_LanePointOnRing`).
- **A chase mode** under `driver+0xcc`: the aim point becomes the player's position
  plus `0.338` of its velocity, and when that point is behind (`dot < -0.4`) and at
  least `45` units away the Eliminator spin is started (`driver+0x20 = 0.5`). The thrust
  law has a branch of its own for it. `driver+0xcc` read `0` on every driver live; its
  writer is outside `0x81198000..0x811a2000` and was not found.
- The player's own driver (autopilot, `driver+0x1a4` set and `driver+0x98` clear) uses
  airbrake gains `1.0`/`0.0`, a `1.73 * |x|` lookahead floor and an extra `libm` call on
  the error: the pilot assist's steering, not an opponent's
  ([pilot-assist.md](pilot-assist.md)).
- Thrust: `AI_ComputeOpponentThrust` on the turn flag, the split-screen multiplier
  after it (`g_GameState+0xe8 > 1`), as on HD.

Confidence 82.

## The thrust law

`AI_ComputeOpponentThrust` (`0x81199dc8`) is HD's line for line at the same record
offsets: `AIThrust` at `+0x68`, `SpreadDist` at `+0x88`, the skill points at `+0xd0`,
`+0xe8`, `+0x100`, the rubber band at `+0xa8..+0xbc`; constants `75`, `100000`,
`1000 - 50 t` floored at `100`, `2 - 0.08 t` floored at `0.5`, `+/-0.3 AIThrust`, the
`100` cap after `20` s within `350` units, the floor `1`. HD's four changes are all
there (finished spread `75` built at `0x81199f90`; the wander's finished branch; the
`WhenBehind` term skipped under `GameMode_IsEliminatorFamily`; several humans' reference
craft when `g_GameState+0xe8 > 1`). The racer table is at `0x8189d8d8`, stride
`0x990`: `+0x04` driver, `+0x7a` finished, `+0x88` lap distance, `+0x890` progress,
`+0x96c` place, `+0x988` the turn flag. Driver fields: `+0x16c` spread, `+0x170..`
the wander, `+0x194` gap, `+0x198` the dead-band accumulator.

**Measured, confidence 94** (below): 528 of 528 predicted samples exact.

## The skill

`AI_ResolveSkillScale` (`0x8119fb5e`): when `GameMode_Current()` is set, its
`m_skillScale` (`+0x1a8`); else HD's per-circuit, per-class, per-level stats value.
HD's mode-9 and mode-3 additions and the HD-campaign interpolation follow. It is
called on every evaluation of the law, so a write to `m_skillScale` mid-race takes
effect at once (used below). `SP.xml` authors `M_SKILLSCALE` on 56 of 141 events
(`0.2` to `4`); the other 85 read the constructor's `2.0`. The level the shuffle
reads, `g_GameState+0xe0`, is the event's `+0x1a4`, `1` from the constructor and
authored by no field. Confidence 90.

## The event tuning

`Ai_SetEventTuning` (`0x8119853c`), called at the end of `Ai_Construct`. Under an HD
lineage mode it stores HD's constants; otherwise it reads the event's two AI skills,
`a = m_aiPadTrackingSkill` and `b = m_aiWeaponAvoidanceSkill`:

```text
driver+0x9c = a; driver+0xa0 = b
c = clamp(max(a, b), 0, 1)
driver+0xc4 = 0.001 + 0.00065 * (1 - (1 - c)^2)    // the ring rate; HD 0.001
driver+0xa4 = 150 + 250 a     // HD 150
driver+0xa8 = 150 - 120 a     // HD 150
driver+0xac = 500 - 200 a     // HD 500
driver+0xb0 = 0.2 + 0.3 a     // HD 0.2
driver+0xb4 = 0.3 + 0.55 a    // HD 0.3
driver+0xbc = 300 + 525 b     // HD 300 (1.5 b * 350)
driver+0xc0 = 0.5 + 0.5 b     // HD 0.5
```

`+0xa4..+0xc0` are read by the pad tracking (`+0xa4`/`+0xa8` its distance window) and
the avoidance (`+0xbc`); only the ring rate reaches what is ported. `SP.xml` leaves both
skills empty on 135 events and authors `0` on 5; empty reads the constructor's `1.0`
(measured), so a 2048 event's ring rate is `0.00165`, 65 % above HD's. Confidence 90.

## The planner

`Ai_PlanLane` (`0x8119c700`, about 1,260 lines of decompile) keeps HD's skeleton: the
authority ramp (`driver+0x140`, up `1.5 dt` toward `1`, the same per second as HD's
`0.025` a 60 Hz tick, and down `0.75 dt`), the contact
push `+/-0.2` per contact clamped `+/-2` and decaying `0.95`, the nine candidates and
the `0.9`/`0.1` smoothing, the inlined bumps (`0.0475`, `3.1667`, `0.3333`), the
preferred lane on `rand() % 400 == 125`, the authored band through `1/127.5`, and the
ring fill's `0.9`, `1.1` and `15` steps and `0.5 / rate` brake, **all built from the
per-driver rate `driver+0xc4`** where Pulse uses `0.001`. The ring holds `0x50` slots.

What 2048 adds:

| Piece | Where | Ported |
| --- | --- | --- |
| The turn flag set every tick, so the scoring and choice run every tick | `AI_UpdateCraft` step 4 | **yes**, `Laws::every_tick` |
| The ring rate from the event's skills | `driver+0xc4` | **yes**, `Laws::event_ring_rate`, `oag_ai::original::event_ring_rate` |
| The choice searches from the own-lane candidate with `-1` as the score to beat, in two passes whose order is the coin `driver+0xc8` (set: up first), the up pass through candidate `8`, strict `>`; a score at or below `-1` ends a pass | `0x8119d534..0x8119d632` | **yes**, `Laws::coin_ordered_search` |
| The coin redrawn, `rand() % 2`, on `rand() % 200 == 1`, every call | after the `400` draw | **yes**, same law |
| When the chosen candidate's smoothed score is above `0.5`, the preferred lane becomes `1 - chosen^2` on the chosen side | `0x8119dcc4..0x8119dd04` | **yes**, `Laws::preferred_follows_choice` |
| Weapon-pad tracking: for the pads ahead (`DAT_818a4ae0`, count `DAT_818a255c`) inside `driver+0xa8..+0xa4`, skipped while the craft holds a weapon, a bump into the nine candidates scaled by `1 - k` | `Ai_TrackWeaponPads` | no |
| Weapon avoidance: a loop over threats (stride `0x40`) that sets `driver+0x160 = 1` (decaying by `dt / 3`), `driver+0x15c` and `driver+0x148`; the sidestep and the steering gain above read them | `0x8119b7b6`, `0x8119cce2..0x8119cfe4` | no |
| A re-seed when the craft's track section changes and the corridor or the slot jumps by more than `7.5` units or one candidate | `0x8119c964..0x8119ca8c` | no |

Two ported details rest on the instructions alone: the coin's draw order across slots
(chosen: one draw a slot at construction, in slot order) and the NEON reciprocal the
fill's `0.5 / rate` is built from (two Newton steps on `vrecpe`), which may differ from
a true division by an ulp. Confidence 72 for the planner as read; the pad tracking and
avoidance were read for their inputs and outputs only.

## Measured (2026-10-12)

Vita3K `0.2.1` (the Arch `vita3k r4095` Qt build), EU `PCSF00007` v1.04 with both DLC,
a private pref-path whose save is a copy of the maintainer's, audio `0`, on a headless
weston with a rooted Xwayland (`:92`). The tool is `scripts/vita3k-mem.py`: a daemon
that starts Vita3K as its own child, serves `/proc/<pid>/mem` reads and writes, and
`ai-sample` reads the racer table, all eight drivers, the race time (`RM+0x10e3c`,
`RM = *0x8189bf3c`) and the class record inside one `SIGSTOP`. The event was
**"2048 - Event 4"**, a Flash race on Unity Square, three laps, the player parked on
the grid in eighth; 45 samples 1.5 s apart from the green light to 66 s.

| Read | Value | What it settles |
| --- | --- | --- |
| `g_GameState+0xe4` | `0x9b3bd491` | a 2048 event runs a CRC mode id, not HD lineage ([ships-effects.md](ships-effects.md) had it at 70) |
| `driver+0x98`, all 8 | `0` | `Ai_Construct`'s lineage byte |
| `racer+0x988`, every opponent in every sample | `1` (308 of 308) | the every-tick flag |
| class record | `RM + 0x115f4` = `RM + 0x114dc + 1 * 0x118` | the `*2048.xml` set, class 1 |
| `g_CurrentGameMode -> m_name`, `m_skillScale`, `+0x1f4`, `+0x1f8` | `"2048 - Event 4"`, `1.0`, `1.0`, `1.0` | the skill as `SP.xml` authors it; empty AI skills read `1.0` |
| `driver+0xc4`, all 8 | `0.0016499999910593033` | the ring rate law |
| `g_GameState+0xe0` | `1` | the level |
| `driver+0xcc`, all 8 | `0` | the chase mode is off in an ordinary race |
| `driver+0x16c` at the grid | `8 - grid` | HD's spread |

The thrust law, predicted in `float32` in the executable's operation order
(`scripts/vita3k-mem.py predict`), against the throttle each driver had just written
(`driver+0xdc`):

| Run | Skill | Predicted | Exact (`< 1e-3`) | Worst residual |
| --- | --- | ---: | ---: | --- |
| `unity-park` | `1.0` (the event's own) | 264 | **264** | `0.00049` |
| `unity-skill3` | `3.0` (written into `m_skillScale` at the intro) | 264 | **264** | `0.00049` |

Two things the prediction has to know:

- **The samples need the race time one 30 fps frame behind `RM+0x10e3c`.** Before
  `20` s the step gain reads the time; with `t - 1/30` every sample is exact, with `t`
  eighteen were off by `0.08/30` times their gap. The cause is not settled: either the
  law reads a lagged clock, or every stop landed after the clock's increment and before
  that frame's AI update, so the stored throttle is the previous frame's. HD's
  breakpoint capture needed no lag. Nothing in the port carries a lag.
- **The drivers listed before the reference racer read its spread from the tick
  before**, because the reference's own update refreshes it later in the same tick. One
  such driver per sample gives that value back from its logged gap and is not counted
  (44 per run); every other driver is predicted with it.

The reference racer (seventh, one place ahead of the parked player) read **`63.0`** on
every sample at skill `1.0` and **`74.0`** at `3.0`; at `3.0` the upper stop
(`100.4`) was reached in the first seconds. Not exercised: the finished branches, an
Eliminator, several humans, the speed floor (the parked player held every opponent
back, so none fell below it), the steering, the planner's choice and the rail.

To reproduce, with a Vita3K of your own (copy the config with `pref-path` pointed at a
directory of yours and `audio-volume: 0`):

```sh
export OAG_VITA3K_CACHE=<dir> OAG_VITA3K_CONFIG=<dir>/config.yml
python3 scripts/vita3k-mem.py serve --display 92 --socket wayland-oag92 &
# walk in: python3 scripts/vita3k-drive.py --display 92 --socket wayland-oag92 key/tap/shot
python3 scripts/vita3k-mem.py ai-sample --count 45 --gap 1.5 --out run.json
uv run --python 3.12 --with numpy python3 scripts/vita3k-mem.py predict run.json --skill 1.0
python3 scripts/vita3k-mem.py quit
```

The walk from a progressed save: `Return`, `x` three times 4 s apart, tap `(295, 162)`
twice (Single Player Campaign), tap the event's node, `(882, 480)` Play, wait for the
load, `(882, 480)` on the mode card and again on the intro to start.

## Flown in this project

`oag_2048::race::ORIGINAL_DRIVER` carries HD's four branches and 2048's four laws;
`oag_2048::race::EventAi` resolves an event's skill, its two AI skills and the level
over the constructor's defaults; a 2048 campaign event flies the original field
(`RaceDefaults::original_opponent`). `crates/game/tests/vita_2048_original_ai_ground_truth.rs`
holds: the event's AI as read live, the reference racer's `63.0` and `74.0`, more than
one throttle moving per tick, the `*2048.xml` set equal to the base set (all five
classes; Super Phantom differs only in whitespace), and the field finishing one race
event on each of nine circuits with the player parked (Arena with up to three respawns
a craft), and an Elimination event keeping this project's driver. **Sol** is not flown in the test: every craft respawns in a loop on lap 1
under this project's own driver as much as under the original's, a rescue fault the
driver does not cause. Arena's respawns were not compared with this project's driver:
that run had weapons destroying craft, the original field's did not. **Super Phantom**
events (20) keep this project's driver: `oag_tables::handling::SpeedClass` has four
classes. **Elimination** events (26) keep it too: `AI_ComputeOpponentThrust` hands a
2048-lineage Eliminator-family mode to `0x81199800`, which is unread, and flying HD's
Eliminator branch in its place would be a substitute. `GameMode_IsEliminatorFamily`'s
game-mode flags `1` and `4` may cover event kinds beyond Elimination; which is unread.

**Chosen, not measured: the rate.** 2048 runs a race at 30 fps and this engine at
60 Hz. With `every_tick`, each per-call term of the planner (the `0.9`/`0.1` score
smoothing, the `rand() % 400` and `rand() % 200` draws, the `0.95` push decay) runs
twice as often a second as it does on the Vita. The thrust law keeps no state between
calls, the ring fill is per slot of distance and the authority ramp per second, so
those three are unaffected.

## Omega: checked, applies, not wired

Omega's `eboot.bin` (`/omega/eboot-ps4-omega-eu.bin`), static. Its
`AiManager_AddDriver` (`FUN_012c80c0`) inlines 2048's `Ai_Construct` whole: the
lineage byte as `DAT_01f999e4 < 0x17`, the coin `rand() % 2`, the four `rand()`-derived
values, the `05_Track`/`20_Track` flags, and 2048's event tuning off the current game
mode's `+0x2ac`/`+0x2b0`: `(1 - (1 - c)^2) * 0.00065 + 0.001` (the only reader of
`0.00065` at `0x01801390`), `150 + 250 a`, `150 - 120 a`, `500 - 200 a`, `0.2 + 0.3 a`,
`0.3 + 0.55 a`, `300 + 525 b`, `0.5 + 0.5 b`. Its strings carry `AIControlStats2048.xml`,
`AIRaceStats_<class>2048.xml`, `m_aiPadTrackingSkill` and `m_aiWeaponAvoidanceSkill`.
So Omega's opponent is 2048's in a 2048-lineage mode and HD's in an HD-lineage one.
**Verdict: checked, applies, not wired**: `oag_omega`'s `original_opponent` stays
`false` until its race is complete enough to fly it
([formats/omega-status.md](../../../formats/omega-status.md)); not measured (no PS4
emulator).
