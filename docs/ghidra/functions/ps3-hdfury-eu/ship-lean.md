# The drawn steering lean (Wipeout HD / Fury)

Read and measured 2026-10-21 (`ship-lean`). HD's craft is drawn rolled about its nose by the
same angle Pulse's is, `lean * 0.5 + roll * 6.28`, and the lean runs Pulse's filter with two
gains read from the team's `handlingstats.xml` and a faster smooth. The Pulse side is
[`../psp-pulse-usa/camera.md`, "The drawn lean"](../psp-pulse-usa/camera.md#the-drawn-lean-entity0x854-read-and-measured-2026-10-21).

| Address | Name | Confidence | Evidence |
| --- | --- | --- | --- |
| `0x000c58d8` | `Handling_ReadTilt` | 85 | the `<Tilt>` reader: one string dispatch over `internalSpeed`/`internalAmount`/`shipSpeed`/`shipAmount` (TOC `-0x4e28`..`-0x4e1c`, strings at `0x00781248`..`0x00781278`) storing to the team block's `+0x14`/`+0x10`/`+0x1c`/`+0x18` (`0x000c5a28`, `0x000c5a54`, `0x000c5a84`, `0x000c5ab0`); live, Feisar's block reads `[1.4, 7.0, 1.4, 6.0]` at `+0x10..+0x1c`, its file's `internalAmount="1.4" internalSpeed="7" shipAmount="1.4" shipSpeed="6"` |
| `0x000eadb8` | `Ship_UpdateEntity` | 72 | the ship entity's per-frame update (`r31` the ship, `ship+0x5fac` the craft): both lean filters (`0x000eb528`-`0x000eb650`), the sideshift input (`bl 0x000d9e18`), the display-matrix angle (`0x000ebb3c`). Pulse's `Ship_UpdateEntity` (`0x088418e0`) does the same three |

## The filters, `0x000eb528`-`0x000eb650`

`rec` is the craft's control record (`*(craft+0x84)`, stick `+0x00`, left airbrake `+0x08`,
right `+0x0c`, each `+/-100` or `0/100`); `team` is `*(craft+0x78)`; `dt` is `f26`. TOC
(`r2 = 0x008ad4d8`) constants: `-0x49f8` `0.015`, `-0x49f4` `0.6`, `-0x49f0` `0.3`, `-0x49ec`
`-0.6`, `-0x49e8` `-0.3`, `-0x49e4` `-0.005`, `-0x49e0` `0.005`, `-0x49dc` `0.01`, `-0x49d8`
`0.0`, `-0x49d4` `0.5`, `-0x49d0` `6.0`, `-0x49cc` `-0.5`, `-0x49c8` `10.0`.

```c
/* The cockpit camera's lean, ship+0x6a48 follower, ship+0x6a44 lean. */
target = rec[0] * 0.015 * team[+0x10];             /* <Tilt internalAmount> */
if (ship[0x6ac8] > 0) target -= 0.5;               /* the two sideshift timers */
if (ship[0x6acc] > 0) target += 0.5;
/* clamp d to 0.6 / 0.3 by the raw stick's side, as Pulse */
ship[0x6a48] += d * (dt * team[+0x14]);            /* <Tilt internalSpeed> */
ship[0x6a44] += (ship[0x6a48] - ship[0x6a44]) * (dt * 6.0);

/* The drawn hull's lean, ship+0x6a58 follower, ship+0x6a54 lean. */
target = (ship[0x6a64] & 0x1000) ? 0.0
       : rec[0] * 0.01 * team[+0x18]               /* <Tilt shipAmount> */
         + rec[+0x08] * -0.005 + rec[+0x0c] * 0.005;
d = target - ship[0x6a58];
/* d > 0: 0.5 if rec[0] > 0, else 0.3; d <= 0: -0.5 if rec[0] < 0, else -0.3 */
ship[0x6a58] += d * (dt * team[+0x1c]);            /* <Tilt shipSpeed> */
ship[0x6a54] += (ship[0x6a58] - ship[0x6a54]) * (dt * 10.0);
```

Against Pulse's `Ship_UpdateSteerLeans`: the drawn lean is the same filter (same `0.01`,
`0.005`, `0.5`/`0.3` limits, raw-stick side test, `0x1000` gate) with the stick term scaled by
`shipAmount`, the follower at `shipSpeed` (Pulse `5.0`) and the smooth at `10.0` (Pulse `3.0`).
HD has no `& 2` negate on either filter. The camera lean differs more (`0.015 * internalAmount`,
`+/-0.5` sideshift terms where Pulse has `+/-10`, smooth `6.0`, follower `internalSpeed`); ours
still runs Pulse's camera lean on HD, which this lane did not change.

## The display matrix, `0x000ebb3c`-`0x000ebc6c`

