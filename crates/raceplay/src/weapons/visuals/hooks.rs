//! What a disc-backed test needs to see of the weapon visuals, and nothing the
//! game itself calls. Split out of `visuals.rs`, which is at the size limit.

use super::*;

impl Race {
    /// The LeachBeam's `WO_LEACHBEAM_ENERGY` instance's handle and the live
    /// particles of that effect on the stage, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn leach_energy_for_tests(&self) -> (Option<oag_fx::psys::Playing>, usize) {
        let alive = self
            .view
            .handles
            .get(Trigger::LeachbeamEnergy)
            .map_or(0, |effect| self.view.stage.alive_of_effect(effect));
        (self.view.leach_beam_effect[0], alive)
    }
}

impl RaceSim {
    /// Puts a locked LeachBeam from `owner` on `target` into the world, off the
    /// race's own weapon table: a verification aid (`--force-leach-lock`, and
    /// the disc-backed tests), and nothing the game itself calls. A slot that
    /// is not racing is left alone, as [`RaceSim::force_destroy`] leaves one, and
    /// so is a title that authors no LeachBeam; both answer `false`.
    pub fn force_leach_lock(&mut self, owner: u8, target: u8) -> bool {
        if !self.ship_active(usize::from(owner)) || !self.ship_active(usize::from(target)) {
            return false;
        }
        let Some(stats) = self
            .weapons
            .as_deref()
            .and_then(oag_tables::weapons::WeaponStats::leach_beam)
        else {
            return false;
        };
        let beam = self.locked_leach_beam(owner, target, &stats);
        self.arm_leach_beam(beam)
    }

    /// Puts craft `slot` on the first laid Bomb another craft owns, for
    /// `--force-bomb-trip`: the next tick's own trip test finds it inside the
    /// bomb's radius and detonates it by the ordinary path, so what follows
    /// (the impact, the blast models, the effects) is the simulation's own.
    /// Writes the world from outside `Race::tick` the way
    /// [`Self::force_leach_lock`] does. `false` when no such Bomb is laid or
    /// the craft is not racing. A verification aid and nothing else.
    pub fn force_bomb_trip(&mut self, slot: usize) -> bool {
        self.put_craft_on_projectile(oag_tables::weapons::Weapon::Bomb, slot)
    }

    /// Moves the player's first weapon in the air or on the road to `at`, for
    /// `--force-shot-at`: a laid Bomb or a Missile then stands where a capture of the
    /// original had it, and `--force-bomb-trip` or `--force-missile-hit` on the same
    /// tick sets it off there. `false` when the player has none out. A verification
    /// aid and nothing else.
    pub fn force_shot_at(&mut self, player: usize, at: oag_core::math::Vec3) -> bool {
        let Some(shot) = self
            .world
            .projectiles
            .slots
            .iter_mut()
            .find(|p| p.kind.is_some() && usize::from(p.owner) == player)
        else {
            return false;
        };
        shot.position = at;
        true
    }

    /// Puts craft `slot` on the first Missile another craft owns, for
    /// `--force-missile-hit`, the same way [`Self::force_bomb_trip`] does for a
    /// Bomb: the next tick's own hit test finds the craft under the missile and
    /// the ordinary impact path runs. A verification aid and nothing else.
    pub fn force_missile_hit(&mut self, slot: usize) -> bool {
        self.put_craft_on_projectile(oag_tables::weapons::Weapon::Missile, slot)
    }

    /// Puts craft `slot` on the first Rocket another craft owns, for
    /// `--force-rocket-hit`: as [`Self::force_missile_hit`], so what follows is
    /// the ordinary craft hit and its explosion. A verification aid and nothing
    /// else.
    pub fn force_rocket_hit(&mut self, slot: usize) -> bool {
        self.put_craft_on_projectile(oag_tables::weapons::Weapon::Rocket, slot)
    }

    /// Takes up to `count` of the player's Rockets out of the air, without a blast, for
    /// `--force-rocket-hit-at`: the blast it stands for is played where the flag says. Returns
    /// how many it took.
    fn retire_player_rockets(&mut self, player: usize, count: usize) -> usize {
        let mut retired = 0;
        for projectile in &mut self.world.projectiles.slots {
            if retired < count
                && projectile.kind == Some(oag_tables::weapons::Weapon::Rocket)
                && usize::from(projectile.owner) == player
            {
                *projectile = oag_weapons::projectile::Projectile::default();
                retired += 1;
            }
        }
        retired
    }
}

