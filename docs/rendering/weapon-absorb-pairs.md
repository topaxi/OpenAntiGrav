# A weapon absorb on the player's craft, ours against the original, per title

2026-11-09 (`absorb-fx-measure`). The player's craft absorbs a held pickup, chase camera, craft at rest on its
grid slot, and the frames at about 0.1 s, 0.5 s and 1.0 s after the press are compared with ours, scored on the craft
box (not the whole frame) as the difference from a no-absorb control taken from the same state on both sides.
Pulse PSP, Pure and HD are paired. 2048 and Omega are not, for the reasons at the end. Burst, hull overlay and HUD are
built on every title (`oag_raceplay::absorb`, `oag_fx::hull_overlay`, `oag_fx::absorb_shell`; see
[`hull-sparks.md`](hull-sparks.md), the burst table, and
[`absorb-feedback.md`](../ghidra/functions/ps3-hdfury-eu/absorb-feedback.md)); this page is what a player still sees
differently.

## What would have falsified it

Recorded after the first captures and before any scoring (the lane's brief named the question, the box and the control, not
the threshold). For each title, "ours already matches": the original's craft-box delta at about 0.5 s
(the overlay's peak) would be inside the noise of two controls, or ours' delta would be within 10 % of it with the same
tint. The noise floor is the original's own control-to-absorb mean absolute difference before the effect starts (Pulse
and Pure, frame 3, 2 to 4 luma units; HD 8 to 11 because the rivals move between runs). The tint is the mean
blue-minus-red of the same difference. Pulse's luma passed, its HUD bar did not; Pure and HD failed on luma, tint or
tail. The control is the same state without the press on the original. Ours' control differs by title: HD's is ours with the
pickup given and `--button none` (so the held-pickup icon cancels); Pulse's and Pure's are ours with no `--give` at all,
so ours' absorb frames carry the held-pickup icon (a `--give` artefact: a purple hexagon with four dots at the top centre
of the frame on Pulse, the top box of Pure's table reads -8 from it) that the original's do not. Every box scored
excludes it except Pure's `hud_top`, which is not ranked.

## Method

| | Pulse (PSP, USA) | Pure (PSP, USA) | HD (PS3, EU) |
| --- | --- | --- | --- |
| Original | PPSSPP v1.20.4, software renderer, 480x272 | same | RPCS3, Write Color Buffers on, 1278x718 |
| State | `race-platinum-white-single-grid.ppst` (Platinum Rush, Assegai, 8th of 8, weapons on, at rest) | `pure-single-vineta-feisar-race.ppst` (made for this lane: Vineta K, Vector, Feisar, Single Race, 8th of 8, at rest) | `hd-racebox-talons-venom-weapons-grid`, START RACE, 5 s of lap clock, craft held at rest |
| Press | the held pickup written at `craft+0x1bc` of the weapon record while stopped, circle held 3 frames (`scripts/psp-absorb-frames.py`) | the held id written at `weapon record+0x1d8`, circle held until it reads 0 (the absorb), see below | pickup state 1 written at `*(ship+0x5edc)+0x204` (mirrored `+0x208`), circle held until `craft+0x7a5c` rises to 1.0 |
| Frame clock | `craft+0x830` minus the absorb stamp `craft+0x878`, one frame per hit of `Weapons_DispatchFire`; the screenshot lags the stop by about 2 frames | frames counted from the first player frame whose held id is back at 0; lag about 2 frames, unmeasured on Pure | single game frames stepped off the lap clock, `age = lap clock - stamp`, the timer read live (`1.0 - timer`) |
| Ours | `--pose-from` the original's pose row, `--camera-view close`, `--camera-fov 60`, `--give mine`, circle at tick 520 (race clock 4.2 s), ticks 520 + frame - 2 | same flags, Pure, circle at tick 780 (HUD 8.4 s, the original's clock at frame 0), ticks 780 + frame - 2 | `scripts/hd-weapon-pair.py --button circle` on the original's pose and RSX camera, fired at the stamp clock; a second run with `--button none` is ours' control |
| Repeats | 2 absorbs, 1 control, 1 boot | 2 absorbs, 1 control, 1 boot | 2 absorbs, 1 control, 3 restores of one state |

