//! The lock-on reticle: which of the two lockable weapons is up, and which
//! craft it is over.
//!
//! Split out of `weapons.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, a move with no behaviour change, to make room
//! for the weapon sound cues landing in the same file. Its tests stay in
//! `race/tests/weapons.rs` alongside the rest of the weapon coverage.
//!
//! Named `reticle` rather than `sight` deliberately: this module sits inside
//! `weapons`, whose own `use super::*` already reaches `oag_race::sight`, and
//! a sibling module named `sight` would shadow that glob import with itself
//! the moment anything here wrote `use super::*;`.

use super::*;

impl Race {
    /// One tick of the lock-on reticle, and the tone state that goes with it.
    ///
    /// **Recovered** from `HudSight_Update` (`0x0881dbcc`) - the law itself is
    /// in [`oag_race::sight`], and this is only what feeds it. See
    /// `docs/ghidra/functions/psp-pulse-usa/lock-sight.md`.
    ///
    /// # Which craft it points at
    ///
    /// The one `Ship_AcquireLock` would pick for the weapon the player is
    /// **holding**, recomputed every tick. The original runs that function per
    /// frame and stores the winner on the entity; running it here reaches the
    /// same answer without storing the target. What *is* stored, per slot and in
    /// the simulation, is the lock the hold has taken on it
    /// ([`Race::advance_locks`]).
    ///
    /// # The lock is not decided here
    ///
    /// This advances the reticle the player sees: it projects the target through
    /// the camera on screen and runs the law over it, so the brackets close on
    /// the craft as drawn. The lock, the red and the tone are the simulation's
    /// and the reticle is told them with [`sight::Sight::follow`].
    ///
    /// **Both lockable weapons drive it.** `Ship_AcquireLock` (`0x08844784`) is
    /// one function serving two: it takes the Missile's window from
    /// `stats+0x50`/`+0x54` and the LeachBeam's from `+0x114`/`+0x118`, and
    /// everything after that switch - the `0.9` cone, the along-track screen,
    /// the nearest-by-longitudinal-distance tie-break - is shared. So the two
    /// differ by two numbers, and this reaches them through
    /// `oag_weapons::projectile::missile::lock_window`.
    ///
    /// The **art** differs too, and that is carried on the reticle rather than
    /// here: [`sight::Sight::set_held`] records which of the two is up, and the
    /// HUD draws the Missile's four brackets plus its inner box, or the
    /// LeachBeam's four arrowheads and no inner.
    ///
    /// # The gate is ours
    ///
    /// The original guards its projection with a condition this project has
    /// read and not understood - see the page's "The gate is the one part not
    /// read". This uses "the held weapon locks, and something is lockable",
    /// which is what the weapon plays like.
    pub(crate) fn update_sight(&mut self) {
        // The reticle is the primary person's; every other person-flown slot
        // reads the same lock through [`Race::sight_target`] without one.
        let primary = self.sim.world.primary_slot();
        if let Some(held) = self.sight_held(primary) {
            self.view.sight.set_held(held);
        }
        let target = self.sight_target(primary);
        let projected = target.and_then(|slot| {
            let world = self.sim.world.ships[slot as usize].physics.body.position;
            // **The title's own virtual screen, not the window's.** The original
            // projects into 480x272 because that *is* its screen; this engine
            // letterboxes the same rectangle into whatever the window is, so
            // the two agree exactly at the authored shape and drift a little as
            // the display aspect is taken away from it. **Ours**, and the
            // alternative - projecting at the window aspect - would put the
            // reticle off the craft on an authored-size capture, which is the
            // frame every comparison against the original is taken at.
            //
            // Off the reticle rather than off a constant, because Wipeout HD
            // authors its HUD in 1920x1080 and the PSP titles in 480x272.
            let screen = self.view.sight.screen();
            let aspect = screen[0] / screen[1];
            let view = self.view();
            // The far plane is irrelevant here - the reticle's own 250-unit
            // range test runs in eye space, before the projection - so this
            // takes a value large enough never to clip a craft the sight would
            // otherwise draw.
            let projection = self.projection(aspect, SIGHT_FAR, self.view.sight_fov);
            sight::project(view, projection * view, world, screen)
        });
        // The lock is the simulation's ([`Race::advance_locks`]); this reticle
        // only puts the brackets on the craft and follows it.
        let locked = self.sim.locks[primary].locked();
        self.view.sight_state = self.view.sight.follow(self.sim.dt, projected, locked);
        self.update_viewed_sights();
    }

