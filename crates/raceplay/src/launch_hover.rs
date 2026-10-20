//! The grid hover clamp: what `Craft_Update` carries in `entry+0x348`, written to every craft each
//! tick from the countdown clock.
//!
//! Wipeout HD's craft sits at a lower hover than its race value from the first frame of the
//! flyby, through the whole countdown, and for `2.46` s after the green light, because its
//! hover target is `min(base, entry+0x348) * 0.75` and `+0x348` is `3.0x` in the grid state and a
//! seconds timer after it. Read and measured in
//! [`hover-target.md`](../../../docs/ghidra/functions/ps3-hdfury-eu/hover-target.md); the numbers are
//! the title's [`oag_title::launch_hover::LaunchHover`].
//!
//! # What is chosen, not measured
//!
//! - The jitter is dropped: the clamp is the mean of the original's `3.000 .. 3.0765`.
//! - The timer's first step lands on the tick the grid state ends, the original's frame order
//!   being unread to the tick. At `1.0` unit a second that is `0.017` units of clamp.
//! - Every craft gets it, AI included: the original's release loop covers the craft array, and
//!   Time Trial's lone player was the only one measured.

use oag_core::math::Vec3;
use oag_physics::hover::{MAX_PROBES, NormalMean, Rig, SpringLaw};
use oag_race::RaceState;
use oag_title::craft_laws::CraftLaws;
use oag_title::hover_rig::HoverRig;
use oag_title::launch_hover::LaunchHover;

use super::Race;

/// The clamp on the pre-scale hover target at `world_tick`, for a race ticking at `dt` seconds.
///
/// On the grid it is the title's `grid_cap`; from the tick the grid state ends it grows by the
/// release rate for every second since, the first step included.
#[must_use]
pub fn cap_at(hover: &LaunchHover, world_tick: u64, dt: f32) -> f32 {
    let grid_cap = hover.grid_cap.value;
    if RaceState::thrust_gated(world_tick + 1) {
        return grid_cap;
    }
    grid_cap + hover.release_rate.value * released_ticks(world_tick) as f32 * dt
}

/// The physics rig a title's [`HoverRig`] describes, or Pulse's two-point law for `None`.
pub(crate) fn physics_rig(rig: Option<&HoverRig>) -> Rig {
    let Some(rig) = rig else {
        return Rig::TWO_POINT;
    };
    let mut offsets = [Vec3::ZERO; MAX_PROBES];
    let probes = rig.probes.value;
    for (slot, [x, y, z]) in offsets.iter_mut().zip(probes) {
        *slot = Vec3::new(*x, *y, *z);
    }
    Rig {
        offsets,
        count: probes.len().min(MAX_PROBES),
        spring_share: rig.spring_share.value,
        derive_rear: !rig.cast_every_probe.value,
        along_normal: rig.along_hit_normal.value,
        normal_mean: if rig.quarter_sum_normal.value {
            NormalMean::QuarterSum
        } else {
            NormalMean::Normalised
        },
        sum_escapes: rig.sum_escapes.value,
        probe_escape: rig.probe_escape.value,
        spring: if rig.four_corner_spring.value {
            SpringLaw::FourCorner
        } else {
            SpringLaw::Racing
        },
    }
}

/// The craft's inertia tensor for a title's [`CraftLaws`], Pulse's for `None`.
pub(crate) fn craft_inertia(laws: Option<&CraftLaws>) -> Vec3 {
    laws.map_or_else(oag_physics::forces::ship_inertia, |laws| {
        oag_physics::forces::ship_inertia_at(laws.inertia_mass.value)
    })
}

/// The frame a title's [`CraftLaws`] apply the inertia diagonal in, Pulse's world axes for
/// `None`.
pub(crate) fn inertia_frame(laws: Option<&CraftLaws>) -> oag_physics::InertiaFrame {
    if laws.is_some_and(|laws| laws.inertia_rotates.value) {
        oag_physics::InertiaFrame::Body
    } else {
        oag_physics::InertiaFrame::World
    }
}
/// The craft's force-law constants for a title's [`CraftLaws`], Pulse's for `None`.
pub(crate) fn physics_laws(laws: Option<&CraftLaws>) -> oag_physics::laws::Laws {
    let pulse = oag_physics::laws::Laws::PULSE;
    laws.map_or(pulse, |laws| oag_physics::laws::Laws {
        engine_output_double: laws.engine_output_double.value,
        drag_ground: laws.drag_ground.value,
        bank_to_yaw: laws.bank_to_yaw.value,
        wall_one_sided: laws.wall_one_sided.value,
        wall_steer_away: laws.wall_steer_away.value,
        wall_point_rate_body_local: laws.wall_point_rate_body_local.value,
        pitch_damping_grid: laws.code_pitch.map(|pitch| pitch.value.grid_damping),
        // Data, not a craft law: the race overwrites it from the disc's `<Collision>`.
        wall_restitution: pulse.wall_restitution,
    })
}

/// Ticks since the grid state ended, counting the tick it ended on as the first.
fn released_ticks(world_tick: u64) -> u64 {
    (world_tick + 1).saturating_sub(oag_race::COUNTDOWN_TICKS) + 1
}

impl Race {
    /// Writes this tick's grid hover clamp to every craft (`None` for a title with none), the
    /// title's hover probe set and its steering clamp.
    pub(super) fn set_hover_caps(&mut self) {
        let cap = self
            .sim
            .launch_hover
            .map(|hover| cap_at(hover, self.sim.world.tick, self.sim.dt));
        for ship in &mut self.sim.world.ships {
            ship.physics.hover_cap = cap;
            ship.physics.hover_rig = self.sim.hover_rig;
            ship.physics.steer_ramp_clamped = self.sim.steer_ramp_clamped;
            ship.physics.air_pitch_bias = self.sim.air_pitch_bias;
            ship.physics.pitch_gate = self.sim.pitch_gate;
            ship.physics.inertia_frame = self.sim.inertia_frame;
        }
    }
}

#[cfg(test)]
mod tests;