Ours' pose (the original's, read at the state's first stop, `psp-trace.py --camera`, world units): Pulse position
`(-542.789, 2.901, -292.866)`, forward `(0.9984, -0.0166, 0.0542)`, up `(0.0141, 0.9988, 0.0472)`, camera `(-553.980,
6.084, -293.325)`, race clock 4.02 s at the stop; Pure position `(177.199, 37.171, -116.810)`, forward `(-1.0, 0.0022,
0.0026)`, up `(0.0022, 1.0, 0.0)`, camera `(188.456, 40.147, -116.839)`, race clock 7.02 s at the stop (HUD 8.4 s at frame 0).
Pulse: `--track Data\Environments\13_Track\track.vex --team Assegai --class venom`; Pure: `--track
Data\Environments\01_Vineta_K\track.vex --team Feisar --class vector`; both `--mode single_race --camera-view close
--camera-fov 60 --size 480x272`, `--world-clock-offset 8.765` on Pulse.

The original's frames are paired by age, not by wall time, because the original runs a variable timestep. Ours is at
60 Hz. A one-frame error in the lag (0.017 s) is inside every number below.

The frame tables score the 480x272 frame (Pulse, Pure) or the frame resized to 1280x720 (HD). Boxes: Pulse craft
`(175,140)-(305,250)`, bar `(305,222)-(445,258)`; Pure craft `(150,150)-(330,272)`; HD craft `(400,330)-(880,690)`.
Each cell is the mean luma difference (Rec. 709, absorb minus control).

## Pulse PSP

Both runs agree to within 1 unit at every age but 1.0 s.

| Age | Original (run 1 / run 2) | Ours |
| --- | --- | --- |
| 0.10 s | +5.5 / +4.9 | +0.9 |
| 0.20 s | +7.8 / +8.6 | +4.3 |
| 0.30 s | +16.6 / +16.8 | +13.2 |
| 0.50 s | **+42.6 / +43.4** | **+41.1** |
| 0.75 s | +32.3 / +32.8 | +31.3 |
| 1.00 s | +7.2 / +4.2 | +9.1 |

Ranked by what a player sees:

1. **The energy bar's flash has no glow and no second state.** The bar fill's mean luma over the window (rows 248 to
   258, x 315 to 440; normal 164, both sides): the original alternates **207** (bright cyan, with a bloom halo) and
   **239** (white, with a halo), 7 to 8 frames each, from frame 3 to frame 62, then 163. Ours alternates **163**
   (nothing changes) and **232** (flat white, no halo) with the same period; the white phases coincide with the
   original's within a frame (original frames 10-16, 25-31, 40-46, 55-61, ours ticks 8-14, 23-29, 38-44, 53-60 after
   the press). So the 8 Hz period and the window are right, the off phase and the bloom are missing. In the original
   the bar is bloomed during the whole window, which reads as the widget's `+0xf4 = 0xff` write feeding the glow mask
   (`+0xf4` is the open question in [`shield.md`](../ghidra/functions/psp-pulse-usa/shield.md)); confidence 60, not
   read at the draw call. Frames: the 0.1 s and 0.5 s pairs of the reference set (below), the bar at the bottom right.
2. **The first 0.2 s are dimmer and about two frames late.** At 0.1 s the original's hull already carries the wing and
   nose crackles and a +5 delta, ours +0.9; at 0.3 s ours reads 21 % below. The hull overlay's ramp starts a frame or two
   earlier in the original. Both draw the wingtip ring glints; the peak (0.5 s) matches to 4 % and the tint to 0.5.
3. **At 1.0 s ours' hull is slightly brighter (+9.1 against +7.2 and +4.2)**, and both draw the dark ground ring on the
   left of the craft. Inside the run-to-run spread of the original.
