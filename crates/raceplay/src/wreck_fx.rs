//! What a craft throws as it goes out of the race: an explosion and a spray of
//! death sparks at each of its wreck's `Ship Collision Fx` locators.
//!
//! # Recovered and captured (Pulse, PSP)
//!
//! `Ship_SetState`'s case 5 swaps the wreck in and calls `FUN_0883e064`
//! (`Ship_SpawnExplosionSmall`, `docs/ghidra/functions/psp-pulse-usa/screen-flash-callers.md`).
//! After the flash, the shake (the player's own) and `EXPLSMALL` it walks the ten
//! nodes `Ship_GatherCollisionFxNodes` collected off the **wreck**, which is now
//! the live model, and for each spawns `WO_SHIP_FXNODE_EXPLO` and then
//! `WO_SHIP_DEATH_SPARKS` through `Psys_Spawn_q`, parented to the node.
//!
//! **The big explosion.** `Ship_UpdateDestroyed` calls `FUN_088407b0`
//! (`Ship_SpawnExplosionBig`) as state 5 times out, 1.5 s later. It copies the
//! live model's world matrix (`FUN_089451dc`: `node+0x3c`, `+0x40`), subtracts
//! `DAT_08ab0de8` times the matrix's own second row from its translation
//! (read 4.0, statically and live), and calls `Psys_Spawn_q(world_root,
//! "WO_SHIP_EXPLOSION", 'EXPL', &matrix, 0, 0)`: flags `0` copy the matrix into
//! the instance and the parent is the particle world root `DAT_08ab2248`, so
//! the blast is placed in the world and does not ride the craft. Read live on
//! PPSSPP (Talon's Junction, Venom, the player's craft put into state 4): the
//! matrix's rows were 0.75 long (the craft model scale), its translation was
//! the craft's node position moved 2.9995 units along `-up`, i.e. `4.0 * 0.7499`.
//!
//! Read live on PPSSPP (2026-10-01, Talon's Junction, `Ship_SetState(entity, 4)`
//! called on a grid opponent, a Piranha): at the state 5 edge, 7 pairs of those
//! two names in a row, one pair per node, each parented to a distinct node;
//! Piranha's `shipwreck.vex` authors 7 `Ship Collision Fx` locators. 1.5 s later
//! `WO_SHIP_EXPLOSION` follows (state 6), which this port does not play: the
//! matrix `FUN_088407b0` places it with is unread.
//!
//! # What this port does
//!
//! The edge is `Destroyed` to `Eliminated` ([`super::craft_flash`] already keys
//! on it), and each effect rides its locator on the craft's model matrix, the
//! way the hit sparks ride theirs ([`super::hit_sparks`]) - the original
//! parents them to the node. **Chosen, not measured:** the severity is `1.0`
//! (`Psys_Spawn_q`'s trailing arguments are zero and no wreck call passes one) and
//! the effects are let go once their emitters stop. **Measured, 2026-10-01:** the
//! instance matrix has rows of `0.75` (the craft's model scale, read off every
//! one of the explosion's and the nodes' instances), which scales each root
//! emitter's spawn offset and velocity by `0.75` and not a particle's size - the
//! smoke ring, the fireball's spread and rise and the debris's rise all read
//! `0.75` of the unscaled law's (`psys::System::set_frame_scale`, `particle-system.md`).
//!
//! Pulse on a PSP disc only: no other title's wreck or `Ship_SetState` is read.

use super::*;
use oag_livery::SparkAnchor;

/// The blast at each wreck node.
pub const FXNODE_EXPLO_EFFECT: &str = "WO_SHIP_FXNODE_EXPLO";

/// The sparks beside it.
pub const DEATH_SPARKS_EFFECT: &str = "WO_SHIP_DEATH_SPARKS";

/// The big blast, 1.5 s after the state 5 edge.
pub const EXPLOSION_EFFECT: &str = "WO_SHIP_EXPLOSION";

/// How far below the craft's model origin the big blast is placed, in the
/// model's own rows: `FUN_088407b0` subtracts `DAT_08ab0de8` (`4.0`, read in
/// `.data` and live) times the world matrix's second row, which is `0.75`
/// long, so about three world units.
pub const EXPLOSION_DROP: f32 = 4.0;

/// The loop bound `FUN_0883e064` and `Ship_GatherCollisionFxNodes` share.
const MAX_NODES: usize = 10;

