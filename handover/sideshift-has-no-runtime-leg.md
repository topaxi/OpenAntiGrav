# Sideshift has no runtime leg

Both control schemes are implemented. Note the scheme resolves once at startup and applies to the next race, not the running one.

**The veteran double-tap now does have a runtime leg.** Captured 2026-08-27 off a real, booted `pulse-psp-usa.chd` (recipe: `docs/reverse-engineering/ppsspp-debugger.md`), with the trigger's own five timer floats added as new trace columns (`scripts/psp_trace_fields.py`'s `ENTITY_FIELDS`) so the capture reads the constants directly rather than inferring them from where the ship ended up. Against a committed control (`sideshift-double-tap-control.inputs`, the same script with the two `l` tokens dropped), the tap window and shift force read exactly `0.25 s`/`0.2 s` at the ticks the static reading in `docs/ghidra/functions/psp-pulse-usa/input-bindings.md` predicted - see that page's new runtime section for the tick-by-tick numbers, now confidence 95 for the left-hand offsets.

**One correction the capture forced on the static reading itself: the gap between sideshifts is ~1.2 s, not 1.0 s.** `ss_lockout` is re-armed to `1.0` on every tick the shift force is still running (12 ticks, ~0.2 s), so it does not start counting down until the force expires - the two timers add rather than overlap. `input-bindings.md`'s old "one second between sideshifts" line undercounted by the shift duration. `crates/physics/src/airbrake.rs`'s `advance_sideshift` already ported the additive version correctly (re-arms `shift_lockout` the same way, and `airbrake/tests.rs` already asserted the `(SIDESHIFT_DURATION + SIDESHIFT_LOCKOUT)` total) - this was a doc-precision fix, not a port bug.

**What that capture does not settle: the displacement claim.** The scripted run-up (`180 cross`, no steering) grazes a wall on Talon's Junction around tick 60-90 - the same collision the standing-start capture already documents - and the capture and its control are already `1.17` units apart in position by the time the first tap fires, against `0.035` units apart at tick 0. A wall contact amplifies the normal pose-to-pose spread between captures into something much larger than the sideshift's own signal, so the original "11.7 units of leftward displacement" figure (which came from `oag-trace drive` through *our own* simulation, not a capture of the original - that sim run does not hit the wall on this script) remains unconfirmed against real hardware.

**The novice flick still has none.** Neither `scripts/psp-drive.py` nor `scripts/psp-trace.py` has a way to select the control scheme (checked directly: no `scheme` reference in either file) - the veteran capture above worked without one only because veteran is the profile's default scheme. Reaching novice needs walking the Options menu to flip `Control Type`, which is unwritten automation, not a data gap.

## Open

- The lateral-displacement claim for the veteran gesture is not confirmed against real hardware - a clean measurement needs a run-up that does not graze a wall, which `sideshift-double-tap.inputs`'s `180 cross` does not provide on Talon's Junction
- No capture of the novice flick gesture off the original exists at all; blocked on Options-menu scheme-selection automation, which does not exist yet in `psp-drive.py`/`psp-trace.py`

## Next Steps

- Author and capture a sideshift run-up that stays off the wall (a shorter run-up, gentle steering, or a different, straighter track) to get a clean displacement comparison for the veteran gesture
- Add scheme selection to `psp-drive.py`'s menu walk (Options -> Control Type -> novice), then capture `sideshift-flick.inputs` the same way the veteran gesture was captured here