impl SimRace {
    /// Launches craft `slot`'s Quake with its bump centred on the player, for
    /// `--force-quake`: the wave reaches the player on the next tick and the
    /// ordinary hit path runs (the cue, the lift, the slowdown, the hull's
    /// sparks and the shake). The original's Quake is launched
    /// [`oag_weapons::projectile::quake::LAUNCH_LEAD`] ahead of the shooter, so
    /// it starts that far behind the player's progress here. **Chosen, not
    /// measured:** a shooter sitting on the player's own road, where a real one
    /// is behind and the wave takes a second or two to arrive. `false` when
    /// `slot` is the player or not racing, a wave is already up, the title
    /// authors no Quake or the player is not yet on the course. A verification
    /// aid and nothing else.
    pub fn force_quake(&mut self, slot: usize) -> bool {
        let player = self.player_slot();
        if slot == player || !self.sim.ship_active(slot) || self.sim.world.quake.is_some() {
            return false;
        }
        let Some(stats) = self
            .sim
            .weapons
            .as_deref()
            .and_then(oag_tables::weapons::WeaponStats::quake)
        else {
            return false;
        };
        let Some(progress) = self.sim.world.ships[player].standing.progress else {
            return false;
        };
        self.sim.world.quake = Some(oag_weapons::projectile::quake::Wave::launch(
            slot as u8,
            progress - oag_weapons::projectile::quake::LAUNCH_LEAD,
            1.0,
            &stats,
        ));
        true
    }
}

impl RaceSim {
    fn put_craft_on_projectile(&mut self, kind: oag_tables::weapons::Weapon, slot: usize) -> bool {
        if !self.ship_active(slot) {
            return false;
        }
        let Some(at) = self
            .world
            .projectiles
            .slots
            .iter()
            .find(|p| p.kind == Some(kind) && usize::from(p.owner) != slot)
            .map(|p| p.position)
        else {
            return false;
        };
        self.world.ships[slot].physics.body.position = at;
        true
    }
}

impl Race {
    /// Gives the craft a forced hit strikes this velocity, in world units a second, for
    /// `--force-rocket-hit-velocity`. The craft takes it now, and every craft blast on `slot` from
    /// here carries exactly it: the tick that lands the hit changes the craft's velocity before the
    /// blast reads it. A craft that is not racing is left alone. A verification aid and nothing
    /// else.
    pub fn force_struck_velocity(&mut self, slot: usize, velocity: oag_core::math::Vec3) {
        if self.core.sim.ship_active(slot) {
            self.core.sim.world.ships[slot].physics.body.linear_velocity = velocity;
            self.view.forced_blast_velocity = Some((slot, velocity));
        }
    }

    /// `count` Rockets striking a craft at `at`, for `--force-rocket-hit-at`: that many craft
    /// blasts (each with its screen flash) stand at that exact world point, and as many of the
    /// player's Rockets in flight are taken out. No craft is struck, so there is no damage,
    /// impulse or cue. `velocity` is what the blasts carry on a title whose spawn writes the
    /// struck craft's (`oag_title::Effects::craft_blast_carry`). Returns how many Rockets it
    /// took, or `None` when the race loaded no craft blast. A verification aid and nothing else.
    pub fn force_rocket_hit_at(
        &mut self,
        at: oag_core::math::Vec3,
        count: usize,
        velocity: Option<oag_core::math::Vec3>,
    ) -> Option<usize> {
        self.view.handles.get(Trigger::CraftBlast)?;
        let player = self.core.player_slot();
        let retired = self.core.sim.retire_player_rockets(player, count);
        let carried = velocity.filter(|_| self.view.craft_blast_carry);
        for _ in 0..count {
            self.view.flash_and_play(
                flash_for(oag_tables::weapons::Weapon::Rocket, true),
                Trigger::CraftBlast,
                at,
                carried,
                0.0,
            );
        }
        Some(retired)
    }

    /// Restart the particle stage's random stream from `seed` instead of
    /// [`crate::STAGE_SEED`], so a test can hold a picture over several arrangements of the
    /// same effects rather than the one the fixed seed deals.
    pub fn reseed_stage(&mut self, seed: u64) {
        self.view.stage_rng = oag_core::Rng::new(seed);
    }

    /// The loaded particle effect a trigger plays, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn effect_for_tests(
        &self,
        trigger: Trigger,
    ) -> Option<std::sync::Arc<oag_fx::psys::Effect>> {
        self.view.handles.get(trigger).cloned()
    }
}
