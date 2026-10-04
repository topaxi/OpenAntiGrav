//! A particle field that fills a wrapping box and keeps still in the world: what
//! the weather's `WO_RAIN` and `WO_SNOW` do, through modifier type `0x13`
//! (`FUN_088fb11c`, `oag_vex::pob::field`).
//!
//! The pool holds **field coordinates**: the box is `[-e, e]` on every axis,
//! centred on the field's own origin, and [`Anchor`] says where that origin
//! is. Each tick, after the pool's own integration, every particle is
//!
//! 1. kept where it was **in the world** while the anchor moves - for a camera
//!    anchor that is the previous frame's view to this frame's, for a world
//!    anchor nothing at all;
//! 2. blown by the wind, a world vector in the camera case and a field vector
//!    in the world case, as the original adds it;
//! 3. wrapped a whole `2e` back inside the box on any axis it left.
//!
//! A streak's other end, [`super::particle::Particle::origin`], is deliberately
//! **not** carried through step 1: it is the particle's field position one tick
//! ago, so the streak reads as the particle's motion through the field - the
//! wind plus the camera's own, which is what makes rain streak outward from
//! where the craft is heading. That is the original's: its second point is only
//! moved by a wrap.

use oag_core::Rng;
use oag_core::math::Vec3;

use super::{Effect, System};

/// A frame in the world: a position and the directions of its three axes,
/// the rows of a row-major matrix with the translation in row 3.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub position: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub back: Vec3,
}

impl Frame {
    /// The frame at the world origin with its axes on the world's.
    pub const IDENTITY: Self = Self {
        position: Vec3::ZERO,
        right: Vec3::X,
        up: Vec3::Y,
        back: Vec3::Z,
    };

    /// A point given in this frame, in the world.
    #[must_use]
    pub fn to_world(&self, local: Vec3) -> Vec3 {
        self.position + self.right * local.x + self.up * local.y + self.back * local.z
    }

    /// A world point, in this frame.
    #[must_use]
    pub fn to_local(&self, world: Vec3) -> Vec3 {
        let offset = world - self.position;
        Vec3::new(
            offset.dot(self.right),
            offset.dot(self.up),
            offset.dot(self.back),
        )
    }

    /// A world direction, in this frame's axes.
    #[must_use]
    pub fn direction_to_local(&self, world: Vec3) -> Vec3 {
        Vec3::new(
            world.dot(self.right),
            world.dot(self.up),
            world.dot(self.back),
        )
    }
}

/// Where a field's origin is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Anchor {
    /// Rides the camera: the box is [`FieldSpec::depth`] ahead of it along
    /// its forward axis (`-back`), and the world holds still under it.
    Camera { previous: Frame, current: Frame },
    /// Fixed in the world at this frame, the camera's motion not reaching it.
    World(Frame),
    /// On the lens, `depth` units in front of it, the emitter frame turned so
    /// its `XZ` plane is the screen's: a field point `(x, y, z)` is `x` right,
    /// `z` up and `y` away. The screen effect's own placement - the instance
    /// matrix's rows `(1,0,0), (0,0,-1), (0,1,0)`, read live - with no
    /// camera compensation, so a droplet stays where it was on the glass.
    Lens(Frame),
}

/// The box and the tick's wind.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldSpec {
    /// `[-half_extent, half_extent]` on every axis.
    pub half_extent: f32,
    /// How far ahead of a camera anchor the box is centred.
    pub depth: f32,
    /// The wind, per tick.
    pub wind: Vec3,
}

impl Anchor {
    /// Where a field coordinate is, in the world, as drawn.
    #[must_use]
    pub fn to_world(&self, field: Vec3, depth: f32) -> Vec3 {
        match self {
            Self::Camera { current, .. } => current.to_world(field - Vec3::Z * depth),
            Self::World(frame) => frame.to_world(field),
            Self::Lens(frame) => frame.to_world(Vec3::new(field.x, field.z, -field.y - depth)),
        }
    }
}

