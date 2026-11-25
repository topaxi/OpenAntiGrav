//! What Detonator's pulse cannon and explosions draw: each round's model, the muzzle flash at
//! the hull's `cannon_flash` locators, and the disc's own `.pob` effects off the tick's
//! [`oag_weapons::detonator::Event`]s.
//!
//! Evidence: `docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`, `detonator-mine.md` and
//! `detonator-bomb.md`. The round's draw (`0x00121e68`) only pushes and pops a matrix, so the
//! round's own placement is **chosen, not measured**: its model rides its position, its nose
//! along its velocity and its up along the craft's up at launch. The flash is re-placed at the
//! firing craft's locator for the round's first [`FLASH_WINDOW_SECONDS`] and rolled every tick as
//! `PulseBullet_Update` rolls it, and `PulseBullet`'s render queue's point lights ride it
//! (`detonator-cannon.md`, "The flash's per-tick roll").
//!
//! The original spawns each effect only when the object is in some view's potentially-visible
//! set; this plays every one, whatever the camera sees.

use oag_fx::weapon_quads::geometry::FLASH_WINDOW_SECONDS;
use oag_mesh::mesh_render::SpuLight;
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
    /// the craft's live pose, rolled the way `PulseBullet_Update` rolls it every tick of the window
    /// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`, "The flash's per-tick roll"):
    /// the three unit axes of the locator in the world as [`Self::cannon_muzzle_matrix`] builds
    /// them for HD's ordinary Cannon, turned about the third axis by a roll of `[0, 2 pi)`
    /// (`Matrix_RotateRowsByAxisAngle`), all three scaled by a roll of
    /// [`FLASH_SIZE_ROLL`] and the third by a further roll of [`FLASH_STRETCH_ROLL`].
    ///
    /// A volley's first round leaves the left locator and the second the right
    /// (`Round::slot`'s parity, the ring staying even). **Chosen, not measured:** the rolls are
    /// drawn from `oag_fx::weapon_quads::random` seeded by the round's slot and the tick, one
    /// draw per 60 Hz tick (the original draws one per frame, 30 Hz in the capture), so the
    /// simulation's own stream is never advanced.
    #[must_use]
    pub(crate) fn detonator_flash_matrices(&self, draw: &CannonDraw) -> Vec<Mat4> {
        use oag_fx::weapon_quads::{geometry::FLASH_SIZE_SCALE, random};
        let ship = self.view.ship_model_matrix_of(&self.core, 0);
        let tick = self.core.sim.world.tick as u32;
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
                let seed = u32::from(round.slot);
                let (angle, half_size, _) = random::ranged_flash_roll(seed, tick, FLASH_SIZE_ROLL);
                let size = half_size / FLASH_SIZE_SCALE;
                let stretch = random::stretch_roll(seed, tick, FLASH_STRETCH_ROLL);
                let (sin, cos) = angle.sin_cos();
                Some(Mat4::from_cols(
                    ((x * cos + y * sin) * size).extend(0.0),
                    ((y * cos - x * sin) * size).extend(0.0),
                    (z * (size * stretch)).extend(0.0),
                    world.w_axis,
                ))
            })
            .collect()
    }

    /// The Detonator cannon's point lights, as `PulseBullet`'s render queue (`0x00122098`) submits
    /// them to `SpuLight_AddCandidate`: a blue light at each muzzle for the round's first
    /// [`FLASH_WINDOW_SECONDS`] and a faint one on each round in flight; and the EMP wave's,
    /// at its centre while it runs (`EMP_DrawLights`, `0x0013b300`). Empty under the same
    /// switches that silence every weapon light ([`Self::hd_weapon_lights`]'s).
    #[must_use]
    pub(crate) fn detonator_cannon_lights(&self, draw: &CannonDraw) -> Vec<SpuLight> {
        if !self.view.detonator_cannon_lights
            || !self.view.hd_trail_active
            || !self.view.spu_vertex_lights
            || !self.view.hd_weapon_lights
        {
            return Vec::new();
        }
        let flashes = self
            .detonator_flash_matrices(draw)
            .into_iter()
            .map(|matrix| weapon_light::detonator_flash_record(matrix.w_axis.truncate()));
        let bolts = self
            .core
            .sim
            .world
            .detonator
            .cannon
            .rounds
            .iter()
            .filter(|round| round.live)
            .map(|round| weapon_light::detonator_bolt_record(round.pos));
        let emp = self
            .view
            .emp_effect
            .and_then(|playing| self.view.stage.anchor_of(playing))
            .map(weapon_light::detonator_emp_record);
        flashes.chain(bolts).chain(emp).collect()
    }
}