4. **"Short crackles against long streaks" (the thread's open item) is not reproduced at this camera distance.** Both
   show thin streaks of the same length range around the hull at 0.3 s to 0.5 s. Closed for the chase view.

Nothing else differs that a player would name. The hull overlay, the ground ring and the burst are matched.

## Pure PSP

Every number is new: no Pure absorb had been captured. How the absorb was reached on the USA `BOOT.BIN` (read this
session, confidence 80, the grant and the 3-frame consume seen live on two boots):

- the player's entity is `*(Ship_UpdateCraft.a0 + 0x1c4)`, the one whose `+0x488` is 0 (rivals read 2); its weapon
  record is `*(entity + 0xd0)` and the **held id is the word at record `+0x1d8`** (0 none, 1 to 10 a weapon);
- `Ship_UpdateWeapons` (`0x0892935c`) reads the absorb button as `*(*(entity + 0xc8) + 0x48) + 0x16`, and with a held id
  calls `FUN_08925e20`, the burst and `ABSORB` cue already read in
  [`rocket-and-collision-fx.md`](../ghidra/functions/psp-pure-usa/rocket-and-collision-fx.md);
- writing id 1 into the record and holding circle took it back to 0 three frames later on both boots;
- `FUN_08925f4c`, the handler of the token that carries the same event, also writes **`*(_DAT_0005364c + 0x98) = 0xff`**
  (a HUD byte, the Pure counterpart of Pulse's `+0xf4 = 0xff`), so Pure has the HUD flash too. Not renamed (confidence
  below 70 on the field).

| Age | Original (run 1 / run 2) | Ours |
| --- | --- | --- |
| 0.17 s | -4.6 / -4.0 (noise) | +1.0 |
| 0.33 s | +10.6 / +7.7 | +3.7 |
| 0.50 s | **+18.3 / +16.2** | **+7.0** |
| 0.75 s | +5.4 / +10.7 | +3.8 |
| 1.00 s | +3.0 / +4.4 | 0 |

Tint (blue minus red) at 0.5 s: original -5.8 and -6.1 (pink), ours -0.4.

Ranked:

1. **Ours drew no hull flash on Pure (fixed: see "Fixes and re-pairs on a second machine").** The original's hull goes pale pink-white at 0.5 s over the whole hull and
   nose, ours only gets the light of its own streaks: peak +7 against +17, tint flat against pink. Pure's disc ships
   `absorb_surface.mip` (see [`weapons.md`](../ghidra/functions/psp-pure-usa/weapons.md)), the texture Pulse's overlay
   and HD's shell use; ours does not draw it on Pure (Pure's `oag_title::Effects` is the all-`UNREAD` table, `hull_overlay` included). Frame: the 0.5 s pair of the reference set.
2. **The burst is long spikes on ours and a short crackle plus a glow on the original.** At 0.5 s ours draws pink
   star-shaped streaks 30 to 60 px long from the wing and tail locators; the original draws short crackles and one bright
   pink bloom at the nose and wings. Visible in every frame from 0.17 s to 0.75 s.
3. **A purple ground ring at 0.33 s to 0.5 s** under the craft in the original, none in ours.
4. **HUD.** The original draws two orange bars at the top centre at 0.33 s and 0.5 s (the pickup notice) and its energy
   bar brightens around 0.75 s (`+12` in run 2, `+5` in run 1: run to run noise, not settled). Ours shows its own
   held-pickup icon in that place (an artefact of `--give`) and no bar change. The `+0x98` flag above is the
   candidate for the original's flash.

## HD (PS3)

Frame step and timer reads are live (the shell fade at `craft+0x7a50` read `0.19, 0.34, 0.65, 0.89, 0.95, 0.99` at
ages 0.07, 0.10, 0.20, 0.30, 0.50, 0.75 s: the shell is up by 0.3 s and falls only after 1.0 s). The tint is
the shell's, not the burst's.

