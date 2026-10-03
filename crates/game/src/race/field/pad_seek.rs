//! Which weapon pad an opponent swings over, in an Eliminator.
//!
//! **Ours, chosen, not measured.** The original's opponents are not documented
//! to steer for a pad at all - see `oag_ai`'s `driver/pads.rs` for the read and
//! `docs/gameplay/race-modes.md`, "Steering for weapon pads", for the sweep that
//! decided whether this ships. It is driving, not cheating: the craft is handed
//! a place in the road, never thrust, speed or shield.
//!
//! Eliminator only, so every other mode's line stays byte-identical: outside it
//! [`PadSeeking::Off`] makes [`Race::pad_for`] return `None`, and a driver
//! handed `None` computes exactly the bits it did before this existed.

use super::*;

/// Which craft steer for a weapon pad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PadSeeking {
    /// Nobody: the authored line, as in every mode but an Eliminator.
    #[default]
    Off,
    /// A craft whose weapon slot is empty. A full slot cannot collect - a pad
    /// crossed with a weapon held grants nothing (`Race::test_weapon_pads`) -
    /// so a detour for it is all cost.
    EmptySlot,
    /// Every craft, held weapon or not. Kept for the sweep that compares the
    /// two, not for play.
    Every,
}

impl PadSeeking {
    /// What a race in `mode` starts with.
    #[must_use]
    pub fn for_mode(mode: oag_race::Mode) -> Self {
        if mode == oag_race::Mode::Eliminator {
            Self::EmptySlot
        } else {
            Self::Off
        }
    }
}

/// How many line samples apart two pads may sit and still count as one
/// crossing to choose between. Pads are authored in pairs straddling the line
/// (`docs/gameplay/ai.md`, "Pads: built, measured, removed"); about 10 units at
/// the line's 2.5-unit spacing. **Chosen.**
const PAIR_SAMPLES: u32 = 4;

/// Where each pad sits on `line`: the index of the sample nearest its centre,
/// and its centre's offset across the line there, positive to the right.
///
/// Searched over the whole line once, at the start, so a craft's per-tick
/// question is integer arithmetic over a handful of pads. Ties go to the
/// earlier sample.
#[must_use]
pub(in crate::race) fn line_positions(
    line: &oag_ai::Line,
    pads: &[oag_vex::pads::PadVolume],
) -> Vec<(u32, f32)> {
    let len = line.len();
    if len == 0 {
        return Vec::new();
    }
    pads.iter()
        .map(|pad| {
            let centre = Vec3::from_array(pad.centre());
            let index = line.nearest(centre, 0, len);
            let offset = line
                .corridor_at(index)
                .map_or(0.0, |frame| (centre - line.point(index)).dot(frame.lateral));
            (index as u32, offset)
        })
        .collect()
}

impl Race {
    /// The weapon pads this race armed, in the track file's node order. Empty
    /// in a mode that disarms them.
    #[must_use]
    pub fn weapon_pads(&self) -> &[oag_vex::pads::PadVolume] {
        &self.sim.weapon_pads
    }

    /// Who steers for a weapon pad in this race.
    #[must_use]
    pub fn pad_seeking(&self) -> PadSeeking {
        self.sim.pad_seeking
    }

    /// Overrides [`PadSeeking::for_mode`]: the sweep that measures this term
    /// runs both arms on one binary. A mode with no armed pads ignores it.
    pub fn set_pad_seeking(&mut self, seeking: PadSeeking) {
        self.sim.pad_seeking = seeking;
    }

    /// The weapon pad slot `slot` should swing over, if any.
    ///
    /// The next pad ahead on the line within [`oag_ai::PAD_LOOKAHEAD`], and of
    /// a pair - pads within [`PAIR_SAMPLES`] of the nearest - the one nearer
    /// across to where the craft already is, so a craft does not cross the road
    /// for the far one. A pad still cooling down is skipped: its colour shows
    /// it, so a player can see it too.
    pub(in crate::race) fn pad_for(&self, slot: usize) -> Option<oag_ai::Pad> {
        let ship = &self.sim.world.ships[slot];
        let wants = match self.sim.pad_seeking {
            PadSeeking::Off => false,
            PadSeeking::EmptySlot => ship.pickup.is_empty(),
            PadSeeking::Every => true,
        };
        let line = &self.sim.racing_line;
        let len = line.len() as u32;
        if !wants || len == 0 || self.sim.weapon_pad_line.is_empty() {
            return None;
        }
        let here = ship.driver.index % len;
        let across = line.corridor_at(here as usize).map_or(0.0, |frame| {
            (ship.physics.body.position - line.point(here as usize)).dot(frame.lateral)
        });
        // Samples ahead, for every armed pad in front.
        let ahead = |index: usize| -> Option<u32> {
            if self.sim.weapon_pad_refresh_left[index] > 0.0 {
                return None;
            }
            let steps = (self.sim.weapon_pad_line[index].0 + len - here) % len;
            (steps > 0).then_some(steps)
        };
        let nearest = (0..self.sim.weapon_pad_line.len())
            .filter_map(ahead)
            .min()?;
        let mut best: Option<(usize, u32)> = None;
        for index in 0..self.sim.weapon_pad_line.len() {
            let Some(steps) = ahead(index) else { continue };
            if steps > nearest + PAIR_SAMPLES {
                continue;
            }
            let better = best.is_none_or(|(incumbent, _)| {
                (self.sim.weapon_pad_line[index].1 - across).abs()
                    < (self.sim.weapon_pad_line[incumbent].1 - across).abs()
            });
            if better {
                best = Some((index, steps));
            }
        }
        let (index, steps) = best?;
        // Along the line, summed sample to sample, and given up past the
        // lookahead.
        let mut distance = 0.0f32;
        for step in 0..steps {
            distance += line
                .point((here + step) as usize)
                .distance(line.point((here + step + 1) as usize));
            if distance >= oag_ai::PAD_LOOKAHEAD {
                return None;
            }
        }
        Some(oag_ai::Pad {
            distance,
            offset: self.sim.weapon_pad_line[index].1,
        })
    }
}
