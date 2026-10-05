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

/// How far past the corridor's edge a pad's centre may sit and still be
/// reachable: about the pad's half-width (its box is 9.9 units across), so a
/// craft on the edge still crosses it. **Chosen**, against a measured gap: on
/// all twelve `Pulse` circuits the reachable pads overhang by at most 2.0
/// units (`16_Track`'s far pad) and the rest by 10.2 or more.
const REACH_SLACK: f32 = 5.0;

/// How far a pad's centre may sit from the line's lateral axis - above, below
/// or along it - and still be on this stretch of road. **Chosen**: reachable
/// pads measure at most 3.3; a pad on a deck stacked over the line, or on the
/// branch of a split the lap ring does not drive, measures 5 to 64.
const MAX_RESIDUAL: f32 = 8.0;

/// Where each pad sits on `line`: the index of the sample nearest its centre,
/// and its centre's offset across the line there, positive to the right.
/// `None` for a pad no craft on this line can reach - see [`REACH_SLACK`] and
/// [`MAX_RESIDUAL`]: `05_Track`, `14_Track` and `07_Track` author pads on the
/// branches their lap does not drive, and steering at one of those would
/// pin a craft to the corridor's edge every lap.
///
/// Searched over the whole line once, at the start, so a craft's per-tick
/// question is integer arithmetic over a handful of pads. Ties go to the
/// earlier sample.
#[must_use]
pub(crate) fn line_positions(
    line: &oag_ai::Line,
    pads: &[oag_vex::pads::PadVolume],
) -> Vec<Option<(u32, f32)>> {
    let len = line.len();
    if len == 0 {
        return Vec::new();
    }
    pads.iter()
        .map(|pad| {
            let centre = Vec3::from_array(pad.centre());
            let index = line.nearest(centre, 0, len);
            let frame = line.corridor_at(index)?;
            let to_pad = centre - line.point(index);
            let offset = to_pad.dot(frame.lateral);
            let residual = (to_pad.length_squared() - offset * offset).max(0.0).sqrt();
            let overhang = offset.abs() - frame.room(offset);
            (overhang <= REACH_SLACK && residual <= MAX_RESIDUAL).then_some((index as u32, offset))
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
    ///
    /// **Opponents only.** The player's own craft is driven through the same
    /// `field_for` when its autopilot is on, and a player who handed over the
    /// controls did not ask to go shopping.
    pub(crate) fn pad_for(&self, slot: usize) -> Option<oag_ai::Pad> {
        if slot == self.player_slot() {
            return None;
        }
        let ship = &self.sim.world.ships[slot];
        let wants = match self.sim.pad_seeking {
            PadSeeking::Off => false,
            PadSeeking::EmptySlot => ship.pickup.is_empty(),
            PadSeeking::Every => true,
        };
        let line = &self.sim.racing_line;
        let len = line.len() as u32;
        // The pads are placed on the ring's indices, so a craft on a route
        // (`crate::routes`) does not seek one until it is back. Ours.
        let on_route = ship.driver.branching.route != 0;
        if !wants || on_route || len == 0 || self.sim.weapon_pad_line.is_empty() {
            return None;
        }
        let here = ship.driver.index % len;
        let across = line.corridor_at(here as usize).map_or(0.0, |frame| {
            (ship.physics.body.position - line.point(here as usize)).dot(frame.lateral)
        });
        // Samples ahead, for every armed pad in front.
        let ahead = |index: usize| -> Option<u32> {
            let (at, _) = self.sim.weapon_pad_line[index]?;
            if self.sim.weapon_pad_refresh_left[index] > 0.0 {
                return None;
            }
            let steps = (at + len - here) % len;
            (steps > 0).then_some(steps)
        };
        let nearest = (0..self.sim.weapon_pad_line.len())
            .filter_map(ahead)
            .min()?;
        let mut best: Option<(u32, f32)> = None;
        for index in 0..self.sim.weapon_pad_line.len() {
            let Some(steps) = ahead(index) else { continue };
            if steps > nearest + PAIR_SAMPLES {
                continue;
            }
            let Some((_, offset)) = self.sim.weapon_pad_line[index] else {
                continue;
            };
            if best
                .is_none_or(|(_, incumbent)| (offset - across).abs() < (incumbent - across).abs())
            {
                best = Some((steps, offset));
            }
        }
        let (steps, offset) = best?;
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
        Some(oag_ai::Pad { distance, offset })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pad of unit box centred on `at`.
    fn pad_at(at: [f32; 3]) -> oag_vex::pads::PadVolume {
        let mut to_world = [0.0; 16];
        for axis in 0..4 {
            to_world[axis * 5] = 1.0;
        }
        to_world[12..15].copy_from_slice(&at);
        oag_vex::pads::PadVolume {
            to_world,
            min: [-0.5; 3],
            max: [0.5; 3],
            disabled: 0.0,
        }
    }

    /// A straight line down `-Z`, a corridor 10 units either side, `+X` right.
    fn straight() -> oag_ai::Line {
        let points: Vec<Vec3> = (0..100).map(|i| Vec3::new(0.0, 0.0, -(i as f32))).collect();
        let frame = oag_ai::Frame {
            lateral: Vec3::X,
            left: -10.0,
            right: 10.0,
        };
        oag_ai::Line::with_corridor(points, vec![frame; 100])
    }

    #[test]
    fn a_pad_in_reach_is_placed_and_one_off_the_road_is_not() {
        let line = straight();
        let placed = line_positions(
            &line,
            &[
                pad_at([4.0, 0.0, -20.0]),
                // Overhanging the corridor by about a pad's half-width: kept.
                pad_at([-14.0, 0.0, -30.0]),
                // A branch the lap does not drive.
                pad_at([40.0, 0.0, -40.0]),
                // A deck stacked over the line.
                pad_at([0.0, 30.0, -50.0]),
            ],
        );
        assert_eq!(placed, vec![Some((20, 4.0)), Some((30, -14.0)), None, None]);
    }

    #[test]
    fn only_an_eliminator_steers_for_pads() {
        assert_eq!(
            PadSeeking::for_mode(oag_race::Mode::Eliminator),
            PadSeeking::EmptySlot
        );
        assert_eq!(
            PadSeeking::for_mode(oag_race::Mode::SingleRace),
            PadSeeking::Off
        );
    }
}
