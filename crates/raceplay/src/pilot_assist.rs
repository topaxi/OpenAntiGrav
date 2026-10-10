//! Pilot Assist for the player's craft: what `oag_physics::pilot_assist` needs
//! each tick, located on this race's spline, and the switch the settings flip.
//!
//! AI craft never get it (`field.rs` passes no input): the maintainer's rule is
//! that opponents fly the player's physics and drive better rather than cheat.
//! See `docs/physics/pilot-assist.md`.

use oag_core::math::Vec3;
use oag_physics::pilot_assist::{Candidates, Corridor, Input, look_ahead};
use oag_race::Mode;
use oag_vex::track::Sample;

use crate::Race;
use crate::spline::Spline;

/// How much further than the nearest record a record on another path may be and
/// still count as its sibling on a fork. **Chosen, not measured**: the original
/// takes the sibling off its junction graph (`0x000a96d8`), which this table does
/// not keep; two paths over one stretch sit well inside this.
const SIBLING_RANGE: f32 = 20.0;

impl Race {
    /// Turns the player's Pilot Assist on or off, from the settings or a pause menu.
    /// A title that authors no Pilot Assist table ignores it.
    pub fn set_pilot_assist(&mut self, on: bool) {
        self.sim.pilot_assist_on = on;
    }

    /// Whether this race offers Pilot Assist at all: the title authors its table.
    #[must_use]
    pub fn pilot_assist_available(&self) -> bool {
        self.sim.pilot_assist.is_some()
    }

    /// The player's assist input for this tick, or `None` when the title has no
    /// table, which leaves the step bit-for-bit as it was.
    pub(crate) fn pilot_assist_input(&self, player: usize) -> Option<Input> {
        let params = self.sim.pilot_assist?;
        let physics = &self.sim.world.ships[player].physics;
        // HD's gate: racing (`craft+0x2f8 == 1`, not the grid) and not Zone,
        // Zone Battle or Detonator (modes 6, 13, 14). Of those this engine runs Zone.
        let enabled =
            self.sim.pilot_assist_on && !physics.on_grid && self.sim.world.mode() != Mode::Zone;
        if !enabled && physics.pilot_assist == Default::default() {
            // Nothing to decay and nothing to apply: skip both scans.
            return Some(Input {
                params,
                enabled,
                here: [None, None],
                ahead: [None, None],
            });
        }
        let body = &physics.body;
        Some(Input {
            params,
            enabled,
            here: candidates(&self.sim.spline, body.position),
            ahead: candidates(&self.sim.spline, look_ahead(body, &params)),
        })
    }
}

/// The nearest record and, on a fork, the nearest on another path.
fn candidates(spline: &Spline, point: Vec3) -> Candidates {
    let Some((first, first_distance, first_path)) = spline.nearest_with_path(point, None) else {
        return [None, None];
    };
    let second = spline
        .nearest_with_path(point, Some(first_path))
        .filter(|(_, distance, _)| *distance <= first_distance + SIBLING_RANGE)
        .map(|(sample, _, _)| corridor(sample));
    [Some(corridor(first)), second]
}

/// A table sample as the law reads a located record: lifted, as the original's are.
fn corridor(sample: &Sample) -> Corridor {
    let down = Vec3::from_array(sample.down);
    Corridor {
        position: Vec3::from_array(sample.pos) - down * oag_vex::track::HOVER_LIFT,
        tangent: Vec3::from_array(sample.tangent),
        down,
        lateral: Vec3::from_array(sample.lateral),
        half_width_left: sample.half_width_left,
        half_width_right: sample.half_width_right,
    }
}
