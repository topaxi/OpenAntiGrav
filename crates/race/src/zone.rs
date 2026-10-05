//! Zone mode's numbers, as read out of the executable.
//!
//! Everything here is recovered rather than chosen. The evidence is
//! `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`; the short version is that
//! `Zone_Update` (`0x0882f5cc`) accumulates the frame delta, steps the zone
//! number every ten seconds, and **assigns** that number into the craft at
//! `+0x28c`, where the engine's auto-speed law reads it.
//!
//! # What is not here
//!
//! The two floats the speed law scales by. They live in `.bss` at `0x08b36be0`
//! and `0x08b36be4` and are parsed at runtime from a
//! `<Zone start=… increment=… recharge=…/>` element in the disc's global-settings
//! XML, so there is no value to write down - and under
//! [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md)
//! there would be no writing it down even if there were. They arrive as
//! arguments to [`thrust`].
//!
//! # Which side of the engine/title seam these are on
//!
//! **A title package's, and this is not hypothetical.** Everything in this file
//! is a literal out of *Pulse's* executable, and Pure ships zone mode too: its
//! `Data\Ships\Zone_01\handlingstats.xml` is a real file, with `team="ZoneMode"`
//! and - uniquely on either disc - no `<Class>` ladder at all
//! (`docs/formats/pure-status.md`). So "ten seconds a zone, 500 for a clean one,
//! 100 for a new pad" is a claim about what Pulse scores, not about what a zone
//! *is*, and nothing here has been checked against Pure's executable.
//!
//! Stage 6 of the engine/title split ([ADR-0022]) names that and **moves
//! nothing**. Relocating these into `oag-pulse` today would put four numbers in
//! a title package with no second corpus to check them against, which is the n=1
//! design [ADR-0009](../../../docs/architecture/adr/0009-multi-game-fanout.md)
//! item 3 warned about and ADR-0022 does not license outside the format layer.
//! Item 2 of the same ADR gates second-title simulation work behind M4's exit
//! anyway. What this paragraph is for is that when that gate opens, the question
//! "is this Pulse's or is it the engine's?" is already answered for this file
//! rather than re-derived from the Ghidra pages.
//!
//! The mechanism around them - accumulating the delta, stepping the zone,
//! assigning into the craft, `thrust`'s shape - is the engine's and stays, the
//! same split `oag_physics::params` draws for the force law.
//!
//! [ADR-0022]: ../../../docs/architecture/adr/0022-title-packages.md

/// Seconds of accumulated frame time between zone steps.
///
/// The immediate `lui a1,0x4120` at `0x0882f5e4`, which is `10.0f`. Confidence
/// **84**: read from the instruction stream rather than from the decompiler.
pub const STEP_SECONDS: f32 = 10.0;

/// Score added every tick the run is under way.
///
/// `*(int *)(obj + 0x1a1c) += 1` on the unconditional path of `Zone_Update`.
/// Confidence **84**.
pub const SCORE_PER_TICK: i32 = 1;

/// Score added for a zone completed without touching anything.
///
/// `*(int *)(obj + 0x1a1c) += 500` under the clean-zone branch, which also
/// increments the perfect-zone tally and grants the shield recharge. Confidence
/// **82**.
pub const CLEAN_ZONE_BONUS: i32 = 500;

/// Score added for entering a speed pad that is not the one already under the
/// ship.
///
/// `*(int *)(obj + 0x1a1c) += 100` under `if (DAT_08b3435c) { DAT_08b3435c = 0; }`
/// in `Zone_Update`. The flag has exactly one writer, `Ship_ApplySpeedupPad`
/// (`0x08848f9c`), which raises it on entering a **new** pad and only when
/// `DAT_08ab07e3 == 0 && DAT_08b31048 == 6` - the same pair
/// `docs/ghidra/functions/psp-pulse-usa/zone-mode.md` identifies as the Zone-mode
/// selector. Confidence **85**.
///
/// **A per-tick flag, not a counter.** `Zone_Update` clears it as it consumes it,
/// so a ship straddling two pads on one tick scores this once. The trigger in
/// `oag_raceplay` reproduces that by scoring on a change of pad rather than per
/// pad tested.
pub const SPEEDUP_PAD_SCORE: i32 = 100;

/// The engine's target speed in Zone mode, for zone number `zone`.
///
/// `base + step * (float)(uint32)n`, from the four-corner branch of
/// `Ship_UpdateEngine` at `0x0884c834`. Two details the older note on that
/// function was missing, both read from the instruction stream at confidence
/// **84**:
///
/// - The conversion is **unsigned** - `bgez` plus `lui 0x4f80` (that is `2^32`)
///   is the standard `(float)(u32)` idiom. Taking it as signed would flip the
///   law's sign after 2^31 zones, which is unreachable, but the PS2 page already
///   records `(uint)` and the two should agree.
/// - There is **no cap**. The ordinary branch clamps its result against
///   `0.5 * speed + accelcap`; this branch does not, and simply overwrites the
///   slot.
///
/// The caller is responsible for the gate: the law only applies when the craft
/// is grounded and the second flag bit is clear, and the target is `0.0` when it
/// is not. That gate lives with the force law, not here.
#[must_use]
pub fn thrust(base: f32, step: f32, zone: u16) -> f32 {
    base + step * f32::from(zone)
}

#[cfg(test)]
mod tests {
    use super::{STEP_SECONDS, thrust};

    #[test]
    fn zone_zero_is_the_base_speed() {
        assert_eq!(thrust(100.0, 5.0, 0), 100.0);
    }

    #[test]
    fn each_zone_adds_one_step() {
        assert_eq!(thrust(100.0, 5.0, 1), 105.0);
        assert_eq!(thrust(100.0, 5.0, 4), 120.0);
        assert_eq!(thrust(100.0, 5.0, 100), 600.0);
    }

    #[test]
    fn the_law_is_uncapped() {
        // No clamp anywhere in the branch, so a long run keeps accelerating.
        // This is the recovered behaviour, not an oversight to be tidied up.
        let far = thrust(100.0, 5.0, u16::MAX);
        assert!(far > 300_000.0, "{far} should be unbounded");
    }

    #[test]
    fn a_step_is_ten_seconds() {
        assert_eq!(STEP_SECONDS, 10.0);
        // 600 ticks at the fixed 60 Hz of ADR-0007.
        assert_eq!((STEP_SECONDS * 60.0) as u32, 600);
    }
}