/// The range `PulseBullet_Update` rolls the flash's size from, applied to all three axes
/// (`0x001231e0`, constants `0x008aa0d8` and `0x008aa080`).
pub(crate) const FLASH_SIZE_ROLL: (f32, f32) = (0.25, 1.0);

/// The range of the further roll that stretches the flash along its own third axis
/// (`0x00123244`, constants `0x008aa0c8` and `0x008aa0cc`).
pub(crate) const FLASH_STRETCH_ROLL: (f32, f32) = (0.7, 1.3);

/// Which of the disc's effects an event plays, or `None` for one that plays none.
///
/// A mine a craft runs into is always dark (the contact darkens it first), a shot, chained or
/// EMP'd mine plays by whether it was laid this stage (`DetonatorMine_Explode`; the EMP's
/// `MineManager_DestroyAtLane` darkens nothing, `detonator-mine.md`); every bomb plays the one
/// (`DetonatorBombBlast_Start`, which `BombManager_DestroyAtLane` reaches, `detonator-bomb.md`).
#[must_use]
pub(crate) fn effect_for(kind: EventKind) -> Option<Trigger> {
    match kind {
        EventKind::MineTouched => Some(Trigger::DetonatorMineExploDead),
        EventKind::MineShot { live }
        | EventKind::MineChained { live }
        | EventKind::MineEmped { live } => Some(if live {
            Trigger::DetonatorMineExplo
        } else {
            Trigger::DetonatorMineExploDead
        }),
        EventKind::BombTouched
        | EventKind::BombShot
        | EventKind::BombReplaced
        | EventKind::BombEmped => Some(Trigger::DetonatorBombExplo),
        EventKind::RoundWall { .. } => Some(Trigger::DetonatorCannonSparks),
        // The wave itself rides its front (`RaceView::advance_emp_visual`), not a one-shot.
        EventKind::Volley | EventKind::EmpReady | EventKind::EmpFull | EventKind::EmpLaunch => None,
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

impl RaceView {
    /// Keeps [`Trigger::DetonatorEmp`] on the EMP wave's front while it runs and releases it when
    /// the wave ends.
    ///
    /// `EMP_SpawnLaunchEffect` (`0x0013afe8`) attaches it to the wave's matrix on the launch tick;
    /// `EMP_SweepSections` (`0x0013b930`, the Repulser's `AdvanceWave` with the effect renamed)
    /// sets that matrix every tick to the midpoint of the track's two edges at the front and the
    /// instance's extent co-factor to the edges' distance over `100` (`0x3c23d70a`); the frame is
    /// the Repulser wave's, `Y` the track's up and `X` across it
    /// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-emp.md`, "What HD draws for a fired EMP").
    /// The front's point is the spline sample nearest the survey entry it stands on.
    pub(crate) fn advance_emp_visual(&mut self, core: &SimRace) {
        let front = core.sim.detonator.as_ref().and_then(|detonator| {
            let entry = oag_weapons::detonator::emp::front_entry(
                &core.sim.world.detonator.emp.wave,
                &detonator.survey,
            )?;
            Some(detonator.survey.entries[entry].centre)
        });
        let Some((_, sample, _)) = front.and_then(|at| core.sim.spline.nearest(at)) else {
            if let Some(playing) = self.emp_effect.take() {
                self.stage.release(playing);
            }
            return;
        };
        let lateral = Vec3::from_array(sample.lateral);
        let up = -Vec3::from_array(sample.down);
        let mid = Vec3::from_array(sample.pos)
            + lateral * ((sample.half_width_right - sample.half_width_left) * 0.5);
        let width = (lateral * (sample.half_width_left + sample.half_width_right)).length();
        let playing = match self.emp_effect {
            Some(playing) => {
                self.stage.follow(playing, mid);
                Some(playing)
            }
            None => {
                let effect = self.handles.get(Trigger::DetonatorEmp).cloned();
                self.emp_effect = effect.and_then(|e| self.stage.attach(&e, mid, 1.0));
                // `EMP_SpawnLaunchEffect` hands the spawn its matrix by pointer (`li r7,1` at
                // `0x0013b09c`), so the local-space emitters (flag `0x2`) ride the wave.
                if let Some(playing) = self.emp_effect {
                    self.stage.set_rides_frame(playing, true);
                }
                self.emp_effect
            }
        };
        if let Some(playing) = playing {
            self.stage.orient(playing, up);
            self.stage.stretch(playing, width / 100.0, -lateral);
        }
    }
}
