# The Eliminator flip, HD and Fury (2026-11-07, `eliminator-spin`)

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`), TOC `r2 = 0x008ad4d8`. Live: RPCS3, EU disc,
Racebox Eliminator, Vineta K, Feisar, Novice, weapons on, `GetMode()==8`, player input only (held
cross, scripted L1 and stick through `rpcs3-drive.py`), no Autopilot.

The disc calls the move **the flip** (`FE_FLIP`, `OPT_ELIM_FLIP`, `BADGE_NAME_MOST_FLIPS`). The player
presses a button, the craft yaws half a turn in `0.2 s`, the camera follows it, and the craft keeps its
velocity. Thrust then pushes against the travel, so it brakes hard, stops, and flies the wrong way
down the circuit at the oncoming field. Pressing again turns it back. No cost, no cooldown.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x000bd3e8` | `Options_GetButtonBinding` | 90 | `lwzx r3, r3, (action*4 + 0x40)`, reached with `0x0098d430 + pad * 0xc4` from every action read in `PlayerInput_Update`. Read live: `[6, 7, 4, 9, 5, 10, 11, 12, 8, 0]` for actions `0..9`, and action `8`'s value `8` is the button that fired the flip. |
| `0x006770c8` | `Pad_WasPressed` | 90 | The edge read: `PlayerInput+0x65` (use, edge) and `+0x68` (action 8) come from it, `+0x66` (use, held) from `0x00677308`. Live: L1 held for `2.5 s` flipped once. |

## Input

Action `8`, `Options_GetButtonBinding(pad, 8)`, on the edge. Default binding: button index `8`,
**L1**. Live confirmation: a tap of L1 started the flip at `t = 10.009`, `16.007`, `20.050` and
`26.506`; L1 held from `20.0` to `22.5` flipped once. Confidence 90.

The binding table, button indices in action order: thrust `6` (cross), use `7` (circle), absorb `4`
(square), reverse camera `9` (R1), change camera `5` (triangle), left airbrake `10` (L2), right
airbrake `11` (R2), voice `12`, flip `8` (L1). The controls page's `value="L1"` on the airbrake row
is that row's placeholder, not the default.

Strings, all in the Fury archives:

- `DATA06` `languages/english/entries.xml`: `FE_FLIP` = `FLIP`; `FE_FLIP_INST` = "Take the battle
  right to your opponents! In an Eliminator event, hit the L1 button (defult controls) to flip your
  ship through 180 degrees to attack your rivals head on!" (the typo is on the disc);
  `MAN_3_ELIMINATOR`: "In a brand new feature you can now flip your ship around 180 degrees (L1
  button in default controls) to fly in the face of the opposition and take the battle straight to
  them."; a controls line "Flip your ship - L1 button (default)"; `OPT_FURY_CTRL` = `FLIP/ EMP/
  INVALID LAP`; `BADGE_NAME_MOST_FLIPS` / `LEAST_FLIPS`.
- `DATA06` `controls_definition_ps3.xml`: `ButtonMapper name="Eliminator Flip"`, `buttonAction="8"`,
  label `OPT_FURY_CTRL`. `DATA02` and `DATA03`'s copies stop at action `7`.
- `DATA05` and `DATA06` carry `OPT_ELIM_FLIP` = `ELIMINATOR FLIP`, which no front-end XML of
  `DATA02`/`03`/`05`/`06` names.
- `DATA03`'s English file has no `FLIP` string. `DATA02`'s has no `FE_FLIP`.
- `DATA00` ships the art: `fe/images/flip.gtf`, `flip_fury.gtf`, `manual/flip_fury.gtf`.

**Behind the Fury package?** The rebind row and every string are Fury's. The executable's own gate has
no Fury test: the block reads only the mode, `0x009384e1` (read `0` live, the byte the loading screen
and the stage loader also read, not the Fury flag at `0x00b979fd`), the race phase and the craft flag.
So the code runs in any mode `8`; whether a disc without Fury content binds action `8` is not
checkable on this one disc (`g_FuryContentFlags` read `00 01` here). Confidence 84.

## The law (player, `PlayerInput_Update`, `0x000ce274..0x000ce660`, `0x000cec20..0x000cec98`)

```text
p = PlayerInput
if *0x009384e1 == 0 and mode in (8, 0x14):                 // Eliminator, Multiplayer Elimination
    if Pad_WasPressed(binding(8)) and !p+0x90:
        if RM+0x1970 == 2 and !(craft+0x6a64 & 0x1000):    // racing, not out of the race
            p+0x90 = 1                                     // flipping
            p+0x94 = 0                                     // angle
            p+0x80 = body+0x1f0                            // forward held at the press
            p+0x91 = craft+0x6a58 >= 0                     // which way round
    if p+0x90:
        p+0x94 += dt * (p+0x91 ? -5pi : +5pi)              // 0x8a8a40, 0x8a8a44
        if p+0x94 >= pi or p+0x94 <= -pi:                  // 0x8a8a48, 0x8a8a4c
            p+0x90 = 0;  p+0x94 = clamp(p+0x94, -pi, pi)
        body+0x1f0 = normalise(rotate(p+0x80, about body+0x1e0, p+0x94))
        p+0x4c = 0                                         // steer, zeroed; pitch is not
