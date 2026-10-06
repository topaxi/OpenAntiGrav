//! What a craft shows when a weapon hit gets through: its own hull's damage
//! sparks, thrown from one or two of its `Ship Collision Fx` locators.
//!
//! # Recovered (Pulse, PSP)
//!
//! The weapon never spawns this. The struck craft does, from the one place
//! every weapon's damage is spent: `Ship_Damage` (`0x088439ac`), on its weapon
//! branch (`source == 2`), after the subtraction and after the destroyed test.
//!
//! ```c
//! if (FUN_0883e37c(e) && source == 2 && e->fx_count /* +0xca8 */ != 0) {
//!     leach = e->craft->kind /* +0x4c -> +0x138 */ == 7;
//!     i = Psys_RandIntRange(0, e->fx_count - 1);
//!     j = Psys_RandIntRange(0, e->fx_count - 1);
//!     ShipCollisionFx_Trigger(1.0, e->fx[i], leach, 1);
//!     if (i != j) ShipCollisionFx_Trigger(1.0, e->fx[j], leach, 1);
//! }
//! ```
//!
//! `kind 0, damaged 1` is `WO_SHIP_COLL_SPARK_DAMAGE` - the same four-emitter
//! tree a damaging wall contact throws, orange smoke puffs included - and
//! `kind 1` is `WO_SHIP_SPARK_DAMAGE_LEACHBEAM`. Severity is the trigger's
//! `intensity * 2.0 + 0.4` at intensity `1.0`: `2.4`, every time. Each locator
//! then refuses to fire again for `0.8` s (`ShipCollisionFx_Trigger`'s own
//! `instance + 100` gate).
//!
//! Measured on PPSSPP (2026-09-24): a Cannon-tagged hit posted into the
//! player's pending-damage channel stops at `ShipCollisionFx_Trigger` with
//! `ra = 0x08844050` (inside `Ship_Damage`), kind 0, damaged 1, intensity 1.0,
//! on one of the player's six locators, and the next `Psys_Spawn_q` is
//! `WO_SHIP_COLL_SPARK_DAMAGE`. A `kind 7` hit spawns the LeachBeam variant. And real
//! weapons in a live race reach the same call on AI craft: 29 stops at that
//! `ra` while the player fired Rockets into the grid across GO, 7 while it
//! fired the Cannon. The AI were firing too, and the weapon behind each stop
//! was not recorded, so none is attributed to one weapon. See
//! `docs/ghidra/functions/psp-pulse-usa/shield.md`.
//!
//! **The smoke is per hit, not a state.** The smoke puffs are the damage
//! file's own root emitter; a craft left at 8 energy for three seconds with no
//! hit draws nothing, and no ship-owned `Psys_Spawn_q` caller spawns anything
//! on a shield level.
//!
//! # Chosen, not measured
//!
//! - **The cooldown is this module's own, per locator.** The original's lives
//!   on the locator's `ShipCollisionFx` instance, so a wall contact on the same
//!   locator shares it. Wall sparks here are the player's alone and keep their
//!   own cooldown (`Race::sparks_cooldown`).
//! - **Every craft is drawn.** `FUN_0883e37c`, the gate's first test, is a
//!   display-mask check this project has not read.
//! - **The picks come from a view-side generator**, not the simulation's: the
//!   original draws them from the particle system's own `Psys_RandIntRange`.
//!
//! # Titles
//!
//! Pulse only here. HD's Cannon throws its own weapon spark from the weapon's side,
//! a different mechanism (`docs/ghidra/functions/ps3-hdfury-eu/ship-collision-fx.md`),
//! and Pure's `Ship_Damage` is unread; both keep an empty anchor list and draw
//! nothing here.

use super::*;
use oag_livery::SparkAnchor;

/// The effect a landed weapon hit throws - the damaging wall contact's own.
pub const HIT_SPARK_EFFECT: &str = oag_fx::sparks::DAMAGE_EFFECT;

/// What a landed LeachBeam drain throws instead (`craft+0x138 == 7`).
pub const LEACHBEAM_HIT_SPARK_EFFECT: &str = "WO_SHIP_SPARK_DAMAGE_LEACHBEAM";

/// The severity every hit spark plays at: `ShipCollisionFx_Trigger`'s
/// `intensity * 2.0 + 0.4` at the hardcoded intensity `1.0`.
pub const HIT_SPARK_SEVERITY: f32 = oag_fx::sparks::SEVERITY_SLOPE + oag_fx::sparks::SEVERITY_FLOOR;

/// The loop bound `Ship_GatherCollisionFxNodes` collects up to.
const MAX_NODES: usize = 10;

/// Seed for the locator picks. View-side, like every other effect seed.
pub const HIT_SPARKS_SEED: u64 = 0x5_9a_2b_03;

