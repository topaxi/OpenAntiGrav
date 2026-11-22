//! What Detonator's pulse cannon and explosions draw: each round's model, the muzzle flash at
//! the hull's `cannon_flash` locators, and the disc's own `.pob` effects off the tick's
//! [`oag_weapons::detonator::Event`]s.
//!
//! Evidence: `docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`, `detonator-mine.md` and
//! `detonator-bomb.md`. The round's draw (`0x00121e68`) is unread, so the placement here is
//! **chosen, not measured**: the round's model rides its position, its nose along its velocity
//! and its up along the craft's up at launch; the flash is re-placed at the firing craft's
//! locator for the round's first [`FLASH_WINDOW_SECONDS`], as HD's ordinary Cannon does
//! (`oag_fx::weapon_quads::geometry`, `docs/ghidra/functions/ps3-hdfury-eu/cannon.md`).
//!
//! The original spawns each effect only when the object is in some view's potentially-visible
//! set; this plays every one, whatever the camera sees.

use oag_fx::weapon_quads::geometry::FLASH_WINDOW_SECONDS;
use oag_title::Trigger;
use oag_weapons::detonator::cannon::Round;
use oag_weapons::detonator::{Event, EventKind};

use super::*;

/// The pose of one round: nose along its velocity, up along its launch up, at its position.
/// `side = up x forward` and `up = forward x side` keep it a rotation.
pub(crate) fn round_matrix(round: &Round) -> Mat4 {
    let forward = round.vel.normalize_or_zero();
    let reference = [round.up, Vec3::Y, Vec3::Z]
        .into_iter()
        .find(|axis| axis.cross(forward).length_squared() > 1e-6)
        .unwrap_or(Vec3::X);
    let side = reference.cross(forward).normalize_or_zero();
    let up = forward.cross(side);
    Mat4::from_cols(
        side.extend(0.0),
        up.extend(0.0),
        forward.extend(0.0),
        round.pos.extend(1.0),
    )
}

impl Race {
    /// One matrix per live pulse cannon round, for `detonator_cannonbolt`.
    #[must_use]
    pub(crate) fn detonator_round_matrices(&self) -> Vec<Mat4> {
        self.core
            .sim
            .world
            .detonator
            .cannon
            .rounds
            .iter()
            .filter(|round| round.live)
            .map(round_matrix)
            .collect()
    }

    /// One matrix per round still inside the flash window, at the locator it left from carried by
    /// the craft's live pose: the position and the three unit axes of the locator in the world,
    /// as [`Self::cannon_muzzle_matrix`] builds them for HD's ordinary Cannon.
    ///
    /// A volley's first round leaves the left locator and the second the right
    /// (`Round::slot`'s parity, the ring staying even).
    #[must_use]
    pub(crate) fn detonator_flash_matrices(&self, draw: &CannonDraw) -> Vec<Mat4> {
        let ship = self.view.ship_model_matrix_of(&self.core, 0);
        self.core
            .sim
            .world
            .detonator
            .cannon
            .rounds
            .iter()
            .filter(|round| round.live && round.age < FLASH_WINDOW_SECONDS)
            .filter_map(|round| {
                let side = usize::from(round.slot % 2);
                let world = ship * draw.muzzles.first()?[side]?;
                let x = world.x_axis.truncate().normalize_or_zero();
                let authored_y = world.y_axis.truncate();
                let y = (authored_y - x * x.dot(authored_y)).normalize_or_zero();
                let z = x.cross(y);
                let z = if z.dot(world.z_axis.truncate()) < 0.0 {
                    -z
                } else {
                    z
                };
                Some(Mat4::from_cols(
                    x.extend(0.0),
                    y.extend(0.0),
                    z.extend(0.0),
                    world.w_axis,
                ))
            })
            .collect()
    }
}

/// Which of the disc's effects an event plays, or `None` for one that plays none.
///
/// A mine a craft runs into is always dark (the contact darkens it first), a shot or chained
/// mine plays by whether it was laid this stage (`DetonatorMine_Explode`, `detonator-mine.md`);
/// every bomb plays the one (`DetonatorBombBlast_Start`, `detonator-bomb.md`).
#[must_use]
pub(crate) fn effect_for(kind: EventKind) -> Option<Trigger> {
    match kind {
        EventKind::MineTouched => Some(Trigger::DetonatorMineExploDead),
        EventKind::MineShot { live } | EventKind::MineChained { live } => Some(if live {
            Trigger::DetonatorMineExplo
        } else {
            Trigger::DetonatorMineExploDead
        }),
        EventKind::BombTouched | EventKind::BombShot | EventKind::BombReplaced => {
            Some(Trigger::DetonatorBombExplo)
        }
        EventKind::RoundWall { .. } => Some(Trigger::DetonatorCannonSparks),
        // The EMP's kill goes through `MineManager_DestroyAtLane`/`BombManager_DestroyAtLane`,
        // whose effect is unread (`detonator-emp.md`, "Open"): nothing is played for it.
        EventKind::Volley
        | EventKind::EmpReady
        | EventKind::EmpFull
        | EventKind::EmpLaunch
        | EventKind::MineEmped { .. }
        | EventKind::BombEmped => None,
    }
}

impl RaceView {
    /// Plays this tick's [`Event`]s' effects. A title that does not answer an event's trigger, or
    /// whose effect did not load (the loader's report names it), plays nothing for it.
    pub(crate) fn play_detonator_events(&mut self, core: &SimRace) {
        for Event { kind, at } in &core.sim.detonator_events {
            log::trace!("tick {}: {kind:?} at {at:?}", core.sim.world.tick);
            let Some(effect) = effect_for(*kind).and_then(|t| self.handles.get(t).cloned()) else {
                continue;
            };
            self.stage.play(&effect, *at, 1.0);
        }
    }
}
