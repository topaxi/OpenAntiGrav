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
    /// same answer without putting a lock on `World`, which would move a
    /// determinism hash for something only the screen reads.
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
        if let Some(held) = self.sight_held() {
            self.view.sight.set_held(held);
        }
        let target = self.sight_target();
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
        self.view.sight_state = self.view.sight.update(self.sim.dt, projected);
    }

    /// Which of the two lockable weapons the player is holding, or `None`.
    ///
    /// **`None` for a weapon this title's table does not author**, not only for
    /// one that cannot lock: Pure ships no `<Weapon type="LeachBeam">` and no
    /// `leachbeam_sight_*` widgets, so a LeachBeam there has no window to run
    /// and gets no reticle rather than borrowing the Missile's.
    pub(crate) fn sight_held(&self) -> Option<sight::Held> {
        let stats = self.sim.weapons.as_ref()?;
        match self.sim.world.ships[0].pickup.weapon? {
            oag_tables::weapons::Weapon::Missile => stats.missile().map(|_| sight::Held::Missile),
            oag_tables::weapons::Weapon::LeachBeam => {
                stats.leach_beam().map(|_| sight::Held::LeachBeam)
            }
            _ => None,
        }
    }

    /// Which craft the reticle is over, or `None`.
    ///
    /// Split out so the choice can be asserted without a camera. Returns
    /// `None` whenever the player is holding something that does not lock,
    /// which is every weapon but the Missile and the LeachBeam.
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
    pub(crate) fn sight_target(&self) -> Option<u8> {
        let held = self.sight_held()?;
        let stats = self.sim.weapons.as_ref()?;
        let count = self.sim.world.ship_count as usize;
        let ship = &self.sim.world.ships[0];
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
            0,
            origin,
            ship.physics.body.forward(),
            min,
            max,
            self.sim.course.as_ref().map(oag_race::Course::length),
        )
    }
}
