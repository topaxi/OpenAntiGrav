//! HD's attached collision spark on the craft a person does not fly.
//!
//! The original's contact ring walk asks for the spark on **every craft a local view can see
//! within 100 units of its eye**, opponents included, and never under a Shield
//! (`Ship_UpdateEntity`, `0x000ec378`..`0x000ec4a4`, distance squared under `10000`;
//! `docs/ghidra/functions/ps3-hdfury-eu/ship-collision-fx.md`). A seat's own craft throws it
//! from [`ContactFx`](super::contact_fx::ContactFx); this is every other craft, on the same
//! rule ([`attach_rule`]) and the same strength law ([`sparks::attached_strength`]), as one
//! riding [`psys::Stage`] instance per craft that follows its locator and is let go the first
//! tick nothing asks for it.
//!
//! Only where the title authors the spark looping: a burst title (Pulse, Pure) throws its
//! opponents' sparks from the hull locators instead ([`crate::stage::Stage::throw_contact_spark`]) and this
//! plays nothing.
//!
//! **Nothing here is simulation state.**
//!
//! # Concurrent instances
//!
//! At most one per opponent, so the field size less the seats, 11 on a twelve-craft grid. Each
//! takes a slot of the stage's 64 ([`psys::MAX_INSTANCES`]) through
//! [`psys::Stage::play_riding`], which is not held back by the slots reserved for bursts.

use super::*;
use crate::contact_fx::{Attach, PairContact, attach_rule, nearest_locator};

/// How close a view's eye must be, squared (`10000`, `0x000ec378`..`0x000ec4a4`).
pub(super) const REACH_SQUARED: f32 = 10_000.0;

/// One craft's spark while it is held.
#[derive(Debug, Clone, Copy)]
struct Held {
    playing: psys::Playing,
    /// The chosen locator, model space.
    anchor: Vec3,
    /// That locator's authored `+Y`, model space.
    up: Vec3,
}

/// The sparks the opponents hold, and the wall contact each stepped craft had this tick.
#[derive(Debug, Clone)]
pub(super) struct FieldSparks {
    held: [Option<Held>; MAX_SHIPS],
    /// Set by `step_opponents`, read by [`crate::RaceView::advance_field_sparks`] after the pairs resolve.
    pub(super) walls: [Option<oag_physics::wall::WallResponse>; MAX_SHIPS],
}

impl Default for FieldSparks {
    fn default() -> Self {
        Self {
            held: [None; MAX_SHIPS],
            walls: [None; MAX_SHIPS],
        }
    }
}

impl FieldSparks {
    /// The strength `slot`'s held spark plays at, `None` when it holds none.
    #[cfg(test)]
    pub(super) fn strength_of(&self, stage: &psys::Stage, slot: usize) -> Option<f32> {
        self.held[slot].and_then(|held| stage.scale(held.playing))
    }
}

#[cfg(feature = "view")]
impl RaceView {
    /// Every local view's eye: the player's and each extra seat's.
    pub(super) fn view_eyes(&self, core: &SimRace) -> Vec<Vec3> {
        let player = self.camera_frame(core).position;
        std::iter::once(player)
            .chain(self.viewpoint_eyes())
            .collect()
    }

    /// Starts, holds or lets go of the spark of every craft no person flies, from the wall
    /// contact `step_opponents` stashed and the pair contact of this tick.
    pub(super) fn advance_field_sparks(&mut self, core: &SimRace) {
        let effect = self.handles.get(Trigger::CollisionSpark).cloned();
        let attached_effect = effect
            .as_deref()
            .is_some_and(|effect| effect.roots().iter().any(|&r| effect.emitters[r].looping));
        let eyes = if attached_effect {
            self.view_eyes(core)
        } else {
            Vec::new()
        };
        for slot in 0..MAX_SHIPS {
            let ship = &core.sim.world.ships[slot];
            let stepped = slot < core.sim.world.ship_count as usize
                && ship.in_world()
                && !core.sim.world.person_flown(slot);
            let wall = self.field_sparks.walls[slot].filter(|_| stepped);
            let pair: Option<PairContact> = self.pair_contacts[slot]
                .or_else(|| {
                    self.field_sparks.walls[slot]
                        .as_ref()
                        .and_then(|wall| crate::floor_sparks::not_racing_record(ship, wall))
                })
                .filter(|_| stepped);
            let shielded = ship.physics.shield_pickup_timer > 0.0;
            let position = ship.physics.body.position;
            let seen = eyes.iter().any(|eye| {
                let d = (position - *eye).length_squared();
                d > 0.0 && d < REACH_SQUARED
            });
            // A record asks for the spark only from a craft a view sees and no Shield covers.
            let asked = attached_effect && seen && !shielded;
            let wall = wall.filter(|_| asked);
            let pair = pair.filter(|_| asked);
            let held = self.field_sparks.held[slot];
            let rule = attach_rule(
                held.is_some(),
                false,
                wall.is_some_and(|wall| wall.impact),
                wall.is_some_and(|wall| wall.resolved.is_some() || wall.floor_contacts > 0),
                pair.is_some(),
            );
            let model = self.ship_model_matrix_of(core, slot);
            match (rule, held) {
                (Attach::Release, Some(held)) => {
                    self.stage.detach(held.playing);
                    self.field_sparks.held[slot] = None;
                }
                (Attach::StartWall | Attach::StartPair, None) => {
                    let Some(effect) = effect.as_ref() else {
                        continue;
                    };
                    let (point, impulse) = match (rule, wall, pair) {
                        (Attach::StartWall, Some(wall), _) => {
                            let Some(contact) = wall.resolved else {
                                continue;
                            };
                            (
                                contact.point + contact.normal * contact.depth,
                                wall.impulse_largest,
                            )
                        }
                        (_, _, Some(pair)) => (pair.point, pair.impulse),
                        _ => continue,
                    };
                    let (anchor, up) = nearest_locator(&self.collision_fx, model, point);
                    let velocity = ship.physics.body.linear_velocity;
                    let strength = sparks::attached_strength(velocity.length(), impulse);
                    let at = model.transform_point3(anchor);
                    if let Some(playing) = self.stage.play_riding(effect, at, 1.0) {
                        self.stage.set_count_scale(playing, strength);
                        self.stage.carry(playing, effect, velocity);
                        self.stage.orient(playing, model.transform_vector3(up));
                        self.field_sparks.held[slot] = Some(Held {
                            playing,
                            anchor,
                            up,
                        });
                    }
                }
                (_, Some(held)) => {
                    // A craft taken out of the world lets go of what it carried.
                    if stepped {
                        self.stage
                            .follow(held.playing, model.transform_point3(held.anchor));
                        self.stage
                            .orient(held.playing, model.transform_vector3(held.up));
                    } else {
                        self.stage.detach(held.playing);
                        self.field_sparks.held[slot] = None;
                    }
                }
                _ => {}
            }
        }
    }
}
