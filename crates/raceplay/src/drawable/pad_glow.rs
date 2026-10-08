//! HD's weapon-pad light-bar colour, per pad. See [`Drawable::glow_weapon_pads`].

use super::*;

/// One weapon pad's place in HD's colour cycle, per pad, plus the race clock
/// it was last advanced at.
#[derive(Default)]
pub(super) struct PadGlow {
    /// Keyframes into the cycle, one per pad node, wrapped over the table.
    pub(super) position: Vec<f32>,
    /// The `seconds` of the last advance, so the next one adds only the gap.
    pub(super) last: Option<f32>,
}

impl Drawable {
    /// Retints each HD weapon pad's light bars from that pad's own cycle - the
    /// colour `WeaponPad_UpdateRefreshTimer` hands the fragment program's
    /// inline constant, see [`oag_title::weapon_pad`].
    ///
    /// `ready[i]` pairs with this drawable's `i`-th
    /// [`mesh::Model::node_vertex_ranges`] entry, as
    /// [`Self::tint_weapon_pads`]' own does. A ready pad's position advances by
    /// the race clock's gap at `cycle.keys_per_second`; a cooling pad's does
    /// not move and it shows `cycle.cooling`, the original's two branches.
    ///
    /// **A pad's starting position is chosen, not measured**: the original
    /// seeds it from the pad object's heap address modulo six, which this
    /// project has no counterpart for, so pad `i` starts on keyframe
    /// `i % 6`. Only the cycle's shape and rate are the original's.
    ///
    /// The values pass through unclamped (the red channel reaches `2.0`), as
    /// the program's own constant does. A no-op unless every pad node carries
    /// its own glow entry, which `mesh::rcs::build_weapon_pads` gives it.
    pub(crate) fn glow_weapon_pads(
        &self,
        queue: &wgpu::Queue,
        seconds: f32,
        cycle: &oag_title::weapon_pad::Cycle,
        ready: &[bool],
    ) {
        let mut state = self.pad_glow.borrow_mut();
        let pads = self.model.node_vertex_ranges.len();
        if state.position.len() != pads {
            state.position = (0..pads)
                .map(|i| (i % cycle.keyframes.len().max(1)) as f32)
                .collect();
        }
        // The first call covers the clock from zero: every pad is ready from the
        // start of the race, so a capture taken after N ticks sits N ticks into
        // its cycle rather than on its starting key.
        let gap = state
            .last
            .map_or(seconds.max(0.0), |last| (seconds - last).clamp(0.0, 0.1));
        state.last = Some(seconds);
        let mut table = mesh_render::Emissives::of(&self.model);
        for (i, range) in self.model.node_vertex_ranges.iter().enumerate() {
            let Some(vertex) = self.model.vertices.get(range.start as usize) else {
                continue;
            };
            let entry = mesh::slots::material_index(vertex.slots) as usize;
            if entry == 0 || entry >= table.tint_offset.len() {
                continue;
            }
            let (position, colour) = cycle.step(
                state.position[i],
                ready.get(i).copied().unwrap_or(true),
                gap,
            );
            state.position[i] = position;
            table.tint_offset[entry][..3].copy_from_slice(&colour);
        }
        queue.write_buffer(&self.emissive, 0, bytemuck::bytes_of(&table));
    }
}
