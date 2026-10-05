//! Wipeout HD's absorb shell drawables: one per craft, the team's own
//! `AbsorbEffect` model on the hull's matrix, faded by the absorb timer.
//!
//! The fade is `oag_fx::absorb_shell`'s and the program is `mesh.wgsl`'s
//! `absorb_shading` path; this is only where the per-frame half is written and
//! drawn, kept out of `frame.rs` under the 1,000-line rule.
//!
//! **The hull's own matrix and nothing on top.** `ShipAbsorbShell_Load` links
//! the node under the craft and `ShipAbsorbShell_Show` gives it an identity
//! local transform, so it sits exactly where the hull does.
//!
//! **Not the player's while the camera is inside the hull.** There the
//! original shows the cockpit shell instead and hides the hull nodes; that
//! shell is not drawn here - see `oag_fx::absorb_shell`.

use super::*;

impl Scene {
    /// The slots whose shell draws this frame, with its fade.
    fn absorb_shells<'a>(
        &'a self,
        race: &'a Race,
    ) -> impl Iterator<Item = (usize, &'a Drawable, f32)> + 'a {
        let drawn = usize::from(race.ship_count());
        self.absorb_shell
            .iter()
            .enumerate()
            .take(drawn)
            .filter_map(move |(slot, shell)| {
                let shell = shell.as_ref()?;
                if !race.ship_active(slot) || (slot == 0 && !race.draws_own_ship()) {
                    return None;
                }
                Some((slot, shell, race.absorb_shell_fader(slot)?))
            })
    }

    /// Writes every live shell's pose and its fade, which the program
    /// multiplies into the vertex alpha - `(1, 1, 1, fade)` through
    /// `Drawable::tint`, from the model's own authored ramp every frame.
    pub(super) fn write_absorb_shells(
        &self,
        race: &Race,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        prev_vp: Mat4,
        prev: &motion::Snapshot,
        scratch: &mut Vec<mesh::GpuVertex>,
    ) {
        for (slot, shell, fade) in self.absorb_shells(race) {
            shell.write(
                queue,
                view_projection,
                race.ship_model_matrix_of(slot),
                prev_vp * prev.ship(slot, race),
            );
            shell.tint(queue, [1.0, 1.0, 1.0, fade], scratch);
        }
    }

    /// Draws every live shell, after the hulls so it tests against their
    /// depth, on the material's own additive blend.
    pub(super) fn draw_absorb_shells(
        &self,
        race: &Race,
        pass: &mut wgpu::RenderPass<'_>,
        stats: &mut SceneStats,
    ) {
        for (_, shell, _) in self.absorb_shells(race) {
            stats.add(shell.draw(pass, None, None, None, None));
        }
    }
}
