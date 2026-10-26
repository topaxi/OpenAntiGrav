//! Where each craft's mesh goes: the model matrix per slot, and how many slots
//! are flying.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// How many ticks behind a craft's body the environment-probe blend position sits.
/// Measured on 52 pauses of a drive round Talon's Junction, where one tick back fits
/// best for 41 of them (`crates/vex/tests/probes_ground_truth.rs`); the rest fit zero
/// or two.
const PROBE_LAG_TICKS: f32 = 1.0;

impl Race {
    /// Where the ship is and how it is oriented, as the mesh shader's model matrix.
    ///
    /// Rotation, translation and the craft's recovered global scale. The scale is
    /// **uniform**, so the shader's assumption - which is what lets it rotate
    /// normals without an inverse transpose - still holds. The model's own origin
    /// is used as it is: no recentring, and the only rotation composed in is
    /// [`MODEL_YAW`], which is a stated finding about the `.vex` and not a nudge
    /// to make the picture look right.
    ///
    /// # The `0.75` is recovered, confirmed three ways, and shared with physics
    ///
    /// `g_craft_scale` (`0x08ab0e1c`) is a code literal `0.75` written once in
    /// `Craft_Construct_q` and read by seventeen sites. Three of them establish
    /// that it belongs on the **render** matrix and not only in the simulation:
    ///
    /// 1. **The craft's world matrix rows carry it**, live-measured: the
    ///    per-sample trail direction `Exhaust_Update` stores is `200000.0` times
    ///    the craft's row 2, and it reads back `150,080` -
    ///    `200000 * 0.75 = 150,000`. See
    ///    `oag_fx::exhaust::CRAFT_ROW_SCALE`.
    /// 2. **The drop-shadow divides it back out.** `FUN_089038c8` - a stencil
    ///    shadow-volume pass - computes its ground projection and then scales it
    ///    by `1.0 / g_craft_scale` before `Gu_SetMatrix(2, ...)` installs the
    ///    node's own 4x4 as the GE world matrix. Undoing the scale is only
    ///    correct if the matrix it draws under has it.
    /// 3. **Physics already applies it**, as
    ///    `oag_physics::hover::TARGET_GLOBAL_SCALE` (confidence 92), and
    ///    `Ship_HoverFourCorner`, `Ship_InitCraft`, `Ship_UpdateCraft`,
    ///    `Ship_UpdateCameraRigs` and the `<Misc>` hull dimensions into the
    ///    collider all read the same global. A simulation scaled by `0.75` and a
    ///    mesh drawn at `1.0` is the one combination that is certainly wrong.
    ///
    /// The rigid body's own basis rows are orthonormal - the capture harness
    /// measures them unit-length over 200 ticks - so the scale is not already
    /// arriving through `body.orientation`, and this is where it belongs.
    ///
    /// # There is no residual: the craft is the right size, and the old gap was the projection
    ///
    /// This comment used to record a further `~1.3x` gap and name
    /// `1/0.75^2 = 1.7778` as the live hypothesis for it. **That is refuted, and
    /// the constant here must not be widened to swallow anything.**
    ///
    /// Measured 2026-08-09 by similarity registration on high-passed gradient
    /// magnitude - no brightness threshold anywhere in it - against 16 emulator
    /// frames: **craft-only boxes register at `0.9989`/`0.9985`/`0.9987` once the
    /// background is zeroed, so the mesh is correct to 0.15 %.** The old figure
    /// came from a lamp-centroid metric that is not linear in the scale it
    /// measures (`1.0` -> `1.740`, `0.75` -> `1.191`, `0.5625` -> `1.067`), and
    /// from one false clause: that measurement was recorded as taken "with the
    /// track and buildings behind aligning", and they do not align. Far scenery,
    /// which no mesh scale can move, registers at `1.1195` in the same frame.
    ///
    /// What is actually missing is in [`Race::projection`], not here: the
    /// original adds `0.075 * dot(forward, velocity)` **degrees** to both tripod
    /// fovs every frame, so its field widens with speed and ours does not. See
    /// `docs/rendering/projection-vs-the-original.md`.
    ///
    /// What ships here is the confirmed constant, applied once - which turns out
    /// to be the whole of it.
    #[must_use]
    pub fn ship_model_matrix(&self) -> Mat4 {
        self.ship_model_matrix_of(0)
    }

    /// One craft's model matrix, by ship slot.
    ///
    /// [`Self::ship_model_matrices`] gives the same matrices for drawing the
    /// field, but it *filters* inactive craft, so its indices are not slot
    /// indices. Anything that has to line a craft's slot up with something else
    /// held per slot - its plume, its flare - asks for the matrix by slot here.
    ///
    /// # Panics
    ///
    /// If `slot` is not a ship slot.
    #[must_use]
    pub fn ship_model_matrix_of(&self, slot: usize) -> Mat4 {
        self.ship_between(slot, self.drawn_model(slot))
    }

