//! A drawable's glow-table entries that carry a per-instance program, rewritten per frame.
//! See [`Drawable::write_program_glow`].

use super::*;

impl Drawable {
    /// Whether this drawable's model carries an entry of glow-table `program` (an
    /// [`oag_mesh::mesh::Emissive::program`] mode). A pool of such a model gives every slot a
    /// glow table of its own ([`Self::instances`]), because each instance has its own values.
    pub(super) fn carries_program(model: &Model, program: f32) -> bool {
        model.emissive.iter().any(|entry| entry.program[0] == program)
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
        let mut table = mesh_render::Emissives::of(&self.model);
        let mut any = false;
        for (index, entry) in self.model.emissive.iter().enumerate() {
            if entry.program[0] != program || index + 1 >= table.tint_offset.len() {
                continue;
            }
            table.tint_offset[index + 1] = [tint[0], tint[1], tint[2], offset];
            any = true;
        }
        if any {
            queue.write_buffer(
                &self.emissive,
                self.offsets.emissive,
                bytemuck::bytes_of(&table),
            );
        }
    }
}
