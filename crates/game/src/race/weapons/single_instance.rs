//! The two weapons that have exactly one instance in the whole race: the
//! Quake's travelling wave and the LeachBeam's link.
//!
//! Split out of `weapons.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests
//! are in `race/tests/weapons.rs`, alongside the rest of the weapon coverage.
//!
//! **They share a seam rather than only a line budget.** Both are an
//! `Option<_>` on `oag_gameplay::World` instead of a slot in
//! `oag_gameplay::projectile::Projectiles`, because both originals gate on a
//! *world* cursor rather than a per-craft cooldown - `Weapon_FireQuake`'s
//! `q->active != 0` and `Weapon_FireLeachBeam`'s `pool->live != 0`. Both are
//! therefore advanced once a tick from here rather than inside the projectile
//! array's own sweep, and both read every craft's freshest state to do it. See
//! `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.

use super::*;

impl Race {
    /// One tick of the Quake's own travelling wave: advances it along the
    /// course, and applies its hit/slowdown to every craft its own radius
    /// currently spans.
    ///
    /// A no-op with no course loaded (nothing for the wave to travel round)
    /// and a no-op with no wave in flight - see `Race::spend_pickup`'s own
    /// Quake arm for what launches one. See
    /// `oag_gameplay::projectile::quake` for the recovered advance rate and
    /// the hit test this reuses whole.
    ///
    /// **Retires the wave once it outlives
    /// `oag_gameplay::projectile::quake::LIFETIME_SECONDS`**, which is what
    /// re-opens `Race::spend_pickup`'s `world.quake.is_some()` guard for the
    /// next Quake in the race. In the original that guard is a byte on the
    /// weapon instance (`q+0x48`) and `Quake_UpdateSpans` (`0x08874a30`) clears
    /// it on the first frame no road span updates - the same "the apparatus is
    /// empty, so the weapon is free again" edge. See
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
    ///
    /// Called from the tick after `Race::advance_cannons`, on this tick's own
    /// standings - so a wave that just passed over a craft credits the hit
    /// the same tick the craft's position moved into and out of its radius,
    /// the same "this tick, not last tick's stale reading" rule every other
    /// per-craft weapon update in this module follows.
    pub(in crate::race) fn advance_quake(&mut self) {
        let Some(course) = self.course.as_ref() else {
            return;
        };
        let Some(mut wave) = self.world.quake else {
            return;
        };
        let length = course.length();
        wave.advance(length, self.dt);
        if wave.expired() {
            self.world.quake = None;
            return;
        }
        let rules = oag_gameplay::damage_rules(self.world.race.mode);
        let mut absorbed = [false; MAX_SHIPS];
        wave.apply_hits(
            &mut self.world.ships,
            self.world.ship_count,
            length,
            rules,
            &mut absorbed,
        );
        for (slot, hit) in absorbed.iter().enumerate() {
            if *hit {
                self.view.shield[slot].hit();
            }
        }
        self.world.quake = Some(wave);
    }

    /// One tick of the single LeachBeam link: age it, test it, drain it, retire
    /// it.
    ///
    /// A no-op with no beam in flight - see `Race::spend_pickup`'s own
    /// LeachBeam arm for what fires one, and
    /// `oag_gameplay::projectile::leach_beam` for the recovered transfer and
    /// the one place it knowingly departs from the original.
    ///
    /// Called from the tick beside `Race::advance_quake`, on this tick's own
    /// craft positions - so a target that pulled out of range this tick breaks
    /// the link this tick rather than one tick late, the same "this tick, not
    /// last tick's stale reading" rule every other per-craft weapon update in
    /// this module follows.
    pub(in crate::race) fn advance_leach_beam(&mut self) {
        let Some(mut beam) = self.world.leach_beam else {
            return;
        };
        let rules = oag_gameplay::damage_rules(self.world.race.mode);
        let report = beam.advance(&mut self.world.ships, self.world.ship_count, rules, self.dt);
        // A swallowed hit is the only thing that makes the target's shell
        // visibly react - the same out-parameter `Race::advance_quake` spends
        // on `self.view.shield[slot].hit()`, narrowed to the beam's one victim.
        if report.absorbed
            && let Some(shell) = self.view.shield.get_mut(beam.target as usize)
        {
            shell.hit();
        }
        self.world.leach_beam = if report.retired { None } else { Some(beam) };
    }
}