    /// How many craft are in play, the player included.
    #[must_use]
    pub fn ship_count(&self) -> u8 {
        self.sim.world.ship_count
    }

    /// Whether the craft in this slot is still racing.
    ///
    /// The slot-indexed counterpart to [`Self::ship_model_matrix_of`], for the
    /// same reason: a caller walking `0..ship_count()` needs to skip an
    /// inactive slot without losing slot alignment, which
    /// [`Self::ship_model_matrices`]' filter-then-collect cannot give it. Every
    /// per-craft loop in `Scene::render` that writes or draws by slot should
    /// test this before it does either - see the four sites this joins.
    ///
    /// Currently a no-op everywhere it is called: nothing deactivates a slot
    /// below [`Self::ship_count`] mid-race today, elimination being the
    /// obvious future candidate. Written ahead of that landing so the loops
    /// already agree with `oag_gameplay::World`'s own `active` flag instead of
    /// assuming every slot in range is racing.
    ///
    /// # Panics
    ///
    /// If `slot` is not a ship slot.
    #[must_use]
    pub fn ship_active(&self, slot: usize) -> bool {
        self.sim.world.ships[slot].active
    }

    /// The word a craft's hull-light scale is written to the model uniform as -
    /// `oag_mesh::mesh_render::pack_hull_scale` of the ambient and directional
    /// bytes the spline blends under the craft (`Craft_ApplyHullLightScale`,
    /// `docs/ghidra/functions/psp-pulse-usa/scene-light.md`), or `0.0` (the
    /// default `255`) for a craft the course has not located yet or a circuit
    /// with no course.
    ///
    /// # Panics
    ///
    /// If `slot` is not a ship slot.
    #[must_use]
    pub fn ship_hull_scale(&self, slot: usize) -> f32 {
        self.hull_light_scale(slot)
            .map_or(0.0, |[ambient, directional, ..]| {
                oag_mesh::mesh_render::pack_hull_scale(ambient, directional)
            })
    }

    /// The four light-scale bytes under a craft: the spline's blend at the
    /// craft's own place, [`oag_race::Course::light_scale_at`], or `None` for a
    /// craft the course has not located or a circuit with no course.
    fn hull_light_scale(&self, slot: usize) -> Option<[u8; 4]> {
        let at = self.ship_model_matrix_of(slot).w_axis;
        self.sim
            .course
            .as_ref()
            .zip(self.sim.world.ships[slot].standing.course_index)
            .and_then(|(course, index)| {
                course.light_scale_at(index as usize, oag_core::math::Vec3::new(at.x, at.y, at.z))
            })
    }

    /// The point and spot lights Pulse's GE list gives a craft's hull in the slots its
    /// directionals leave, for the craft where it is drawn: two unit-vector-plus-packed-colour
    /// slots, all zero for none. `oag_mesh::mesh_render::HullPointLights::at`, with the third
    /// and fourth spline scale bytes the craft sits under (`255` where the course has not
    /// located it).
    ///
    /// `seconds` is the scenery clock the track's `Anim Transform` nodes are drawn at, so a
    /// light that sweeps with them (Arc Prime's floor spotlights) is where the picture shows it.
    ///
    /// # Panics
    ///
    /// If `slot` is not a ship slot.
    #[must_use]
    pub fn ship_hull_points(
        &self,
        slot: usize,
        seconds: f32,
    ) -> [oag_mesh::mesh_render::HullPoint; oag_mesh::mesh_render::HULL_POINT_SLOTS] {
        if self.view.hull_points.is_empty() {
            return Default::default();
        }
        let at = self.ship_model_matrix_of(slot).w_axis;
        let [.., point, spot] = self.hull_light_scale(slot).unwrap_or([255; 4]);
        self.view
            .hull_points
            .at([at.x, at.y, at.z], seconds, [point, spot])
    }

    /// Where a craft's environment probes are blended: its body, one tick back.
    ///
    /// The original's SPU job takes the previous frame's position (one tick fits best
    /// on 41 of 52 measured pauses, [`PROBE_LAG_TICKS`]), so the blend
    /// trails the craft by about a tick's travel.
    ///
    /// # Panics
    ///
    /// If `slot` is not a ship slot.
    #[must_use]
    pub fn ship_probe_position(&self, slot: usize) -> [f32; 3] {
        let body = &self.sim.world.ships[slot].physics.body;
        let at = body.position - body.linear_velocity * (PROBE_LAG_TICKS / 60.0);
        at.to_array()
    }

    /// One model matrix per craft in play, the player's first.
    ///
    /// The player is index 0 and the seven opponents follow, which is the array
    /// order and **not** the grid order - the player sits on grid slot 8. See
    /// [`Race::start`].
    #[must_use]
    pub fn ship_model_matrices(&self) -> Vec<Mat4> {
        self.sim
            .world
            .ships
            .iter()
            .take(self.sim.world.ship_count as usize)
            .enumerate()
            .filter(|(_, ship)| ship.active)
            .map(|(slot, _)| self.ship_model_matrix_of(slot))
            .collect()
    }
}
