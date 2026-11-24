//! The pieces an ordinary craft breaks into as its blast ends
//! (`docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`, "The debris throw"): 19 models thrown
//! from just ahead of the body, two of them burning.
//!
//! **Render-side view state only**, like [`crate::zone_end`], whose flight it shares: the
//! simulation never sees a piece.
//!
//! Read: the piece count, the spawn point, the launch velocity (the craft's own plus a scattered
//! term), the torque's direction, which pieces burn and when a piece goes. **Chosen, not
//! measured:** the pull on a piece in flight (the throw's record carries `(0, -19.62, 0)`, what
//! the physics world does with it is unread), the floor the pieces rest on, the spin rate (the
//! torque's size needs the piece's inertia, which `0x00677968` decides and is unread), and that
//! no piece collides with anything.

use super::*;
use crate::zone_end::{Piece, Thrown};
use oag_livery::ShipDebrisModels;
use oag_title::Trigger;
use oag_title::destroyed::ShipDebris;
use std::sync::Arc;

/// A piece's spin, radians a second. **Chosen, not measured.**
const SPIN_RATE: f32 = 6.0;

#[derive(Debug, Clone, Copy)]
struct Burning {
    copy: usize,
    piece: usize,
    playing: psys::Playing,
}

/// The throws in the air, the pieces' models and the fires riding them.
#[derive(Debug, Clone, Default)]
pub(super) struct ShipDebrisFx {
    models: Option<Arc<ShipDebrisModels>>,
    spec: Option<ShipDebris>,
    copies: Vec<Option<Thrown>>,
    next: usize,
    burning: Vec<Burning>,
    /// Pieces thrown, ever, for tests.
    pieces_thrown: u32,
    /// Throws, ever, for tests.
    throws: u32,
}

impl ShipDebrisFx {
    pub(super) fn new(models: Option<Arc<ShipDebrisModels>>, spec: Option<ShipDebris>) -> Self {
        Self {
            models,
            spec,
            copies: vec![None; oag_livery::ship_debris::COPIES],
            ..Self::default()
        }
    }
}

#[cfg(feature = "view")]
impl RaceView {
    /// Whether any piece is in the air.
    #[must_use]
    pub fn ship_debris_thrown(&self) -> bool {
        self.ship_debris
            .copies
            .iter()
            .flatten()
            .any(|thrown| thrown.lifetime.is_none_or(|life| thrown.age < life))
    }

