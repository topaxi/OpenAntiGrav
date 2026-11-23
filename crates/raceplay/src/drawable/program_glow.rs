//! A drawable's glow-table entries that carry a per-instance program, rewritten per frame.
//! See [`Drawable::write_program_glow`].

use super::*;

impl Drawable {
    /// Whether this drawable's model carries an entry of a per-instance glow-table program (an
    /// [`oag_mesh::mesh::Emissive::program`] mode whose tint and offset the engine writes per object). A pool of such a model gives every slot a
    /// glow table of its own ([`Self::instances`]), because each instance has its own values.
    pub(super) fn carries_per_instance_program(model: &Model) -> bool {
        model.emissive.iter().any(|entry| {
            [
                oag_mesh::mesh::PROGRAM_MINE_RIM,
                oag_mesh::mesh::PROGRAM_BOMB_TINT,
            ]
            .contains(&entry.program[0])
        })
    }

    /// Writes `tint` and `offset` into every entry of `program`, in this drawable's own glow
    /// table: what the original's per-object `float4` binds (`MineColour`, `MineBrightness`) are
    /// here. The table's other lanes (scale, rate, program) are the model's own and are not
    /// touched.
    pub(crate) fn write_program_glow(
        &self,
        queue: &dyn oag_gpu::writes::Writes,
        program: f32,
        tint: [f32; 3],
        offset: f32,
    ) {
        let table = mesh_render::Emissives::of(&self.model);
        let row = std::mem::size_of::<[f32; 4]>() as u64;
        for (index, entry) in self.model.emissive.iter().enumerate() {
            if entry.program[0] != program || index + 1 >= table.tint_offset.len() {
                continue;
            }
            // Only the entry's `tint_offset` row: the table's `scale` rows (the program lane
            // among them) are the model's own and stay as uploaded.
            queue.write_buffer(
                &self.emissive,
                self.offsets.emissive + (index as u64 + 1) * row,
                bytemuck::bytes_of(&[tint[0], tint[1], tint[2], offset]),
            );
        }
    }
}