impl System {
    /// Holds `res+0x54`, the azimuth an aimed emitter spawns along, for this pool.
    ///
    /// The original's `Weather_Update` writes it into the *resource* every frame
    /// (the lens's wind direction on the glass); an [`super::Effect`] here is shared,
    /// so the pool keeps the live value instead. Only an aimed emitter reads it.
    pub fn set_azimuth(&mut self, azimuth: f32) {
        self.azimuth = Some(azimuth);
    }

    /// Runs [`System::advance`] with the field's origin at the pool's own
    /// zero, then applies the field's step to every particle that has
    /// aged. See the module doc.
    pub fn advance_field(
        &mut self,
        effect: &Effect,
        dt: f32,
        anchor: &Anchor,
        field: &FieldSpec,
        rng: &mut Rng,
    ) {
        // A particle born this tick has not been through the pool's own
        // step, and the original's modifier does not run on it either. Its
        // slot was empty before this tick's emission, which also holds for an
        // immortal particle, whose life never visibly drops below its maximum.
        let aged: Vec<bool> = self.particles.iter().map(|p| p.alive()).collect();
        self.advance(effect, dt, Vec3::ZERO, Vec3::Y, rng);
        let extent = field.half_extent;
        let span = 2.0 * extent;
        for (particle, aged) in self.particles.iter_mut().zip(aged) {
            if !particle.alive() || !aged {
                continue;
            }
            let streak = particle.origin;
            let mut position = match anchor {
                Anchor::Camera { previous, current } => {
                    let world = previous.to_world(particle.position - Vec3::Z * field.depth);
                    current.to_local(world + field.wind) + Vec3::Z * field.depth
                }
                Anchor::World(_) | Anchor::Lens(_) => particle.position + field.wind,
            };
            let mut origin = match anchor {
                Anchor::Camera { .. } => streak,
                Anchor::World(_) | Anchor::Lens(_) => streak + field.wind,
            };
            for axis in 0..3 {
                if position[axis] > extent {
                    position[axis] -= span;
                    origin[axis] -= span;
                }
                if position[axis] < -extent {
                    position[axis] += span;
                    origin[axis] += span;
                }
            }
            particle.position = position;
            particle.origin = origin;
        }
    }

