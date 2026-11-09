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

Written before the captures. For each title, "ours already matches": the original's craft-box delta at about 0.5 s
(the overlay's peak) would be inside the noise of two controls, or ours' delta would be within 10 % of it with the same
tint. The noise floor is the original's own control-to-absorb mean absolute difference before the effect starts (Pulse
and Pure, frame 3, 2 to 4 luma units; HD 8 to 11 because the rivals move between runs). The tint is the mean
blue-minus-red of the same difference. Pulse's luma passed, its HUD bar did not; Pure and HD failed on luma, tint or
tail. A control is rendered the same way on both sides: the original from the same state without the press, ours with
the pickup given and no press (`--input-script` holding `none`), so a held-pickup icon in ours cancels.

## Method

| | Pulse (PSP, USA) | Pure (PSP, USA) | HD (PS3, EU) |
| --- | --- | --- | --- |
| Original | PPSSPP v1.20.4, software renderer, 480x272 | same | RPCS3, Write Color Buffers on, 1278x718 |
| State | `race-platinum-white-single-grid` (Platinum Rush, Assegai, 8th of 8, weapons on, at rest) | a Single Race made for this lane: Vineta K, Vector, Feisar, 8th of 8, at rest | `hd-racebox-talons-venom-weapons-grid`, START RACE, 5 s of lap clock, craft held at rest |
| Press | the held pickup written at `craft+0x1bc` of the weapon record while stopped, circle held 3 frames (`scripts/psp-absorb-frames.py`) | the held id written at `weapon record+0x1d8`, circle held until it reads 0 (the absorb), see below | pickup state 1 written at `*(ship+0x5edc)+0x204` (mirrored `+0x208`), circle held until `craft+0x7a5c` rises to 1.0 |
| Frame clock | `craft+0x830` minus the absorb stamp `craft+0x878`, one frame per hit of `Weapons_DispatchFire`; the screenshot lags the stop by about 2 frames | frames counted from the first player frame whose held id is back at 0; lag about 2 frames, unmeasured on Pure | single game frames stepped off the lap clock, `age = lap clock - stamp`, the timer read live (`1.0 - timer`) |
| Ours | `--pose-from` the original's pose row, `--camera-view close`, `--camera-fov 60`, `--give mine`, circle at tick 520 (race clock 4.2 s), ticks 520 + frame - 2 | same flags, Pure, circle at tick 780 (HUD 8.4 s, the original's clock at frame 0), ticks 780 + frame - 2 | `scripts/hd-weapon-pair.py --button circle` on the original's pose and RSX camera, fired at the stamp clock; a second run with `--button none` is ours' control |
| Repeats | 2 absorbs, 1 control, 1 boot | 2 absorbs, 1 control, 1 boot | 2 absorbs, 1 control, 3 restores of one state |

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

1. **Ours draws no hull flash on Pure.** The original's hull goes pale pink-white at 0.5 s over the whole hull and
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

Ranked:

1. **The shell's colour.** The original's hull stays dark blue and the gold livery shows through at 0.5 s and 1.0 s;
   ours washes the whole hull to white-cyan, its blue-minus-red delta half the original's (+9.4 against +18.4) at the
   peak. The thread's "ours reads bluer than the original's white-lavender" is the other way round on this pair.
   Exposure is not matched between the two sides (ours is the 1920x1080 composite, the original RPCS3's), so part of
   this may be tone mapping, not the shell; the shell's own colour, `ShieldColour`, is the first thing to check.
2. **The tail.** Ours is still at +17.8 at 1.0 s, the original's has fallen to +6.5: about 0.25 s longer. Unexplained
   on the original's side: its shell fade still reads 0.998 at that frame, so the picture dims before the fade does (the
   presented frame's lag against the stop was not measured on this state).
3. **The burst.** The original's wingtip locators throw large bright balls with electric streaks that also light the floor
   (0.3 s to 0.75 s); ours draws a smaller white glow at the same places.
4. **The rise.** Ours is brighter early (+13.4 against +6 at 0.2 s) though the tint is right: the shell fades in
   faster than the original's picture does, or the 3-tick lag assumed for ours is one frame short.
5. **HUD.** Both blink the shield hex between its blue `100` state and a pale state (blue minus red, hex box:
   original 41 to 23, ours 37 to 18); with eight ages sampled the phase cannot be told apart. The original's frames
   carry the `DAMAGE` and `Absorb` labels beside the hex, which ours does not (ours prints the held pickup's name
   there, a `--give` artefact). Whether those labels are absorb-only was not isolated: the control shows a faint `DAMAGE`
   too.

The cockpit shell (`vr_absorbinternal_cockpit`) is not exercised: the camera is the chase view.

## 2048 and Omega

Not paired. Exactly where it stopped:

- **Omega (shadPS4 0.20).** `scripts/shadps4-omega-pair.py` reaches the grid and stops before the countdown. A held
  pickup needs: the Racebox Weapons row turned on (HD's Racebox default is off, `emu-hd-race-walk.sh` presses the row;
  the Omega walk does not), a race driven to a pad with no steering feedback, and an absorb press timed on a frame clock
  shadPS4 does not expose (no memory readout). No Omega held-slot or absorb address is recovered in
  `docs/ghidra/functions/ps4-omega-eu/` (the shell colour is, in `shield.md`). Unblocks it: the Omega Racebox weapons walk,
  or the native port's `/proc/<pid>/mem` (`docs/tools/compare-page.md`, "Omega") with the HD held-slot offsets
  checked against the eboot.
- **2048 (Vita3K).** `scripts/vita-2048-pair.py --walk` reaches the grid before GO; Vita3K has no state channel, and its
  guest `/proc/<pid>/mem` can be written (the hull-shake patch does) but no 2048 held-slot or absorb address is read
  (`docs/ghidra/functions/vita-2048-eu-v104/` has `weapon-ai-fire.md`, `autopilot.md` and the loading screen only).
  Unblocks it: read `Ship_PlayAbsorbFeedback` and the pickup slot in the 2048 eboot (HD's are `0x000d9398` and
  `ship+0x5edc`), then the HD recipe above with a write through `/proc`.

Cross-title status (CLAUDE.md, "A 2048 finding is checked against Omega"): the burst is **ported** (both ship the six
`absorb` locators and HD's three mirrored pairs, `hull-sparks.md`); the shell colour, tail and tint measured on HD
are **checked, applies, not wired** for both: both take HD's `absorb_shell` and Omega's `shield.md` records the same
`ShieldColour`, but no live frame of either exists, so the HD differences above are inherited unmeasured, not shown.

## Reference set

The frames behind every number (original, original control, ours, ours' control at 0.1, 0.5 and 1.0 s, per title, with
side sheets) are derived game data and live in `data/reference/absorb-pairs/<title>/` once the lead adopts them; they are
not in this tree.

## Reproducing

Pulse: restore the state, `scripts/psp-absorb-frames.py --port P --frames 66 --edram --display $DISPLAY` (and
`--control`), then `oag-game ... --pose-from <pair trace> --give mine --input-script <(tick none, 1 circle)` at ticks
520 + age - 2. Pure: the same with the entity and held-id reads above (a scratch driver, the Pure addresses are on this
page). HD: `rpcs3-hd-missile-film.py`'s frame stepping with the grant and the circle press of this page's method table,
then `scripts/hd-weapon-pair.py --weapon missile --team feisar --button circle` and `--button none`.