    /// How many throws and how many pieces there have been, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn ship_debris_counts_for_tests(&self) -> (u32, u32) {
        (self.ship_debris.throws, self.ship_debris.pieces_thrown)
    }

    /// How many pieces are in the air now, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn ship_debris_flying_for_tests(&self) -> usize {
        self.ship_debris
            .copies
            .iter()
            .flatten()
            .filter(|thrown| thrown.lifetime.is_none_or(|life| thrown.age < life))
            .map(|thrown| thrown.pieces.len())
            .sum()
    }

    /// How many fires are riding a piece, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn ship_debris_burning_for_tests(&self) -> usize {
        self.ship_debris.burning.len()
    }

    /// `DebrisManager_SpawnShipDebris` for `slot`: `spec.pieces` models, each from `spec.spawn_ahead`
    /// in front of the body with the body's velocity and a scattered term.
    pub(super) fn throw_ship_debris(&mut self, core: &SimRace, slot: usize) {
        let (Some(models), Some(spec)) = (self.ship_debris.models.clone(), self.ship_debris.spec)
        else {
            return;
        };
        let body = &core.sim.world.ships[slot].physics.body;
        let (right, up, forward) = (body.right(), body.up(), body.forward());
        let at = body.position + forward * spec.spawn_ahead;
        let spawn = Mat4::from_rotation_translation(body.orientation, at);
        let normal = spawn.inverse().transpose();
        let stride = models.stride as usize;
        let base: Vec<(Vec3, Vec3)> = models.debris.vertices[..stride]
            .iter()
            .map(|v| {
                (
                    spawn.transform_point3(Vec3::from(v.position)),
                    normal
                        .transform_vector3(Vec3::from(v.normal))
                        .normalize_or(up),
                )
            })
            .collect();
        let rng = &mut self.stage_rng;
        let mut draw = move || rng.next_f32() * 2.0 - 1.0;
        let pieces: Vec<Piece> = models
            .pieces
            .iter()
            .map(|vertices| {
                // The three draws in the order `0x00104f18` makes them.
                let (a, b, c) = (draw(), draw(), draw());
                let scatter = (right * a + up * b + forward) * spec.spread;
                let velocity = body.linear_velocity + scatter * (1.0 + spec.jitter * c);
                let centre = vertices.iter().map(|&v| base[v as usize].0).sum::<Vec3>()
                    / vertices.len().max(1) as f32;
                Piece {
                    centre,
                    offset: Vec3::ZERO,
                    velocity,
                    axis: up.cross(velocity).normalize_or(Vec3::X),
                    spin: SPIN_RATE,
                    angle: 0.0,
                }
            })
            .collect();
        log::debug!(
            "ship debris: slot {slot} throws {} piece(s) from {at:?}, forward {forward:?}, up {up:?}",
            pieces.len()
        );
        let copy = self.ship_debris.next;
        self.ship_debris.next = (copy + 1) % self.ship_debris.copies.len();
        self.stop_debris_fires(copy);
        self.ship_debris.throws += 1;
        self.ship_debris.pieces_thrown += pieces.len() as u32;
        if let Some(effect) = self.handles.get(Trigger::DebrisFire).cloned() {
            for (piece, flying) in pieces.iter().enumerate() {
                if piece > spec.fire_after
                    && let Some(playing) = self.stage.play_riding(&effect, flying.centre, 1.0)
                {
                    self.ship_debris.burning.push(Burning {
                        copy,
                        piece,
                        playing,
                    });
                }
            }
        }
        self.ship_debris.copies[copy] = Some(Thrown {
            pieces,
            base,
            up: Vec3::Y,
            floor: at - Vec3::Y,
            gravity: -Vec3::Y * spec.gravity,
            age: 0.0,
            lifetime: Some(spec.lifetime_secs),
        });
    }

    fn stop_debris_fires(&mut self, copy: usize) {
        let stage = &mut self.stage;
        self.ship_debris.burning.retain(|fire| {
            let ends = fire.copy == copy;
            if ends {
                stage.kill(fire.playing);
            }
            !ends
        });
    }

    /// One tick: every piece flies, and a fire follows its piece until the piece goes.
    pub(super) fn advance_ship_debris(&mut self, core: &SimRace) {
        let dt = core.sim.dt;
        for thrown in self.ship_debris.copies.iter_mut().flatten() {
            thrown.advance(dt);
        }
        let copies = &self.ship_debris.copies;
        let stage = &mut self.stage;
        self.ship_debris.burning.retain(|fire| {
            let alive = copies[fire.copy]
                .as_ref()
                .is_some_and(|thrown| thrown.lifetime.is_none_or(|life| thrown.age < life));
            let piece = copies[fire.copy]
                .as_ref()
                .and_then(|thrown| thrown.pieces.get(fire.piece));
            match piece {
                Some(piece) if alive && stage.is_emitting(fire.playing) => {
                    stage.follow(fire.playing, piece.centre + piece.offset);
                    true
                }
                _ => {
                    stage.kill(fire.playing);
                    false
                }
            }
        });
    }

    /// Every copy of the model posed for this frame; a copy not in the air collapses to a point.
    pub(crate) fn ship_debris_pose(&self, out: &mut Vec<oag_mesh::mesh::GpuVertex>) -> bool {
        let Some(models) = &self.ship_debris.models else {
            return false;
        };
        out.clear();
        out.extend(models.debris.vertices.iter().copied());
        for (copy, thrown) in self.ship_debris.copies.iter().enumerate() {
            let offset = copy as u32 * models.stride;
            match thrown {
                Some(thrown) => thrown.pose(&models.pieces, offset, out),
                None => {
                    for vertex in &mut out[offset as usize..(offset + models.stride) as usize] {
                        vertex.position = [0.0; 3];
                    }
                }
            }
        }
        true
    }
}