| Age | Original (run 1 / run 2) | Ours | Blue minus red, original / ours |
| --- | --- | --- | --- |
| 0.05 s | +0.1 / +0.1 | +1.7 | -0.1 and +1.2 / +1.5 |
| 0.10 s | +1.3 / +0.2 | +5.2 | +0.8 and +0.7 / +4.3 |
| 0.20 s | +6.4 / +5.6 | +13.4 | +8.6 and +7.5 / +8.5 |
| 0.30 s | +13.2 / +13.5 | +19.4 | +15.6 and +15.7 / +9.7 |
| 0.50 s | **+21.4 / +22.0** | **+24.4** | +18.4 and +19.0 / +9.4 |
| 0.75 s | +20.2 / +23.5 | +24.4 | +17.3 and +18.4 / +8.3 |
| 1.00 s | +6.5 / +6.0 | +17.8 | +10.8 and +9.8 / +7.4 |
| 1.20 s | +2.8 / +2.7 | +6.8 | +6.0 / +5.5 |

The original's own craft-box mean absolute difference is 8 to 12 even before the effect (the rivals and the craft's
hover move between runs), so differences under about 3 luma units are not read.

The controls agree: the original's and ours' control craft boxes read mean RGB `(68.0, 97.5, 106.6)` and `(67.2, 97.6,
107.2)` at 0.5 s, luma 91.9 on both, so the difference is not exposure. At 1.0 s the shell is at full fade on the
original (fade 0.998, timer 0, read live) and the craft box is only +6.5 over its control, RGB `(+3.6, +5.8, +14.1)`:
lavender-blue, the livery showing. Ours at 1.0 s is `(+16.1, +17.8, +23.8)`, +17.7. The 0.5 s to 0.75 s peak is the
shell plus the burst; with the shell at about +6.5 on the original and +17.7 on ours, the burst is about +15 on the
original and about +7 on ours.

Ranked:

1. **The shell is about 2.7 times too bright and too white (brightness fixed, tint open: see "Fixes and re-pairs on a second machine").** The same craft box, shell alone: +6.5 against +17.7 luma,
   blue-minus-red +10 against +7, red gain 4 against 16. A player sees the hull bleach to white-cyan where the original
   keeps a blue hull with the gold livery showing (frames at 0.5 s and 1.0 s). The thread's "ours reads bluer than the
   original's white-lavender" is the other way round on this pair.
2. **The burst is about half as strong.** The original's wingtip locators throw large bright balls with electric
   streaks that also light the floor from 0.3 s to 0.75 s (gone by 1.0 s); ours draws a smaller white glow at the same
   places (about +7 against +15 once the shell is taken out).
3. **Timing matches.** The original's shell fade rose `0.19, 0.34, 0.65, 0.89, 0.95, 0.99` at ages 0.07, 0.10, 0.20, 0.30,
   0.50, 0.75 s (live), and after 1.0 s both sides decay at a similar rate (original 6.5 to 2.8, ours 17.8 to 6.8). The
   +13.4 against +6 at 0.2 s is the same shell gain showing earlier, not a faster rise. There is no tail or rise
   difference to wire.
4. **HUD.** Both blink the shield hex between its blue `100` state and a pale state (blue minus red, hex box: original
   41 to 23, ours 37 to 18); with eight ages sampled the phase cannot be told apart. The original's frames carry
   the `DAMAGE` and `Absorb` labels beside the hex, which ours does not (ours prints the held pickup's name there, a
   `--give` artefact). Whether those labels are absorb-only was not isolated: the control shows a faint `DAMAGE` too.

The cockpit shell (`vr_absorbinternal_cockpit`) is not exercised: the camera is the chase view.

## Fixes and re-pairs on a second machine (`absorb-fx-wire`, 2026-11-09)

The originals were captured again with the scripts named under "Reproducing"; every number below is the box mean luma of the
absorb frame minus the control frame, Rec. 709, on the page's boxes. A whole-frame delta under 1 grey is noise. The numbers
above (the first capture) and these differ in what the original did, not only in ours: see each item.

### Pure: the hull overlay is drawn (landed)