```

| Claim | Value | Evidence | Confidence |
| --- | --- | --- | ---: |
| Rate | `5 pi` rad/s, so `pi` in `0.2 s` | floats at `0x8a8a40/44`; `0.5236` per step at `dt = 1/30` in the log | 90 |
| The way round | stick centred or right: `angle < 0`; lean negative (stick left): `angle > 0`. `cross(f0, f1) . up` has `angle`'s sign, so it turns toward the lean | `0x000cec84`; `dir 1` at lean `0`, `dir 0` at lean `-1.14`, cross `-0.864` and `+0.909` | 90 |
| Steering | zero while flipping | `stw r9, 0(r22)` at `0x000ce62c`; `steer -99.87` before, `0.00` during | 90 |
| Velocity | untouched by the block; `body+0x190` is not written | the block's only body store is `0x000ce628`; `fw.v` goes through `0.51`, `-0.93`, `-0.99` with `|v|` falling, not turning | 90 |
| Heading base | the forward held at the press; up is the current one | `p+0x80` saved once, `body+0x1e0` loaded each tick | 88 |
| Refused | phase not racing, craft flag `0x1000`, a flip already running | `0x000cec3c`, `0x000cec4c`, `+0x90` test; refusal itself not run live | 84 |
| Not gated | speed, shield, airborne, weapon: no read of any in the block; flips at `|v|` from `75` to `117` all ran | block read in full | 84 |
| Cost, cooldown | none. A flip at `t = 16.0` after one at `10.0`, one at `20.0`, one at `26.5`; `+0x90` is the only guard | live | 88 |
| Facing flag | `craft+0x6ad8`, below | live | 90 |

What follows is the ordinary thrust against the travel, measured on a held throttle (wall clock, the
emulator ran about `0.8` of real time): `|v|` `117.7` at the press, `40.8` `0.25 s` later, `6.8` at
`0.85 s`, through zero at about `1.2 s`, then the wrong way up the circuit, `-85` at `3.4 s`, `-109` at `5.9 s`.

A counter at `0x000ce648..0x000ce65c` (`*(0x0004d7e0()) + 0x98 += 1`) runs per flipping tick on a human
craft with `+0x7a60 == 0`. It feeds no law; the Most Flips badge is not traced.

## `craft+0x6ad8` is a facing flag, not the flip

The lead's reading was that this byte might be the move's state. It is not. `Ship update (0x000eadb8)`,
`0x000ecb78..0x000eccfc`, every tick in mode `8` or `0x14` and only while `*0x009384e1 == 0`:

```text
craft+0x6ad8 = dot3(body+0x1f0, craft+0x7840) < -0.4      // threshold vector at 0x00781da0
```

`craft+0x7840` is unit length in every sample and tracks the track's direction (`fw.tr` `0.95` flying
the right way, `-0.95` after a flip): a tangent. Its other readers are the magstrip wake
(`0x001092e0`, `0x00109578`, `MagstripWake_UpdateRibbons`). Live the byte rose at `fw.tr = -0.971`
and fell at `0.274` on the way back, and opponents' bytes rise on their own flips. It also rises on
any spin-round, so a crash facing backward counts. Confidence 90.

Its consumers: `Hud_UpdateRaceLine` turns each marker by pi (`0x0008a208` for opponents, `0x0008a584`
for the player) and `WeaponAi_DecideFireOrAbsorb` refuses the absorb.

## The HUD

The player's marker on the bottom bar turns to point left, frame `run1` `after1` against `before`: the
bar's chevron reverses. That is the whole of it. **`ReverseHUD` is not this:** the group is
`HUD_reverse_view.xml`, the look-back camera's HUD (`hud+0x4f0`, read only by `0x0008fb18`); the frames
of a flip show the ordinary HUD. Confidence 88.

## The AI's flip

`AI_UpdateCraft` step 11 ([ai-controls.md](ai-controls.md)) is the same move, same constants at
`0x8a96e4..0x8a96f0`, on a craft with `+0x628c == 2`, which `ai-controls.md` measured as "opponent".
Live: opponents flip. Six reversed spells in `140 s` (craft 1, 5, 1, 0, 5, 4, 6, 5 of the eight), `2.2 s`
to `11 s` of wall clock. Confidence 90 that they flip, 84 on the chain below.

`driver` fields: `+0x20` flipping, `+0x21` way round (`craft+0x6a58 >= 0`), `+0x24` angle, `+0x28` a
timer, `+0x10` block the forward held.

```text
each tick, mode 8 or 0x14:
    if driver+0x28 > 0 and !driver+0x20:  driver+0x20 = 1, +0x24 = 0, hold forward, set +0x21
    if driver+0x20:
        driver+0x28 -= dt
        +0x24 = clamp(+0x24 +- 5pi dt, -pi, pi)
        if |+0x24| reached pi and +0x28 <= 0:  driver+0x20 = 0
        write the rotated forward; steer, airbrakes, thrust zero
