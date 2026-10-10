//! The craft's wreck: `shipwreck.vex` drawn **instead of** the hull while the
//! craft is out of the race.
//!
//! `Ship_SetState`'s case 5 points the entity's live model at the wreck through
//! `Ship_SelectWreckModel` (`0x0883eb68`) and the model's visible bit moves with
//! it; cases 0 to 3 and the respawn put the hull back
//! (`docs/ghidra/functions/psp-pulse-usa/ship-wreck-model.md`). Case 5 is
//! `oag_physics::CraftState::Eliminated` here, so the hull stays through
//! `Destroyed` (state 4, the explosion) and is replaced from the tick the craft
//! reaches `Eliminated` until it is put back `Racing`. Seen on a running
//! original: the hull is drawn in state 4, a dark scorched model in states 5
//! and 6 (`docs/ghidra/functions/psp-pulse-usa/ship-wreck-model.md`, "Measured on a
//! running original").

use oag_physics::CraftState;

use super::per_slot::Build;
use super::*;

/// Which of a craft's two models is the live one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Live {
    Hull,
    Wreck,
}

/// The model a craft in `state` draws, for a craft that has a wreck.
///
/// State 4 keeps the hull (`Ship_SetState` case 4 never calls
/// `Ship_SelectWreckModel`); case 5 is the one caller that does.
pub(super) fn live_model(state: CraftState) -> Live {
    match state {
        CraftState::Racing | CraftState::Destroyed => Live::Hull,
        CraftState::Eliminated => Live::Wreck,
    }
}

/// Every slot's wreck, `None` for a slot whose source ships none.
#[derive(Debug)]
pub(super) struct Wrecks {
    hull: Vec<Option<Drawable>>,
}

impl Wrecks {
    /// Builds the wreck drawables from each slot's livery: the hull's own
    /// pipeline, depth and glow mask (the wreck is a `.vex` mesh of the same
    /// class).
    pub(super) fn build(
        slots: &Build<'_>,
        depth: mesh_render::Depth,
        zone_art: &mesh_render::zone::StageArt,
    ) -> Result<Self> {
        let mut hull = Vec::with_capacity(GRID_SLOTS as usize);
        for slot in 0..GRID_SLOTS as usize {
            let livery = &slots.liveries[slot.min(slots.liveries.len().saturating_sub(1))];
            let model = livery
                .wreck
                .as_ref()
                .map(|wreck| wreck.model.clone())
                .filter(|model| !model.indices.is_empty());
            hull.push(match model {
                Some(model) => Some(Drawable::new(
                    slots.device,
                    slots.queue,
                    model,
                    slots.format,
                    slots.anisotropy,
                    slots.sample_count,
                    depth,
                    mesh_render::TRANSPARENT_BLEND,
                    mesh_render::GlowMask::Protected,
                    zone_art,
                    slots.shadow_maps,
                    mesh_render::ShadowReceiver::Mapped,
                    mesh_render::Velocity::Write,
                )?),
                None => None,
            });
        }
        Ok(Self { hull })
    }
}

impl Scene {
    /// Whether this slot draws its wreck this frame: the craft is in the race,
    /// out of it, and has a wreck to show.
    pub(super) fn is_wrecked(&self, race: &Race, slot: usize) -> bool {
        self.wrecks.hull.get(slot).is_some_and(Option::is_some)
            && race.ship_active(slot)
            && live_model(race.sim.world.ships[slot].physics.craft_state) == Live::Wreck
    }

    /// Whether the hull is left undrawn for `slot`: the player's own in the
    /// cockpit view (`Race::draws_own_ship`), or any craft whose wreck is live.
    pub(super) fn hull_skipped(&self, race: &Race, slot: usize) -> bool {
        (slot == 0 && !race.draws_own_ship()) || self.is_wrecked(race, slot)
    }

    /// Writes every live wreck's pose, as the hull's own is written.
    pub(super) fn write_wrecks(
        &self,
        queue: &wgpu::Queue,
        race: &Race,
        view_projection: Mat4,
        prev_vp: Mat4,
        prev: &motion::Snapshot,
    ) {
        for slot in 0..usize::from(race.ship_count()) {
            if !self.is_wrecked(race, slot) {
                continue;
            }
            if let Some(wreck) = &self.wrecks.hull[slot] {
                wreck.write_hull(
                    queue,
                    view_projection,
                    race.ship_model_matrix_of(slot),
                    prev_vp * prev.ship(slot, race),
                    self.sun_occlusion_layer(slot),
                );
            }
        }
    }

    /// Writes the frame's scene uniform into every wreck, the same one the
    /// hulls get.
    pub(super) fn write_wreck_scenes(&self, queue: &wgpu::Queue, ship_scene: &mesh_render::Scene) {
        for wreck in self.wrecks.hull.iter().flatten() {
            queue.write_buffer(&wreck.fog, 0, bytemuck::bytes_of(ship_scene));
        }
    }

    /// Chooses the `LodGroup` child of every live wreck.
    pub(super) fn select_wreck_lod(&self, race: &Race, eye: oag_mesh::mesh::LodEye) {
        for slot in 0..usize::from(race.ship_count()) {
            if self.is_wrecked(race, slot)
                && let Some(wreck) = &self.wrecks.hull[slot]
            {
                wreck.select_lod(race.ship_model_matrix_of(slot), eye);
            }
        }
    }

    /// Draws every live wreck, where the hulls are drawn.
    pub(super) fn draw_wrecks(
        &self,
        race: &Race,
        pass: &mut wgpu::RenderPass<'_>,
        stats: &mut SceneStats,
    ) {
        for slot in 0..usize::from(race.ship_count()) {
            if !self.is_wrecked(race, slot) || (slot == 0 && !race.draws_own_ship()) {
                continue;
            }
            if let Some(wreck) = &self.wrecks.hull[slot] {
                stats.add(wreck.draw(pass, None, None, None, None));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hull is the live model until the craft is `Eliminated`, which is
    /// `Ship_SetState`'s case 5 - the one caller of `Ship_SelectWreckModel`.
    #[test]
    fn the_wreck_is_live_from_state_5_and_not_before() {
        assert_eq!(live_model(CraftState::Racing), Live::Hull);
        assert_eq!(live_model(CraftState::Destroyed), Live::Hull);
        assert_eq!(live_model(CraftState::Eliminated), Live::Wreck);
    }
}
