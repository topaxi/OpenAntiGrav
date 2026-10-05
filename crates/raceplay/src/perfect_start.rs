//! The perfect start's own effect: the engine flare's boost and the `TURBO`
//! cue, on the tick a human craft's launch grade becomes
//! [`Grade::Perfect`].
//!
//! **Read from `BOOT.BIN`, confidence 85, not watched.**
//! `Race_UpdateLaunchGrade` (`0x0882773c`) writes grade 2 for a craft whose
//! player record is the local human (`player+0x368 == 0`) on its first-thrust
//! edge inside the perfect window, and only then calls
//! `ExhaustFlare_OnPerfectStart` (`0x08904fd4`) on that craft's flare. That
//! function is `ExhaustFlare_OnSpeedupPad` with two differences: it arms the
//! boost timer (`flare+0xb8`) from `flare+0xb4`, which the flare's
//! constructor (`0x089044d8`) sets to `0.8` and nothing else writes - the
//! same `0.8` the pad stores as a literal - and it plays `"TURBO"` out of
//! `weapons.bnk` where the pad plays `"SPEEDUPPAD"` out of `hud.bnk`. So a
//! perfect start looks like a pad crossing (the wider flare and the
//! `<Team>boost.vex` plume) and sounds like a Turbo. No `.POB` is involved.
//! See `docs/ghidra/functions/psp-pulse-usa/perfect-start.md`.

use super::*;

use oag_physics::launch::Grade;

impl Race {
    /// Every slot's launch grade, taken before this tick steps the field.
    pub(super) fn launch_grades(&self) -> [Grade; oag_gameplay::MAX_SHIPS] {
        self.sim
            .world
            .ships
            .each_ref()
            .map(|ship| ship.physics.launch.grade)
    }

    /// Fires the perfect-start effect on each human craft whose grade became
    /// [`Grade::Perfect`] this tick. The grade is written once, on the
    /// first-thrust edge, so a change is that edge.
    pub(super) fn fire_perfect_starts(&mut self, before: &[Grade; oag_gameplay::MAX_SHIPS]) {
        let count = self.sim.world.ship_count as usize;
        for (slot, &was) in before.iter().enumerate().take(count) {
            if !self.sim.world.controllers[slot].is_human() {
                continue;
            }
            let now = self.sim.world.ships[slot].physics.launch.grade;
            if now == Grade::Perfect && was != Grade::Perfect {
                self.view.exhaust[slot].boost(exhaust::BOOST_SECONDS);
                self.sim.cues.push(oag_sound::sfx::CueEvent::new(
                    oag_sound::sfx::Cue::Turbo,
                    slot,
                ));
            }
        }
    }
}