/// Each slot's wreck locators, empty for a slot with no wreck and on every
/// title whose wreck is not read.
pub(super) fn anchors(
    title: &oag_title::Title,
    liveries: &[oag_livery::Livery],
) -> Vec<Vec<SparkAnchor>> {
    if title.effect_on(oag_title::Trigger::Wreck).is_none() {
        return Vec::new();
    }
    liveries
        .iter()
        .map(|livery| {
            livery
                .wreck
                .as_ref()
                .map(|wreck| wreck.fx.iter().take(MAX_NODES).copied().collect())
                .unwrap_or_default()
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct Riding {
    slot: usize,
    node: usize,
    playing: psys::Playing,
}

/// The wreck effects' own state: the locators and the instances riding them.
#[derive(Debug, Clone, Default)]
pub(super) struct WreckFx {
    anchors: Vec<Vec<SparkAnchor>>,
    riding: Vec<Riding>,
    /// How many have started, ever - for tests.
    started: u32,
    /// Where the big blast last went, for tests.
    last_explosion_at: Option<Vec3>,
}

impl WreckFx {
    pub(super) fn new(anchors: Vec<Vec<SparkAnchor>>) -> Self {
        Self {
            anchors,
            riding: Vec::new(),
            started: 0,
            last_explosion_at: None,
        }
    }
}

impl Race {
    /// `FUN_0883e064`'s node loop for `slot`, on the tick it reaches state 5.
    pub(super) fn throw_wreck_fx(&mut self, slot: usize) {
        let nodes = self.view.wreck_fx.anchors.get(slot).map_or(0, Vec::len);
        let model = model_matrix_of(&self.sim.world.ships[slot]);
        for node in 0..nodes {
            // The explosion first and the sparks second, per node, as the
            // original's loop spawns them.
            for name in [FXNODE_EXPLO_EFFECT, DEATH_SPARKS_EFFECT] {
                let Some(effect) = self.view.effects.get(name).cloned() else {
                    continue;
                };
                let anchor = self.view.wreck_fx.anchors[slot][node];
                let at = model.transform_point3(anchor.position);
                let up = model.transform_vector3(anchor.up).normalize_or(Vec3::Y);
                let Some(playing) = self.view.stage.play_riding(&effect, at, 1.0) else {
                    continue;
                };
                self.view.stage.orient(playing, up);
                self.view
                    .stage
                    .set_frame_scale(playing, oag_fx::exhaust::CRAFT_ROW_SCALE);
                let fx = &mut self.view.wreck_fx;
                fx.riding.push(Riding {
                    slot,
                    node,
                    playing,
                });
                fx.started += 1;
            }
        }
    }

    /// `FUN_088407b0`'s `WO_SHIP_EXPLOSION`, when state 5 times out: placed in
    /// the world at the wreck's model matrix, `EXPLOSION_DROP` rows below its
    /// origin, and let go. `model` is the matrix the craft was wrecked with
    /// (a wreck lies still, so it is the one the original reads 1.5 s on; a
    /// craft put back on the track in between is not where the blast goes). A
    /// source with no wreck read (every title but Pulse) throws nothing.
    pub(super) fn throw_wreck_explosion(&mut self, slot: usize, model: Mat4) {
        if self.view.wreck_fx.anchors.get(slot).is_none() {
            return;
        }
        // The shockwave is built from the matrix before its translation is moved, and is
        // its own `.vex`, so it does not wait on the particle effect having loaded.
        self.spawn_ship_shockwave(
            model.w_axis.truncate(),
            model.transform_vector3(Vec3::Y).normalize_or(Vec3::Y),
        );
        let Some(effect) = self.view.effects.get(EXPLOSION_EFFECT).cloned() else {
            return;
        };
        let at = model.transform_point3(Vec3::new(0.0, -EXPLOSION_DROP, 0.0));
        let up = model.transform_vector3(Vec3::Y).normalize_or(Vec3::Y);
        // Chosen, not measured: scale `1.0`, as for the node effects.
        let Some(playing) = self.view.stage.play(&effect, at, 1.0) else {
            return;
        };
        self.view.stage.orient(playing, up);
        self.view
            .stage
            .set_frame_scale(playing, oag_fx::exhaust::CRAFT_ROW_SCALE);
        self.view.wreck_fx.started += 1;
        self.view.wreck_fx.last_explosion_at = Some(at);
    }

    /// Keeps every emitting wreck effect on its locator, letting go once it
    /// stops emitting. Before `Stage::advance`, like the hit sparks.
    pub(super) fn advance_wreck_fx(&mut self) {
        let mut riding = std::mem::take(&mut self.view.wreck_fx.riding);
        riding.retain(|fx| {
            let ship = &self.sim.world.ships[fx.slot];
            if !ship.active || !self.view.stage.is_emitting(fx.playing) {
                self.view.stage.detach(fx.playing);
                return false;
            }
            let anchor = self.view.wreck_fx.anchors[fx.slot][fx.node];
            let model = model_matrix_of(ship);
            self.view
                .stage
                .follow(fx.playing, model.transform_point3(anchor.position));
            self.view.stage.orient(
                fx.playing,
                model.transform_vector3(anchor.up).normalize_or(Vec3::Y),
            );
            true
        });
        self.view.wreck_fx.riding = riding;
    }

    /// How many are still riding a locator, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn wreck_fx_riding_for_tests(&self) -> usize {
        self.view.wreck_fx.riding.len()
    }

    /// Where the big blast last went, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn wreck_explosion_at_for_tests(&self) -> Option<Vec3> {
        self.view.wreck_fx.last_explosion_at
    }

    /// How many wreck effects have ever started, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn wreck_fx_started_for_tests(&self) -> u32 {
        self.view.wreck_fx.started
    }
}
