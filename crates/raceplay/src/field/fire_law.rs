//! When an opponent fires the forward weapon it holds: the original's law,
//! measured and handed to `oag_ai::weapon_ai`.
//!
//! The law is `oag_ai`'s; what lives here is everything it reads off the race -
//! the gaps, the player, the craft in the shot's path, the authored odds - and
//! the switch between it and this project's older rule, which a race without
//! `WeaponAIstats.xml` still runs.

use super::*;
use oag_ai::weapon_ai::{Mover, Odds, Situation, target_in_path};
use oag_tables::weapons::Weapon;

/// Which rule decides when an opponent fires a forward weapon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireLaw {
    /// `oag_ai::Driver::wants_to_fire`: this project's own gate - a craft
    /// noticed ahead inside 200 units and a cone, straight road between - and
    /// a per-tick trigger roll. **Chosen, not measured.** What a race runs when
    /// its title names no `WeaponAIstats.xml` or the file did not read.
    Ours,
    /// `oag_ai::weapon_ai`: `WeaponAi_DecideFireOrAbsorb`'s fire half, read
    /// off `BOOT.BIN` (`docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`).
    /// The default wherever the odds it needs were read.
    Original,
}

impl Race {
    /// Overrides the fire law, for the sweep that measures it. `Original`
    /// without a weapon-AI table to read falls back to `Ours`.
    pub fn set_fire_law(&mut self, law: FireLaw) {
        self.sim.fire_law = law;
    }

    /// Which fire law this race runs.
    #[must_use]
    pub fn fire_law(&self) -> FireLaw {
        if self.sim.weapon_ai.is_some() {
            self.sim.fire_law
        } else {
            FireLaw::Ours
        }
    }

    /// Advances `slot`'s weapon-AI clocks and answers whether its forward
    /// weapon fires this tick. **Once a tick per opponent**, holding or not:
    /// the clocks count ticks, and a second call would count one twice.
    pub(crate) fn opponent_fires(&mut self, slot: usize, field: &oag_ai::Field) -> bool {
        let weapon = self.sim.world.ships[slot].pickup.weapon;
        self.sim.world.ships[slot].weapon_ai.tick(weapon.is_some());
        let Some(weapon) = weapon else {
            return false;
        };
        let aimed = match weapon {
            // `+0x58`: the forward weapons that leave the nose as a shot or a
            // beam. The Cannon is one too in the original, and is not here:
            // its fire byte reaches nothing (`Driver::holds_fire`).
            Weapon::Rocket
            | Weapon::Missile
            | Weapon::Plasma
            | Weapon::Shuriken
            | Weapon::LeachBeam
            // `WeaponAi_Update`'s switch puts id 11 with these, "ahead" and
            // `+0x58` both set - `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`.
            | Weapon::Repulser => true,
            // Flagged "ahead" and not aimed: fired on the roll alone.
            Weapon::Quake => false,
            _ => return false,
        };
        if self.fire_law() == FireLaw::Ours {
            return self.wants_to_fire(slot, field);
        }
        // **Chosen, not measured**: a craft that is not racing does not fire.
        // The original's own guard is `entity+0xae4`, which is not read.
        if self.sim.world.ships[slot].physics.craft_state != oag_physics::CraftState::Racing {
            return false;
        }
        let Some(situation) = self.fire_situation(slot, weapon, aimed) else {
            return false;
        };
        let ship = &mut self.sim.world.ships[slot];
        let driver = ship.driver;
        ship.weapon_ai.decide_for(&driver, &situation)
    }

    /// [`FireLaw::Ours`]: `oag_ai::Driver::wants_to_fire`.
    fn wants_to_fire(&self, slot: usize, field: &oag_ai::Field) -> bool {
        let context = oag_ai::Context {
            line: self.line_of(slot),
            tuning: &self.sim.ai_tuning,
            pilot: &self.sim.ai_pilots[slot],
            field,
            yaw_ceiling: None,
            plan: None,
        };
        self.sim.world.ships[slot]
            .driver
            .wants_to_fire(&context)
            .is_some()
    }

    /// What `WeaponAi_ScanTraffic` (`0x08850d70`) and
    /// `WeaponAi_FindTargetInPath` (`0x08850edc`) measure, for `slot` holding
    /// `weapon`. `None` without a course, or without a row of odds for it.
    fn fire_situation(&self, slot: usize, weapon: Weapon, aimed: bool) -> Option<Situation> {
        let odds = self.sim.weapon_ai.as_ref()?.get(weapon)?;
        let course = self.sim.course.as_ref()?;
        let length = course.length();
        let count = self.sim.world.ship_count as usize;
        let ships = &self.sim.world.ships[..count];
        let mine = &ships[slot];
        let my_distance = mine.standing.distance(course);
        let (mut gap_ahead, mut gap_behind, mut player_gap) = (None::<f32>, None::<f32>, None);
        // Every other craft, whatever its state, as the scan's own loop is.
        for (other, them) in ships.iter().enumerate() {
            if other == slot || !them.active {
                continue;
            }
            let gap = fold(them.standing.distance(course) - my_distance, length);
            if other == self.player_slot() {
                player_gap = Some(gap);
            }
            if gap > 0.0 && gap_ahead.is_none_or(|best| gap < best) {
                gap_ahead = Some(gap);
            }
            if gap < 0.0 && gap_behind.is_none_or(|best| -gap < best) {
                gap_behind = Some(-gap);
            }
        }
        let body = &mine.physics.body;
        // The path test's candidates are `Race_CountLiveCrafts`' list: neither
        // destroyed nor retired, which is a racing craft here.
        let target_in_path = self.shot_speed(weapon).is_some_and(|speed| {
            target_in_path(
                body.position,
                body.up(),
                body.forward(),
                speed,
                ships.iter().enumerate().filter_map(|(other, them)| {
                    (other != slot
                        && them.active
                        && them.physics.craft_state == oag_physics::CraftState::Racing)
                        .then_some(Mover {
                            position: them.physics.body.position,
                            velocity: them.physics.body.linear_velocity,
                        })
                }),
            )
        });
        Some(Situation {
            aimed,
            eliminator: self.sim.world.mode() == oag_race::Mode::Eliminator,
            gap_ahead,
            gap_behind,
            player_gap,
            target_in_path,
            odds: Odds {
                use_against_player: odds.use_against_player,
                use_against_ai: odds.use_against_ai,
            },
        })
    }

    /// The speed `WeaponAi_FindTargetInPath` predicts `weapon`'s shot at: the
    /// Missile's per-class figure for the Missile (`0x0885a0e4`), the Plasma's
    /// for the Plasma (`Plasma_ClassSpeed`) and the Rocket's for everything
    /// else (`0x0885d22c`) - as authored, with no `/ 3.6`.
    fn shot_speed(&self, weapon: Weapon) -> Option<f32> {
        let weapons = self.sim.weapons.as_ref()?;
        let class = &self.sim.class;
        match weapon {
            Weapon::Missile => weapons.missile()?.speed_for_named(class),
            Weapon::Plasma => weapons.plasma()?.speed_for_named(class),
            _ => weapons.rocket()?.speed_for_named(class),
        }
    }
}

/// A signed along-track gap folded into half a lap either way, as
/// `0x0883d3c0` folds the original's.
fn fold(mut gap: f32, length: f32) -> f32 {
    if length <= 0.0 {
        return gap;
    }
    while gap > length * 0.5 {
        gap -= length;
    }
    while gap < -(length * 0.5) {
        gap += length;
    }
    gap
}
