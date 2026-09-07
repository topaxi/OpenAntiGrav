# Novice scheme's airbrakes do not follow the recovered action table

Found while fixing the CONTROLS page's `circle`/`square` rows (2026-09-07),
not something that page's fix touches or needs.

`docs/ghidra/functions/psp-pulse-usa/input-bindings.md` (confidence 90,
static analysis of `Options_LoadDefaultControlMapping`) recovers the
original's eight-action table and its two schemes precisely: in **novice**,
action 4 (`OPT_CTRL_AIRBRAKES`, "both airbrakes") is bound to `R` alone and
action 7 (`OPT_CTRL_SS`, sideshift) to `L` alone - `L` is a **dedicated**
sideshift button in novice, not an airbrake at all, and holding `R` alone is
meant to apply both airbrakes at once. `oag_gameplay::controls::ControlScheme`'s
own doc comment already says this in English ("`R` is both airbrakes at once
and `L` is a dedicated sideshift button").

`ship_controls` does not implement it. `airbrake_left`/`airbrake_right` are
filled unconditionally from `InputSnapshot::airbrake_left`/`airbrake_right` -
themselves just "is `L` held" / "is `R` held" off the raw button state, with
no scheme parameter anywhere in `oag_input`'s snapshot builders (`Keyboard::
snapshot`, `crates/input/src/lib.rs`; the pad path, `crates/input/src/pad.rs`).
So today, in novice: holding `L` sets `airbrake_left = 1` (wrong - should be
`0`, `L` should only arm the sideshift) and holding `R` sets `airbrake_right =
1` only (wrong - should also set `airbrake_left = 1`, since `R` alone is
meant to be both). `shift_modifier` (`novice && L held`) is correct on its
own; it is `airbrake_left`/`airbrake_right` that miss the scheme entirely.

Veteran is unaffected: its table (`LAB`->`L`, `RAB`->`R`) matches what
`ship_controls` already does regardless of scheme, which is presumably why
this was never noticed - the default profile is veteran
(`ControlScheme::Veteran` is `#[default]`), so a novice player is the only
one who would see it, and CONTROLS-page review (this thread's own trigger)
looks at labels, not scheme-conditional behaviour.

## Open

- Confirm this is really unimplemented and not handled somewhere else in the
  physics/gameplay path this search did not reach - checked
  `oag_gameplay::controls::ship_controls`,
  `oag_input::{lib.rs,pad.rs}::snapshot` and `oag_physics::airbrake::
  advance_sideshift` directly; did not check every caller of `ShipControls`.
- No runtime capture of novice on real hardware exists yet -
  `handover/gameplay/sideshift-has-no-runtime-leg.md`'s own "Open" section
  already says the novice flick has never been captured, blocked on Options-
  menu scheme-selection automation that `psp-drive.py`/`psp-trace.py` don't
  have. The same missing automation blocks confirming this airbrake claim
  against real hardware too, not just the flick gesture.

## Next Steps

- If confirmed, the fix is scheme-aware in `ship_controls` (it already takes
  `scheme`): when `novice`, both `airbrake_left` and `airbrake_right` should
  read `snapshot.airbrake_right` (`R` alone drives both, per action 4 =
  `R`), and `snapshot.airbrake_left` (`L`) should be ignored entirely for
  airbrake purposes - `L` in novice only arms the sideshift
  (`shift_modifier`), it contributes no airbrake per action 7 = `L`. Update
  `ship_controls`'s own doc comment, which currently only calls out
  `shift_modifier`/`shift_tap_left`/`shift_tap_right` as scheme-filtered.
- `crates/gameplay/src/controls.rs`'s own test module is the place for a
  case pinning "novice + `R` held only -> both airbrakes; novice + `L` held
  only -> no airbrake at all, `shift_modifier` true".
- `sideshift-has-no-runtime-leg.md`'s opening line, "Both control schemes
  are implemented," is true of the *gesture* halves only (the double-tap and
  the flick's arm/fire logic) - this thread is the airbrake half of novice
  that line does not cover, and is worth a cross-reference from there once
  this lands.
