//! The small reads and writes on a running race: the display and control
//! settings a menu changes under it, and the accessors everything outside asks
//! for.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

impl Race {
    /// Chooses which control scheme maps the pilot's buttons.
    ///
    /// A setter for the same reason `set_boost_fov_kick` is one: it is a
    /// preference read from the settings file, and `Setup` is what a *headless*
    /// race needs. Unlike the kick it changes what the simulation sees, so it is
    /// set once before the first tick and not touched again - a scheme swapped
    /// mid-race would leave a half-finished gesture armed in `ShipState`.
    pub fn set_control_scheme(&mut self, scheme: ControlScheme) {
        self.scheme = scheme;
    }

    /// How far each airbrake flap has swung, left then right, in **radians**.
    ///
    /// Read by the renderer once a frame. Render-only state - see
    /// [`Self::flaps`], which holds the same thing on the airbrake's `0..=100`
    /// scale, and note the rates driving it are **not** the force law's.
    #[must_use]
    pub fn airbrake_flaps(&self) -> [f32; 2] {
        self.flaps
            .map(|level| self.flap_graphics.amount * level / 100.0)
    }

    /// Which scheme this race is being driven with.
    #[must_use]
    pub fn control_scheme(&self) -> ControlScheme {
        self.scheme
    }

    /// Sets how strong the boost's field-of-view kick is. `[graphics]
    /// boost_fov_kick`.
    ///
    /// A setter rather than a [`Setup`] field because the kick is a display
    /// choice and `Setup` is what a headless race needs; threading a graphics
    /// setting through the loader would put it in front of every caller that has
    /// no screen. [`crate::display::BoostFovKick::OFF`] leaves [`Race::projection`]
    /// bit-identical to what it returned before the effect existed.
    pub fn set_boost_fov_kick(&mut self, kick: crate::display::BoostFovKick) {
        self.boost_fov_kick = kick;
        if kick == crate::display::BoostFovKick::OFF {
            self.boost_kick = 0.0;
        }
    }

    /// What this race was actually built with, for the `BOOST FOV KICK` row's
    /// restart note - see `Self::set_boost_fov_kick`, which is called once at
    /// `Race::start` and does not track the settings file afterwards.
    #[must_use]
    pub fn boost_fov_kick(&self) -> crate::display::BoostFovKick {
        self.boost_fov_kick
    }

    /// Chooses which of the three perspectives to render from. `[graphics]
    /// camera_view`.
    ///
    /// A setter for the same reason `set_boost_fov_kick` is one - and unlike that
    /// one this is called **during** a race as well as before it, because the
    /// original binds the choice to a button. Everything it touches is
    /// render-only, so calling it mid-race cannot change what the simulation does
    /// next; that is the property
    /// `tests::cycling_the_camera_changes_no_simulation_state` pins.
    ///
    /// **The chase eye is snapped on a change between the two external views**,
    /// not sprung across. They share one eye and their anchors are metres apart,
    /// so springing would sweep the camera through the world for a second - the
    /// same reason [`Chase::snapped`] exists for a race start and a respawn.
    /// Whether the original springs or snaps between them is **not recovered**:
    /// it keeps a separate previous-eye per rig, so it does neither, and
    /// reproducing that needs a second [`Chase`] rather than a decision here.
    pub fn set_camera_view(&mut self, view: crate::display::CameraView) {
        if view == self.view {
            return;
        }
        self.view = view;
        self.chase_params = match view {
            crate::display::CameraView::Close => self.chase_close,
            // The cockpit view does not use these, but leaving the *far* block
            // installed means a cycle back out of the cockpit lands on the block
            // the next external view will want anyway.
            crate::display::CameraView::Internal | crate::display::CameraView::Far => {
                self.chase_far
            }
        };
        self.camera = Chase::snapped(target_of(self.ship()), &self.chase_params);
    }

    /// Which perspective this race is rendering from.
    #[must_use]
    pub fn camera_view(&self) -> crate::display::CameraView {
        self.view
    }

    /// Whether the player's own hull should be drawn this frame.
    ///
    /// False in the cockpit view alone. Read by [`Scene::render`], which skips
    /// the draw call rather than moving the model: see
    /// [`crate::display::CameraView::draws_own_ship`] for the evidence and its
    /// confidence.
    ///
    /// **The exhaust plume, the boost plume and the collision sparks are still
    /// drawn.** The recovered flag covers the hull and nothing else, so the
    /// alternative would be inventing three more consequences for it; and from a
    /// cockpit at the nose the nozzle is behind the eye, so the plume is normally
    /// off screen anyway rather than visibly wrong.
    #[must_use]
    pub fn draws_own_ship(&self) -> bool {
        self.view.draws_own_ship()
    }

    /// How many times a `Reset` contact has respawned the player this race.
    #[must_use]
    pub fn respawns(&self) -> u32 {
        self.respawns[0]
    }

    /// The same, for any craft on the grid.
    ///
    /// Out of bounds reads zero rather than panicking: a caller sweeping
    /// [`oag_gameplay::MAX_SHIPS`] slots on a six-craft grid is asking a fair
    /// question and the answer is "none".
    #[must_use]
    pub fn respawns_of(&self, slot: usize) -> u32 {
        self.respawns.get(slot).copied().unwrap_or(0)
    }

    /// The fixed timestep, from [`TickRate::DEFAULT`].
    #[must_use]
    pub fn dt(&self) -> f32 {
        self.dt
    }

    /// The player's ship.
    #[must_use]
    pub fn ship(&self) -> &Ship {
        &self.world.ships[0]
    }

    /// What the player's craft is carrying, if anything.
    ///
    /// A convenience over `ship().pickup.weapon`, and the accessor the HUD
    /// reads: which pickup is held is presentation-facing in a way the rest of
    /// [`Ship`] is not.
    #[must_use]
    pub fn ship_pickup(&self) -> Option<oag_formats::weapons::Weapon> {
        self.world.ships[0].pickup.weapon
    }

    /// The resampled spline, for a caller that wants to measure against it.
    #[must_use]
    pub fn spline(&self) -> &Spline {
        &self.spline
    }
}

impl Race {
    /// The Missile's authored `<Stats>`, or `None` when the table did not load or
    /// authors no Missile.
    ///
    /// An accessor rather than a public field, because `weapons` is the whole
    /// table and only the blocks with a consumer should be reachable from
    /// outside. Read by `crates/game/tests/missile_ground_truth.rs`, which links
    /// the library and so cannot see the field.
    #[must_use]
    pub fn missile_stats(&self) -> Option<oag_formats::weapons::MissileStats> {
        self.weapons
            .as_ref()
            .and_then(oag_formats::weapons::WeaponStats::missile)
    }
}
