//! The Repulser in the race: firing one and walking every live one a tick.
//!
//! The law is `oag_weapons::projectile::repulser`'s; this is the part that needs
//! the course, the cue queue and the hit bookkeeping. See
//! `docs/ghidra/functions/psp-pulse-usa/repulser.md`.

use super::*;
use oag_weapons::projectile::repulser::Repulser;

impl Race {
    /// `Weapon_FireRepulser` (`0x0886ce8c`) for `slot`: claims a pool slot and
    /// plays `REPULSOR` on the firer.
    ///
    /// Returns whether the pickup is spent. **It is spent even when all sixteen
    /// slots are live**: the original clears the held weapon and the fire bit
    /// before it tests `pool->live < 0x10`. Only a table with no Repulser block
    /// keeps the pickup, the same "nothing to fire" rule every arm follows.
    pub(crate) fn fire_repulser(&mut self, slot: usize) -> bool {
        let Some(stats) = self
            .sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::repulser)
        else {
            return false;
        };
        let Ok(owner) = u8::try_from(slot) else {
            return false;
        };
        if let Some(free) = self.sim.world.repulsers.iter_mut().find(|r| r.is_none()) {
            *free = Some(Repulser::launch(owner, &stats));
            self.sim.cues.push(oag_sound::sfx::CueEvent::new(
                oag_sound::sfx::Cue::Repulsor,
                slot,
            ));
        }
        true
    }

    /// One tick of `RepulserPool_Update` (`0x0886cc70`) over every live
    /// Repulser, in pool order: `Repulser_Update`, then
    /// `RepulserPool_SweepTargets`, then retire the ones whose lifetime is up.
    ///
    /// Called after the standings, beside `Race::advance_quake`, so the sweep
    /// reads every craft's ring index from this tick.
    ///
    /// A swept Mine or Bomb has its fuse cut to zero, so the projectile step
    /// ends it next tick through the path its own fuse takes: a Mine goes off
    /// quietly and a Bomb detonates. That is what the destroy bit the original
    /// raises (`+0x3c |= 4`) leads to in `MinePool_Update` and `BombPool_Update`
    /// ([mine.md](../../../../../docs/ghidra/functions/psp-pulse-usa/mine.md)).
    pub(crate) fn advance_repulsers(&mut self) {
        let Some(course) = self.sim.course.as_ref() else {
            return;
        };
        let rules = self.sim.damage_rules();
        let dt = self.sim.dt;
        let mut hits = [oag_weapons::projectile::WeaponHit::default(); MAX_SHIPS];
        let mut credits: Vec<(usize, u8)> = Vec::new();
        for index in 0..self.sim.world.repulsers.len() {
            let Some(mut repulser) = self.sim.world.repulsers[index] else {
                continue;
            };
            let owner_index = self.sim.world.ships[repulser.owner as usize]
                .standing
                .course_index
                .map(|i| i as usize);
            let alive = repulser.advance(course, owner_index, dt);
            let struck = repulser.apply_hits(
                &mut self.sim.world.ships,
                self.sim.world.ship_count,
                course,
                rules,
                &mut hits,
            );
            for (slot, &was_struck) in struck.iter().enumerate() {
                if was_struck {
                    self.sim.cues.push(oag_sound::sfx::CueEvent::new(
                        oag_sound::sfx::Cue::RepulsorHit,
                        slot,
                    ));
                    credits.push((slot, repulser.owner));
                }
            }
            if repulser.waves_running() {
                for laid in &mut self.sim.world.projectiles.slots {
                    if !matches!(
                        laid.kind,
                        Some(oag_tables::weapons::Weapon::Mine | oag_tables::weapons::Weapon::Bomb)
                    ) {
                        continue;
                    }
                    let Some(width) = course
                        .locate(laid.position, None)
                        .and_then(|located| course.corridor_width(located.index))
                    else {
                        continue;
                    };
                    if repulser.swept_by(laid.position, width).is_some() {
                        laid.lifetime = 0.0;
                    }
                }
            }
            self.sim.world.repulsers[index] = alive.then_some(repulser);
        }
        for (slot, hit) in hits.iter().enumerate() {
            if hit.absorbed {
                self.view.shield[slot].hit();
            }
        }
        self.throw_hit_sparks(&hits, false);
        for (slot, owner) in credits {
            if hits[slot].landed {
                self.record_pending_hit(slot, owner);
            }
        }
    }
}

