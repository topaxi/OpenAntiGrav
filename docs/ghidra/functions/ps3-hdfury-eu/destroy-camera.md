# The destroy cameras: HD mode 8, the Zone end's mode 2, 2048 and Omega mode 9

| | |
| --- | --- |
| **Binary** | `PS3_GAME/USRDIR/EBOOT.elf` (HD/Fury, BCES-00664), Ghidra `/hdfury/EBOOT-ps3-hdfury-eu.elf` |
| **Twins** | 2048 `/2048/eboot-vita-2048-eu-v104.elf`, Omega `/omega/eboot-ps4-omega-eu.bin` |
| **Subsystem** | race camera: per-frame update, the spectator (station) view, the orbit view, hand-back |
| **Related** | [ship-destroy.md](ship-destroy.md) (row 5: the cut to mode 8), [zone-destroy.md](zone-destroy.md) (Z2: the cut to mode 2), [camera.md](camera.md) (the chase rig), [photo-mode.md](photo-mode.md); Pulse's mode 5: [camera.md](../psp-pulse-usa/camera.md) ("The destroy camera"); the station data: `crates/vex/src/camera.rs` |

Read 2026-11-23, `destroy-cameras-re`. Static only: the bridge-connected Ghidra, every
TOC-relative load resolved against TOC `0x008ad4d8`, both jump tables of `Camera_SetMode`
and the per-mode table of `Camera_UpdateView` decoded by hand. Ghidra stops several of these
functions at Cell `lvlx` loads it cannot decode (`0x000ac0dc`, `0x000ac274`, `0x000acbc4`,
`0x000aeb04`, `0x000b04e8`); each was decoded from its word and is a load of the `1.0` and
`0.5` constants of a Newton-Raphson reciprocal square root, so no instruction is hidden there.
No live read: RPCS3 belongs to another lane for this lane's life (recipe under "Open").

**Answer in one line.** HD's mode 8 is Pulse's mode 5 kept almost to the instruction: the eye
stands at the circuit's own `Camera` node whose aim point is nearest the wreck, the field eases
toward the one that frames 35 units at the wreck, the look-at eases onto the wreck and freezes in
state 6. HD adds a look-at wobble of `0.015 x distance`, random factors on both ease rates, and
eases in `trunc(dt * 6000)` substeps. 2048 and Omega keep the same view as mode 9, the station
modes moved up one, and add a hold mode 8. HD's Zone end, mode 2, is not a station view: it is a
close orbit around the craft. It spins fast and tight for 2.5 s, then drifts slowly at a 65 degree
field until the results screen.

## Mode numbers across the line

