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
        self.sim.scheme = scheme;
    }

    /// How far each airbrake flap has swung, left then right, in **radians**.
    ///
    /// Read by the renderer once a frame. Render-only state - see
    /// [`RaceView::flaps`], which holds the same thing on the airbrake's `0..=100`
    /// scale, and note the rates driving it are **not** the force law's.
    #[must_use]
    pub fn airbrake_flaps(&self) -> [f32; 2] {
        self.view
            .flaps
            .map(|level| self.view.flap_graphics.amount * level / 100.0)
    }

    /// Which scheme this race is being driven with.
    #[must_use]
    pub fn control_scheme(&self) -> ControlScheme {
        self.sim.scheme
    }

    /// Sets how strong the boost's field-of-view kick is. `[graphics]
    /// boost_fov_kick`.
    ///
    /// A setter rather than a [`Setup`] field because the kick is a display
    /// choice and `Setup` is what a headless race needs; threading a graphics
    /// setting through the loader would put it in front of every caller that has
    /// no screen. [`oag_display::display::BoostFovKick::OFF`] leaves [`Race::projection`]
    /// bit-identical to what it returned before the effect existed.
    /// The field of view the picture is being drawn at, for the lock-on reticle.
    ///
    /// See [`RaceView::sight_fov`]. Cheap and idempotent, so the frame loop may set
    /// it every frame rather than tracking whether the setting moved.
    pub fn set_sight_fov(&mut self, fov: oag_display::display::Fov) {
        self.view.sight_fov = fov;
    }

    /// How far out the authored `LodGroup`s switch to their coarser tiers -
    /// `[render_profiles.<title>] model_detail`. Render-only, cheap and
    /// idempotent like [`Self::set_sight_fov`], so the frame loop sets it
    /// every frame and the menu row applies live.
    pub fn set_model_detail(&mut self, detail: oag_render::mesh::ModelDetail) {
        self.view.model_detail = detail;
    }

    /// How far out a PSP `.vex` model keeps its finer texture levels -
    /// `[render_profiles.<title>] texture_detail`. Render-only and idempotent
    /// like [`Self::set_model_detail`], so the frame loop sets it every frame
    /// and the menu row applies live.
    pub fn set_texture_detail(&mut self, detail: oag_render::mesh_render::TextureDetail) {
        self.view.texture_detail = detail;
    }

    /// The preset [`Self::set_texture_detail`] last set; the frame writes it
    /// into the scene uniform.
    #[must_use]
    pub fn texture_detail(&self) -> oag_render::mesh_render::TextureDetail {
        self.view.texture_detail
    }

    /// Which grid the lock-on reticle's coordinates are in.
    ///
    /// The title's own HUD space - `oag_game::hud::Assets::space` - because the
    /// reticle is drawn beside that layout's widgets and has to project into the
    /// same grid. Wipeout HD authors 1920x1080 and the PSP titles 480x272; a
    /// reticle left at the default on HD lands in the top-left ninth of the
    /// screen and never leaves it.
    ///
    /// **Resets the reticle**, because its centre, its extent and its chase are
    /// all in the old grid's units. Called once before the first tick.
    pub fn set_sight_screen(&mut self, screen: (f32, f32)) {
        self.view.sight = oag_race::sight::Sight::new([screen.0, screen.1]);
    }

    /// Whether a held LeachBeam's reticle runs the LeachBeam's own law
    /// (`FUN_0881e8c8`: spinning arrowheads, no hold timer) rather than the
    /// Missile's - on for the PSP dialect, which is what authors
    /// `leachbeam_sight_*` brackets, and left off for Wipeout HD's concentric
    /// rings, whose own update is a different function. Called once before the
    /// first tick, after [`Self::set_sight_screen`], which resets the reticle.
    pub fn set_sight_leach_law(&mut self, on: bool) {
        self.view.sight.set_leach_law(on);
    }

    /// The lock-on reticle, for whoever draws it.
    ///
    /// Render-only state - see [`RaceView::sight`] - so this returning a borrow
    /// rather than a copy costs nothing and cannot be mistaken for world state.
    #[must_use]
    pub fn sight(&self) -> &oag_race::sight::Sight {
        &self.view.sight
    }

    /// What the reticle is doing, which is what `~ROCKLOCK` plays off.
    #[must_use]
    pub fn sight_state(&self) -> oag_race::sight::State {
        self.view.sight_state
    }

    pub fn set_boost_fov_kick(&mut self, kick: oag_display::display::BoostFovKick) {
        self.view.boost_fov_kick = kick;
        if kick == oag_display::display::BoostFovKick::OFF {
            self.view.boost_kick = 0.0;
        }
    }

    /// What this race was actually built with, for the `BOOST FOV KICK` row's
    /// restart note - see `Self::set_boost_fov_kick`, which is called once at
    /// `Race::start` and does not track the settings file afterwards.
    #[must_use]
    pub fn boost_fov_kick(&self) -> oag_display::display::BoostFovKick {
        self.view.boost_fov_kick
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
    pub fn set_camera_view(&mut self, view: oag_display::display::CameraView) {
        if view == self.view.camera_view {
            return;
        }
        self.view.camera_view = view;
        self.view.chase_params = match view {
            oag_display::display::CameraView::Close => self.view.chase_close,
            // The cockpit view does not use these, but leaving the *far* block
            // installed means a cycle back out of the cockpit lands on the block
            // the next external view will want anyway.
            oag_display::display::CameraView::Internal | oag_display::display::CameraView::Far => {
                self.view.chase_far
            }
        };
        self.view.camera = Chase::snapped(target_of(self.ship()), &self.view.chase_params);
    }

    /// Which perspective this race is rendering from.
    #[must_use]
    pub fn camera_view(&self) -> oag_display::display::CameraView {
        self.view.camera_view
    }

    /// Whether the player's own hull should be drawn this frame.
    ///
    /// False in the cockpit view alone. Read by [`Scene::render`], which skips
    /// the draw call rather than moving the model: see
    /// [`oag_display::display::CameraView::draws_own_ship`] for the evidence and its
    /// confidence.
    ///
    /// **The exhaust plume, the boost plume and the collision sparks are still
    /// drawn.** The recovered flag covers the hull and nothing else, so the
    /// alternative would be inventing three more consequences for it; and from a
    /// cockpit at the nose the nozzle is behind the eye, so the plume is normally
    /// off screen anyway rather than visibly wrong.
    #[must_use]
    pub fn draws_own_ship(&self) -> bool {
        self.view.camera_view.draws_own_ship()
    }

    /// How many times a `Reset` contact has respawned the player this race.
    #[must_use]
    pub fn respawns(&self) -> u32 {
        self.sim.respawns[0]
    }

    /// The same, for any craft on the grid.
    ///
    /// Out of bounds reads zero rather than panicking: a caller sweeping
    /// [`oag_gameplay::MAX_SHIPS`] slots on a six-craft grid is asking a fair
    /// question and the answer is "none".
    #[must_use]
    pub fn respawns_of(&self, slot: usize) -> u32 {
        self.sim.respawns.get(slot).copied().unwrap_or(0)
    }

    /// Which trigger fired this craft's most recent respawn, `None` if it has
    /// not had one. See [`respawn::RespawnCause`](super::respawn::RespawnCause).
    #[must_use]
    pub fn last_respawn_cause_of(&self, slot: usize) -> Option<RespawnCause> {
        self.sim.last_respawn_cause.get(slot).copied().flatten()
    }

    /// Whether respawning has given up on this craft after
    /// [`RESPAWN_GIVE_UP`](oag_race::recovery::RESPAWN_GIVE_UP) back-to-back
    /// rescues. Once set no trigger fires for it again, so a survey has to
    /// know.
    #[must_use]
    pub fn respawn_given_up_of(&self, slot: usize) -> bool {
        self.sim
            .respawn_disabled
            .get(slot)
            .copied()
            .unwrap_or(false)
    }

    /// How many barrel rolls an **opponent** has armed this race.
    ///
    /// Counted for the deviation our AI carries and the original's does not -
    /// see `Race::rolls_armed`. Out of bounds reads zero, as
    /// [`Self::respawns_of`] does, and **slot 0 is structurally always zero**:
    /// the player's craft is stepped by `Race::tick` rather than by
    /// `Race::step_opponents`, which is the only thing that counts. A roll the
    /// player flies - or one their own driver flies under `--autopilot` - is
    /// not counted here.
    #[must_use]
    pub fn rolls_armed_of(&self, slot: usize) -> u32 {
        self.sim.rolls_armed.get(slot).copied().unwrap_or(0)
    }

    /// What those rolls cost it, in shield-pool units.
    #[must_use]
    pub fn roll_shield_spent_of(&self, slot: usize) -> f32 {
        self.sim.rolls_spent.get(slot).copied().unwrap_or(0.0)
    }

    /// How many ticks an **opponent** has spent touching a wall this race.
    ///
    /// Slot 0 is structurally always zero, the same way
    /// [`Self::rolls_armed_of`] is: the player's craft is stepped by
    /// `Race::tick` and only `Race::step_opponents` counts. See
    /// [`oag_game::race::RaceSim::wall_contact_ticks`] for why this counts
    /// contacts rather than inbound impacts, and why a contact here is a wall
    /// by construction rather than by a geometric test on the normal.
    ///
    /// [`oag_game::race::RaceSim::wall_contact_ticks`]: crate::race::RaceSim
    #[must_use]
    pub fn wall_contact_ticks_of(&self, slot: usize) -> u32 {
        self.sim.wall_contact_ticks.get(slot).copied().unwrap_or(0)
    }

    /// The inbound subset of [`Self::wall_contact_ticks_of`] - ticks the craft
    /// arrived at a wall rather than ticks it was scraping one.
    #[must_use]
    pub fn wall_inbound_ticks_of(&self, slot: usize) -> u32 {
        self.sim.wall_inbound_ticks.get(slot).copied().unwrap_or(0)
    }

    /// What those contacts **charged** an opponent, in shield-pool units.
    ///
    /// Charged, not lost: a craft already at zero is charged the same and loses
    /// nothing. Read it beside the end-of-run pool.
    #[must_use]
    pub fn wall_shield_charged_of(&self, slot: usize) -> f32 {
        self.sim.wall_damage.get(slot).copied().unwrap_or(0.0)
    }

    /// How many ticks an opponent was still racing - the denominator
    /// [`Self::wall_contact_ticks_of`] and [`Self::wall_shield_charged_of`] are
    /// counted over. A wrecked craft is released and keeps being integrated, so
    /// a row that died early covers fewer ticks than one that finished.
    #[must_use]
    pub fn racing_ticks_of(&self, slot: usize) -> u32 {
        self.sim.wall_racing_ticks.get(slot).copied().unwrap_or(0)
    }

    /// The fixed timestep, from [`TickRate::DEFAULT`].
    #[must_use]
    pub fn dt(&self) -> f32 {
        self.sim.dt
    }

    /// Which grid slot the person at this screen is flying.
    ///
    /// [`oag_gameplay::World::primary_slot`], and `0` under every session this
    /// engine currently starts. **The accessor every presentation-side reader
    /// goes through** - a HUD, a camera, a medal, a records row - so that when
    /// a second local player arrives, "which craft is this view about" is a
    /// parameter in one place rather than a literal `0` in thirty.
    #[must_use]
    pub fn player_slot(&self) -> usize {
        self.sim.world.primary_slot()
    }

    /// The player's ship.
    #[must_use]
    pub fn ship(&self) -> &Ship {
        &self.sim.world.ships[self.player_slot()]
    }

    /// The player's lap, place and finish tick.
    ///
    /// [`Self::ship`]'s `standing`, named because the HUD and the records row
    /// want exactly this and nothing else off the craft.
    #[must_use]
    pub fn player_standing(&self) -> &oag_race::Standing {
        &self.ship().standing
    }

    /// What the player's craft is carrying, if anything.
    ///
    /// A convenience over `ship().pickup.weapon`, and the accessor the HUD
    /// reads: which pickup is held is presentation-facing in a way the rest of
    /// [`Ship`] is not.
    #[must_use]
    pub fn ship_pickup(&self) -> Option<oag_tables::weapons::Weapon> {
        self.ship().pickup.weapon
    }

    /// The resampled spline, for a caller that wants to measure against it.
    #[must_use]
    pub fn spline(&self) -> &Spline {
        &self.sim.spline
    }

    /// The collision world the race queries, for a caller that wants to run the
    /// same probe a projectile does and see what it saw.
    ///
    /// Read-only, and a verification aid in the same sense [`Self::spline`]
    /// is: a projectile that vanishes mid-flight is explained by re-running
    /// its surface probe and its flight sweep against exactly this world, not
    /// one rebuilt from the same disc by a test's own loader.
    #[must_use]
    pub fn collision(&self) -> &CollisionWorld {
        &self.sim.collision
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
    pub fn missile_stats(&self) -> Option<oag_tables::weapons::MissileStats> {
        self.sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::missile)
    }

    /// The LeachBeam's authored `<Stats>`, or `None` when the table did not
    /// load or authors no LeachBeam.
    ///
    /// An accessor for [`Self::missile_stats`]'s reason. Read by
    /// `crates/game/tests/lock_sight_ground_truth.rs`, which asks it of **each**
    /// title: whether a disc authors this block is what decides whether its
    /// reticle can be driven for the second lockable weapon at all, and the
    /// answer differs across the three.
    #[must_use]
    pub fn leach_beam_stats(&self) -> Option<oag_tables::weapons::LeachBeamStats> {
        self.sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::leach_beam)
    }

    /// The Mine's authored `<Stats>`, or `None` when the table did not load or
    /// authors no Mine.
    ///
    /// An accessor for [`Self::missile_stats`]'s reason. Read by
    /// `crates/game/tests/mine_ground_truth.rs`.
    #[must_use]
    pub fn mine_stats(&self) -> Option<oag_tables::weapons::MineStats> {
        self.sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::mine)
    }

    /// The Quake's authored `<Stats>`, or `None` when the table did not load
    /// or authors no Quake.
    ///
    /// An accessor for [`Self::missile_stats`]'s reason. Read by
    /// `crates/game/tests/mine_ground_truth.rs`, which launches a wave over a
    /// laid mine.
    #[must_use]
    pub fn quake_stats(&self) -> Option<oag_tables::weapons::QuakeStats> {
        self.sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::quake)
    }

    /// The Shuriken's authored `<Stats>`, or `None` when the table did not load
    /// or authors no Shuriken.
    ///
    /// An accessor for [`Self::missile_stats`]'s reason. Read by
    /// `crates/game/tests/shuriken_ground_truth.rs`.
    #[must_use]
    pub fn shuriken_stats(&self) -> Option<oag_tables::weapons::ShurikenStats> {
        self.sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::shuriken)
    }

    /// The Rocket's authored `<Stats>`, or `None` when the table did not load or
    /// authors no Rocket.
    ///
    /// An accessor for [`Self::missile_stats`]'s reason. Read by
    /// `crates/game/tests/plasma_ground_truth.rs`, which compares the two
    /// blocks against each other to prove they are two rather than one read
    /// twice.
    #[must_use]
    pub fn rocket_stats(&self) -> Option<oag_tables::weapons::RocketStats> {
        self.sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::rocket)
    }

    /// The Plasma's authored `<Stats>`, or `None` when the table did not load or
    /// authors no Plasma.
    ///
    /// An accessor for [`Self::missile_stats`]'s reason. Read by
    /// `crates/game/tests/plasma_ground_truth.rs`.
    #[must_use]
    pub fn plasma_stats(&self) -> Option<oag_tables::weapons::PlasmaStats> {
        self.sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::plasma)
    }

    /// The Cannon's authored `<Stats>`, or `None` when the table did not load or
    /// authors no Cannon.
    ///
    /// An accessor for [`Self::missile_stats`]'s reason. Read by
    /// `crates/game/tests/cannon_ground_truth.rs`.
    #[must_use]
    pub fn cannon_stats(&self) -> Option<oag_tables::weapons::CannonStats> {
        self.sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::cannon)
    }

    /// `<Weapon type="Global"><Stats slowdown_limit>`, or `None` when the table
    /// did not load.
    ///
    /// The ceiling on **seconds of weapon slowdown outstanding** that
    /// [`oag_gameplay::slowdown::drain`] is handed every tick. An accessor for
    /// [`Self::missile_stats`]'s reason, and the one figure of the mechanic a
    /// test can reach without reproducing an authored number: it is what a
    /// craft's timer is asserted *against* rather than a value to assert.
    #[must_use]
    pub fn slowdown_limit(&self) -> Option<f32> {
        self.sim.weapons.as_ref().map(|table| table.slowdown_limit)
    }

    /// The Bomb's authored `<Stats>`, or `None` when the table did not load or
    /// authors no Bomb.
    #[must_use]
    pub fn bomb_stats(&self) -> Option<oag_tables::weapons::BombStats> {
        self.sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::bomb)
    }
}