impl Race {
    /// Keeps each live Repulser's three particle instances where the original
    /// keeps them, and releases them when it retires.
    ///
    /// - [`Trigger::RepulserBlast`] starts on the Repulser's first tick
    ///   (`Repulser_Init`) at the firer and follows the firer from then on,
    ///   because the matrix it is anchored to (`+0x1a0`) is rebuilt from the
    ///   firer's node every tick.
    /// - Two [`Trigger::Repulser`]s start on the tick the waves start
    ///   (`Repulser_SpawnWaves`) and follow the two wave centres.
    /// - A third [`Trigger::Repulser`] starts on the update a wave forks at a
    ///   split. All four are released when the slot empties (`Repulser_Reset`).
    ///
    /// The field model `Data\Weapons\pulse_repulsorwave.vex` steps here too
    /// ([`Race::advance_repulser_fields`]), and the blast takes the field's
    /// unscaled basis turned by its spin, the matrix `+0x1a0` it is anchored
    /// to (`Repulser_UpdateFieldModel`, `0x08876168`).
    pub(crate) fn advance_repulser_visual(&mut self) {
        let dt = self.sim.dt;
        self.advance_repulser_fields();
        for index in 0..self.sim.world.repulsers.len() {
            let handles = self.view.repulser_effects[index];
            let Some(repulser) = self.sim.world.repulsers[index] else {
                for playing in handles.into_iter().flatten() {
                    self.view.stage.release(playing);
                }
                self.view.repulser_effects[index] = [None; 4];
                continue;
            };
            let firer = self.sim.world.ships[repulser.owner as usize]
                .physics
                .body
                .position;
            // The first tick, and only the first: a refused attach is not
            // retried later, so the blast cannot appear late.
            if handles[0].is_none() && repulser.age <= dt * 1.5 {
                if let Some(effect) = self.view.handles.get(Trigger::RepulserBlast).cloned() {
                    let playing = self.view.stage.attach(&effect, firer, 1.0);
                    // `Repulser_SpawnBlastEffect` hands `Psys_Spawn_q` its `+0x1a0` by
                    // pointer (`param_5 = 1`), so the beads (flag `0x2`) ride it.
                    if let Some(playing) = playing {
                        self.view.stage.set_rides_frame(playing, true);
                    }
                    self.view.repulser_effects[index][0] = playing;
                }
            } else if let Some(playing) = handles[0] {
                self.view.stage.follow(playing, firer);
            }
            if let (Some(playing), Some(field), Some((_, sample, _))) = (
                self.view.repulser_effects[index][0],
                self.view.repulser_fields[index],
                self.sim.spline.nearest(firer),
            ) {
                let (side, up, _) = crate::repulser_field::field_basis(
                    Vec3::from_array(sample.lateral),
                    Vec3::from_array(sample.down),
                );
                self.view.stage.orient(playing, up);
                let spun = oag_core::math::Quat::from_axis_angle(up, field.spin) * side;
                self.view.stage.stretch(playing, 1.0, spun);
            }
            let Some(fronts) = repulser.fronts else {
                continue;
            };
            // The spawn tick is the one where the fronts have not moved. It is
            // also when `Repulser_SpawnWaves` starts screen flash kind 2 at the
            // firer (`FUN_088f00c0(.., 2, ..)`, `0x088765ac`).
            let spawning = fronts[0].previous == fronts[0].point && handles[1].is_none();
            if spawning && let Some(flash) = &mut self.view.screen_flash {
                flash.start(oag_fx::flash::REPULSER, firer);
            }
            // The fork's own `WO_REPULSER` (`REP2`, `0x32504552`), started on the
            // update it forks (`Repulser_AdvanceWave`, `0x08876914`) and anchored
            // at `+0xe0`, its matrix.
            let fork = repulser
                .fork
                .map(|fork| (fork.front, fork.front.previous == fork.front.point));
            let waves = [
                Some((fronts[0], spawning)),
                Some((fronts[1], spawning)),
                fork,
            ];
            for (slot, (front, starts)) in waves
                .into_iter()
                .enumerate()
                .filter_map(|(slot, wave)| Some((slot, wave?)))
            {
                let playing = match handles[1 + slot] {
                    Some(playing) => {
                        self.view.stage.follow(playing, front.point);
                        Some(playing)
                    }
                    None if starts => {
                        let effect = self.view.handles.get(Trigger::Repulser).cloned();
                        let attached =
                            effect.and_then(|e| self.view.stage.attach(&e, front.point, 1.0));
                        self.view.repulser_effects[index][1 + slot] = attached;
                        attached
                    }
                    None => None,
                };
                // `Repulser_AdvanceWave` builds the wave's matrix at unit scale
                // with `Y` the point's negated `down` and `Z` `across x up`
                // negated for direction 0, so `X` (`Y x Z`) is `-across` for the
                // forward wave and its fork and `across` for the backward one:
                // either way `Z` is the wave's travel, and shape 8's half ring
                // bows ahead of it. It then sets the instance's extent co-factor
                // (`+0x2c`) to the distance between the track's two edges over
                // `100` (`0x08876fc0..0x08877020`), so the root's `50` is half
                // the track's width. The same frame and width here, from the
                // nearest spline sample to the wave's centre.
                if let Some(playing) = playing
                    && let Some((_, sample, _)) = self.sim.spline.nearest(front.point)
                {
                    let lateral = Vec3::from_array(sample.lateral);
                    let backward = slot == 1;
                    let across = if backward { lateral } else { -lateral };
                    let up = -Vec3::from_array(sample.down);
                    let width =
                        (lateral * (sample.half_width_left + sample.half_width_right)).length();
                    self.view.stage.orient(playing, up);
                    self.view.stage.stretch(playing, width / 100.0, across);
                }
            }
        }
    }
}