```

So an opponent's timer outlasts the turn and it holds that attitude with its controls at zero, unlike
the player's. The trigger is `WeaponAi` ([weapon-ai-fire.md](weapon-ai-fire.md)):

1. `WeaponAi_DecideFireOrAbsorb`, fire passed, `+0x78` set, the held weapon not Bomb or Mine, the target
   in `10 < d < 150` (Quake `400`) or `+0x50 == 0`: `rand01() < (+0x50 == 0 ? infrontFlip : normalFlip) *
   flipScale[diff]`, always true on the authored numbers. On it: `driver+0x28 = 1.5` for LeachBeam,
   Missile, Plasma, Cannon, else `0.75`; `+0x7c = 1.2` for LeachBeam or Missile, else `0`; `+0x78 = 0`;
   no fire.
2. Next pass, `+0x78 == 0`, `|driver+0x24| >= pi`, `+0x7c` run out: use pressed and held
   (`0x000fa0c0`, the controls' `+0x19` and `+0x1a`), `+0x79 = 1`.
3. `WeaponAi_Update` (`0x000fb9fc`): with no weapon held, `+0x78 = 1`; and if `+0x79` and the driver is
   not flipping, `+0x79 = 0`, `driver+0x28 = 0.1` (`0x8a9490`): it flips back.

**Correction to [weapon-ai-fire.md](weapon-ai-fire.md):** controller `+0x1a` is not a flip request. The
controls record is `PlayerInput+0x4c` (`craft+0x5fac -> +0x84` read `0x30837bdc` on a `PlayerInput`
at `0x30837b90`), so `+0x19` is the use edge `+0x65`, `+0x1a` the use hold `+0x66`, `+0x1b` absorb
`+0x67` and `+0x1c` action `8`'s edge `+0x68`, the same button's `EMP` and `INVALID LAP` uses.

## The camera: open

A burst through a live flip (`data/reference/hd-eliminator-flip/original-burst.png`, local) shows the
hull spinning on screen while the chase camera turns far less, then the view lands on the reversed
heading with the hull centred. Ours keeps the chase camera rigid behind the hull, so for `0.2 s` it
swings round the craft and can pass through a trackside wall. The settled frames agree. The
camera's law is unread: the reader of `PlayerInput+0x90` or `+0x94` is the next address to find.
No hull-side effect or sound of the flip was looked for.

## Ours

`oag_title::Title::eliminator_flip` is `0.2 s` on HD and `oag_raceplay::score_law::of` resolves it
once at load into `Setup::flip_law`, `None` outside an Eliminator. `oag_physics::flip` is the turn;
`Race::tick_flips` starts it on a person's `Button::Flip` press, past the countdown, on a craft in
the race. `Button::Flip` is bit 10 of the masks a replay and the wire already carry, so neither
format moves and a replay without the move reads as before; the flip enters the state hash only once a
craft has flipped.

- L1 on a pad raises both `L` and `Flip`, because our pad's shoulders are the airbrakes on every other
  title. Where the race has a flip, `InputSnapshot::shoulder_flips` reads the press as the flip
  alone. `F` on a keyboard, and a row in the bindings page. **Chosen, not measured:** the key.
  Touch has no flip button.
- The lean is `ShipState::steer_lean_follower`, HD's `craft+0x6a58` follower.
- The race line's marker turns on `Race::facing_back`, the test `wrong_way_kill` runs. The original's
  frames of a flip show no `WRONG WAY`, so a race with a flip raises none. That is read off frames,
  not off the code that drives the warning.
- `crates/raceplay/tests/hd_flip_ground_truth.rs`: ours fell through `41.0`, `22.4`, `9.1`, about
  zero at `1.0 s`, against the original's `40.8`, `20.1`, `6.8`, about zero at `1.0 s` of game time. No
  tuning: it is the velocity left alone and the engine HD already has.

**The AI's flip is not wired.** Its chain is read above, but it needs the weapon AI's per-weapon
range tests and a fire on the far side of the turn. On ours the absorb gate on the facing byte stays
unused for the same reason: no opponent turns round.

## Per title

| Title | State | Answer |
| --- | --- | --- |
| HD / Fury | one disc, one executable | **wired** |
| 2048 | **checked, differs, not wired.** `PlayerInput_Update` (`0x811b3072`) has the block: the gate is the mode test **or** the handling table's `race_eliminator_flip` (`handling+0x4b8`, `0x811b363c`), the rate is `pi / handling+0x4b4` (`0x811b36c6..0x811b36e4`), the attribute `eliminator_flip_time` under `<Misc>` (`0x811ab9aa`), default `0.2` (`0x3e4ccccd`, `0x811ab8e4`). So HD's `5 pi` is the default. A second trigger, `input+0x99` with a stick value below `-0.97` (`0x811b369c`), is unread. Confidence 80 |
| Omega | **checked, differs, not wired.** The same two attributes in its `eboot.bin` (`0x01814b5e`, `0x01814b73`, read by `FUN_012d8e30`), and its `data00.psarc` ships them: on the five `*2048` teams' handling files, class 1 `0.4` on every team, class 3 `0.2`, class 4 `0.2`, Qirex `0.4`, Auricom `0.5`, AG-Systems `race_eliminator_flip="true"`, class 2 unset. Ours has no per-craft value to read them into yet. The reader on Omega's own `PlayerInput_Update` was not opened. Confidence 75 |
| Pulse | **absent.** No flip string, no tunable, no `5 pi` word in the PSP or PS2 executable. The state's ancestor exists: Pulse's `craft+0x1c0 & 0x2000` is this same `dot < -0.4` test, and there it kills the engine (`oag_raceplay::wrong_way_kill`). HD turned the punishment into the move: `CraftLaws::eliminator_wrong_way_kill` is false on HD |
| Pure | **absent.** The same census on its executable |
