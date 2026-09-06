//! The chase camera's authored parameters: the sign convention the disc's own
//! offset carries, and one `<ExternalCamera*>` block as [`ChaseParams`].
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/camera.rs`.

use super::*;

/// Converts the disc's `pos_length` into the one
/// [`ChaseParams::pos_length`] documents.
///
/// The two conventions differ by a sign, and this is the only place that is
/// reconciled.
///
/// `oag_render::camera::chase` documents `pos_length` as a **positive distance
/// behind** and computes the eye as `position - forward * pos_length`. The disc
/// stores a **signed offset along forward**: every `<ExternalCameraFar>` and
/// `<ExternalCameraClose>` block observed has it negative, and the *close* camera's
/// value is the smaller magnitude of the two, which is what "closer behind the
/// ship" looks like under that reading and is not consistent with any other.
///
/// Passing the file's value through unchanged puts the eye that far **in front** of
/// the ship, aimed at a point `lookat_length` further ahead still, so the ship is
/// behind the camera and nothing of it is drawn. `chase.rs` records that exact
/// mistake as "obvious on screen, so this one is cheap to check once there is a ship
/// to look at" - this is that check, and the negation is its answer.
///
/// Confidence **80**: the geometry admits no other reading and both external blocks
/// of every team agree, but nothing has been compared against the original running,
/// and the fix arguably belongs in `oag-render`'s documented convention rather than
/// here.
#[must_use]
pub(super) fn chase_pos_length(from_disc: f32) -> f32 {
    -from_disc
}

/// One `<ExternalCamera*>` block as the renderer's [`ChaseParams`].
///
/// Both external views go through this, so [`chase_pos_length`] is applied to
/// exactly one of them never and to both of them always: a second transcription
/// of the seven fields is how the close view would end up with the far view's
/// sign convention.
///
/// # The rig carries the craft's global `0.75` scale, and does not pre-multiply it
///
/// `oag_physics::hover::TARGET_GLOBAL_SCALE` is a **world scale on craft-space
/// geometry**, not a suspension tuning knob, and the external camera rigs are one
/// of the places the original applies it. A camera reading a scale out of the
/// physics crate looks odd, which is why it is spelled out here:
///
/// - It is one global, `DAT_08ab0e1c`, set to a literal `0.75` in the craft
///   constructor (`FUN_08840c74`, the store at `0x08841000`). The **same** global
///   is read by `Ship_HoverFourCorner`'s four probe corners, by the `<Misc>` hull
///   dimensions on their way into the collider, by the initial body position, and
///   by both camera rigs in `Ship_UpdateCameraRigs` - which end with
///   `eye = shipPos + (eye - shipPos) * 0.75` and the same line for the look-at
///   point. Confidence **85**.
/// - `fov` and the two springs are **not** scaled: they are not lengths.
///
/// **It goes in [`ChaseParams::craft_scale`] rather than into the four offsets,
/// and that is a correction, not a preference.** Scaling the eye-and-look-at pair
/// about the ship and scaling the four offsets are the same thing for the aim
/// point, which is rigid, and are **not** the same thing for the eye, which is
/// sprung: the original scales *after* writing its spring state back, so the
/// state it keeps is unscaled, while a pre-multiplied offset set keeps a scaled
/// one - and the ship those two are measured from has moved in between. Over the
/// 150 ticks of `data/traces/pad0-boost.csv` that is `0.121` RMS against `0.008`.
/// See `crates/game/tests/chase_camera_ground_truth.rs`.
///
/// Three independent numbers agree that the scale belongs here at all. The
/// authored close block times `0.75` is `11.643` units from the craft; a static
/// probe of the original in that view measured `11.643`; and a 150-tick capture
/// of the original measured `11.571`-`11.674` across 40-154 units/s. The far
/// block lands on `14.562` against a measured `14.562`. Before this scale existed
/// here the eye sat `15.5` and `19.4` units out, which is the framing difference
/// `docs/ghidra/functions/psp-pulse-usa/camera.md` recorded as an unexplained
/// factor of three quarters and this closes.
///
/// **`<InternalCamera>` is deliberately *not* scaled** - see
/// `oag_render::camera::internal`. The original's internal rig reads this global
/// nowhere, and the cockpit eye was measured at `+3.000` against an authored
/// `length` of `3`. Counterintuitive next to the two external blocks, and
/// measured twice, so it stays.
///
/// `pub` so that `crates/game/tests/chase_camera_ground_truth.rs` compares the
/// **shipped** conversion against the original's own camera rather than a second
/// transcription of it - which would pass while the game rendered something
/// else.
#[must_use]
pub fn chase_params(block: handling::ExternalCamera) -> ChaseParams {
    ChaseParams {
        fov: block.fov,
        lookat_height: block.lookat_height,
        lookat_length: block.lookat_length,
        pos_height: block.pos_height,
        pos_length: chase_pos_length(block.pos_length),
        spring_horiz: block.spring_horiz,
        spring_vert: block.spring_vert,
        craft_scale: oag_physics::hover::TARGET_GLOBAL_SCALE,
    }
}

/// The chase camera's view of a ship.
///
/// Three vectors rather than a ship, because `oag-render` may not see a gameplay
/// type; that is rule 1 of `docs/architecture/workspace-layout.md`.
pub(super) fn target_of(ship: &Ship) -> Target {
    let body = &ship.physics.body;
    Target {
        position: body.position,
        forward: body.forward(),
        up: body.up(),
    }
}

impl Race {
    /// Where the camera is and what it is aimed at, as a view matrix.
    ///
    /// This is the one place a [`CameraOverride`] takes effect: everything else
    /// that needs the camera - the PVS eye, the fog eye - derives from this
    /// matrix through [`Self::camera_position`], so overriding here overrides
    /// everywhere at once, and a consumer that read the chase camera directly
    /// instead would silently miss the override.
    ///
    /// [`Self::shake`] applies to both the external and the internal view and
    /// not to an override: `Camera_ArmShake`'s `camera` argument is the
    /// per-craft camera object both tripods share, not a property of one view,
    /// but an override exists to reproduce a captured pose exactly and the
    /// shake's phase is randomised, so a matched-pose comparison must not see
    /// it. See `oag_render::camera::shake`.
    #[must_use]
    pub fn view(&self) -> Mat4 {
        if let Some(over) = &self.camera_override {
            // A view matrix is the inverse of the camera's world transform.
            return Mat4::from_rotation_translation(over.orientation, over.eye).inverse();
        }
        let target = target_of(self.ship());
        let base = if self.view == crate::display::CameraView::Internal {
            // Rigid, so there is no per-tick state to advance and nothing to
            // snap: the cockpit is bolted to the hull. The roll phase rides
            // along so the cockpit view rolls with the manoeuvre too - see
            // `oag_render::camera::internal::view`.
            oag_render::camera::internal::view(
                target,
                &self.internal_params,
                self.ship().physics.roll_phase,
            )
        } else {
            self.camera.view(target, &self.chase_params)
        };
        if self.shake.active() {
            // A view matrix is the inverse of the camera's world transform, so
            // rotating the camera's own basis by `Q` post-multiplies `Q` into
            // that transform and pre-multiplies `Q`'s inverse into its
            // inverse - see `oag_render::camera::shake`'s module documentation
            // for what the rotation itself reproduces.
            Mat4::from_quat(self.shake.rotation().inverse()) * base
        } else {
            base
        }
    }

    /// The camera's own world position, from the view matrix it produces.
    ///
    /// A view matrix is the inverse of the camera's world transform, so
    /// inverting it back and taking the translation column is the camera's
    /// position. Read out rather than tracked separately so it cannot drift
    /// from what is actually being rendered.
    #[must_use]
    pub fn camera_position(&self) -> Vec3 {
        self.view().inverse().w_axis.truncate()
    }

    /// The authored sections the craft and the camera are in, for PVS culling.
    ///
    /// [`UNPLACED`] means "do not trust this", which the visibility set turns
    /// into *draw everything*. Three things produce it, and all three are
    /// states where culling to a section would be most likely to be visibly
    /// wrong:
    ///
    /// - **The track has no spline**, so nothing can be located at all.
    /// - **The point is off the track**, more than
    ///   [`OFF_TRACK_HALF_WIDTHS`] half-widths from the nearest sample. This is
    ///   the case that matters: a craft that has fallen off, gone airborne over
    ///   a gap or been knocked into scenery is usually still inside some
    ///   authored box and would otherwise get a confidently wrong answer. See
    ///   that constant for why the lookup's own out-of-range fallback does not
    ///   cover this.
    /// - **A respawn is in flight** ([`RESPAWN_COOLDOWN_TICKS`]), where the
    ///   craft teleports and the camera spring is still catching up, so neither
    ///   position describes the shot for several frames.
    ///
    /// The craft and the camera are located independently, so the common case
    /// of a camera swinging off the racing line while the craft is fine
    /// degrades only the camera's half.
    #[must_use]
    pub fn visibility_sections(&self) -> (u8, u8) {
        // The player's, because this places the player's camera. An opponent
        // recovering across the circuit must not blank the shot.
        if self.respawn_cooldown[0] > 0 {
            return (UNPLACED, UNPLACED);
        }
        let ship = self.ship();
        // The simulation already located the craft this tick and left the
        // sample index in `Ship::segment`, so the craft's half is a table read
        // rather than a second scan of the whole track. Doing it again here
        // would cost more per frame than the culling it enables saves.
        // `u16::MAX` is the never-located sentinel, and it is out of range of
        // any real table, so the bounds check covers both.
        let index = usize::from(ship.segment);
        if index >= self.spline.len() {
            return (UNPLACED, UNPLACED);
        }
        let craft = self.section_of(index, ship.physics.body.position);
        // The camera is a chase spring a few units behind, so it is a few
        // samples away in the same table. See `Spline::nearest_within` for why
        // a local search is allowed to miss.
        let camera =
            match self
                .spline
                .nearest_within(self.camera_position(), index, CAMERA_SEARCH_SAMPLES)
            {
                Some((camera_index, _, _)) => self.section_of(camera_index, self.camera_position()),
                None => UNPLACED,
            };
        (craft, camera)
    }

    /// The section of the sample at `index`, or [`UNPLACED`] when `position` is
    /// too far from it to trust. See [`Self::visibility_sections`].
    pub(super) fn section_of(&self, index: usize, position: Vec3) -> u8 {
        let Some(sample) = self.spline.sample(index) else {
            return UNPLACED;
        };
        let distance = (Vec3::from_array(sample.pos) - position).length();
        let half_width = sample.half_width_left.max(sample.half_width_right);
        if distance > half_width * OFF_TRACK_HALF_WIDTHS {
            return UNPLACED;
        }
        sample.section_id
    }

    /// The projection for a viewport of the given aspect ratio, with the
    /// player's field-of-view setting applied.
    ///
    /// **`<ExternalCameraFar fov>`'s unit is vertical degrees**, settled
    /// 2026-08-09 at confidence 94 against `g_camera_fov_degrees`
    /// (`0x08b34310`) on the running game, and read here as such.
    /// `oag_render::camera::projection` still names its parameter
    /// `fov_radians` precisely so that the conversion has to be written at a call
    /// site rather than assumed in a library. The value read from the disc is
    /// printed in the load report, so the assumption is checkable by whoever looks.
    ///
    /// The value is also authored for **one** viewport shape, the PSP's own, which
    /// is the only one the original ever renders into. A window narrower than that
    /// goes through [`oag_render::camera::fit_vertical_fov`], which holds the
    /// horizontal field the authored shape would have had instead of cropping the
    /// sides away; at the PSP's aspect and anything wider it is the authored value
    /// unchanged. That is a presentation choice and is documented as one.
    ///
    /// `setting` is a **percentage of that authored value** and is applied
    /// *before* the fit, so the two compose in the order a reader would expect:
    /// the player widens the field the disc asked for, and the result is then
    /// fitted to whatever shape the window is. At
    /// [`Fov::AUTHORED`](crate::display::Fov::AUTHORED) - the default - nothing
    /// is applied at all and this is the projection every capture under
    /// `data/traces/` was taken with.
    ///
    /// `near` is 1.0 rather than something tiny: track half-widths are tens of
    /// units, so a near plane at 0.01 spends the depth range on nothing and
    /// z-fights in the distance.
    ///
    /// # The field widens with speed, and that is the original's own behaviour
    ///
    /// `Ship_UpdateCameraRigs` adds `craft+0x790` to both tripod fovs every
    /// frame, and that term is `0.075 * dot(forward, velocity)` **additive
    /// degrees**. Implemented here as [`SPEED_FOV_GAIN_DEG`], which carries the
    /// evidence: verified live against `g_camera_fov_degrees` to a worst
    /// residual of 0.0007 degrees over 19 samples. See
    /// `docs/rendering/projection-vs-the-original.md`.
    ///
    /// [`Self::boost_kick`] is a different thing and is **not** a port of it:
    /// the original's term is present with no boost at all, is driven by speed
    /// rather than by a pad, and adds degrees where `BoostFovKick` multiplies a
    /// tangent. The two compose, in that order.
    ///
    /// **A matched-pose comparison still needs `--camera-fov`.**
    /// [`oag_gameplay::spawn::Ship::place_at`] resets the body, so a posed craft
    /// has zero velocity and this term contributes exactly `0` - the posed frame
    /// renders at the authored field whatever the captured tick's speed was.
    /// Pass the fov computed from that tick's own forward velocity. Any
    /// comparison taken before 2026-08-09 is misregistered by 1.12-1.26x
    /// depending on the tick's speed and has to be re-taken.
    #[must_use]
    pub fn projection(&self, aspect: f32, far: f32, setting: crate::display::Fov) -> Mat4 {
        // An overridden fov stands in for the authored one and still passes
        // through the player's setting, whose default is identity; it exists to
        // match a captured frame's own field exactly, so it must sit at the
        // same point in the chain as the value it replaces. (It settled the
        // authored unit once; that unit is now recovered, and the flag's job is
        // matched-pose comparison - see `Options::fov_deg`.)
        // Each view authors its own fov, in the same unit at the same aspect, so
        // the live one is whichever perspective is being rendered - a cockpit
        // framed with the chase view's field would be the wrong picture with the
        // right camera.
        let view_fov = if self.view == crate::display::CameraView::Internal {
            self.internal_params.fov
        } else {
            self.chase_params.fov
        };
        let authored_fov = self
            .camera_override
            .as_ref()
            .and_then(|over| over.fov_deg)
            .unwrap_or(view_fov);
        // The original's speed widen, added to the authored degrees before
        // anything else - which is where `Ship_UpdateCameraRigs` adds it, and it
        // is additive degrees rather than a tangent multiplier. Composed
        // *before* the player's setting so a widened field scales the whole
        // thing proportionally, the same order the boost kick argues for below.
        //
        // It is deliberately not skipped for a `camera_override`: the override
        // stands in for the authored value and this is a separate physical
        // effect. It costs nothing in the calibration path either, because
        // `Ship::place_at` resets the body, so a posed craft has zero velocity
        // and the term is exactly `0` - which is why matching a captured frame
        // still means passing `--camera-fov` computed from that frame's own
        // forward velocity.
        let body = &self.ship().physics.body;
        let forward_speed = body.forward().dot(body.linear_velocity);
        let widened_deg = (authored_fov + SPEED_FOV_GAIN_DEG * forward_speed)
            .clamp(FOV_GUARD_DEG.0, FOV_GUARD_DEG.1);
        let authored = setting.apply(widened_deg.to_radians());
        // Composed *after* the setting, so a player who has widened the field
        // gets the same proportional kick rather than a fixed number of degrees.
        // The zero case returns the input untouched rather than through
        // `atan(tan(x))`, for the reason `Fov::apply` gives: every capture taken
        // before this effect existed has to stay bit-identical.
        let kicked = if self.boost_kick == 0.0 {
            authored
        } else {
            let widen = 1.0 + self.boost_kick * self.boost_fov_kick.gain();
            2.0 * ((authored * 0.5).tan() * widen).atan()
        };
        let fov = oag_render::camera::fit_vertical_fov(kicked, AUTHORED_ASPECT, aspect);
        oag_render::camera::projection(fov, aspect, 1.0, far)
    }
}
