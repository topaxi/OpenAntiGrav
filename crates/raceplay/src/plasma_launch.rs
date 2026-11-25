//! Wipeout HD's Plasma release effect, `WO_PLASMA_LAUNCH`: played once on the tick a
//! bolt's wind-up ends, riding the firing craft's `Ship Muzzle` node while it emits.
//!
//! `Plasma_Launch` (`0x0011f810`) spawns it on the node's own matrix, so it stays on the
//! nose of a craft that flies on. A title whose table answers no
//! [`Trigger::PlasmaLaunch`] plays nothing
//! (`docs/ghidra/functions/ps3-hdfury-eu/plasma.md`, "The effects and their triggers").

use super::*;

/// The release effects still emitting, and which bolts have already left.
#[derive(Debug, Clone)]
pub(crate) struct PlasmaLaunch {
    /// Per projectile slot: the Plasma in it has been released.
    released: [bool; oag_weapons::projectile::MAX_PROJECTILES],
    /// The firing craft's slot and its riding instance.
    riding: Vec<(usize, psys::Playing)>,
}

impl PlasmaLaunch {
    pub(crate) fn new() -> Self {
        Self {
            released: [false; oag_weapons::projectile::MAX_PROJECTILES],
            riding: Vec::new(),
        }
    }
}

/// The row length of the `Ship Muzzle` node's world matrix across the craft: `0.75^2`, the
/// bolt ball's own (`weapons::visuals::PLASMA_BALL_SCALE`), which `Plasma_Init` copies from
/// that node (`plasma.md`, "The bolt's two frames").
pub(crate) const MUZZLE_NODE_SCALE: f32 = 0.5625;

/// The `+Y` the Plasma's head and wind-up effects emit about: the bolt's effect frame
/// keeps the firing craft's backward axis there, from the press to the detonation (the
/// live object's `+0xb0` rows, `plasma.md`, "The bolt's two frames"). The release effect
/// rides the muzzle node's own matrix instead, whose `+Y` is the craft's up.
pub(crate) fn effect_up(heading: Vec3) -> Vec3 {
    (-heading).try_normalize().unwrap_or(Vec3::Y)
}

#[cfg(feature = "view")]
impl RaceView {
    /// Starts the release effect for every bolt whose wind-up ended this tick, keeps the
    /// emitting ones on their craft's muzzle and lets go of the finished ones. After
    /// `projectile::step`, before `Stage::advance`.
    pub(crate) fn advance_plasma_launch(&mut self, core: &SimRace) {
        let effect = self.handles.get(Trigger::PlasmaLaunch).cloned();
        for slot in 0..oag_weapons::projectile::MAX_PROJECTILES {
            let projectile = &core.sim.world.projectiles.slots[slot];
            let released = projectile.kind == Some(oag_tables::weapons::Weapon::Plasma)
                && projectile.charge <= 0.0;
            let just_released = released && !self.plasma_launch.released[slot];
            self.plasma_launch.released[slot] = released;
            let (Some(effect), true) = (effect.as_ref(), just_released) else {
                continue;
            };
            let owner = usize::from(projectile.owner);
            let at = self.muzzle_of(core, owner).unwrap_or(projectile.position);
            // The node's own matrix: the hull's, whose `+Y` is the craft's up.
            let up = core.sim.world.ships[owner].physics.body.up();
            if let Some(playing) = self.stage.play_riding(effect, at, 1.0) {
                self.stage.orient(playing, up);
                // Its three emitters are local-space (flag `0x2`): drawn through the
                // node's matrix, they ride the craft at the node's row length.
                self.stage.set_ride_scale(playing, MUZZLE_NODE_SCALE);
                self.stage.set_rides_frame(playing, true);
                self.plasma_launch.riding.push((owner, playing));
            }
        }
        let mut riding = std::mem::take(&mut self.plasma_launch.riding);
        riding.retain(|&(owner, playing)| {
            let ship = &core.sim.world.ships[owner];
            if !ship.in_world() || !self.stage.is_emitting(playing) {
                self.stage.detach(playing);
                return false;
            }
            if let Some(at) = self.muzzle_of(core, owner) {
                self.stage.follow(playing, at);
                self.stage.orient(playing, ship.physics.body.up());
            }
            true
        });
        self.plasma_launch.riding = riding;
    }
}