/// Each slot's `Ship Collision Fx` locators for the hit sparks, or none at all
/// on a title whose weapon-hit path is not the one read here.
pub(super) fn anchors(
    title: &oag_title::Title,
    liveries: &[oag_livery::Livery],
) -> Vec<Vec<SparkAnchor>> {
    if title.name != oag_pulse::TITLE.name {
        return Vec::new();
    }
    liveries
        .iter()
        .map(|livery| {
            livery
                .collision_fx
                .iter()
                .take(MAX_NODES)
                .copied()
                .collect()
        })
        .collect()
}

/// Each slot's locators for HD's Cannon craft-hit spark, or none on a title
/// that does not throw one. Same ten-slot cap as [`anchors`]: HD's
/// `Ship_DispatchCollisionFx` reads ten pointers at `craft + 0x79d0`.
pub(super) fn weapon_anchors(
    title: &oag_title::Title,
    liveries: &[oag_livery::Livery],
) -> Vec<Vec<SparkAnchor>> {
    if title.name != oag_hd::TITLE.name {
        return Vec::new();
    }
    liveries
        .iter()
        .map(|livery| {
            livery
                .collision_fx
                .iter()
                .take(MAX_NODES)
                .copied()
                .collect()
        })
        .collect()
}

/// The world position of the locator nearest `contact`, by squared distance -
/// `Ship_DispatchCollisionFx`'s pick (`FUN_002d91e8` is the squared distance).
pub(super) fn nearest_locator(anchors: &[SparkAnchor], model: Mat4, contact: Vec3) -> Option<Vec3> {
    anchors
        .iter()
        .map(|anchor| model.transform_point3(anchor.position))
        .min_by(|a, b| {
            a.distance_squared(contact)
                .total_cmp(&b.distance_squared(contact))
        })
}

/// `Camera_ArmShake`'s magnitude argument on a weapon hit, as `Ship_Damage`
/// passes it (`0x3f19999a`).
const WEAPON_HIT_SHAKE_MAGNITUDE: f32 = 0.6;

/// One hit spark riding its locator.
#[derive(Debug, Clone, Copy)]
struct Riding {
    slot: usize,
    node: usize,
    playing: psys::Playing,
}

/// The hit sparks' own state: per-locator cooldowns, the instances riding the
/// hull, and the picks' generator.
#[derive(Debug, Clone)]
pub(super) struct HitSparks {
    /// Model-space locators per slot - see [`anchors`].
    anchors: Vec<Vec<SparkAnchor>>,
    /// HD's Cannon-hit locators per slot - see [`weapon_anchors`]. Empty on
    /// every other title.
    weapon_anchors: Vec<Vec<SparkAnchor>>,
    /// Seconds before each slot's locator may fire again.
    cooldown: [[f32; MAX_NODES]; MAX_SHIPS],
    riding: Vec<Riding>,
    rng: Rng,
    /// How many have started, ever - for tests.
    started: u32,
    /// How many Cannon-hit bursts have started, ever - for tests.
    weapon_started: u32,
}

impl HitSparks {
    pub(super) fn new(anchors: Vec<Vec<SparkAnchor>>) -> Self {
        Self {
            anchors,
            weapon_anchors: Vec::new(),
            cooldown: [[0.0; MAX_NODES]; MAX_SHIPS],
            riding: Vec::new(),
            rng: Rng::new(HIT_SPARKS_SEED),
            started: 0,
            weapon_started: 0,
        }
    }

    /// Adds HD's Cannon-hit locators.
    pub(super) fn with_weapon_anchors(mut self, anchors: Vec<Vec<SparkAnchor>>) -> Self {
        self.weapon_anchors = anchors;
        self
    }

    /// How many sparks are still riding their locator.
    #[cfg(test)]
    pub(super) fn riding_len(&self) -> usize {
        self.riding.len()
    }
}

impl Race {
    /// `Ship_Damage`'s weapon branch, for every slot `hits` says a hit landed
    /// on this tick. `leach` selects the LeachBeam's variant.
    pub(super) fn throw_hit_sparks(
        &mut self,
        hits: &[oag_weapons::projectile::WeaponHit],
        leach: bool,
    ) {
        let player = self.sim.world.primary_slot();
        for (slot, hit) in hits.iter().enumerate() {
            if hit.landed {
                if slot == player {
                    self.arm_weapon_hit_shake();
                }
                self.throw_hit_spark(slot, leach);
            }
        }
    }

    /// `Ship_Damage`'s weapon branch, for the local player: `Camera_ArmShake(0.6,
    /// 0.6, camera, 1)` (`Ship_Damage` `0x088439ac`, the end of its
    /// `source == 2` block), beside the spark and gated on `+0x368 == 0` like
    /// it. Magnitude `0.6` is passed as authored, where a wall contact's is
    /// its clamped severity times `0.3` - hence the division; the duration is
    /// the same `0.6` s and the mode `1`, [`Side::Elsewhere`]. Read
    /// 2026-09-30, confidence 85. **The game-mode gate** (not mode 2 or 12) is
    /// not applied: neither mode is one this port races in.
    fn arm_weapon_hit_shake(&mut self) {
        use oag_render::camera::shake::{MAGNITUDE_SCALE, Side};
        self.view.shake.arm(
            WEAPON_HIT_SHAKE_MAGNITUDE / MAGNITUDE_SCALE,
            Side::Elsewhere,
            &mut self.view.shake_rng,
        );
    }

