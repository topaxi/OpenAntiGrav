//! A Zone craft's end, as the player sees it: the hull stays, a white shell animates over it for
//! `shell_secs`, then hull, engine flare and shell hide and the 112 pieces of
//! `zoneship_dead.vex` fly off (`docs/ghidra/functions/ps3-hdfury-eu/zone-destroy.md`, rows Z7,
//! Z8, Z10, Z11).
//!
//! **Render-side view state only.** The simulation ends the run when its state 5 has played out
//! ([`crate::RaceSim::zone_end`], row Z9); what is drawn in between is a clock started by the
//! craft leaving the race, so the picture keeps playing under the results while the world is
//! frozen ([`crate::Race::tick_cosmetics`]), as the Pulse Zone blast does.

use super::*;
use oag_livery::ZoneEndModels;
use std::sync::Arc;

/// The pull on a flying piece, in units a second squared along the craft's up. **Chosen, not
/// measured:** the original's pieces are rigid bodies in the physics world (`Debris_Launch`,
/// body type 7) and which gravity they feel is unread.
const GRAVITY: f32 = 40.0;

/// How fast a piece leaves the hull, outward and up, in units a second, added to the craft's own
/// velocity. **Chosen, not measured:** the per-piece formula in `DebrisManager_SpawnZoneDebris`
/// (`0x00104be4`-`0x00104d50`) is unread beyond "body velocity plus a random term"
/// (`zone-destroy.md`, row Z11, confidence 55).
const OUTWARD: (f32, f32) = (10.0, 26.0);
const LIFT: (f32, f32) = (4.0, 16.0);
const SPIN: (f32, f32) = (2.0, 9.0);

/// One flying piece.
#[derive(Debug, Clone, Copy)]
struct Piece {
    /// Where its vertices' centre was when it was thrown.
    centre: Vec3,
    /// How far it has travelled since.
    offset: Vec3,
    velocity: Vec3,
    axis: Vec3,
    spin: f32,
    angle: f32,
}

/// What a throw fixed: the pieces and the world-space vertices they are posed from.
#[derive(Debug, Clone)]
struct Thrown {
    pieces: Vec<Piece>,
    /// Each debris vertex's position and normal at the throw, in the world.
    base: Vec<(Vec3, Vec3)>,
    /// The craft's up, and a point on the plane the pieces come to rest on.
    up: Vec3,
    floor: Vec3,
}

/// The Zone end's own state on [`crate::RaceView`].
#[derive(Debug, Clone, Default)]
pub(super) struct ZoneEndFx {
    /// The shell and the pieces' vertex lists, `None` where the title ships none or they did not
    /// load: then nothing is drawn and the clock still runs.
    models: Option<Arc<ZoneEndModels>>,
    /// Seconds since the craft left the race.
    age: Option<f32>,
    thrown: Option<Thrown>,
    /// How many throws there have been, for tests.
    throws: u32,
}

impl ZoneEndFx {
    pub(super) fn new(models: Option<Arc<ZoneEndModels>>) -> Self {
        Self {
            models,
            ..Self::default()
        }
    }
}

/// The shell's animation time at `age` for a shell that plays for `shell_secs`:
/// `AnimNode_UpdateTransformTree(shell, 1 - max(0.4 * timer, 0))` with `timer` the remaining
/// state 5 time (row Z8), so `0` at the blast and `1` at `shell_secs`.
pub(crate) fn shell_time(age: f32, shell_secs: f32) -> f32 {
    let remaining = (shell_secs - age).max(0.0);
    1.0 - (0.4 * remaining).max(0.0)
}

#[cfg(feature = "view")]
impl RaceView {
    /// Seconds since the player's craft left the race in a Zone end, `None` before.
    #[must_use]
    pub fn zone_end_age(&self) -> Option<f32> {
        self.zone_end.age
    }

    /// The shell's animation time while it plays, `None` before the blast and after it hides.
    #[must_use]
    pub fn zone_end_shell_time(&self, core: &SimRace) -> Option<f32> {
        let end = core.sim.zone_end()?;
        let age = self.zone_end.age?;
        (age < end.shell_secs).then(|| shell_time(age, end.shell_secs))
    }

    /// Whether the craft's hull, flare and shell are hidden and only the debris is left.
    #[must_use]
    pub fn zone_end_hides_craft(&self, core: &SimRace, slot: usize) -> bool {
        slot == core.player_slot()
            && core
                .sim
                .zone_end()
                .zip(self.zone_end.age)
                .is_some_and(|(end, age)| age >= end.shell_secs)
    }

    /// Whether the shell is drawn over the craft this frame.
    #[must_use]
    pub fn zone_end_draws_shell(&self, core: &SimRace) -> bool {
        self.zone_end_shell_time(core).is_some()
    }

    /// Whether the pieces are in the air.
    #[must_use]
    pub fn zone_end_thrown(&self) -> bool {
        self.zone_end
            .thrown
            .as_ref()
            .is_some_and(|thrown| !thrown.pieces.is_empty())
    }

