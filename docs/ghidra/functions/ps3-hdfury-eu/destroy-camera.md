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
| Z2a | **Enter** | `cam+0x48 &= ~2`. Focus `cam+0x390` = the craft's body position. Yaw `+0x2a4` = pitch `+0x2a0` = 0, clock `+0x2c4 = +0x3a0` (a resume time). If `+0x3a0 > 0`: yaw and pitch from `+0x3a4`/`+0x3a8`, field `+0x288 = 35.0`, view field `+0x22c = +0x230 = 65.0`, field rate `+0x234 = 0.01`, distance `+0x238 = 0.5`, distance target `+0x23c = 4.0`, distance rate `+0x240 = 0.02` (block `+0x0..+0xc`). Otherwise it looks up `"AI track data"` within `10000.0` through `0x000a97f0`. Ghidra marks that function no-return, and it is unread. Omega's enter (`0x01382720` case 2) writes the same `35.0`, `4.0`, `65/65`, `0.01`, `0.5`, `0.02` on **both** paths, and picks the spin direction `+0x5fc` from the track's direction at the craft, or at random. **Read live on HD (2026-11-24, "Measured on RPCS3"): the object's `+0x238` does start at about `0.5`** (0.83 at its fourth frame), `+0x23c` = 5.0, `+0x240` = 0.02, `+0x234` = 0.01, `+0x288` = 35.0, `+0x3ac` = 1 (spin positive); the eye that reaches the screen is nevertheless far at the cut (below). | `0x000aa090`; Omega `0x01382720` | 85 (the object's values, read live); the eye's ease: fitted, 45 |
| Z2b | **Spin** | Clock `t += dt`. Yaw `+= dt x s x k`, wrapped to `2 pi`, with `s = 9 t + 1` for `t < 2.5` (block `+0x14`, `+0x18`, `+0x1c`) and `0.5` after (`+0x20`), and `k = +0.1` if the byte `+0x3ac` is set, else `-0.1`. The yaw rate climbs from 0.1 to 2.35 rad/s over the first 2.5 s, then drops to 0.05 rad/s. For `t < 2.5` the pitch `+0x2a0` also turns, at `0.35` rad/s (block `+0x34`). | `0x000ae710`-`0x000ae7c4`, `0x000aede0`-`0x000aee08` | 65 |
| Z2c | **Look-at** | `focus += (craft - focus) x 0.6` per frame (block `+0x24`). | `0x000ae7c8`-`0x000ae7fc` | 70 |
| Z2d | **Eye** | `R = Ry(pi) . Ry(yaw) . Rx(-0.7) . Rx(pitch)` (block `+0x28` = `-0.7` rad), applied on the craft's body rotation. `Matrix_RotationY` (`0x006ca868`) and `Matrix_RotationX` (`0x006ca948`), both read whole. Eye = `focus + normalize(row) x dist + row x 5.0` (`cam+0x3b4`; the 5.0 term was missed until 2026-11-25, see the last section), where the row is the composed matrix's row the code takes at `0x000aebd8`; which row that is, is read from register flow only (55). | `0x000ae800`-`0x000aec50` | 60 |
| Z2e | **Distance and field, per frame** | `dist += (dist_target - dist) x 0.02`, `fov += (fov_target - fov) x 0.01`, **per frame** (not `dt`-scaled). Targets: for `t < 2.5`, field **20.0** and distance **5.0** (block `+0x2c`, `+0x30`); after, field **65.0** and distance **3.0** (`+0x38`, `+0x3c`). The 20 degree field is a target, not reached: from 65 it is near 30 degrees at 2.5 s (`20 + 45 x 0.99^150`), then eases back toward 65. | `0x000aec80`-`0x000aecdc` | 65 |
| Z2f | **Hand-back** | Modes 6, 13, 14 and 21 are outside H1's 8 s clock, so mode 2 holds until the results screen: `SPZone_UpdateFinish` waits 1.0 s, then `RaceManager+0x2dc4` (4.5 s), then opens `EndRace Results`. That is **5.5 s** of orbit (zone-destroy.md Z9). | `0x000758a0`, `0x000b08e4` | 75 |

What the player sees: the eye is a few units off the craft, so the craft and its white death
shell fill the frame. The view swings round fast and tight while the shell plays for 2.5 s,
then pulls back to a slow drift at a normal field while the debris flies.

## 2048 and Omega: mode 9, against HD and against each other

**2048 against HD: checked, differs** (the mode numbers and the hold mode).

**Omega against 2048: ported** (wired 2026-11-23, see "Wired"; cases 9 to 11 of the view dispatcher `0x01383a10` stay unread; its case 2 is read, see "The orbit on 2048 and Omega"). Same update, same mode set, same timer, plus
one Omega-only fallback.

| Row | 2048 v1.04 | Omega | Against HD |
| --- | --- | --- | --- |
| W1 enter | `Camera_SetMode` `0x8121963a` case 9: field `+0x270 = 35.0` | `Camera_SetMode` `0x01382720` case 9: field `+0x468 = 35.0` | HD mode 8, same value |
| W2 framing | `Camera_FramingFov` `0x81218576`: `2 atan(0.5 field / d) x 180/pi`, `65.0` with no station, eye at station `+0x108` | inline in `Camera_Update` `0x01381c70` (`atan(...) x 114.59155`), eye at station `+0xe0` | the same law |
| W3 subject | `Camera_SetSubject` `0x8121a156`: station from `ship+0x6fa4` else the pick (`0x812199c0`, unread); `fov0 = framing + rand(-10, 20)` at least `3.0`; sets mode 11 (HD 10) | the pick `0x013836a0` (unread) | the same; 2048's own-station path also copies the rates unrandomised |
| W4 view | `Camera_UpdateSpectatorView` `0x81218638`: `trunc(dt x 5999.9995)` substeps, focus frozen in state 6, `z.y x max(1 - fov x 0.008, 0.4)`, field x 1.5 on the same two flags | cases 9 to 11 of the view dispatcher `0x01383a10`, unread | the same law (D5 to D7) |
| W5 wobble | `Camera_UpdateView` `0x8121888a`: modes 9 to 11, `0.015 x distance` and three noise calls on `cam+0x1f4`, `+0x204`, `+0x214` (Ghidra drops the noise return, so its listing shows `d x d x 0.015`: not the law) | the three noise calls on `+0x3d4/+0x3e4/+0x3f4` are seen in `0x01383a10`; the factor is not read | the same as D5 |
| W6 8 s clock | the update at `0x81219c3c` (not a function in Ghidra): `cam+0x48 > 8.0`, gate byte `+0x274`, `>= 2.0`, `Camera_SetMode(cam, 0)` | `Camera_Update` `0x01381c70`: `cam+0x84 > 8.0`, Zone family `0x8181 >> (mode - 6)` plus an online check, gate byte `+0x46c`, `Camera_SetMode(cam, 0)` | H1 |
| W7 re-pick | not read | `d^2 >= 3600`, random among aims within 60 | D8 |
| W8 **hold mode 8** (new) | In mode 0, a subject in state 4, 5, 6 or 8 switches the camera to **mode 8**. In mode 8, a subject in any other state switches back to **mode 0** (`0x81219eb8`-`0x8121a008`). Mode 8's enter (`Camera_EnterHoldView_q`, `0x81219040`) freezes the eye where the last view was. Its view (`Camera_UpdateHoldView_q`, `0x81217616`) looks at the craft plus a lead of `7.5 x` its velocity, ties the look-at to the track section through `0x8121476a`, and holds the field at `65.0` (`97.5` with the two flags). | the same rule on the state set `{4, 5, 6, 8}` (`(s \| 1) == 5`, 6, 8). Mode 8's enter is `0x013832f0`. **Omega only**: when the station pick in mode 9 finds no station (mode 7), it goes to mode 8 instead | HD has no mode 8 of this kind and no state-driven switch: HD's chase camera does not leave a respawning craft |

So on 2048 and Omega the blast cuts to mode 9 as on HD (ship-destroy.md row 5). The 8 s clock (W6)
hands back to the chase camera (its subject chosen as in HD's H1). On a craft that respawns, the chase
camera's subject sits in states 5, 6 and 8, so W8 holds the view still until the craft races again.

## Wired

**2026-11-23 (`destroy-cameras-wire`).** `oag_title::destroyed::DestroyCamera` carries each title's law as data (`HD`, `VITA`, `OMEGA`, Pulse's `PULSE`); `oag_render::camera::destroy` has the station view's substeps, the hold view and `orbit`; `oag_raceplay::destroy_camera` runs the camera clock, the latch and the cut. Zone's orbit is `ZoneEnd::orbit_camera`.

| Title | Wired | Ground truth |
| --- | --- | --- |
| HD | mode 8: nearest-aim `Camera` node, `framing(60) + rand(-10, 20)` start, substeps of `trunc(dt x 6000)`, both rates drawn with their random factor, the 8 s camera clock then the chase camera on the player's own wreck. Zone end: the orbit | `destroy_camera_ground_truth::an_hd_destroyed_seat_picks_a_camera_node_near_the_wreck` (Talon's Junction, ten nodes) |
| 2048 | mode 9 as HD (substep constant `5999.9995`, which is also 100 substeps at 60 Hz in `f32`) plus the hold view after the hand-back. **2048's own circuits author no `Camera` node** (`altima`, `tower`, `sol`: no class `0xf7` in `track.vex`), so the pick has nothing and the camera is the hold view from the cut | `a_2048_destroyed_seat_holds_on_a_circuit_with_no_camera_node` |
| Omega | **ported** from 2048: its HD circuits under `Data\environments\` author the same ten to thirteen nodes as HD (Talon's Junction 10, Amphiseum 11, Modesto Heights 13, Tech de Ra 10, Vineta K 13, Zone 1 10); its 2048-era circuits author none and fall to the hold view, as the page's Omega-only fallback reads | `an_omega_destroyed_seat_is_cut_to_a_camera_node_then_held` |
| Pulse, Pure | unchanged: mode 5, one step a frame, exact rates, never hands back | `a_wrecked_player_is_seen_from_the_circuits_own_camera` |

**Chosen, not measured** (each cites this page's "Open" item):

- **No look-at wobble** (D5, Open 1): the noise's time source and tables are unread, so the look-at is the craft itself.
- **The camera clock** (H1): free-running from the race's first tick, reset at 8 s as the original's does in mode 0; what starts the original's clock (`0x000ab2d8`) is unread, so the phase at the blast, and so how long the station view lasts (0 to 8 s), is ours.
- **The hand-back subject** (H1, Open 2): the player's own wreck, as read for a single player; `RaceManager+0xe8` is unread.
- **The orbit's first entry** (Z2a, Open 3): the spin direction is fixed positive (read positive on Vineta K), the eye direction is the craft's rear axis turned by `Ry(yaw) Rx(-0.7 + pitch)` (which row of the composed matrix the original takes is unread), the up is the craft's.
- **The hold view** (W8, Open 4): eye frozen at the chase camera's at the hand-back (or at the cut with no station), look-at the craft plus `7.5 x` its velocity as the sim holds it, field 65; the track-section tie `0x8121476a` is unread and left out. Omega's substep constant is HD's (cases 9 to 11 of the view `0x01383a10` unread). 2048's pick `0x812199c0` is unread: 2048 takes Omega's no-station fallback.
- **2048's and Omega's Zone end** run HD's orbit; the law is read on both ("The orbit on 2048 and Omega").

Not run against the original: RPCS3 belonged to another lane, so there is no live pair; frames are ours, judged against the law, not a capture.

## Measured on RPCS3: the Zone end orbit (2026-11-24, `zone-orbit-clip`)

**Question.** Does our Zone end orbit ever put the eye inside the hull (`destroy-cameras-wire` saw it 0.5 off the craft at the cut and about 2.5 out at 0.5 s, inside a hull 13 units long)? **Falsifier written first:** an eye distance from the craft near 0.5 to 2.5 at the cut and 0.5 s, or the original's frames showing the craft from inside, would say the original does the same. Neither happened.

**Method.** RPCS3 0.0.43-20574 (own config copy, Null audio, Xvfb `:93`), HD EU, Racebox Zone on the first circuit (Vineta K), Feisar, a saved state at the grid. `scripts/emu-hd-zone-walk.sh` builds the grid, `scripts/emu-save-state.sh` saves it, `scripts/emu-restore-state.sh` restores it with the GDB stub, `scripts/rpcs3-hd-zone-end-orbit.py` does the rest: taps START RACE, at speed writes the player's shield `ship+0x5fa0` to 0, polls `ship+0x5fa4` for state 4, pauses at once and steps the game with resume/pause chosen from the craft's age `ship+0x6a30` (`+= dt`, it agreed with the world clock to 3 ms over 0.48 s). At each pause: a frame through `grab_with_clock` after 2.5 s for the RSX to present, and the pushbuffer camera (`ps3_pose.pick_camera`'s candidates; a candidate is the camera where its eye is unique to the shot). The distance is the eye to the craft's body position (`body+0x200`).

**Frames** (run 1, ages since the blast `0.00 0.25 0.53 0.98 1.60 2.52`; paired with ours at ticks 400+15, 30, 60, 150 of `--mode zone --force-wreck 400:0`, 960x540):

| Age since the blast | What the original shows |
| --- | --- |
| 0.25 s | The chase view still: the textured craft from behind and above, whole in the frame, the track around it |
| 0.53 s | The same view turned a little, the craft still whole and textured |
| 0.98 s | The craft seen whole from outside at an angle, under a wireframe shell, the circuit visible |
| 2.52 s | The wire shell filling the middle of the frame from outside, the eye below it, no circuit |
| 3.9 and 5.6 s | Debris flung around the eye (state 6: the craft is gone) |

**Distances** (superseded by "The orbit eye, read from the render camera" below: these are pushbuffer eyes, a frame or more old against the craft's present position; same state, same circuit; the blast eye was 13.87 in run 1 and 13.88 in run 2, to the digit):

| Age (s) | State | Eye to craft | `5 + 8.88 x 0.98^n` at n frames |
| ---: | ---: | ---: | --- |
| 0.00 | 4 | 13.88 | n = 0: 13.88 |
| 1.37 | 5 | 9.12 | n = 38 |
| 1.55 | 5 | 8.91 | n = 41 |
| 2.50 | 5 | 6.96 (and 9.43, a second unique eye) | n = 75 |
| 3.85 | 6 | 8.17 | the craft is no longer drawn |

The object's clock steps 1/30 s a row: RPCS3 runs the game at about 28 to 30 frames a second here, and the per-frame eases (0.02, 0.01, 0.6) are therefore half as fast per second as on a console at 60, which is what our 60 Hz tick gives. **Z2e's "per frame, not `dt`-scaled" is confirmed** by the object's own `dist` (0.83 at the fourth frame is `5 - 4.5 x 0.98^4`).

**The object, read per frame** (second pass, same state; `scripts/rpcs3-hd-zone-orbit-camera.py`). The camera object was found at `0x306efe90` by pausing at the blast and scanning for its rates (`+0x234` = 0.01, `+0x240` = 0.02, mode `+0x40` = 2); rows are one per game frame (the clock `+0x2c4` steps 1/30 s, RPCS3's rate here), 210 rows to 7 s:

| Field | Read | Law (Z2) |
| --- | --- | --- |
| mode `+0x40`, field `+0x288`, spin byte `+0x3ac` | 2, 35.0, 1 | 2, 35.0, positive: **confirmed** (this circuit; Omega picks it by track or at random) |
| `+0x238` dist | 0.83 at clock 0.133 (4th frame), 1.99 at 0.65, 3.14 at 1.45, 4.03 at 2.50, then 3.87 .. 3.07 at 2.76 .. 6.98 | `0.5 -> 5.0` at 0.02 a frame to 2.5 s, then toward 3.0: **confirmed**; targets `+0x23c` read 5.00 then 3.00, rate `+0x240` 0.020 |
| `+0x22c` fov | 63.2 at 0.133, 53.96 at 0.92, 41.4 at 2.50, then up: 58.99 at 6.98 | `65 -> 20` then `-> 65` at 0.01 a frame (targets `+0x230` 20.0 then 65.0, rate `+0x234` 0.010): **confirmed** |
| `+0x2a4` yaw | 3.027 at 2.50 s, then 3.251 at 6.98 | `0.1 (9t + 1)` for 2.5 s integrates to 3.06, then 0.05 rad/s (0.224 / 4.48 s): **confirmed** |
| `+0x2a0` pitch | 0.864 at 2.50, then constant | `0.35 t` for 2.5 s: **confirmed** (0.875) |
| `+0x390` focus | the craft's position within 1.3 units at 0.07 s, 0.5 at 1.8 s, exact from 3.5 s | `0.6` a frame to the craft: **confirmed** (the craft's own motion at 70 units/s is the residue) |
| state `+0x5fa4` | 5 to 2.50 s, 6 after, mode stays 2 | Z2f: **confirmed** |

So Z2b to Z2e hold as written (confidence 85 now, from 62 to 65). What the page's law did not say is why the eye on the screen is far: the pushbuffer eye is 13.9 from the craft at the cut and 9.1, 8.9 and 7.0 out at 1.4, 1.6 and 2.5 s, while the object's `dist` is 0.5, 3.1, 3.3 and 4.0. The eye is in no field of the object's first `0x800` bytes (no vec3 there follows it; the two vec3s `+0x250`/`+0x260` hold the craft's position at the blast and stay put). The modelled orbit eye `focus + dir * dist` points the same way as the read eye (behind and above, about 20 degrees apart); only the length differs. A low-pass on the eye's offset from the focus, `off += (dir * dist - off) * 0.01` a frame from the chase eye's offset, run over the read rows, gives 10.2, 9.6, 9.2 and 6.3 against the read 10 to 11, 9.1, 8.9 and 7.0 at 0.6, 1.37, 1.55 and 2.5 s (rates 0.005, 0.015 and 0.02 give 12.8 / 11.5 / 11.3 / 9.4, 8.1 / 7.5 / 4.1 and 6.8 / 6.2 / 2.6). That is the ease ours uses.

**The cause.** Ours placed the eye `focus + dir * dist` with `dist` starting at 0.5, so the eye sat 0.5 off the craft at the cut and 2.5 out at 0.5 s, inside the hull; the original's eye is carried in from the chase eye and is outside throughout. Confidence **85** for the object's law, **45** for the eye's ease (a fit to four distances, one circuit, one speed, one rate tried to two digits; the code that makes it is unread, the start branch `0x000a97f0` too).

**Superseded 2026-11-25** (see "Resolved by a live read of the render camera"): the fit, the chase-eye start and `ORBIT_CLEARANCE` below are withdrawn. **The fix** (`oag_render::camera::destroy::orbit::Orbit`): `dist` starts at 0.5 as read, and the eye's offset from the focus starts at the chase eye's offset and eases toward `dir * dist` at **0.01 a frame**. **Chosen, not measured:** that ease (above) and its start being the chase eye's offset; the eye direction's row (Z2d, 55); the hull box in the test (`5.5 x 3.5 x 13` scaled by `HULL_SCALE`, a Pulse-sized fixture, not HD's `<Misc>`). The test `hd_zone_end_orbit_never_puts_the_eye_inside_the_hull` fails without the fix (tick 0, the eye on the craft) and passes with it; `oag_physics::pair::half_extents` is now public for it. The first pass's "start at the chase distance and ease `dist` in at 0.02" fitted three distances by coincidence of rate and was replaced by this once the object's own `dist` was read.

**Review follow-ups (2026-11-24).** (1) The cut latches the chase rig's own eye (`Chase::eye`), not `tick_view_unshaken`, which is the cockpit's bolted eye in the cockpit view; what the original does when the player dies in the cockpit view is **not read** (the capture was in an external view). (2) A seek (`restore_for_viewing`) snaps the chase rig to the restored craft and restarts the destroy camera, so the orbit does not start hundreds of units out; in live play the rig's eye is one tick old at the cut. (3) Tests run on HD's 8.7-unit Close rig (the fixture's 4.1 made the first test a tautology) and on a craft flying at 0, 19, 93 and 240 units/s. **A departure the tests found:** with the eased eye on HD's rig the eye passes through the hull's box near 1.8 s (stationary craft), so while the craft is drawn (first 2.5 s) the eye is pushed out to the box's surface plus the 1.0 near plane (`ORBIT_CLEARANCE`, **chosen, not measured**; the original's own 28 frames-a-second frames never showed it, and a console at 60 Hz was not read). The eye's ease is also kept at or beyond the orbit radius.

**What still differs in the frames.** Ours draws the white shell's wire from the first frame; the original shows the textured craft at 0.25 and 0.53 s and the wire from about 1 s (Z8's `1 - max(0.4 timer, 0)` animation time, or its material, not ported that way). At 1.0 s ours is closer than the original's 1.0 s because of the frame rate above, not a law.

**2048 and Omega.** Superseded: see "The orbit on 2048 and Omega" (2026-11-25).

## Ready to wire (before 2026-11-23)

| Title | Mode | Eye | Look-at | Field | Motion | Hands back | Conf. |
| --- | --- | --- | --- | --- | --- | --- | ---: |
| HD | 8 | the circuit's `Camera` node whose aim is nearest the wreck (D1, D2) | the wreck + `0.015 d` noise wobble, frozen in state 6 (D5) | `2 atan(17.5 / d)` target, from `framing(60) + rand(-10, 20)` (D3) | eased in `trunc(6000 dt)` substeps at `0.06 x rand(0.5, 2)` and class rate `x rand(0.5, 1)` (D4, D6) | the 8 s camera clock, 0 to 8 s after the cut, to the chase camera on the player's own wreck in single player (H1); the race end does not cut (H3) | 70 |
| HD Zone | 2 | orbit `dist` 0.5 -> 5 -> 3 (read live), the eye `focus + dir x (5 + dist)`, no smoothing (read live, 90), `Ry(pi) Ry(yaw) Rx(-0.7) Rx(pitch)` on the body (Z2d) | the craft, `0.6` per frame (Z2c) | 65 -> 20 (2.5 s) -> 65, `0.01` per frame (Z2e) | yaw `+-0.1 (9t + 1)` rad/s for 2.5 s, then `+-0.05`; pitch `0.35` rad/s for 2.5 s (Z2b) | results at 5.5 s (Z2f) | 62 |
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
| Omega | `0x01383a10` | `Camera_UpdateView` | 80 | the dispatcher: three noise channels on `+0x3d4/+0x3e4/+0x3f4` for modes 9 to 11, then a switch on `+0x74` whose case 2 is the orbit view |
| Omega | `0x01388d70` | `Camera_Construct` | 80 | field `+0x468 = 60.0`, the class focus rates at `+0x464`, the noise channels, `+0x604 = 5.0` |
| 2048 v1.04 | `0x812156a0` | `Camera_Construct` | 82 | field `+0x270 = 60.0`, rate `0.06`, the class focus rates at `+0x26c`, the noise channels, `+0x3e4 = 5.0` |
| 2048 v1.04 | `0x81217ff4` | `Camera_UpdateOrbitView` | 84 | case 2 of `Camera_UpdateView` `0x8121888a`: the yaw, pitch, focus, dist and field laws of HD's `0x000ae6c0` |
| 2048 v1.04 | `0x81218cd8` | `Camera_EnterOrbitView` | 82 | mode 2's enter, called by `Camera_SetMode`: `"AI track data"`, dist `0.5`, target `4.0`, rate `0.02`, field `65.0` |

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
4. **Cases 9 to 11 of Omega's view** `0x01383a10` (the station view and its substep constant) and mode 8's enter `0x013832f0`; 2048's pick `0x812199c0`.
5. **A live read** (all rows; RPCS3 was another lane's here). Recipe: a Racebox race on
   Talon's Junction with weapons on, the player's shield written to `0.05` (`ship+0x5fa0`);
   break on `Camera_SetMode` (`0x000abc20`) with `r4 == 8`. From there, log per frame
   `cam+0x40`, `+0x4c`, `+0x1e4` (and its `+0x110`/`+0x120`), `+0x22c`/`+0x230`/`+0x234`,
   `+0x250`/`+0x260`/`+0x270`, and the craft's `+0x5fa4`. Rows D3, D4 and D6 predict the
   field's first steps; H1 predicts the frame mode 0 returns, at the camera clock's next 8 s.
   For Z2, the same in a Zone race with `r4 == 2`, logging `+0x2a4`, `+0x2a0`, `+0x238`,
   `+0x22c` and `+0x2c4`. **Z2's eye distance was read live 2026-11-24** ("Measured on RPCS3: the Zone end orbit"); the camera object itself was not found (the `+0x1f0` target scan matched no object with a mode field of 2), so yaw, pitch, field and clock are unread live, and mode 8's rows are still unread.

## The orbit eye, read statically against the live eye (2026-11-25, `zone-orbit-eye-read`)

Static, from the disassembly of `Camera_UpdateOrbitView_q` (`0x000ae6c0`, Ghidra's decompile of it is useless: VMX128 stops it); the live numbers are the ones under "Measured on RPCS3".

| What | Finding | Address | Conf. |
| --- | --- | --- | ---: |
| Eye build | `eye = focus + K x v + dist x unit(v)`, where `v` is row 2 of `Rx(pitch)` carried through the earlier matrix product (a unit direction), `K = cam+0x3b4`, `dist` the value just eased at `+0x238`. The eye is written to row 3 of the matrix passed in r5 (the render camera, `+0x40..+0x70`). Z2d's "eye = focus + normalize(row) x dist" **omits the `K x v` term**. | `0x000aeab8` (loads `+0x3b4`), `0x000aeb50` (stores it to the stack), `0x000aebe0` (`v x K + focus`), `0x000aec4c` (`+ unit x dist`), `0x000aec7c` (stores row 3) | 70 |
| `K` | `cam+0x3b4` = block `0x008c1520 + 0x10` = **5.0**, written only by the constructors `Camera_Construct` `0x000ab014` and `0x000ab89c`; the live rows (`cam3`, `cam5`, 228 rows) read 5.0 in every row. It is a constant, not a state. | above | 85 |
| Smoothing | **None in this function.** It reads no previous eye: the only eases are focus (0.6, `0x000ae7e8`-`0x000ae7fc`), `dist` (`+0x240`), field (`+0x234`) and yaw/pitch. The code after the dispatch (`0x000b0268`-`0x000b03a0`) only inverts the matrix; `0x000a3328`/`0x000a3340` are a two-word camera-id setter; `0x000a97f0` is a wrapper over `0x000a8b20` (the "AI track data" lookup, an out vector and a bool), not an eye. | as listed | 75 (negative) |
| Cockpit view | The orbit depends on the craft's body matrix and the object's own state only; nothing in `0x000ae6c0` or `0x000aa090` reads the chase or cockpit eye. A cut from the cockpit view therefore enters the same orbit as one from an external view. What the cut shows on its first frame is the live question below. | `0x000aa090`, `0x000ae6c0` | 60 |

**Resolved by a live read of the render camera (2026-11-25).** `scripts/rpcs3-hd-zone-orbit-camera.py` now logs, beside the object's rows, the render camera's matrix at `*0x987884 + 0x40` (the pointer the view code loads through `0x008a7f38`). The matrix is the view matrix as the post-dispatch code leaves it: its rotation rows are the camera basis and its row 3 is the **negated eye** (`eye = -row3`; checked: the eye is 7 units from the craft at the cut, where `R^T` or `R` applied to it gives hundreds). Over a 7 s run (240 rows, cut to state 6, `cap1`) the render camera's eye is **`5.0 + dist` from the object's focus on every row**, to 0.01: 5.83 at 0.13 s (`dist` 0.83), 7.68 at 0.98 s (2.68), 9.12 at 2.43 s (4.12), then 8.98 .. 8.05 with state 6 as `dist` settles toward 3 (3.98 .. 3.05). There is no eye smoothing. A second run (`--eye-probe`, 1.53 s) put the render eye at 8.32 from the focus (`5 + 3.32`), and the direction from the focus, `(0.79, 0.09, -0.60)`, matches `-X l0 + Y l1 - Z l2` of the body rows with `l = Ry(yaw) Rx(-0.7 + pitch) (0,0,1)` to 0.012 (HD's body Z is forward and X left, so that is ours `orientation * l`, and Z2d's `Ry(pi)` is the body's handedness). Confidence **95** for the length law ("History"), **80** for the direction (one frame).

**Why the earlier eye looked far and shrinking.** The pushbuffer scan reads eyes a frame or more old and measures them against the craft's *present* position, and the craft flies about 3 units a frame: at 1.53 s the pushbuffer's nearest eye was 9.66 from the craft where the render camera's, read in the same pause, was 8.54. The 13.88 "at the cut" is the chase camera's eye of the frame before the mode changed (state 4 row). The fit `5 + 8.88 x 0.98^n` and the "0.01 low-pass" were artefacts of that, and the `5` in it was `cam+0x3b4`. So the fitted ease (45) is **withdrawn**, not refined.

**Cockpit view.** `0x000ae6c0` and its enter read no previous eye, so a death in the cockpit view takes the same orbit; ours no longer latches the chase rig's eye at the cut. Confidence 75 (static plus the render camera's independence of the object's start). Not run live from the cockpit view.

**Ours (`oag_render::camera::destroy::orbit::Orbit`, 2026-11-25).** `eye = focus + dir x (EYE_STANDOFF + dist)` with `EYE_STANDOFF = 5.0`, no offset state. **`ORBIT_CLEARANCE` is removed**: with the read law the eye is at least 5.5 from the focus and never inside the hull's box at rest or at 19, 93 and 240 units a second (`hd_zone_end_orbit_never_puts_the_eye_inside_the_hull`, without the push-out). The hull test and a new law test (`the_eye_is_five_beyond_the_radius_from_the_focus_on_every_frame`, `hd_zone_end_is_the_close_orbit_five_beyond_its_radius`) fail with the 0.01 fit (13.9 at the cut). **Chosen, not measured:** the per-frame eases run at our 60 Hz tick, as for the rest of the camera (the console's own frame rate was 28 to 30 on RPCS3), so `dist` opens about twice as fast in seconds.

**2048 and Omega.** Read and **ported**: the next section.

## The orbit on 2048 and Omega (2026-11-25, `zone-orbit-omega-read`)

**Answer.** Both run HD's orbit: the same eye `focus + K x v + dist x unit(v)` with `K = 5.0`, the same constants, no eye smoothing. **Ported**: `ZoneEnd::VITA` and `HD_SILENT` take `..Self::HD`, so ours already runs `orbit_camera` with `EYE_STANDOFF = 5.0` on both, and `a_zone_end_is_the_close_orbit_five_beyond_its_radius_on_hd_2048_and_omega` (`crates/raceplay/src/tests/destroy_camera_titles.rs`) holds it for the three titles' data. Static only (no Vita3K or shadPS4 run), so capped below 85.

The cut into mode 2 at the Zone end was already read on both ([zone-destroy.md](zone-destroy.md), row Z2). Omega's `0x01383a10` is the view dispatcher (`Camera_UpdateView`: the three noise channels on `+0x3d4/+0x3e4/+0x3f4` for modes 9 to 11, then a switch on `cam+0x74`), not the orbit view alone; the orbit is its `case 2`.

| What | HD | 2048 v1.04 (`Camera_UpdateOrbitView` `0x81217ff4`) | Omega (`case 2` of `0x01383a10`) | Conf. (2048 / Omega) |
| --- | --- | --- | --- | ---: |
| Eye | `focus + K v + dist unit(v)`, `v` row 2 of the composed matrix | `vld1.32 {d2[]}` from `cam+0x3e4` at `0x8121839e`, `vmla.f32 q6,q8,d2[0]` at `0x812183ae` onto the focus `cam+0x3c0`, then `+ unit(v) x dist` (`0x8121843a`) | `eye = row x cam+0x604 + focus + unit(row) x dist`, `row` the third matrix row at `0x01a164a0` | 84 / 82 |
| `K` | `cam+0x3b4 = 5.0`, constructors only | `cam+0x3e4`, written once, `vstr.32` at `0x8121590e` in the constructor `0x812156a0` from `0x815202ec` = `5.0`; no other store in the camera code; the other `+0x3e0`/`+0x3e4` stores are on other objects (e.g. the vtable constructors `0x812bb914`, `0x812be628`, `0x812bfe14`, the AI's `Ai_Construct`) | `cam+0x604`, written once, `MOV qword [RBX+0x604], 0x40a00000` at `0x01389098` in the constructor `0x01388d70`; no other store in the camera code reaches it; the other stores covering that offset are on other objects (`0x012e5470`'s `+0x604` is on `ship+0x6518`, built by `0x0131c170`; `ShaderConstants_InitNameTable`'s 16-byte store at `+0x5f8` is on its shader table) | 84 / 82 |
| Smoothing | none | none: the view reads no previous eye; the code after the switch only inverts the matrix | none, same | 80 / 78 |
| Focus ease | `0.6` a frame | `0x81520318` = `0.6` | literal `0.6` | 84 / 82 |
| Yaw rate | `(9t + 1)`, then `0.5`, times `+-0.1` | `0x81520320` = `9.0`, `0x815202e8` = `1.0`, `0x8152031c` = `0.5`, `0x81520304` = `2.5` | literals `9.0`, `1.0`, `0.5`, `2.5`, `+-0.1` on `cam+0x5fc` | 84 / 82 |
| Pitch rate | `0.35`, first 2.5 s | `0x81520324` = `0.35` | literal `0.35` | 84 / 82 |
| Turn | `Ry(pi) Ry(yaw) Rx(-0.7) Rx(pitch)` on the body | `0x81520328` = `-0.7`, `pi` built at `0x81218114` | constants `pi` (`0x017ed700`) and `-0.7` (`0x017ed790`) | 75 / 65 |
| Targets and rates | dist `5.0` then `3.0` at `0.02`; field `20` then `65` at `0.01` | `0x8152030c`/`0x81520314`, `0x81520308`/`0x81520310` | literals, and `+0x424` = `0.02`, `+0x418` = `0.01` set by the enter | 84 / 82 |
| Enter (dist `0.5`, target `4.0`, field `65`) | Z2a | `Camera_EnterOrbitView` `0x81218cd8` | `Camera_SetMode` case 2 (`0x01382720`) | 84 / 82 |

The turn's multiplication order was not re-derived on Omega (the sin and cos are inlined), hence 65; the direction's live check is HD's alone.

**The update order, wired (2026-11-26).** All three ease `dist` toward the target written on the previous frame (`4.0` on the first, from the enter) and step the pitch after the eye is built (HD: the ease at `0x000aeb70`-`0x000aebd0` reads `+0x23c`, the new targets are stored after the eye at `0x000aecb4`/`0x000aecb8`, the pitch step is at `0x000aede0`; the field does use this frame's target). HD's live `+0x238` of 0.83 at the fourth frame fits that order (`0.8305`; an ease toward 5.0 from the first frame gives 0.849). Ours does the same (`oag_render::camera::destroy::orbit::Orbit`: `dist_target`, and a pitch step held to the next frame), tested by `the_radius_eases_toward_the_target_the_previous_frame_wrote` and `the_pitch_steps_after_the_eye_is_built` (`crates/render/src/camera/destroy/orbit.rs`).

**2048 and Omega: ported.** One `Orbit` serves all three, with no per-title data, and `a_zone_end_is_the_close_orbit_five_beyond_its_radius_on_hd_2048_and_omega` pins the first frame's `5 + 0.57` under each title's data. The Detonator end takes the same orbit (`DetonatorEnd::orbit_camera`).

## History

- 2026-11-25: HD's length law (`eye = focus + (5 + dist) x unit(v)`) raised from 90 to 95: the live render camera read, plus the same code in 2048 and Omega. The direction stays at 80.
