//! The three message lines the original raises at a lap crossing:
//! `IG_HUD_PLAP`, `IG_HUD_NEWLAP_REC` and `IG_HUD_FIN_LAP`.
//!
//! The law is `Craft_UpdateLapProgress` (`0x08842a18`, psp-pulse-usa), which
//! ORs one bit each into the flag word `Hud_UpdateMessages` drains; see
//! `docs/ghidra/functions/psp-pulse-usa/hud-message-triggers.md`. All three are
//! the *player's* craft only and none fires on the finishing crossing.
//!
//! Render-side: nothing here is in the hashed world.

/// The three ids, in the order `Hud_UpdateMessages` drains the bits (`1`, `2`, `4`).
pub const IDS: [&str; 3] = [PERFECT_LAP, NEW_LAP_RECORD, FINAL_LAP];

const PERFECT_LAP: &str = "IG_HUD_PLAP";
const NEW_LAP_RECORD: &str = "IG_HUD_NEWLAP_REC";
const FINAL_LAP: &str = "IG_HUD_FIN_LAP";

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
#[derive(Debug, Clone, Copy, Default)]
pub struct LapWatch {
    /// The stored best lap for this circuit, mode and class as it stood at
    /// load, in ticks. `None` is "no record yet", which never announces.
    stored_best_ticks: Option<u32>,
    /// A wall was hit since the last completed lap (`craft+0x860 & 0x20`).
    /// Cleared on a completed lap and **not** on the first crossing, so lap 1's
    /// verdict includes the drive from the grid to the line.
    dirty: bool,
}

impl LapWatch {
    /// A watch comparing laps against `stored_best_ticks`.
    #[must_use]
    pub fn new(stored_best_ticks: Option<u32>) -> Self {
        Self {
            stored_best_ticks,
            dirty: false,
        }
    }

    /// One tick: `hit` is whether the craft scraped a wall that nothing
    /// absorbed. Returns the ids to raise, in drain order.
    pub fn observe(&mut self, hit: bool, edge: LapEdge) -> Vec<&'static str> {
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
            out.push(PERFECT_LAP);
        }
        // `FUN_08808bec`: strictly under a stored, nonzero record, in
        // centiseconds, and the best lap of this race.
        if let (Some(ticks), Some(stored)) = (edge.lap_ticks, self.stored_best_ticks)
            && centis(ticks) != 0
            && centis(ticks) < centis(stored)
            && edge.best_lap_ticks == Some(ticks)
        {
            out.push(NEW_LAP_RECORD);
        }
        if edge.laps_target == Some(edge.lap) {
            out.push(FINAL_LAP);
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
    pub(super) fn raise_lap_messages(&mut self, outcome: &oag_race::Outcome, hit: bool) {
        let state = *self.sim.world.primary_race();
        let Some(watch) = self.view.lap_watch.as_mut() else {
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
        for id in watch.observe(hit, edge) {
            self.view.messages.push(id, true);
        }
    }
}

#[cfg(test)]
mod tests;