`FUN_0886e954` is Pure's `HullOverlay_Submit` and `FUN_08924998` its `HullOverlay_AbsorbFade`: texture `absorb_surface.mip`, state
`0x282`, `10.0` repeats, `-1.5 * pulse` scroll, the pulse folded at `1.0`, and `(clock + 0x960 - stamp + 0x9ac) * 2` inside one
second, read at instruction level and the fade live (`docs/ghidra/functions/psp-pure-usa/hull-overlay.md`, confidence 85 for
the draw, 90 for the fade). `oag_pure::effects::LOOKS.hull_overlay` is on; the shared `oag_fx::hull_overlay` does the rest.
Test: `absorb_ground_truth.rs`, `pure_lights_the_hull_for_one_second_after_a_pickup_absorb` (fails with the rule off).

Capture: Pure USA on PPSSPP 1.20.4 (software renderer), the Time Trial state `time-trial-vineta-vector-feisar-restart.ppst`
(Vineta K, Vector, Feisar, craft at rest; the Single Race state of the first capture is not on this machine, and the
absorb needs only the weapon record), held id 1 at record `+0x1d8`, circle held until it reads 0, 30 updates per second. Two
absorbs and one control; ours with `--give mine`, circle at tick 780, shot at `780 + 2 * frame - 2`. Craft box `(150,150)-(330,272)`.

| Age | Original (run 1 / run 2) | Ours before | Ours after |
| --- | --- | --- | --- |
| 0.17 s | +0.4 / +0.9 | +1.0 | +1.8 |
| 0.33 s | +8.7 / +11.4 | +3.3 | +10.2 |
| 0.50 s | **+12.9 / +14.5** | **+6.6** | **+14.3** |
| 0.73 s | +5.0 / +12.2 | +4.7 | +10.2 |
| 1.00 s | -1.6 / +1.9 | 0.0 | +0.1 |

Blue minus red at 0.5 s: original -4.2 and -3.9, ours before -0.3, after -3.1. Read as a player: the hull goes pale pink-white over
its whole surface in both, with the same decay. The first page's +17 at 0.5 s was a Single Race state (rivals, a different pose);
this capture's peak is +13 to +15.

**Still open on Pure (the burst and the ring, blocked, not fixed).** Pure's `WO_WEAPON_ABSORB.POB` is not Pulse's: one emitter (the
streak one, class 6, life 7 ticks, colour mode 0 over a purple `(109, 22, 167)` to magenta `(181, 14, 147)` table) and **no `rings`
emitter**. The ground-ring halos the original draws round the wingtips are therefore not in that file; their source is unread.
The burst's total light matches (ground boxes either side of the craft at 0.5 s: original +8.4 and +8.3, ours +7.3 and +13.8, both
carrying the same tint), so what differs is its shape: the original's streaks are wide, soft and dim purple; ours are 1 to 2 px
hard lines with a white core. That is the streak draw in `crates/fx/src/psys`, which another lane holds, and it was not touched.

### HD: the shell is halved and reads the craft's block (landed); the tint is open

The player of `hd-racebox-talons-venom-weapons-grid` is **`Feisar_c1`** (read from the craft array: `*(*(craft + 0x6298) + 0x1b4)`
is `Data\Ships\Feisar_c1`), the dark concept hull with blue wingtips, not the standard Feisar: ours is run with
`--team feisar_c1 --variant concept1`. The first page's "blue hull with the gold livery showing" is the standard hull; the numbers
there were taken against it.

Two causes found in the path, both from the disc:

