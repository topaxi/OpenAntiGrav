//! The three message lines the original raises at a lap crossing: a perfect
//! lap, a new lap record and the final lap.
//!
//! The law is the lap code's ORing one bit each into the flag word the HUD's
//! message drain reads: Pulse's `Craft_UpdateLapProgress` (`0x08842a18`) into
//! `Hud_UpdateMessages`, Pure's `0x08927a00`-ish into `0x0881f2ec`, HD's
//! `0x000e4a30` into `0x00091d20`; see each binary's `hud-message-triggers.md`
//! under `docs/ghidra/functions/`. All three are the *player's* craft only and
//! none fires on the finishing crossing. The ids are the title's own
//! ([`oag_title::lap_hud::LapHud`]): Pulse and HD `IG_HUD_*`, Pure
//! `HUD_<English>`.
//!
//! Render-side: nothing here is in the hashed world.

use oag_title::lap_hud::LapHud;

/// What a tick's lap update said, as far as these lines care.
#[derive(Debug, Clone, Copy, Default)]
pub struct LapEdge {
    /// A lap was completed this tick (the first crossing is not one).
    pub completed: bool,
    /// That crossing also ended the race.
    pub finished: bool,
    /// The lap now being driven, 1-based.
    pub lap: u32,
    /// The lap count the race ends after.
    pub laps_target: Option<u32>,
    /// The completed lap's time, in ticks.
    pub lap_ticks: Option<u32>,
    /// The player's best lap this race, in ticks, this one included.
    pub best_lap_ticks: Option<u32>,
}

/// The player's per-lap state for the three lines.
#[derive(Debug, Clone, Copy)]
pub struct LapWatch {
    /// The title's ids and its one mode rule.
    hud: LapHud,
    /// The stored best lap for this circuit, mode and class as it stood at
    /// load, in ticks. `None` is "no record yet", which never announces.
    stored_best_ticks: Option<u32>,
    /// A wall was hit since the last completed lap (`craft+0x860 & 0x20`).
    /// Cleared on a completed lap and **not** on the first crossing, so lap 1's
    /// verdict includes the drive from the grid to the line.
    dirty: bool,
}

impl LapWatch {
    /// A watch comparing laps against `stored_best_ticks`, raising the title's
    /// own ids.
    #[must_use]
    pub fn new(hud: LapHud, stored_best_ticks: Option<u32>) -> Self {
        Self {
            hud,
            stored_best_ticks,
            dirty: false,
        }
    }

    /// One tick: `hit` is whether the craft scraped a wall that nothing
    /// absorbed and `time_trial` whether the mode is Time Trial. Returns the ids
    /// to raise, in drain order.
    pub fn observe(&mut self, hit: bool, time_trial: bool, edge: LapEdge) -> Vec<&'static str> {
        self.dirty |= hit;
        let mut out = Vec::new();
        if !edge.completed {
            return out;
        }
        let perfect = !self.dirty;
        self.dirty = false;
        if edge.finished {
            return out;
        }
        if perfect {
            out.push(self.hud.perfect_lap);
        }
        // `FUN_08808bec`: strictly under a stored, nonzero record, in
        // centiseconds, and the best lap of this race.
        if let (Some(ticks), Some(stored)) = (edge.lap_ticks, self.stored_best_ticks)
            && (self.hud.record_line_in_time_trial || !time_trial)
            && centis(ticks) != 0
            && centis(ticks) < centis(stored)
            && edge.best_lap_ticks == Some(ticks)
        {
            out.push(self.hud.new_lap_record);
        }
        if edge.laps_target == Some(edge.lap) {
            out.push(self.hud.final_lap);
        }
        out
    }
}

fn centis(ticks: u32) -> u64 {
    u64::from(ticks) * 100 / 60
}

impl super::Race {
    /// Raises the lap-crossing lines for this tick's lap update. `hit` is a wall
    /// scrape nothing absorbed (the original's `0x20` bit is set on the
    /// collision-damage path and not on the shield's, `shield-pickup.md`).
    pub(super) fn raise_lap_messages(
        &mut self,
        slot: usize,
        outcome: &oag_race::Outcome,
        hit: bool,
    ) {
        let state = self.sim.world.race[slot];
        let Some(watch) = self.view.huds[slot].lap_watch.as_mut() else {
            return;
        };
        let edge = LapEdge {
            completed: outcome.lap_completed,
            finished: outcome.finished,
            lap: state.lap,
            laps_target: state.laps_target,
            lap_ticks: outcome.lap_ticks,
            best_lap_ticks: state.best_lap_ticks,
        };
        let time_trial = self.sim.world.mode() == oag_race::Mode::TimeTrial;
        for id in watch.observe(hit, time_trial, edge) {
            self.view.huds[slot].messages.push(id, true);
        }
    }
}

#[cfg(test)]
mod tests;