```
f13 = ship[0x6a90] * 6.28        ; TOC -0x4714 = 0x008a8dc4 = 0x40c8f5c3, the roll's eased phase
f1  = ship[0x6a54] * 0.5 + f13   ; TOC -0x49d4 = 0.5
bl  0x006776e8                   ; the rotation, then multiplied into the body's rows (r29)
                                 ; and stored to the node at *(ship+0x6adc)
```

Pulse's `0x08842140` angle exactly. A raw scan of the ELF for every load at displacement
`0x6a54` (Ghidra's own instruction search missed this one) finds it the lean's only reader
besides its two writers (`FUN_000eadb8` and `FUN_000cf0c8`, a second copy of both filters) and
the constructors. The follower `0x6a58` has two more: `PlayerInput_Update` (`0x000cec84`) and
`Hud_Update` (`0x0009e710`-`0x0009e754`), which offsets two HUD widgets by the follower, so
HD's HUD sways with the steer (not ported, open).

## Measured on RPCS3, 2026-10-21

Time Trial, Talon's Junction, Feisar, from `hd-timetrial-talons-venom-start`, under player input
(`scripts/rpcs3-hd-air-hold.py --script`, cross held, the stick's x and L2/R2 through the
virtual pad; L1/R1 leave the record's airbrakes at `0`, so L2/R2 are the airbrakes). Settled
values of `ship+0x6a54`:

| Input | Settles at | Law |
| --- | --- | --- |
| stick right (record `99.867`) | `1.3981` | `0.99867 * 1.4 = 1.3981` |
| left airbrake | `-0.4998` | `-0.5` |
| right airbrake | `+0.4999` | `+0.5` |
| stick + right airbrake | `1.8981` | `1.8981` |
| stick + left airbrake | `0.8981` | `0.8981` |
| both airbrakes | `0.0000` | `0` |

The cockpit lean settled at `2.0971` under the stick, `0.99867 * 0.015 * 1.4`, and stayed at
`0` under either airbrake. Replaying the drawn filter frame by frame on the race clock
(`craft+0x308`) over the 1,559 frames (`data/traces/hd-talons-steer-lean-airbrakes.csv`) gives
RMS `0.0020`, worst `0.046` at the input edges (the rows are polled from a running emulator, so
record and lean can sit either side of a frame's update). Pulse's follower `5.0` gives RMS
`0.045`, Pulse's smooth `3.0` `0.137`, and `shipAmount` left out `0.217`; pinned by
`crates/trace/tests/steer_lean_ground_truth.rs`. **Confidence 90** for the drawn lean's law.

**The sign, measured the same day.** `scripts/rpcs3-hd-lean-display.py` holds the stick and
reads the hull node, `*(ship+0x6adc) + 0x80` (`0x00323780` is the `+0x80` accessor), against the
body's rows. With the stick right, lean `+1.3981`, roll `0`, angle `+0.6991`: node row 0 in the
body's basis `(+0.5774, +0.4784, -0.0021)` = `0.75 * (cos, sin, 0)`, row 1 `(-0.4784, +0.5774)`,
row 2 `0.75` on the nose; with the stick left the mirror, `(+0.5741, -0.4823, 0)`. So HD's
`0x006776e8` builds the same rotation Pulse's VFPU code does, `row0 = 0.75 * (cos * body_row0 +
sin * body_row1)`. `crates/raceplay/src/drawable.rs`'s
`hds_lean_turns_the_hull_the_way_its_captured_display_matrix_does` reproduces the captured row 0's
world height through our own composition (sign and size, within `0.02`) and fails with
`ROLL_DIRECTION` flipped. Confidence 92 for the sign on HD.

**Ported**: `oag_physics::SteerLean` per craft, filled from `<Tilt>` by
`oag_raceplay::load::code_laws` under `CraftLaws::tilt_steer_lean` (`oag_hd::race::CRAFT_LAWS`,
measured), drawn through `oag_render::roll::display_angle`.

**Withdrawn**: the "about 15 degrees of visual banking" in
[rpcs3-capture.md](../../../reverse-engineering/rpcs3-capture.md) was a teleport with no input,
and is the craft's yaw in the frame, not a roll: this filter reads input alone and is `0` there.

## 2048 and Omega

- **Omega: checked, applies, inherited.** Its teams author `<Tilt>` and its parser reads the
  four names (`shipAmount` at `0x018145aa`, referenced from `FUN_012d8e30`, which stores the
  previous one at `+0x1c`). The consumer was not located: the two `-0.005` constants' readers
  (`0x01261f2c`, `0x012bbfd7`) are other code. Live: not checkable (no PS4 emulator).
  `oag_omega::race::CRAFT_LAWS` inherits HD's law.
- **2048: checked, applies, inherited.** The data ships `<Tilt>` in every team file (HD ships at
  `1.4`/`6`, 2048's own at `1.3`/`5`) and the executable carries both strings (`0x814900d8`,
  `0x814900e4`); the consumer was not located. `oag_2048::race::CRAFT_LAWS` inherits HD's law.