1. **The shell had no `/2`.** `hd_absorbinternal`'s fogged blocks #2 to #9 all close on `MAD /2 H0.xyz` (`ps3-microcode.py fp-file`
   on the material, the scale field), like every scene program of draws 87 to 522
   (`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "Every lit program ends on a /2 output scale"). `shade.wesl`'s absorb branch
   now multiplies by `scene.light.output_scale` (0.5 on HD, 1.0 elsewhere). Confidence 85 (the scale field is read for all eight
   blocks; which block the race draws is the fogged family on Talon's Junction, not seen in a draw).
2. **The shell was on a scene block of its own.** The shield and absorb drawables were never given a `SceneKind`, so the shell read
   `Scene::off`: no output scale, no fog, and `scene.time` zero, so the `5 * time` scroll of its texture tap never ran.
   `share_scene_blocks` now puts it on `SceneKind::Ship` (the craft's block). Test:
   `tests::scene_blocks::the_absorb_shell_reads_the_craft_scene_block`.

Capture: RPCS3 (Write Color Buffers on, 1760x990 trimmed, compared at 1280x720), the same state, `scripts/rpcs3-hd-absorb-film.py`
(grant the pickup, hold circle until `craft + 0x7a5c` rises, single game frames, `age = clock - press clock`). One absorb and one
control (a second absorb run reached its camera on only three frames and was not scored). Craft box `(400,330)-(880,690)`.

| Age | Original | Ours before | Ours after |
| --- | --- | --- | --- |
| 0.08 s | -0.4 | +1.2 | +1.2 |
| 0.13 s | -0.2 | +4.5 | +4.6 |
| 0.21 s | +2.8 | +7.9 | +2.6 |
| 0.33 s | +6.9 | +11.4 | +11.8 |
| 0.53 s | +7.7 | +15.3 | +15.9 |
| 0.76 s | +8.3 | +17.3 | +13.6 |
| 1.02 s | **+5.8** | **+13.1** | **+5.8** |
| 1.24 s | +3.4 | +4.5 | +2.9 |

At 1.0 s the shell is alone (the burst is gone): ours drops from 2.3 times the original to equal. Blue minus red at 1.0 s: original
+10.8, ours +6.7: **the colour is still pale lavender-white where the original is a saturated blue** (seen in the pair, not fixed;
the texture tap and its transfer are the lead). The first page's "2.7 times" was this factor over a different hull.

**The burst on HD is about twice too strong, not half as strong.** At 0.3 s to 0.75 s ours is +11.8 to +15.9 against the original's
+6.9 to +8.3 although the shell is now right, and the shell is about +4 of that. `renderer.md` item 4 already says particles and
effects (draws 444 to 522) end on `/2` and `oag-fx` does not apply it: the burst is `psys`, held by another lane, so this is the
number for it, not a fix.

### Pulse: the energy bar (not re-measured)

The Pulse state of the first capture (`race-platinum-white-single-grid.ppst`) is not on this machine and a GE dump of the bar's draw
(the confirmation the thread asks for before wiring `+0xf4`) was not taken. Nothing changed.

### 2048 and Omega

The shell fix is **checked, applies, not wired**: neither title draws the shell (their effects tables are the unread one, and whether
their archives ship an `AbsorbEffect` model was not censused), and Omega's `.rcsmaterial` and 2048's are re-cooked, so the `/2` of
the fogged blocks is not assumed. The Pure overlay is **not checkable** there (Pure is not their lineage). The unpaired reasons
above stand.

## 2048 and Omega

Not paired. Exactly where it stopped:

- **Omega (shadPS4 0.20).** `scripts/shadps4-omega-pair.py` reaches the grid and stops before the countdown. A held
  pickup needs: the Racebox Weapons row turned on (HD's Racebox default is off, `emu-hd-race-walk.sh` presses the row;
  the Omega walk does not), a race driven to a pad with no steering feedback, and an absorb press timed on a frame clock
  shadPS4 does not expose (no memory readout). No Omega absorb function or player pickup record is recovered in
  `docs/ghidra/functions/ps4-omega-eu/` (the shell colour is, in `shield.md`; `weapon-ai-fire.md` has the AI object's held
  id at `+0x44`, a lead). Unblocks it: the Omega Racebox weapons walk,
  or the native port's `/proc/<pid>/mem` (`docs/tools/compare-page.md`, "Omega") with the HD held-slot offsets
  checked against the eboot.
- **2048 (Vita3K).** `scripts/vita-2048-pair.py --walk` reaches the grid before GO; Vita3K has no state channel, and its
  guest `/proc/<pid>/mem` can be written (the hull-shake patch does) but no 2048 held-slot or absorb address is read
  (the AI's `weapon-ai-fire.md` in that folder reads a held id and clock from the AI's own object, which is a lead, not
  the player's pickup record or the absorb function).
  Unblocks it: read `Ship_PlayAbsorbFeedback` and the pickup slot in the 2048 eboot (HD's are `0x000d9398` and
  `ship+0x5edc`), then the HD recipe above with a write through `/proc`.

Cross-title status (CLAUDE.md, "A 2048 finding is checked against Omega"): the burst is **ported** (both ship the six
`absorb` locators and HD's three mirrored pairs, `hull-sparks.md`). The HD shell is **not wired** on either: `oag_hd`'s
effects table sets `absorb_shell` and the 2048 and Omega tables are the all-unread one, so neither draws a shell; whether
their archives ship an `AbsorbEffect` model was not censused (named open in the thread). The HD tint and brightness
differences above are **not checkable** on 2048 and Omega until a live absorb frame exists for them.

## Confidence

Per `docs/reverse-engineering/confidence-rubric.md`, measured frames, not decompiled claims; each is two absorb runs on one
emulator boot (Pulse, Pure) or three restores of one state on one boot (HD), so "seen once" on the boot level:
Pulse's bar finding 75 (both runs show the same two phases and period); Pulse's overlay match 75; Pure's missing hull
overlay and burst look 75; Pure's held-id and consume reading 80 (live on two runs, decompile read); Pure's `+0x98` HUD
write 60 (read, effect not seen); HD's shell ratio and tint 70 (controls equal, run to run spread of the original
small, ours seen once); HD's burst ratio 60 (derived by subtracting the shell). Pose and lag: one frame of lag
uncertainty on every age.

## Reference set

The frames behind every number are derived game data, not in this tree. The original's frames (`theirs_*` and
`theirs-control_*`, 0.1, 0.5 and 1.0 s per title) are the ones `data/reference` admits, for the lead to adopt as
`data/reference/absorb-pairs/<title>/`; ours' frames and the side sheets regenerate from the poses and flags above.

## Reproducing

New on a second machine (`absorb-fx-wire`), each script's docstring has the full line:

- Pure original: `scripts/psp-pure-absorb-frames.py --port P --display :N --home $PP_HOME --state <ppst> --out DIR [--control]`; ours:
  `scripts/absorb-ours-frames.py --circle-tick 780 --frames 5,10,15,22,30 --step-ticks 2 --lag-ticks 2 --out DIR -- <oag-game
  command with --pose-from the pair trace of scripts/psp-pure-pair.py>` (and `--control`); score with `scripts/absorb-score.py --box
  150,150,330,272 absorb.png control.png ...`.
- HD original: `scripts/emu-restore-state.sh <lane> data/saves/hd-fury/hd-racebox-talons-venom-weapons-grid.SAVESTAT.zst --pause`, then
  `OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-absorb-film.py --image data/images/hdfury-ps3-eu-dec.iso --out DIR
  [--control]`; fill each `NN.json`'s `track` by hand (`/data/environments/talons_junction/track.vex`), then
  `scripts/hd-weapon-pair.py --capture DIR --weapon mine --team feisar_c1 --button circle|none --camera-from NN --extra --variant
  concept1`, and score `theirs_NN.png` and `ours_NN.png` of the absorb run against the control's with `absorb-score.py --size 1280x720
  --box 400,330,880,690`.

Earlier recipe:

Pulse: restore the state, `scripts/psp-absorb-frames.py --port P --frames 66 --edram --display $DISPLAY` (and
`--control`), then `oag-game ... --pose-from <pair trace> --give mine --input-script <(tick none, 1 circle)` at ticks
520 + age - 2. Pure: the same with the entity and held-id reads above (a scratch driver, the Pure addresses are on this
page). HD: `rpcs3-hd-missile-film.py`'s frame stepping with the grant and the circle press of this page's method table,
then `scripts/hd-weapon-pair.py --weapon missile --team feisar --button circle` and `--button none`.