| Pulse | HD | 2048, Omega | Field target written on enter (`Camera_SetMode`'s enter table) |
| --- | --- | --- | --- |
| 5 | **8** | **9** | `35.0` (HD `0x000abd14`: `lis r0,0x420c; stw r0,0x288(r31)`; 2048 `0x8121963a` case 9; Omega `0x01382720` case 9) |
| 6 | 9 | 10 | HD `Random_RangeFloat(10, 17)`; Omega `10 + 7 r`; 2048 `0x812ac11e(10.0, ...)` |
| 7 | 10 | 11 | HD `Random_RangeFloat(40, 50)`; Omega `40 + 10 r`; 2048 `0x812ac11e(40.0, ...)` |
| - | 2 | 2 | the orbit view (`Camera_EnterTrackView_q`, HD `0x000aa090`) |
| - | - | 8 | 2048 and Omega only: the hold view (below) |

The field term is the "units across the picture" the framing law fits at the subject:
`fov_target = 2 atan(0.5 * field / |eye - subject|)` in degrees. HD's constant is
`114.592 = 2 x 57.296` at `TOC-0x5660`. Pulse's three constants were 35, 17 and 50. HD keeps 35 and
draws the other two at random.

## HD: the spectator view, mode 8 (and 9, 10)

What runs each frame, in order. `Camera_Update` (`0x000b0830`, vtable `0x00862ff0` slot 3) is the
camera's per-frame entry. `Camera_UpdateView` (`0x000b0060`) builds the look-at target and
dispatches on `cam+0x40` (table at `0x000b0234`: modes 8, 9 and 10 all go to `0x000b07c4`).
`Camera_UpdateSpectatorView` (`0x000ac048`) is the law.

| Row | What | Law (HD) | Address | Conf. |
| --- | --- | --- | --- | ---: |
| D1 | **Station pick** (eye) | `Camera_SetSubject(cam, ship, 1)` (ship-destroy.md row 5) with `ship+0x7984 == 0` calls `Camera_PickStation(cam, body+0x200, 1)`. It loops over the `cam+0x1e0` stations in `cam+0x50`, keeps the one whose **aim point** (`station+0x120`) is nearest the craft (`d^2` strictly less than the best, the first of equals wins), behind a y-sign test against `100000.0` that never fails on a circuit. No station gives mode 7 instead of 10. This is Pulse's `Camera_PickStation` with the eye at `+0x110` (Pulse `+0x90`) and the aim at `+0x120` (Pulse `+0xa0`). The ship's own station `+0x7984`, when set, replaces the pick and skips the random rate factors (D4). | `0x000ac7a0`, `0x000acab0` | 82 |
| D2 | **Stations are the circuit's `Camera` nodes** | Every HD circuit authors them, and **every one has a non-zero aim**: a census of all sixteen environments' `track.vex` through `oag_vex::camera::cameras` (2026-11-23, decrypted EU image) found 10 to 19 per circuit (`talons_junction` 10, `zone_4` 19), with 0 zero aims anywhere. So "zero on an HD flyer" (`crates/vex/src/camera.rs`) holds for the menu flyers only. That the runtime `+0x110`/`+0x120` are the node's translation and payload `+0x10` is read from Pulse's measured layout (both offsets shift by `0x80` together), not from HD's loader. | census, `0x000ac7a0` | 70 |
| D3 | **Field** | On the cut: `fov0 = framing(+0x288) + Random_RangeFloat(-10, 20)`, at least `3.0` (`TOC-0x5614`). `framing` uses whatever `cam+0x288` holds **before** `Camera_SetMode(8)`: the constructor's `60.0` (so `2 atan(30 / d)`, Pulse's start) unless an earlier mode-2, 8, 9 or 10 enter changed it. Target: `fov_target = framing(35.0)`, recomputed each frame from the station's eye to the look-at. No station: `65.0` both. | `0x000ac9bc`-`0x000aca00`, `0x000ac0ac`-`0x000ac138` | 80 |
| D4 | **Ease rates** | Field rate `cam+0x234 = 0.06 x Random_RangeFloat(0.5, 2.0)`. Focus rate `cam+0x270 = r_class x Random_RangeFloat(0.5, 1.0)`, with `r_class` from the constructor by speed class `g_GameState+0xd4`: Venom `0.4`, Flash `0.5`, Rapier and Phantom `0.6`, else `0.3`. These are Pulse's per-craft values, and Pulse has no random factor (it measured `0.06` exactly). | `0x000ac9c4`-`0x000aca48`, `0x000aaa50` | 80 |
| D5 | **Look-at target** | `craft = target body+0x200` (`cam+0x1f0`, the craft under the camera), plus a wobble `w = 0.015 x |station_eye - craft| x (n1, n2, n3)`. The `n` are three independent fractal 1-D noise channels (`Noise_SampleFractal1D_q` on `cam+0x1f8`, `+0x208`, `+0x218`; each `(seed = rand % 0x4000, 2 octaves, 0.5, 1.0)` after the constructor's second write). The noise's time source, tables and persistence are unread (55). The target freezes once the craft is in **state 6**: `cam+0x260` is written only while `Ship_GetState != 6`. | `0x000b04c0`-`0x000b05a0`, `0x000ac13c`-`0x000ac15c` | 75 |
| D6 | **Easing** | `n = trunc(dt x 6000)` substeps (`TOC-0x564c`), each `focus += (target - focus) x rate / (dt x 6000)` and the same for the field, so a frame moves `1 - (1 - rate/n)^n` of the way. At 60 Hz that is `n = 100`: a `0.4` rate moves `0.330` per frame, `0.06` moves `0.0582`. Pulse moves exactly `rate` per frame. | `0x000ac160`-`0x000ac228` | 85 |
| D7 | **Basis** | `z = station_eye - focus`, `z.y *= max(1 - fov x 0.008, 0.4)` (`TOC-0x5648`, `TOC-0x5668`), normalised; the basis from `0x006776d8`; eye = the station's eye. The view's field is `cam+0x22c` (x 1.5 when the two flags `TOC-0x56c0` and `TOC-0x563c` are set). This is Pulse's rule to the constant. | `0x000ac22c`-`0x000ac40c` | 85 |
| D8 | **Re-pick** | Each frame in modes 8 to 10, if the current station's aim is **60 or more units** (`d^2 >= 3600.0`) from the craft, pick at random among the other stations whose aim is within 60 units. The new pick keeps the ease rates unrandomised. A wreck that lies still never fires it. Pulse's `Camera_RepickNearSubject_q`. | `0x000ac468`, `0x000b0a40` | 65 |
| D9 | **Hidden subject** | `cam+0x34 \|= 6` and `Camera_UpdateView` keeps the visible bit of every racing craft's model set, outside Zone and Detonator. The flare gate in [engine-trail.md](engine-trail.md) skips the subject's flare in mode 8 (ship-destroy.md row 5). | `0x000b00fc`-`0x000b0204` | 70 |

### The station census (D2)

`oag_vex::camera::cameras` over each environment's `track.vex` in the decrypted EU image
(`hdfury-ps3-eu-dec.iso`), 2026-11-23. Every node has a non-zero aim.

| Environment | `Camera` nodes | Environment | `Camera` nodes |
| --- | ---: | --- | ---: |
| `amphiseum` | 11 | `01_vineta_k` | 13 |
| `modesto_heights` | 13 | `02_track` | 11 |
| `talons_junction` | 10 | `03_track` | 10 |
| `tech_de_ra` | 10 | `04_chenghou_project` | 14 |
| `zone_1` | 10 | `05_ubermall` | 12 |
| `zone_2` | 10 | `10_sebenco_climb` | 12 |
| `zone_3` | 14 | `12_sol_2` | 11 |
| `zone_4` | 19 | `15_anulpha_pass` | 11 |

Talon's Junction's first node, for a spot check: eye `(-339.649, 30.186, 90.449)`, aim
`(-281.450, -74.622, 41.270)`.

### HD: how long mode 8 lasts and what hands back

**Nothing in mode 8 itself times out.** In single player only H1 ends it:

| Row | Hand-back | Law | Address | Conf. |
| --- | --- | --- | --- | ---: |
| H1 | **The camera's own 8 s clock** | `Camera_Update` adds `dt` to `cam+0x4c` every frame in game modes below 16. When the clock passes **`8.0`** (`TOC-0x5588`) outside the Zone family (`0x206040`: modes 6, 13, 14, 21; the byte at `*(TOC-0x5594)` lifts that exclusion), it calls `Camera_PickRandomSubject`. That sets the subject to `RaceManager+0x13e8`, then to a random entry (not the current target) of `0x00055c90`'s list: the `RaceManager+0xe8` crafts, `*g_GameState` of them, that are neither out of the race (`+0x6a64 & 0x1000`) nor in state 7. Then target = subject and `cam+0x1f4 = -1`. If `cam+0x28c` is set (the constructor writes `1`) and the clock is at least `2.0`, the clock resets to 0 and `Camera_SetMode(cam, 0)` runs, unless `cam+0x48 & 0x100` is set and pad check `9` fails. Nothing resets `cam+0x4c` at the blast: not `Ship_SetState` (the window `0x000e2a10`-`0x000e2c9c` was read for it), not `Camera_SetSubject`, not `Camera_SetMode`. So mode 8 lasts **0 to 8 s**, set by where this free-running clock stood at the blast. **In single player** the destroyed craft is out (ship-destroy.md row 8's `0x1000`), so the list is empty and the subject stays `RaceManager+0x13e8`, the player's own craft. The hand-back is the **chase camera on the player's own wreck**. The `+0xe8` list cannot be every craft (H1 runs in mode 0 too, and a full list would swap a racing player's chase view to a random opponent every 8 s), so it is read as the human players' crafts (60). | `0x000b0868`-`0x000b09a4`, `0x000a9f20`, `0x00055c90` | 62 |
| H2 | **The wreck reaching state 6** | While the subject is in state 6 with its state timer `+0x6a78 <= 0` and `dt > 0`, `Camera_PickRandomSubject` runs, and mode 8 stays. State 6's entry writes `+0x6a78 = 0` (ship-destroy.md row 12), so this fires on state 6's first frame, once the wreck has lain still for 6 s (or after 20 s). In single player it changes nothing (H1's empty list). With several human players, the subject moves to another one still racing. | `0x000b09bc`-`0x000b09d0`, `0x000b0b00` | 62 |
| H3 | **The race end does not cut** | `RaceManager_UpdateEndCut_q` (`0x00060f90`, names `Race End Photo`). Sub-phase 0's camera block runs only when the local craft is still racing, or with more than one player (`g_GameState+0xe4 != 1`). There it waits `1.0` s (`*0x008a6b2c`; `0.0` with more players), targets the local craft (or the list's first if it is out) with `Camera_SetMode(cam, 0)`, and puts an out craft not in state 6 back in mode 8 with several players. **A single player whose craft is out takes the other branch: sub-phase 1 and `Race End Photo`, with no camera call**, so mode 8 (or H1's chase) runs on. Sub-phase 1 waits `RaceManager+0x2dc4`, set to **4.5** at the blast in single player (ship-destroy.md row 5), then a pad press opens the results. `FUN_00072bc8` is a second chooser of the same shape (mode 8 on an out craft not in state 6, else mode 0). When the race-end phase starts after an ordinary race's blast is unread. | `0x00060f90`, `0x00072f50`, `0x000730dc` | 62 |
| H4 | A watched network craft's wreck | `Ship_UpdateWrecked` branches on `ship+0x628c`: `0` runs the local path, `1` this one, and anything else returns. The value 1 is the network-remote craft: `+0x628c != 0` is remote in [absorb-feedback.md](absorb-feedback.md), and ship-destroy.md's recipe takes the opponents at `2`. When camera 0's subject is that craft and `0x00055c90` lists others, `Camera_PickRandomSubject` and `Camera_SetMode(cam, 6)` move the camera to a new subject. | `0x000e3870`-`0x000e3a14` | 65 |

## HD: the Zone end, mode 2 (zone-destroy.md Z2)

`Camera_EnterTrackView_q` (`0x000aa090`) is mode 2's enter, and the name is a misnomer: the view is
an **orbit**. The per-frame law is `Camera_UpdateOrbitView_q` (`0x000ae6c0`, dispatch entry 2 at
`0x000b0768`). Its tuning is the block at `0x008c1520` (`TOC-0x56a4`).

| Row | What | Law (HD) | Address | Conf. |
| --- | --- | --- | --- | ---: |
| Z2a | **Enter** | `cam+0x48 &= ~2`. Focus `cam+0x390` = the craft's body position. Yaw `+0x2a4` = pitch `+0x2a0` = 0, clock `+0x2c4 = +0x3a0` (a resume time). If `+0x3a0 > 0`: yaw and pitch from `+0x3a4`/`+0x3a8`, field `+0x288 = 35.0`, view field `+0x22c = +0x230 = 65.0`, field rate `+0x234 = 0.01`, distance `+0x238 = 0.5`, distance target `+0x23c = 4.0`, distance rate `+0x240 = 0.02` (block `+0x0..+0xc`). Otherwise it looks up `"AI track data"` within `10000.0` through `0x000a97f0`. Ghidra marks that function no-return, and it is unread. Omega's enter (`0x01382720` case 2) writes the same `35.0`, `4.0`, `65/65`, `0.01`, `0.5`, `0.02` on **both** paths, and picks the spin direction `+0x5fc` from the track's direction at the craft, or at random. HD's first-entry branch is very likely the same. | `0x000aa090`; Omega `0x01382720` | 65 |
| Z2b | **Spin** | Clock `t += dt`. Yaw `+= dt x s x k`, wrapped to `2 pi`, with `s = 9 t + 1` for `t < 2.5` (block `+0x14`, `+0x18`, `+0x1c`) and `0.5` after (`+0x20`), and `k = +0.1` if the byte `+0x3ac` is set, else `-0.1`. The yaw rate climbs from 0.1 to 2.35 rad/s over the first 2.5 s, then drops to 0.05 rad/s. For `t < 2.5` the pitch `+0x2a0` also turns, at `0.35` rad/s (block `+0x34`). | `0x000ae710`-`0x000ae7c4`, `0x000aede0`-`0x000aee08` | 65 |
| Z2c | **Look-at** | `focus += (craft - focus) x 0.6` per frame (block `+0x24`). | `0x000ae7c8`-`0x000ae7fc` | 70 |
| Z2d | **Eye** | `R = Ry(pi) . Ry(yaw) . Rx(-0.7) . Rx(pitch)` (block `+0x28` = `-0.7` rad), applied on the craft's body rotation. `Matrix_RotationY` (`0x006ca868`) and `Matrix_RotationX` (`0x006ca948`), both read whole. Eye = `focus + normalize(row) x dist`, where the row is the composed matrix's row the code takes at `0x000aebd8`; which row that is, is read from register flow only (55). | `0x000ae800`-`0x000aec50` | 60 |
| Z2e | **Distance and field, per frame** | `dist += (dist_target - dist) x 0.02`, `fov += (fov_target - fov) x 0.01`, **per frame** (not `dt`-scaled). Targets: for `t < 2.5`, field **20.0** and distance **5.0** (block `+0x2c`, `+0x30`); after, field **65.0** and distance **3.0** (`+0x38`, `+0x3c`). The 20 degree field is a target, not reached: from 65 it is near 30 degrees at 2.5 s (`20 + 45 x 0.99^150`), then eases back toward 65. | `0x000aec80`-`0x000aecdc` | 65 |
| Z2f | **Hand-back** | Modes 6, 13, 14 and 21 are outside H1's 8 s clock, so mode 2 holds until the results screen: `SPZone_UpdateFinish` waits 1.0 s, then `RaceManager+0x2dc4` (4.5 s), then opens `EndRace Results`. That is **5.5 s** of orbit (zone-destroy.md Z9). | `0x000758a0`, `0x000b08e4` | 75 |

What the player sees: the eye is a few units off the craft, so the craft and its white death
shell fill the frame. The view swings round fast and tight while the shell plays for 2.5 s,
then pulls back to a slow drift at a normal field while the debris flies.

## 2048 and Omega: mode 9, against HD and against each other

**2048 against HD: checked, differs** (the mode numbers and the hold mode).

**Omega against 2048: checked, applies, not wired.** Same update, same mode set, same timer, plus
one Omega-only fallback.

| Row | 2048 v1.04 | Omega | Against HD |
| --- | --- | --- | --- |
| W1 enter | `Camera_SetMode` `0x8121963a` case 9: field `+0x270 = 35.0` | `Camera_SetMode` `0x01382720` case 9: field `+0x468 = 35.0` | HD mode 8, same value |
| W2 framing | `Camera_FramingFov` `0x81218576`: `2 atan(0.5 field / d) x 180/pi`, `65.0` with no station, eye at station `+0x108` | inline in `Camera_Update` `0x01381c70` (`atan(...) x 114.59155`), eye at station `+0xe0` | the same law |
| W3 subject | `Camera_SetSubject` `0x8121a156`: station from `ship+0x6fa4` else the pick (`0x812199c0`, unread); `fov0 = framing + rand(-10, 20)` at least `3.0`; sets mode 11 (HD 10) | the pick `0x013836a0` (unread) | the same; 2048's own-station path also copies the rates unrandomised |
| W4 view | `Camera_UpdateSpectatorView` `0x81218638`: `trunc(dt x 5999.9995)` substeps, focus frozen in state 6, `z.y x max(1 - fov x 0.008, 0.4)`, field x 1.5 on the same two flags | the view dispatcher `0x01383a10`, unread | the same law (D5 to D7) |
| W5 wobble | `Camera_UpdateView` `0x8121888a`: modes 9 to 11, `0.015 x distance` and three noise calls on `cam+0x1f4`, `+0x204`, `+0x214` (Ghidra drops the noise return, so its listing shows `d x d x 0.015`: not the law) | not read | the same as D5 |
| W6 8 s clock | the update at `0x81219c3c` (not a function in Ghidra): `cam+0x48 > 8.0`, gate byte `+0x274`, `>= 2.0`, `Camera_SetMode(cam, 0)` | `Camera_Update` `0x01381c70`: `cam+0x84 > 8.0`, Zone family `0x8181 >> (mode - 6)` plus an online check, gate byte `+0x46c`, `Camera_SetMode(cam, 0)` | H1 |
| W7 re-pick | not read | `d^2 >= 3600`, random among aims within 60 | D8 |
| W8 **hold mode 8** (new) | In mode 0, a subject in state 4, 5, 6 or 8 switches the camera to **mode 8**. In mode 8, a subject in any other state switches back to **mode 0** (`0x81219eb8`-`0x8121a008`). Mode 8's enter (`Camera_EnterHoldView_q`, `0x81219040`) freezes the eye where the last view was. Its view (`Camera_UpdateHoldView_q`, `0x81217616`) looks at the craft plus a lead of `7.5 x` its velocity, ties the look-at to the track section through `0x8121476a`, and holds the field at `65.0` (`97.5` with the two flags). | the same rule on the state set `{4, 5, 6, 8}` (`(s \| 1) == 5`, 6, 8). Mode 8's enter is `0x013832f0`. **Omega only**: when the station pick in mode 9 finds no station (mode 7), it goes to mode 8 instead | HD has no mode 8 of this kind and no state-driven switch: HD's chase camera does not leave a respawning craft |

So on 2048 and Omega the blast cuts to mode 9 as on HD (ship-destroy.md row 5). The 8 s clock (W6)
hands back to the chase camera (its subject chosen as in HD's H1). On a craft that respawns, the chase
camera's subject sits in states 5, 6 and 8, so W8 holds the view still until the craft races again.

## Ready to wire

| Title | Mode | Eye | Look-at | Field | Motion | Hands back | Conf. |
| --- | --- | --- | --- | --- | --- | --- | ---: |
| HD | 8 | the circuit's `Camera` node whose aim is nearest the wreck (D1, D2) | the wreck + `0.015 d` noise wobble, frozen in state 6 (D5) | `2 atan(17.5 / d)` target, from `framing(60) + rand(-10, 20)` (D3) | eased in `trunc(6000 dt)` substeps at `0.06 x rand(0.5, 2)` and class rate `x rand(0.5, 1)` (D4, D6) | the 8 s camera clock, 0 to 8 s after the cut, to the chase camera on the player's own wreck in single player (H1); the race end does not cut (H3) | 70 |
| HD Zone | 2 | orbit `dist` 0.5 -> 5 -> 3 units off the craft, `Ry(pi) Ry(yaw) Rx(-0.7) Rx(pitch)` on the body (Z2d) | the craft, `0.6` per frame (Z2c) | 65 -> 20 (2.5 s) -> 65, `0.01` per frame (Z2e) | yaw `+-0.1 (9t + 1)` rad/s for 2.5 s, then `+-0.05`; pitch `0.35` rad/s for 2.5 s (Z2b) | results at 5.5 s (Z2f) | 62 |
| 2048 | 9 | as HD mode 8 | as HD | as HD | as HD | the 8 s clock to chase (subject as HD's H1); then hold mode 8 while the chase subject is wrecked or resetting (W6, W8) | 70 |
| Omega | 9 | as 2048 | as 2048 (view not read: W4, W5) | as 2048 | as 2048 | as 2048 (W6, W8) | 68 |

The engine already has Pulse's mode 5 as `oag_render::camera::destroy`. Wiring HD mode 8 and 2048/Omega
mode 9 onto it takes four changes. Draw the stations from each title's own `track.vex` (D2).
Replace the single step with the substep ease (D6). Add the two random rate factors (D4) and the
wobble (D5). Add the hand-back rule (H1, and W8 on 2048 and Omega). The wobble's noise source is the
one input still unread. Labelled "chosen, not measured" until it is read, it can stand in for
`n1..n3`.

## Functions

| Address | Name | Conf. | Evidence |
| --- | --- | ---: | --- |
| `0x000b0830` | `Camera_Update` | 80 | vtable `0x00862ff0` slot 3. Owns the 8 s clock `+0x4c`, the hand-overs through `Camera_PickRandomSubject`, and the mode 8 to 10 station upkeep, then calls `Camera_UpdateView`. The 2048 update at `0x81219c3c` and Omega's `0x01381c70` are the same, field for field (W6) |
| `0x000b0060` | `Camera_UpdateView` | 78 | builds the look-at target (mode 8 to 10 wobble), keeps the craft models visible, and dispatches on `cam+0x40` through the twelve-way table at `0x000b0234`. 2048's `0x8121888a` has the same shape |
| `0x000ac048` | `Camera_UpdateSpectatorView` | 85 | dispatch entries 8, 9 and 10; Pulse's `Camera_UpdateSpectatorView` mode 5 law (framing, focus freeze in state 6, `0.008`/`0.4`), with substeps |
| `0x000ac7a0` | `Camera_PickStation` | 82 | the nearest-aim loop over `cam+0x50`; mode 10 or 7; starting field and randomised rates; Pulse's `Camera_PickStation` |
| `0x000ac468` | `Camera_RepickNearSubject` | 65 | the `3600.0` (60 squared) test, then a random station among those within 60 units; Pulse's `Camera_RepickNearSubject_q` |
| `0x000a9f20` | `Camera_PickRandomSubject` | 72 | subject = `RaceManager+0x13e8`, then a random entry of `0x00055c90`'s list (`0x00055b30` online), never the current target; one entry is taken as is, an empty list leaves `+0x13e8` |
| `0x000aaa50` | `Camera_Construct` | 72 | stores the vtable `0x00862ff0`; field `60.0`, rates `0.06` and the class focus rate, the three noise channels, `+0x28c = 1` |
| `0x000ae6c0` | `Camera_UpdateOrbitView` | 62 | dispatch entry 2; the block at `0x008c1520`; yaw, pitch, distance and field as in Z2b to Z2e |
| `0x0027e908` | `Noise_SampleFractal1D` | 55 | reseeds out of range with `rand % 0x4000`, sums `param[1]` octaves of table noise at `param[2]` scaling, times `param[3]`; the time source and tables unread |
| `0x00060f90` | `RaceManager_UpdateEndCut` | 60 | the race-end step naming `Race End Photo`: for a racing or multi-player local craft the `1.0`/`0.0` wait and the chase cut (or mode 8 on an out craft); for a single out player no camera call; then the `+0x2dc4` wait and a pad press |

Twins named here (rows in each binary's `names.tsv`):

| Binary | Address | Name | Conf. | Evidence |
| --- | --- | --- | ---: | --- |
| 2048 v1.04 | `0x8121963a` | `Camera_SetMode` | 82 | `+0x40 = +0x3c; +0x3c = mode` between two switches (leave, enter); case 9 writes `35.0`, case 2 calls the orbit enter `0x81218cd8`; HD's `Camera_SetMode` with the station modes moved up one |
| 2048 v1.04 | `0x81218576` | `Camera_FramingFov` | 85 | whole function: `65.0` with no station, else `2 atan(0.5 x +0x270 / d)` in degrees |
| 2048 v1.04 | `0x81218638` | `Camera_UpdateSpectatorView` | 82 | modes 9 to 11 of `Camera_UpdateView`; HD `0x000ac048`'s substeps, freeze and basis |
| 2048 v1.04 | `0x8121888a` | `Camera_UpdateView` | 75 | the wobble for modes 9 to 11, the visibility pass, the thirteen-way switch on `+0x3c` |
| 2048 v1.04 | `0x8121a156` | `Camera_SetSubject` | 75 | `+0x1e8 = ship`; station `ship+0x6fa4` or the pick `0x812199c0`; mode 11; framing start `+ rand(-10, 20)`, at least 3 |
| 2048 v1.04 | `0x81219040` | `Camera_EnterHoldView` | 58 | mode 8's enter: the eye `+0x310` from the current view, the look-at `+0x330` from the craft, track-section terms through `0x8121476a` |
| 2048 v1.04 | `0x81217616` | `Camera_UpdateHoldView` | 55 | mode 8's view: fixed eye, look-at with a `7.5 x` velocity lead, field `65.0` or `97.5` |
| Omega | `0x01382720` | `Camera_SetMode` | 82 | the same pair of switches on `+0x74`/`+0x7c`; case 9 `35.0`, cases 10 and 11 `10 + 7r` and `40 + 10r`, case 2 the orbit enter with HD's constants, case 8 `0x013832f0` |
| Omega | `0x01381c70` | `Camera_Update` | 78 | the 8 s clock `+0x84`, the 60-unit re-pick, the framing start, and the mode 0 to 8 and 8 to 0 switches (W6 to W8) |

Already named ([hull-shake.md](hull-shake.md)), and read whole again here for Z2d: `Matrix_RotationY` `0x006ca868` (`m0 = m10 = cos`, `m8 = sin`, `m2 = -sin`) and `Matrix_RotationX` `0x006ca948` (`m5 = m10 = cos`, `m6 = sin`, `m9 = -sin`), each reached through an import stub (`0x006776f8`, `0x00677708`).

Not renamed: `0x000ab2d8` (a second constructor-shaped writer of `+0x4c`/`+0x28c`, unread),
`0x000abf70` (the `+0x28c`/2 s gate alone, reached through `0x00873a28`), `FUN_00072bc8`
(H3's second chooser), `0x00055c90` (H1's list; whether `+0xe8` holds only human players is read from behaviour, not from its writer), `FUN_000a3c38` (mode 8 with
`Camera_SetSubject(.., 0)`, `0x000a4cec`), and 2048's `0x81219c3c` (no function defined there).

## Open

1. **The wobble's noise** (D5): `Noise_SampleFractal1D_q` `0x0027e908` with its tables at
   `*0x008b289c` and the time source through `+0x808`; the three channels' per-frame step.
2. **What fills `RaceManager+0xe8`** (H1): its writer, to settle that it holds the human players' crafts
   only; and when an ordinary race's end phase starts after a blast (H3).
3. **The orbit's first entry** (Z2a): `0x000a97f0` from the disassembly (Ghidra truncates it),
   which sets the spin direction byte `+0x3ac`; and which row of the composed matrix is the eye
   direction (Z2d).
4. **Omega's view** `0x01383a10` and mode 8's enter `0x013832f0`; 2048's pick `0x812199c0`.
5. **A live read** (all rows; RPCS3 was another lane's here). Recipe: a Racebox race on
   Talon's Junction with weapons on, the player's shield written to `0.05` (`ship+0x5fa0`);
   break on `Camera_SetMode` (`0x000abc20`) with `r4 == 8`. From there, log per frame
   `cam+0x40`, `+0x4c`, `+0x1e4` (and its `+0x110`/`+0x120`), `+0x22c`/`+0x230`/`+0x234`,
   `+0x250`/`+0x260`/`+0x270`, and the craft's `+0x5fa4`. Rows D3, D4 and D6 predict the
   field's first steps; H1 predicts the frame mode 0 returns, at the camera clock's next 8 s.
   For Z2, the same in a Zone race with `r4 == 2`, logging `+0x2a4`, `+0x2a0`, `+0x238`,
   `+0x22c` and `+0x2c4`.
