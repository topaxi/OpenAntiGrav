//! Pilot Assist for the player's craft: what `oag_physics::pilot_assist` needs
//! each tick, located on this race's spline, and the switch the settings flip.
//!
//! AI craft never get it (`field.rs` passes no input): the maintainer's rule is
//! that opponents fly the player's physics and drive better rather than cheat.
//! See `docs/physics/pilot-assist.md`.

use oag_core::math::Vec3;
use oag_physics::pilot_assist::{Candidates, Corridor, Input, Level, MIN_STRENGTH, look_ahead};
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
    /// Sets the player's Pilot Assist level, from the settings. A level this race has
    /// no law for does nothing.
    pub fn set_pilot_assist(&mut self, level: Level) {
        self.sim.pilot_assist_level = level;
    }

    /// The level the player has chosen.
    #[must_use]
    pub fn pilot_assist_level(&self) -> Level {
        self.sim.pilot_assist_level
    }

    /// The laws this race loaded for its two levels, and so what the load report said
    /// about where their numbers came from.
    #[must_use]
    pub fn pilot_assist_laws(&self) -> oag_physics::pilot_assist::Laws {
        self.sim.pilot_assist
    }

    /// Whether this race has a law for `level`.
    #[must_use]
    pub fn pilot_assist_available(&self, level: Level) -> bool {
        self.sim.pilot_assist.get(level).is_some()
    }

    /// Whether the HUD shows the assist as on: both originals read the option
    /// byte itself here, not the craft's per-tick gate, so the countdown shows it.
    /// 2048 ties its background and icon to the Extreme byte alone; Normal, which
    /// runs on a state object the indicator never reads, draws none.
    #[must_use]
    pub fn pilot_assist_shown(&self) -> bool {
        self.sim.pilot_assist_level == Level::Extreme && self.sim.pilot_assist.extreme.is_some()
    }

    /// Advances the HUD indicator off the player's assist state this tick.
    pub(crate) fn advance_assist_indicator(&mut self) {
        let acting = self.ship().physics.pilot_assist.acting;
        let (shown, dt) = (self.pilot_assist_shown(), self.sim.dt);
        self.view.assist_indicator.advance(shown, acting, dt);
    }

    /// The player's assist input for this tick, or `None` when the race has no law
    /// for the chosen level, which leaves the step bit-for-bit as it was.
    pub(crate) fn pilot_assist_input(&self, player: usize) -> Option<Input> {
        let level = self.sim.pilot_assist_level;
        let law = self.sim.pilot_assist.get(level)?;
        let physics = &self.sim.world.ships[player].physics;
        let body = &physics.body;
        let strength = law
            .ramp
            .map_or(1.0, |ramp| ramp.strength(body.linear_velocity.length()));
        // HD's gate: racing (`craft+0x2f8 == 1`, not the grid) and not Zone,
        // Zone Battle or Detonator (modes 6, 13, 14). Of those this engine runs Zone.
        let enabled = !physics.on_grid && self.sim.world.mode() != Mode::Zone;
        let params = law.params;
        if !(enabled && strength > MIN_STRENGTH) && physics.pilot_assist == Default::default() {
            // Nothing to decay and nothing to apply: skip both scans.
            return Some(Input {
                params,
                strength,
                enabled: false,
                here: [None, None],
                ahead: [None, None],
            });
        }
        Some(Input {
            params,
            strength,
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
