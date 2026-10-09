# HD's craft inertia and steering clamp (2026-10-09, hd-airbrake)

Why one airbrake turned this engine 10-14% harder than Wipeout HD, and why full-lock steering
had looked right anyway. Two laws differ from Pulse's: HD builds the craft's box inertia with
mass `1.0` where Pulse passes `0.9`, and HD's steering ramp stops at its target where Pulse's
overshoots and cycles. The old steering match was those two errors cancelling.

Read with the capstone listing of `0x000ed000`-`0x000f7000` and Ghidra (TOC `r2 = 0x008ad4d8`,
so `-0x4388(r2)` is `0x008a9150`, the documented `-5`). Checked against the per-frame craft dumps
of `scripts/rpcs3-trace.py` (`data/scratch/hd-handling/b5`, `b6`: Racebox Time Trial, Talon's
Junction, Venom, Feisar concept1). The measured comparison is on
[hd-handling-ground-truth.md](../../../physics/hd-handling-ground-truth.md); the hover and the
other craft terms are on [hover-four-point.md](hover-four-point.md).

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x000f5aa0` | `Body_SetBoxInertia` | 90 | static read below; its output read live at `body+0x10`/`+0x90` as `(17.333, 24, 17.333)` and the reciprocals, `L / omega = 23.98` on the yaw axis |

Not renamed, outside this lane's function range: `0x000f6568` (hypothesis `Body_SetMass`: stores
`f1` to `body+0x4a4`/`+0x4ac` and `1 / f1` to `+0x4a8`/`+0x4b0`, `1.0` at `0x008a92e0`) and
`0x000f57f0` (stores only `+0x4ac` and its reciprocal, the per-frame mass write).

## `Ship_Construct` builds the tensor with mass `1.0`

At `0x000deb58` `Ship_Construct` (`0x000ddd58`) loads `f26` from `-0x49c4(r2)` = `0x008a8b14`,
which holds `0x3f800000` (`1.0`). Then:

    0x000deba4  fmr  f1, f26               ; 1.0
    0x000debac  bl   0x000f6568            ; Body_SetMass(body, 1.0)
    0x000debb4  lfs  f2, -0x44f0(r2)       ; 0x008a8fe8 = 12.0
    0x000debb8  fmr  f4, f2                ; 12.0
    0x000debc0  fmr  f1, f26               ; 1.0
    0x000debc4  lfs  f3, -0x4680(r2)       ; 0x008a8e58 = 8.0
    0x000debc8  bl   0x000f5aa0            ; Body_SetBoxInertia(body, 1.0, 12, 8, 12)

The mass is a **code literal**, not the class's `<Physical mass>`. HD's Feisar classes all author
`mass="1"`, so the live tensor alone could not tell the two apart; the read does. Pulse's
constructor passes `0.9` at the same point (`FUN_08840c74`, `0x08841414`; see
`oag_physics::forces::INERTIA_MASS`).

`Body_SetBoxInertia` (`0x000f5aa0`) squares the three extents, sums them in pairs, multiplies
each sum by the mass and divides `12.0` (`0x008a92fc`) by it, storing the inverse diagonal to the
`body+0x90` rows; then it multiplies by `1/12` (`-0x41e0(r2)`) for the tensor itself at the
`body+0x10` rows. So `I = m (b^2 + c^2) / 12` per axis, the solid box, as on Pulse.

**Live, every frame of every run**: `body+0x10 = 17.333`, `+0x24 = 24.000`, `+0x38 = 17.333`,
`+0x90 = 0.057692`, `+0xa4 = 0.041667`, `+0xb8 = 0.057692`. With mass `0.9` the yaw entry would be
`21.6`. The same three constants sat on all eight bodies of a Vineta K Zone race
([physics.md](physics.md#live-eight-bodies-in-an-eight-craft-race)), an independent session.

`body+0x50` carries the tensor rotated into world axes (off-diagonals `0.0216`, `0.116` at a few
degrees of pitch and roll), and `body+0x1c0` is the world angular momentum: `I_world *
body+0x1a0` reproduces it to three digits. That answers the "inertia frame" question the
previous pass left open: HD rotates the body tensor (`R I R^T`), where Pulse applies its diagonal
in world axes. Negligible while the craft is level; not ported.

**Confidence 90**: the literal, its two call sites and the writer read statically; the tensor read
live on two sessions; and the one-airbrake yaw rate, the measurement that found it, runs
`0.900` of this engine's on every frame (`21.6 / 24 = 0.900`).

## `Craft_UpdateSteering` clamps its ramp at the target

The ramp at `craft+0x314` moves toward the control's `+0x00` (`f12`) at the class block's `+0x38`
up / `+0x40` down, as on Pulse, but each step stops at the target. For a positive target:

    0x000edc50  f0 = cur + dt * gain                      ; fmadds
    0x000edc5c  if target < f0: f0 = target               ; fcmpu, bge, stfs f12
    ...
    0x000edb9c  f13 = cur - dt * falloff                  ; fnmsubs
    0x000edba4  f0  = (target - f13 >= 0) ? target : f13  ; fsel: max(cur - dt * falloff, target)

and mirrored for a negative target (`0x000edcb0`-`0x000edcd4`, `0x000edd1c`-`0x000edd30`). A zero
target decays to exactly zero, as on Pulse. A craft byte at `+0x37c` additionally floors the ramp
(`-0x4380(r2)`, `-0x437c(r2)`), and controller states 5 and 6 (`craft+0x2f8`) zero it, as on Pulse.

**Pulse's ramp (`0x08848788`) has no clamp**: `if (target <= cur) cur -= dt * falloff; else cur +=
dt * gain`. On HD's `<Turning gain="500" falloff="1000">` at 60 Hz, twelve `8.333334` steps make
`100.00002`, the next frame takes the falloff branch to `83.3`, and a held stick cycles `83.3 /
91.7 / 100`, averaging `91.7`. This engine's trace showed exactly that cycle; HD's `craft+0x314`
parks at `100` in every steering capture.

**Confidence 90**: the clamp read statically in both directions; the parked ramp read live; and
flying the clamp brings the full-lock yaw rate within 1% of HD on every frame.

## What it does to the comparison

Per frame, yaw rate HD over this engine, Feisar concept1 Venom, four runs (two boots):

| scenario | before | inertia `1.0` | inertia and clamp |
| --- | --- | --- | --- |
| one airbrake, 0.05..0.75 s | `0.89`..`0.92` | `0.99`..`1.00` | same |
| full-lock steering, 0.05..0.75 s | `0.91` rising to `0.99` | `1.01` rising to `1.10` | `1.007`..`1.016` |

Heading at the usual check points (HD four runs):

- one airbrake right, 0.5 s: `-12.82` before, `-11.54` after, HD `-11.22`..`-11.36`;
  at 1.0 s `-36.85`, `-33.29`, HD `-32.73`..`-33.07`.
- full-lock steering, 0.8 s: `-48.00` before, `-46.28` after, HD `-46.20`..`-46.25`.

Pitch, thrust and sideshift move by under `0.4` in any reported figure.

## Omega and 2048

- **Omega**: not located in this pass. No `12.0f` immediate sits near the craft code (the five
  `0x41400000` immediates are elsewhere, so the box extents load from rodata), and the
  `vminss` census in `0x01310000`-`0x01330000` holds no steering ramp (`FUN_01320c10` is a
  geometry clamp). Next: Omega's craft update from `Craft_UpdateSurfaceProbes` (`0x0131b510`,
  reached by a virtual call, so no static caller), then its steering by the class block's
  `Turning` offsets. No live capture path (no PS4 emulator), so Omega keeps Pulse's laws
  (`craft_laws: None`). Recorded as open, not as "applies".
- **2048**: not checked.

## Wired

`oag_title::craft_laws::CraftLaws` on HD's `RaceDefaults` (`oag_hd::race::CRAFT_LAWS`): the
tensor through `oag_physics::forces::ship_inertia_at`, the clamp as
`oag_physics::controls::ramp_steering_clamped` behind `ShipState::steer_ramp_clamped`. Titles
with `None` keep Pulse's tensor to the bit and its unclamped ramp. Pinned by
`oag_physics::forces::tests::a_titles_inertia_mass_rebuilds_the_same_box`,
`oag_physics::controls::tests::a_held_stick_parks_the_clamped_ramp_and_cycles_the_unclamped_one`
and `oag_raceplay::launch_hover::tests::a_titles_craft_laws_build_its_inertia_and_none_keeps_pulses`.

Not touched: `oag_ai::driver::pace::hull_yaw_ceiling` still divides by Pulse's `I_yy` (21.6), so
on HD the AI's turn-rate ceiling reads about 11% high (`1.667` against the `1.5` rad/s HD's
Feisar reaches). The AI owns that.
