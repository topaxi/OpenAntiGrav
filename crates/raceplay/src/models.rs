//! Where each craft's mesh goes: the model matrix per slot, and how many slots
//! are flying.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

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
        model_matrix_of(&self.sim.world.ships[slot])
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
            .filter(|ship| ship.active)
            .map(model_matrix_of)
            .collect()
    }
}
