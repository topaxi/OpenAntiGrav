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
/* d bounded only while the raw stick pushes it (see "The cockpit lean, measured") */
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
`+/-0.5` sideshift terms where Pulse has `+/-10`, smooth `6.0`, follower `internalSpeed`, and a
different step bound, below); ported 2026-10-21 as `oag_physics::CameraLean`.

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
`Hud_Update` (`0x0009e710`-`0x0009e754`), which offsets two HUD containers by it
(see "The HUD sway").

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

## The cockpit lean, measured 2026-10-21 (`hd-cockpit-lean`)

`0x000eb528`-`0x000eb5d0` and the two clamp tails `0x000ecd04`, `0x000ed290`-`0x000ed2ac`, read
at instruction level. The step bound is **not** Pulse's `0.6` out / `0.3` back:

```c
d = target - follower;
if (d > 0 && stick > 0)       d = (d > 0.6) ? 0.6 : min(d, 0.3);   /* 0x000eb574-0x000eb594 */
else if (d <= 0 && stick < 0) d = (d < -0.6) ? -0.6 : max(d, -0.3); /* 0x000ed290-0x000ed2ac */
/* else: d is unbounded - a centred or opposing stick lets go at the follower rate */
```

Live (Talon's Junction, Feisar, `scripts/rpcs3-hd-air-hold.py --script`, a stick-only timeline
of full right, half left, full left, with the centre between, 2,681 rows,
`data/traces/hd-talons-cockpit-lean.csv`): the lean settles at `+/-2.0972`
(`0.99868 * 0.015 * 1.4`, twice Pulse's `1.0`) and, released from there, the follower falls as
`exp(-7 t)` (`2.1 -> 1.28 -> 0.61 -> 0.26` in 0.1 s steps), where Pulse's `0.3 * 5.4` limit would
fall linearly. Replayed through `oag_physics::controls::update_camera_lean` with the HD
`CameraLean`: RMS `0.009`, worst `0.18` at the polled edges; Pulse's gains `0.35`, Pulse's step
bound `0.24`, `internalAmount` left out `0.6`
(`crates/trace/tests/camera_lean_ground_truth.rs`,
`hds_cockpit_lean_runs_its_tilt_gains_and_lets_go_unbounded`). Confidence 88 for the stick, return
and settle; the polled rows cannot resolve the `(0.3, 0.6]` knee on the way out.

**The sideshift terms are read, not measured**: `ship+0x6ac8` > 0 takes `0.5` off the target,
`ship+0x6acc` > 0 adds it (`0x000eb530`-`0x000eb564`), ported as `CameraLean::sideshift_offset`.
The capture's double-taps of L1/R1 never raised either timer (`shift_l`/`shift_r` read 0
throughout), so the term has no live number yet. Confidence 75.

Whether Pulse's `FUN_0883fab4` has the same one-sided bound was not checked; its capture (a full
right held, then released) reproduces with the `0.3` return limit to RMS `0.0003`, which an
unbounded return would break, so it differs.

**Ported**: `oag_physics::CameraLean` per craft (`Handling::camera_lean`), filled by
`oag_raceplay::load::code_laws` under `CraftLaws::tilt_camera_lean` (HD measured; Omega and 2048
inherited from HD). Display state: unhashed, read by no force, golden hashes unchanged. The camera
reads it through `oag_render::camera::internal`'s `headtilt`; the lean now reaches `+/-2.1` on HD
where the Pulse law capped at `+/-1`.

## The HUD sway, read and measured 2026-10-21 (`hd-cockpit-lean`, `hd-hud-sway`)