    /// This pool with every particle moved from field coordinates to the
    /// world, for drawing: [`System::extend_vertices`] reads world positions.
    #[must_use]
    pub fn in_world(&self, anchor: &Anchor, depth: f32) -> Self {
        let mut world = self.clone();
        for particle in &mut world.particles {
            if particle.alive() {
                particle.position = anchor.to_world(particle.position, depth);
                particle.origin = anchor.to_world(particle.origin, depth);
            }
        }
        world
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psys::tests::effect;

    const DT: f32 = 1.0 / 60.0;

    fn spec(wind: Vec3) -> FieldSpec {
        FieldSpec {
            half_extent: 50.0,
            depth: 60.0,
            wind,
        }
    }

    fn live(system: &System) -> Vec<(Vec3, Vec3)> {
        system
            .particles
            .iter()
            .filter(|p| p.alive())
            .map(|p| (p.position, p.origin))
            .collect()
    }

    /// Runs `ticks` steps with the camera standing still.
    fn still(system: &mut System, effect: &Effect, ticks: usize, wind: Vec3) {
        let mut rng = Rng::new(1);
        let anchor = Anchor::Camera {
            previous: Frame::IDENTITY,
            current: Frame::IDENTITY,
        };
        for _ in 0..ticks {
            system.advance_field(effect, DT, &anchor, &spec(wind), &mut rng);
        }
    }

    #[test]
    fn a_still_camera_lets_the_wind_carry_the_particles() {
        let effect = effect("wind", true, 100.0);
        let mut system = System::new();
        system.ignite(&effect, Vec3::ZERO, 1.0);
        still(&mut system, &effect, 4, Vec3::new(0.0, -3.0, 0.0));
        // The spawn tick moves nothing; the three after it each fall three.
        let lowest = live(&system)
            .iter()
            .map(|(p, _)| p.y)
            .fold(f32::MAX, f32::min);
        assert!((lowest - -9.0).abs() < 1e-4, "{lowest}");
    }

    /// `WO_SNOW`'s flakes are immortal (flag `0x800`): their life is
    /// `FLT_MAX` and never visibly drops, and the wind still has to carry them.
    #[test]
    fn the_wind_carries_an_immortal_particle_too() {
        let mut effect = (*effect("snow", true, 100.0)).clone();
        effect.emitters[0].lifetime_ticks = (f32::MAX, 0.0);
        let mut system = System::new();
        system.ignite(&effect, Vec3::ZERO, 1.0);
        still(&mut system, &effect, 4, Vec3::new(0.0, -3.0, 0.0));
        let lowest = live(&system)
            .iter()
            .map(|(p, _)| p.y)
            .fold(f32::MAX, f32::min);
        assert!((lowest - -9.0).abs() < 1e-4, "{lowest}");
    }

    #[test]
    fn the_world_holds_still_while_the_camera_moves() {
        let effect = effect("hold", true, 100.0);
        let mut system = System::new();
        system.ignite(&effect, Vec3::ZERO, 1.0);
        let mut rng = Rng::new(1);
        let mut camera = Frame::IDENTITY;
        let anchor = |previous, current| Anchor::Camera { previous, current };
        system.advance_field(
            &effect,
            DT,
            &anchor(camera, camera),
            &spec(Vec3::ZERO),
            &mut rng,
        );
        let before = live(&system)[0].0;
        let world_before = anchor(camera, camera).to_world(before, 60.0);
        let previous = camera;
        // The camera drives 5 units to its right and 8 forward (-back).
        camera.position += Vec3::new(5.0, 0.0, -8.0);
        system.advance_field(
            &effect,
            DT,
            &anchor(previous, camera),
            &spec(Vec3::ZERO),
            &mut rng,
        );
        let aged = system
            .particles
            .iter()
            .find(|p| p.alive() && p.life < p.max_life)
            .expect("one particle has aged");
        let world_after = anchor(previous, camera).to_world(aged.position, 60.0);
        assert!(
            (world_after - world_before).length() < 1e-3,
            "{world_before:?} {world_after:?}"
        );
        // And the streak's tail is where the particle was in the old field
        // coordinates, so it is as long as the camera's own move.
        assert!((aged.position - aged.origin).length() > 9.0);
    }

    #[test]
    fn a_particle_that_leaves_the_box_comes_back_a_whole_span_in() {
        let effect = effect("wrap", true, 100.0);
        let mut system = System::new();
        system.ignite(&effect, Vec3::ZERO, 1.0);
        // 30 per tick along x: out of a 50 box on the second aged tick.
        still(&mut system, &effect, 4, Vec3::new(30.0, 0.0, 0.0));
        for (position, origin) in live(&system) {
            assert!(position.x.abs() <= 50.0, "{position:?}");
            assert!(origin.x.abs() <= 130.0, "{origin:?}");
        }
        assert!(
            live(&system).iter().any(|(p, _)| p.x < -10.0),
            "nothing wrapped round to the far side"
        );
    }

    #[test]
    fn a_world_anchor_adds_the_wind_raw_and_ignores_the_camera() {
        let effect = effect("anchored", true, 100.0);
        let mut system = System::new();
        system.ignite(&effect, Vec3::ZERO, 1.0);
        let mut rng = Rng::new(1);
        let mut frame = Frame::IDENTITY;
        frame.position = Vec3::new(672.0, 115.0, -170.0);
        let anchor = Anchor::World(frame);
        for _ in 0..3 {
            system.advance_field(
                &effect,
                DT,
                &anchor,
                &spec(Vec3::new(0.0, -1.0, 0.0)),
                &mut rng,
            );
        }
        let lowest = live(&system)
            .iter()
            .map(|(p, _)| p.y)
            .fold(f32::MAX, f32::min);
        assert!((lowest - -2.0).abs() < 1e-4, "{lowest}");
        let world = system.in_world(&anchor, 60.0);
        assert!(
            world
                .particles
                .iter()
                .filter(|p| p.alive())
                .all(|p| (p.position.x - 672.0).abs() < 1e-3),
            "a world anchor puts the field at its own position"
        );
    }
}
