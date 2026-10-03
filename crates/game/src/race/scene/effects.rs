//! The race pass's last stretch: the depth-tested, depth-unwritten effects
//! that draw over everything solid. Pulled out of `frame.rs` under the
//! 1,000-line rule in `scripts/check-file-size.py` - a move, with no
//! behaviour change. The order is `frame.rs`'s and is load-bearing; see the
//! comment at its call.

use super::*;

impl Scene {
    pub(super) fn draw_effects(
        &self,
        race: &Race,
        eye: oag_render::mesh::LodEye,
        pass: &mut wgpu::RenderPass<'_>,
        stats: &mut SceneStats,
    ) {
        oag_render::perfprobe::marks::mark(pass, "exhaust, sparks, clouds, ghost");
        self.exhaust.borrow().draw(pass);
        self.sparks.borrow().draw(pass);
        self.draw_beam(pass);
        self.clouds.borrow().draw(pass);
        self.weapon_quads.pipeline.borrow().draw(pass);
        self.draw_ghost(race, eye, pass, stats);
        self.sparks.borrow().draw_flash(pass);
        oag_render::perfprobe::marks::mark(pass, "end");
    }
}