The x term is **ported** (`oag_title::hud::HudSway`, `oag_hud`'s `sway`); the y term is read and
**drawn as zero**.

### The store

`Hud_Update` (`0x0009e3d0`, `r28` the HUD, `r31` the ship) at `0x0009e704`-`0x0009e754`, when
`*(*0x009389a0) + 0x488` (a byte, live `1`) and `hud+0x63c` are non-zero:

```c
dx = -15.0 * ship[0x6a58];   /* TOC -0x5d20 = 0x008a77b8 = 0x41700000; f13 */
dy =  15.0 * ship[0x6a5c];   /* r7 */
for node in (*(hud+0x4e4), *(hud+0x4e8)) { node[0x40] = dx; node[0x44] = dy; node[0x48] = 0; }
```

(both zero when the gate is clear). The two nodes are looked up by name in `Hud_LoadDefinition`
(`0x00097ef0`, `0x00097f14`): `*(0x8a7950)` = `"HUD"` and `*(0x8a7968)` = `"InGame"` (strings at
`0x0077f138`, `0x0077f170`), and `+0x40`/`+0x44` are the screen's translation in the HUD layout's
own space.

**Which widgets.** Every race layout is that one screen: seventeen of the eighteen root files
are a single `<Screen name="HUD">` under the file's own `<Screen>`, nothing beside it
(`hd_hud_ground_truth::every_race_layout_hangs_all_its_widgets_from_the_one_hud_screen`); the
eighteenth, `mptag_hud.xml`, is `<Screen name="MPTag_HUD">`, which `Hud_Update` does not move and
no mode here draws. So the bars, clocks, message lines, pickup slot and the fixed reticle all move
together. `"InGame"` is the plugin screen tree of `ingame_definition.xml` (the pause menu, the
track description, the end-of-race screens); those are not drawn through `oag_hud` and are **not
ported**, open.

### Measured on RPCS3 (x)

Time Trial, Talon's Junction, from `hd-timetrial-talons-venom-start`, player input only (cross
and the left stick through the virtual pad), `scripts/rpcs3-hd-sway-shots.py`: the game paused
over GDB, `ship+0x6a54/58/5c` read, three seconds for the RSX to present, a root screenshot,
1280x720. The shift of each HUD region against a reference frame is a gradient cross-correlation
with a parabolic sub-pixel fit (`scripts/hud-shift.py`).

| Stick | `ship+0x6a58` | HUD shift in x (px) | per unit |
| --- | ---: | ---: | ---: |
| right | `+1.39815` | `-13.33` (against stick 0) | `-9.53` |
| left | `-1.39815` | `+13.35` | `-9.55` |
| half right | `+0.26241` | `-2.7` | `-10.3` |
| half left | `-0.28015` | `+2.78` | `-9.9` |

All five regions measured (the lap counter, the record panel, the clock, the speed bar, the top
centre) agree within `0.1` px, and the fixed reticle by the ship moves by the same `-13.42`, so
it is one translation of the whole HUD. **`-15` layout units per unit of follower** is the
literal; the `9.54` px per unit it comes to on screen is `15 * 1280/1920 * 0.954`, and the
`0.954` is this RPCS3 instance's whole-HUD scale about the frame centre: correlating our HUD,
scaled about the centre, against the original's frame at stick 0 puts it at `0.964`-`0.968`
(`Safe Area Setting` is a user option, [hd-frontend.md](../../../formats/hd-frontend.md) open
item 12, so the scale is the profile's and not the law's). The `1.2%` between the two estimates
is inside the correlation's own noise. Ours at its default scale: `13.95` px for the same follower
(`-15 * 1.396 * 1280/1920`), `-13.95` right and `+13.96` left. Confidence **85** for the sign, the
gain and the whole-HUD reach; the `0.954` against `0.966` is unexplained to 1%.

### `ship+0x6a5c` (y), the writer

Ghidra's instruction search finds no store to `0x6a5c` but the constructors'. A raw scan of the
executable segment for every store opcode with that displacement finds **one more**, at
`0x000d8878`: `stfs f31, 0x6a5c(r31)` inside `Ship_UpdateCameraRigs` (`0x000d80e0`,
[camera.md](camera.md)), in a stretch Ghidra had not disassembled (its Cell `lvlx` words, `0x7e60840e`
and the like, do not decode). Confidence **75** for the structure below; the vector code was read,
not traced instruction by instruction.

