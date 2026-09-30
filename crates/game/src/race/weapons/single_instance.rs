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
        let Some(course) = self.sim.course.as_ref() else {
            return;
        };
        let Some(mut wave) = self.sim.world.quake else {
            return;
        };
        let length = course.length();
        wave.advance(length, self.sim.dt);
        if wave.expired() {
            self.sim.world.quake = None;
            return;
        }
        let rules = self.sim.damage_rules();
        let mut hits = [oag_gameplay::projectile::WeaponHit::default(); MAX_SHIPS];
        // Snapshotted before `apply_hits` runs, for `Cue::QuakeHit`'s own
        // rising edge below - the same before/after shape `Race::tick`'s own
        // `bounces_before` already takes for the Missile's bounce cue.
        let hit_before = wave.hit;
        wave.apply_hits(
            &mut self.sim.world.ships,
            self.sim.world.ship_count,
            length,
            rules,
            &mut hits,
        );
        for (slot, hit) in hits.iter().enumerate() {
            if hit.absorbed {
                self.view.shield[slot].hit();
            }
        }
        // `QUAKEHIT` (confidence 85), on the rising edge of the wave's own
        // per-craft latch - see `Cue::QuakeHit`'s own doc comment. Placed on
        // the **struck** craft, which is what `CueEvent::new(cue, slot)`'s
        // slot means for a `Placement::Craft` cue.
        for (slot, &now) in wave.hit.iter().enumerate() {
            if now && !hit_before[slot] {
                self.sim.cues.push(crate::audio::sfx::CueEvent::new(
                    crate::audio::sfx::Cue::QuakeHit,
                    slot,
                ));
            }
        }
        // **A wave passing under a laid mine sets it off, quietly.**
        // `Mine_SweepCraftTrigger` (`0x08867b50`) ends by asking
        // `Quake_SpanIntensityAt` at the mine's own track cursor and raises the
        // mine's destroy bit when the road there is rippling above `0.1`;
        // `MinePool_Update`'s teardown then plays the explosion and credits
        // nobody. This wave model has no per-span intensity, only a front
        // and a radius, so "rippling under the mine" is the same
        // within-radius test the craft take. Mines only: the Bomb pool's own
        // update was not read for a Quake test, and one is not assumed. See
        // `docs/ghidra/functions/psp-pulse-usa/mine.md`, 2026-09-16.
        let tripped: Vec<(usize, Vec3)> = self
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .enumerate()
            .filter(|(_, laid)| laid.kind == Some(oag_tables::weapons::Weapon::Mine))
            .filter_map(|(slot, laid)| {
                let located = course.locate(laid.position, None)?;
                (wave.progress_delta(located.progress, length) <= wave.radius)
                    .then_some((slot, laid.position))
            })
            .collect();
        for (slot, point) in tripped {
            self.sim.world.projectiles.slots[slot] =
                oag_gameplay::projectile::Projectile::default();
            // Identity: only a Mine reaches this filter above, and
            // `ignite_blast`'s `orientation` is unused by every arm but the
            // Bomb's.
            self.ignite_blast(
                oag_tables::weapons::Weapon::Mine,
                point,
                None,
                Quat::IDENTITY,
            );
        }
        // The wave's landed hits spark the struck hulls - see
        // `race::hit_sparks`. Last, once `course`'s borrow has ended.
        self.throw_hit_sparks(&hits, false);
        self.sim.world.quake = Some(wave);
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
        let Some(mut beam) = self.sim.world.leach_beam else {
            return;
        };
        let rules = self.sim.damage_rules();
        let report = beam.advance(
            &mut self.sim.world.ships,
            self.sim.world.ship_count,
            rules,
            self.sim.dt,
        );
        // A swallowed hit is the only thing that makes the target's shell
        // visibly react - the same out-parameter `Race::advance_quake` spends
        // on `self.view.shield[slot].hit()`, narrowed to the beam's one victim.
        if report.absorbed
            && let Some(shell) = self.view.shield.get_mut(beam.target as usize)
        {
            shell.hit();
        }
        // A drain that got through sparks the target's hull with the
        // LeachBeam's own variant, once per locator per 0.8 s like every
        // other hit - see `race::hit_sparks`.
        if report.landed {
            let mut hits = [oag_gameplay::projectile::WeaponHit::default(); MAX_SHIPS];
            if let Some(hit) = hits.get_mut(beam.target as usize) {
                hit.landed = true;
            }
            self.throw_hit_sparks(&hits, true);
        }
        self.sim.world.leach_beam = if report.retired { None } else { Some(beam) };
    }
}