    /// HD's Cannon on a craft: `Cannon_ApplyCraftHit` (`0x0010f730`) calls
    /// `Ship_DispatchCollisionFx(craft, contact, 1)`, which picks the
    /// `Ship Collision Fx` locator nearest the contact (squared distance, ten
    /// at most) and has `ShipCollisionFx_Trigger` kind 1 spawn
    /// [`WEAPON_SPARK_EFFECT`] there as a one-shot.
    ///
    /// **Chosen, not measured:** the burst's severity is `1.0` and it takes
    /// the locator's own up axis. The original rolls the spawn matrix by two
    /// random angles (`U(-pi/2, pi/2)` and `U(-pi, pi)`, constants at TOC
    /// `0x008b3f58`) whose axes were not read, and applies no per-locator
    /// cooldown (Pulse's `instance + 100` gate has no counterpart in that
    /// branch). Nothing fires on a hull with no locators.
    pub(super) fn throw_weapon_spark(&mut self, slot: usize, contact: Vec3) {
        let Some(anchors) = self.view.hit_sparks.weapon_anchors.get(slot) else {
            return;
        };
        let ship = &self.sim.world.ships[slot];
        if anchors.is_empty() || !ship.active {
            return;
        }
        let Some(effect) = self.view.effects.get(WEAPON_SPARK_EFFECT).cloned() else {
            return;
        };
        let Some(at) = nearest_locator(anchors, model_matrix_of(ship), contact) else {
            return;
        };
        if self.view.stage.play(&effect, at, 1.0).is_some() {
            self.view.hit_sparks.weapon_started += 1;
        }
    }

    fn throw_hit_spark(&mut self, slot: usize, leach: bool) {
        let nodes = self.view.hit_sparks.anchors.get(slot).map_or(0, Vec::len);
        if nodes == 0 {
            return;
        }
        let name = if leach {
            LEACHBEAM_HIT_SPARK_EFFECT
        } else {
            HIT_SPARK_EFFECT
        };
        let Some(effect) = self.view.effects.get(name).cloned() else {
            return;
        };
        let rng = &mut self.view.hit_sparks.rng;
        let first = rng.below(nodes as u32) as usize;
        let second = rng.below(nodes as u32) as usize;
        let ship = &self.sim.world.ships[slot];
        let model = model_matrix_of(ship);
        for node in [Some(first), (second != first).then_some(second)]
            .into_iter()
            .flatten()
        {
            let sparks = &mut self.view.hit_sparks;
            if sparks.cooldown[slot][node] > 0.0 {
                continue;
            }
            let anchor = sparks.anchors[slot][node];
            let at = model.transform_point3(anchor.position);
            let up = model.transform_vector3(anchor.up).normalize_or(Vec3::Y);
            let Some(playing) = self.view.stage.play_riding(&effect, at, HIT_SPARK_SEVERITY) else {
                continue;
            };
            self.view.stage.orient(playing, up);
            let sparks = &mut self.view.hit_sparks;
            sparks.cooldown[slot][node] = oag_fx::sparks::COLLISION_COOLDOWN;
            sparks.riding.push(Riding {
                slot,
                node,
                playing,
            });
            sparks.started += 1;
        }
    }

    /// Counts the cooldowns down and keeps every emitting spark on its
    /// locator, letting go once it stops emitting. Before `Stage::advance`,
    /// like the absorb bursts.
    pub(super) fn advance_hit_sparks(&mut self) {
        let dt = self.sim.dt;
        for slot in &mut self.view.hit_sparks.cooldown {
            for left in slot {
                *left = (*left - dt).max(0.0);
            }
        }
        let mut riding = std::mem::take(&mut self.view.hit_sparks.riding);
        riding.retain(|spark| {
            let ship = &self.sim.world.ships[spark.slot];
            if !ship.active || !self.view.stage.is_emitting(spark.playing) {
                self.view.stage.detach(spark.playing);
                return false;
            }
            let anchor = self.view.hit_sparks.anchors[spark.slot][spark.node];
            let model = model_matrix_of(ship);
            self.view
                .stage
                .follow(spark.playing, model.transform_point3(anchor.position));
            self.view.stage.orient(
                spark.playing,
                model.transform_vector3(anchor.up).normalize_or(Vec3::Y),
            );
            true
        });
        self.view.hit_sparks.riding = riding;
    }

    /// How many Cannon-hit bursts have ever started, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn weapon_sparks_started_for_tests(&self) -> u32 {
        self.view.hit_sparks.weapon_started
    }

    /// How many hit sparks have ever started, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn hit_sparks_started_for_tests(&self) -> u32 {
        self.view.hit_sparks.started
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_severity_is_the_trigger_law_at_full_intensity() {
        assert!((HIT_SPARK_SEVERITY - 2.4).abs() < 1e-6);
    }
}