    /// The reticle of every other slot a split-screen view follows.
    ///
    /// **Display only, and view-side.** Each is a [`sight::Sight`] of its own in
    /// that slot's [`SlotHud`](crate::slot_hud::SlotHud), fed by
    /// [`Self::sight_target`] and projected through that slot's own chase camera
    /// and field of view, and told the slot's simulation lock exactly as the
    /// primary's is. A no-op with one view.
    ///
    /// Projects at the title's HUD aspect, as the primary's does, and a view
    /// narrower than that sees a wider vertical field than this assumes
    /// (`fit_vertical_fov`), so on a tall quadrant the brackets sit a little off
    /// the craft. **Chosen, not measured.**
    fn update_viewed_sights(&mut self) {
        let primary = self.sim.world.primary_slot();
        for slot in (0..self.viewport_count()).filter(|&slot| slot != primary) {
            if let Some(held) = self.sight_held(slot) {
                self.view.huds[slot].sight.set_held(held);
            }
            let target = self.sight_target(slot);
            let screen = self.view.huds[slot].sight.screen();
            // The camera accessors follow the viewpoint; put the player's back
            // after, as the draw does.
            self.set_viewpoint(slot);
            let projected = target.and_then(|target| {
                let world = self.sim.world.ships[target as usize].physics.body.position;
                let view = self.view();
                let projection =
                    self.projection(screen[0] / screen[1], SIGHT_FAR, self.view.sight_fov);
                sight::project(view, projection * view, world, screen)
            });
            self.reset_viewpoint();
            let dt = self.sim.dt;
            let locked = self.sim.locks[slot].locked();
            let hud = &mut self.view.huds[slot];
            hud.sight_state = hud.sight.follow(dt, projected, locked);
        }
    }

    /// Which of the two lockable weapons `slot` is holding, or `None`.
    ///
    /// **`None` for a weapon this title's table does not author**, not only for
    /// one that cannot lock: Pure ships no `<Weapon type="LeachBeam">` and no
    /// `leachbeam_sight_*` widgets, so a LeachBeam there has no window to run
    /// and gets no reticle rather than borrowing the Missile's.
    pub(crate) fn sight_held(&self, slot: usize) -> Option<sight::Held> {
        let stats = self.sim.weapons.as_ref()?;
        match self.sim.world.ships[slot].pickup.weapon? {
            oag_tables::weapons::Weapon::Missile => stats.missile().map(|_| sight::Held::Missile),
            oag_tables::weapons::Weapon::LeachBeam => {
                stats.leach_beam().map(|_| sight::Held::LeachBeam)
            }
            _ => None,
        }
    }

    /// Which craft `slot`'s lock is over, or `None`: the answer
    /// `Ship_AcquireLock` would give for the weapon `slot` holds.
    ///
    /// **A per-slot simulation quantity**, computed from the world alone for
    /// whichever slot asks. [`Race::advance_locks`] runs a person-flown slot's
    /// reticle law on it; the fire paths read the lock that comes of that. Split
    /// out so the choice can be asserted without a camera. Returns `None` whenever
    /// `slot` is holding something that does not lock, which is every weapon
    /// but the Missile and the LeachBeam.
    ///
    /// # The two arms differ by their window and by where they measure from
    ///
    /// The window is the original's own, off two pairs of `<Stats>` offsets.
    /// The **origin** is not: `Ship_AcquireLock` uses one weapon-independent
    /// origin for both branches, and this engine already moved the Missile's to
    /// the craft's *nose* - a deliberate deviation argued at
    /// [`Race::fire_missile`], where the missile actually starts. The LeachBeam
    /// has no recovered launch offset to move to, so its arm measures from the
    /// craft's own position. **Ours, and the absence of an invention rather
    /// than one**: borrowing the Missile's nose offset for a weapon with no
    /// projectile would be putting a number where the disc authors none. The
    /// two origins are a hull's length apart against a window that starts ten
    /// units out, so nothing visible turns on it.
    pub(crate) fn sight_target(&self, slot: usize) -> Option<u8> {
        let held = self.sight_held(slot)?;
        let stats = self.sim.weapons.as_ref()?;
        let count = self.sim.world.ship_count as usize;
        let ship = &self.sim.world.ships[slot];
        let (origin, min, max) = match held {
            sight::Held::Missile => {
                let missile = stats.missile()?;
                // From the nose and along the craft's forward, the same two the
                // fire path takes the lock from - see [`Race::fire_missile`] on
                // why the nose and not the centre.
                let (origin, _, _) = oag_weapons::projectile::missile::launch(
                    &ship.physics,
                    &ship.handling.dimensions,
                    &missile,
                    &self.sim.class,
                );
                (origin, missile.lock_min_dist, missile.lock_max_dist)
            }
            sight::Held::LeachBeam => {
                let leach = stats.leach_beam()?;
                (
                    ship.physics.body.position,
                    leach.lock_min_dist,
                    leach.lock_max_dist,
                )
            }
        };
        oag_weapons::projectile::missile::lock_window(
            &self.sim.world.ships[..count],
            slot as u8,
            origin,
            ship.physics.body.forward(),
            min,
            max,
            self.sim.course.as_ref().map(oag_race::Course::length),
        )
    }
}
