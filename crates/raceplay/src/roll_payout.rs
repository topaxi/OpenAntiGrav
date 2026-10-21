//! A barrel roll's landing payout: the engine flare's boost and the `TURBO`
//! cue, on the tick a craft's roll turbo timer is armed.
//!
//! **Read from `BOOT.BIN`, confidence 88 (static; see
//! `docs/ghidra/functions/psp-pulse-usa/roll-boost.md` for what was watched).**
//! `Ship_UpdateSideshiftInput_q` (`0x08846a54`) pays a completed roll out at
//! `0x08846b4c`: it stores `<Special roll_turbotime>` to `craft+0x898` and,
//! when the craft has a flare (`craft+0x78`), calls `ExhaustFlare_OnTurbo`
//! (`0x0890514c`) - the flare's `+0xb8` set to `0.8` and `"TURBO"` played out
//! of `weapons.bnk`, the call a fired Turbo makes. It runs for every craft, an
//! AI's included; only the human's cue is dry.
//!
//! The trigger here is the timer's rise across a tick: physics decays it
//! before arming it, so it only rises on the arming tick, and a second
//! landing inside a payout rises again, as the original calls the flare again.

use super::*;

impl Race {
    /// Every slot's roll turbo timer, taken before this tick steps the field.
    pub(super) fn roll_payout_timers(&self) -> [f32; oag_gameplay::MAX_SHIPS] {
        self.sim
            .world
            .ships
            .each_ref()
            .map(|ship| ship.physics.roll_payout_timer)
    }

    /// Lights the flare and raises `TURBO` for each craft whose roll payout was
    /// armed this tick.
    pub(super) fn fire_roll_payouts(&mut self, before: &[f32; oag_gameplay::MAX_SHIPS]) {
        let count = self.sim.world.ship_count as usize;
        for (slot, &was) in before.iter().enumerate().take(count) {
            if self.sim.world.ships[slot].physics.roll_payout_timer > was {
                self.boost_plume(slot);
                self.sim.cues.push(oag_gameplay::cue::CueEvent::new(
                    oag_gameplay::cue::Cue::Turbo,
                    slot,
                ));
            }
        }
    }
}