The second half of the function (`0x000d8500`-`0x000d8878`) runs the spring of the rig whose block
sits at `team+0x40` (live: `4.0, -15.0, 0.0, 25.0, 60.0, 10.0, 10.0`, springs `10`/`10` at
`+0x54`/`+0x58`; the other block, `3.7, -11.0, -4.0, 25.0, 60.0, 12.0, 12.0`, is the first half's).
With `A`, `B`, `C` the three rows `*(ship+0x629c)`, `*(ship+0x62a0)`, `*(ship+0x62a4)` (live: the
body's forward `body+0x1f0`, up `+0x1e0` and right `+0x1d0` rows), `P0 = body[0x200] + body[0x1b0] +
0.5 * (V.x * C + V.y * B + V.z * A)` with `V = ship+0x6b00`, and `S` the spring state at
`ship+0x69d0`:

```c
P2   = P0 + team[0x44] * A;            /* the anchor, 15 behind the craft */
d    = P2 - S;
f31  = dot(B, d);                      /* -> ship+0x6a5c, BEFORE the update below */
S   += f30 * team[0x58] * dot(B, d) * B + f30 * team[0x54] * dot(C, d) * C + K * dot(A, d) * A;
/* then S is pulled back inside a bounded distance of the rig's aim point */
```

(`f30` the clamped `dt`; `K` a TOC constant not read.) So `ship+0x6a5c` is **how far the sprung eye
lags its anchor along the craft's up axis**, one tick old, and Hud_Update's `15 * ship+0x6a5c` is
that lag in HUD units. It is the same spring `oag_render::camera::chase::Chase` runs for the
external rigs.

Measured on RPCS3 under the same stick script (`scripts/rpcs3-hd-sway-poll.py`,
`rpcs3-hd-sway-corr.py`): it does **not** follow the stick (`-0.001` at rest, `+/-0.6` swings of
about `0.7` s while the lean follower is steady), and over 13,751 distinct frames it correlates
`-0.83` with `body+0x1b0 . up` (`-0.27` per unit), as `P0` carrying `body+0x1b0` predicts. The
screen follows it: over 23 frames at stick 0, `dy = 9.58 * ship+0x6a5c + 0.09` px (`r = 0.987`,
RMS `1.0` px), the same `9.54` px per unit as x, and positive is down.

**Drawn as zero.** Two terms of `P0` are unread: `body+0x1b0` (a vector whose length is `0.3%` to
`8%` of the speed and which is not along the velocity, `cos` `0.18`; the dominant source of the
bumps, `+/-20` px on a rough stretch) and `ship+0x6b00`. Without them the lag a smooth track gives
is well under a pixel, so a ported spring would draw almost nothing and look like the law. Open:
the writer of `body+0x1b0` (`FUN_000d80e0` reads it as `lvx v30, body, 0x1b0`).

**What slides in ours.** Everything `draw_list` builds: the bars, text, message lines, the pickup
slot (an `<Image>` on HD, `pickup_icon_models` is `None`; measured to slide by the same `-13.96`
px in the top-centre region of our frame) and the lock-on sight. The sight slides **by structure**
(it is a widget of the HUD screen, so the screen's translation carries it); the fixed reticle was
measured, a world-anchored lock never captured.

### Ported

`HudArt::sway` (`Sourced<HudSway { x_per_lean: -15 }>`): HD **measured**; Omega and 2048 **inherited
from HD** (see below); Pulse and Pure `None`. `oag_hud::draw_list` slides every sprite and every
text by `x_per_lean * Readout::steer_lean_follower`, through the same `Placement` a split view's
HUD uses, so a split-screen view slides by its own craft's lean. `readout_for` fills the follower
from `ship.physics.steer_lean_follower`, the number the hull is drawn with. Display only, unhashed.

## 2048 and Omega

- **Cockpit lean (`<Tilt internalAmount internalSpeed>`)**: both ship the attributes (Omega's
  teams author `<Tilt>`, 2048's at `1.3`/`5`); the consumer is the same unlocated code as above,
  so both inherit HD's gains, smooth `6.0`, sideshift `0.5` and step bound, labelled inherited
  (`oag_omega::race::CRAFT_LAWS`, `oag_2048::race::CRAFT_LAWS`). Not checkable live.
- **Omega: checked, applies, inherited.** Its teams author `<Tilt>` and its parser reads the
  four names (`shipAmount` at `0x018145aa`, referenced from `FUN_012d8e30`, which stores the
  previous one at `+0x1c`). The consumer was not located: the two `-0.005` constants' readers
  (`0x01261f2c`, `0x012bbfd7`) are other code. Live: not checkable (no PS4 emulator).
  `oag_omega::race::CRAFT_LAWS` inherits HD's law.
- **The HUD sway: checked, applies, inherited, not wired against a consumer.** Omega's in-race HUD
  is HD's own files (`Data/xml/timetrial_hud.xml` read out of `data00.psarc` is HD's, the same
  `<Screen name="HUD">` root and fragment list), and its `2048_hud` set carries the same root;
  `oag_omega::hud::ART` and `oag_2048::hud::ART` carry `HudArt::sway` as inherited from HD. Neither
  executable's `Hud_Update` was read for the store (no PS4 emulator; the Vita's not located). What
  is inherited is the **literal** `-15`, and it acts in each title's own grid: Omega's is HD's
  1920x1080, so the same on-screen slide; 2048's `setResolution="544"` HUD is 960x544, so the same
  literal slides it **twice HD's fraction of the screen** (15/960 against 15/1920). If the Vita
  code scales for the smaller grid the right number is about `-7.5`; unread.
- **2048: checked, applies, inherited.** The data ships `<Tilt>` in every team file (HD ships at
  `1.4`/`6`, 2048's own at `1.3`/`5`) and the executable carries both strings (`0x814900d8`,
  `0x814900e4`); the consumer was not located. `oag_2048::race::CRAFT_LAWS` inherits HD's law.