    /// How many times the pieces have been thrown, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn zone_end_throws_for_tests(&self) -> u32 {
        self.zone_end.throws
    }

    /// Starts the clock on the craft's leaving the race, runs it, and throws the debris when the
    /// shell has played, once a tick.
    pub(super) fn advance_zone_end(&mut self, core: &SimRace) {
        let Some(end) = core.sim.zone_end() else {
            return;
        };
        let slot = core.player_slot();
        let dt = core.sim.dt;
        let out = core.sim.world.ships[slot].physics.craft_state != oag_physics::CraftState::Racing;
        match self.zone_end.age {
            None if out => self.zone_end.age = Some(0.0),
            Some(age) => self.zone_end.age = Some(age + dt),
            None => {}
        }
        let Some(age) = self.zone_end.age else {
            return;
        };
        if age >= end.shell_secs && self.zone_end.thrown.is_none() {
            self.throw_zone_debris(core, slot);
        }
        if let Some(thrown) = &mut self.zone_end.thrown {
            thrown.advance(dt);
        }
    }

    /// `DebrisManager_SpawnZoneDebris`: every piece leaves with the craft's own velocity and a
    /// random term, from the craft's matrix.
    fn throw_zone_debris(&mut self, core: &SimRace, slot: usize) {
        let Some(models) = self.zone_end.models.clone() else {
            // Nothing loaded to throw: the end still counts as thrown, so it is not retried.
            self.zone_end.thrown = Some(Thrown {
                pieces: Vec::new(),
                base: Vec::new(),
                up: Vec3::Y,
                floor: Vec3::ZERO,
            });
            return;
        };
        let ship = &core.sim.world.ships[slot].physics;
        let model = self.drawn_model(core, slot);
        let up = ship.body.up();
        let velocity = ship.body.linear_velocity;
        let origin = model.w_axis.truncate();
        let normal = model.inverse().transpose();
        let base: Vec<(Vec3, Vec3)> = models
            .debris
            .vertices
            .iter()
            .map(|v| {
                (
                    model.transform_point3(Vec3::from(v.position)),
                    normal
                        .transform_vector3(Vec3::from(v.normal))
                        .normalize_or(up),
                )
            })
            .collect();
        let rng = &mut self.stage_rng;
        let mut between = |(low, high): (f32, f32)| low + (high - low) * rng.next_f32();
        let pieces = models
            .pieces
            .iter()
            .map(|vertices| {
                let centre = vertices.iter().map(|&v| base[v as usize].0).sum::<Vec3>()
                    / vertices.len().max(1) as f32;
                let outward = (centre - origin).normalize_or(up);
                let axis = Vec3::new(
                    between((-1.0, 1.0)),
                    between((-1.0, 1.0)),
                    between((-1.0, 1.0)),
                )
                .normalize_or(Vec3::X);
                Piece {
                    centre,
                    offset: Vec3::ZERO,
                    velocity: velocity + outward * between(OUTWARD) + up * between(LIFT),
                    axis,
                    spin: between(SPIN),
                    angle: 0.0,
                }
            })
            .collect();
        self.zone_end.throws += 1;
        self.zone_end.thrown = Some(Thrown {
            pieces,
            base,
            up,
            floor: origin - up * 1.0,
        });
    }

    /// The debris model's vertices posed for this frame, or `None` before the throw.
    pub(crate) fn zone_end_debris(&self, out: &mut Vec<oag_mesh::mesh::GpuVertex>) -> bool {
        let (Some(models), Some(thrown)) = (&self.zone_end.models, &self.zone_end.thrown) else {
            return false;
        };
        if thrown.pieces.is_empty() {
            return false;
        }
        out.clear();
        out.extend(models.debris.vertices.iter().copied());
        for (piece, vertices) in thrown.pieces.iter().zip(&models.pieces) {
            let turn = Quat::from_axis_angle(piece.axis, piece.angle);
            for &index in vertices {
                let (position, normal) = thrown.base[index as usize];
                let vertex = &mut out[index as usize];
                vertex.position =
                    (piece.centre + piece.offset + turn * (position - piece.centre)).to_array();
                vertex.normal = (turn * normal).to_array();
            }
        }
        true
    }
}

impl Thrown {
    /// One tick: every piece flies, falls and comes to rest on the floor plane.
    fn advance(&mut self, dt: f32) {
        for piece in &mut self.pieces {
            let resting = piece.velocity == Vec3::ZERO;
            if resting {
                continue;
            }
            piece.velocity -= self.up * GRAVITY * dt;
            piece.offset += piece.velocity * dt;
            piece.angle += piece.spin * dt;
            let height = (piece.centre + piece.offset - self.floor).dot(self.up);
            if height < 0.0 {
                piece.offset -= self.up * height;
                piece.velocity = Vec3::ZERO;
                piece.spin = 0.0;
            }
        }
    }
}
