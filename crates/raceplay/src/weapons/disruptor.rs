//! Firing Pure's Disruptor, for the player and for an opponent.
//!
//! Its own file rather than two more arms in `weapons.rs` and
//! `field/opponent_weapons.rs`, because the two share everything but the
//! trigger: the roll, the lock and the spawn are one function, and the
//! dispatch chains call it. See `oag_weapons::projectile::disruptor` for the
//! bolt and `oag_weapons::disruption` for what a hit does; the reading is
//! `docs/ghidra/functions/psp-pure-usa/weapons.md`.

use super::*;

impl Race {
    /// Fires one Disruptor from `slot`, returning whether a bolt left.
    ///
    /// # The effect is rolled here, and the original rolls it at the pad
    ///
    /// `WeaponPickup_ArmDisruptor` (`0x0884f9ec`) rolls `rand() % 8` when the
    /// pad hands the weapon over and parks the kind on the firing craft;
    /// `Disruptor_Init` copies it onto the bolt. This rolls the same eight
    /// ways, from the same seeded generator, **at the press instead** - a
    /// deviation, recorded as one. It is invisible to a player (nothing on
    /// Pure's HUD names the kind before it lands) and it keeps the roll out of
    /// `oag_weapons::pickup::Held`, which `--give` fills with no generator to
    /// hand. `Disruptor_RollEffect`'s own switch order is
    /// `DisruptorEffectKind::ROLLED`.
    ///
    /// # The lock is the Missile's, run at the press
    ///
    /// `DisruptorPool_Fire` passes `craft+0x194` - the lock the sight keeps -
    /// and a locked bolt homes. This engine's sight does not run for a
    /// Disruptor (`Race::sight_held` answers `None`), so the same lock test
    /// `Race::fire_missile` runs is run here for both the player and an
    /// opponent, without the reticle hold; `None` is a bolt that flies
    /// straight, which is what the original fires with no lock. The window is
    /// the Missile's authored one, and a table with no Missile block has no
    /// window, so its Disruptor never homes - which no shipped Pure table is.
    pub fn fire_disruptor(&mut self, slot: usize) -> bool {
        if self
            .sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::disruptor)
            .is_none()
        {
            // No authored Disruptor: nothing to put in the air, and the
            // pickup is kept, the rule every projectile arm follows.
            return false;
        }
        let count = self.sim.world.ship_count as usize;
        let ship = &self.sim.world.ships[slot];
        let (position, velocity, up) =
            oag_weapons::projectile::disruptor::launch(&ship.physics, &ship.handling.dimensions);
        let missile = self
            .sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::missile);
        let target = missile.and_then(|missile| {
            oag_weapons::projectile::missile::lock(
                &self.sim.world.ships[..count],
                slot as u8,
                position,
                ship.physics.body.forward(),
                &missile,
                self.sim.course.as_ref().map(oag_race::Course::length),
            )
        });
        let roll = self.sim.world.rng.next_u32() % 8;
        let effect = oag_tables::weapons::DisruptorEffectKind::ROLLED[roll as usize];
        self.sim
            .world
            .projectiles
            .fire_disruptor(position, velocity, up, slot as u8, target, effect)
    }

    /// Fires an opponent's Disruptor, if its driver wants to: two gates, the
    /// driver's `oag_ai::Driver::wants_to_fire` and then the weapon's own lock,
    /// the way `fire_opponent_missile`'s two used to be. **Not on the original's
    /// fire law**: the Disruptor is Pure's, and the law is read off Pulse, which
    /// has no Disruptor row to read.
    pub(crate) fn fire_opponent_disruptor(&mut self, slot: usize, field: &oag_ai::Field) -> bool {
        let context = oag_ai::Context {
            line: self.line_of(slot),
            tuning: &self.sim.ai_tuning,
            pilot: &self.sim.ai_pilots[slot],
            field,
            yaw_ceiling: None,
            plan: None,
        };
        if self.sim.world.ships[slot]
            .driver
            .wants_to_fire(&context)
            .is_none()
        {
            return false;
        }
        self.fire_disruptor(slot)
    }

    /// The controls `slot` actually gets this tick, after its disruption.
    ///
    /// `ai_driven` is whether an AI produced `controls` - see
    /// `oag_weapons::disruption::Disruption::filter` for why the Stall asks.
    /// An Autopilot effect's thrust scale is applied here too, so the two
    /// call sites - the player's in `Race::tick`, the field's in
    /// `Race::step_opponents` - are one line each.
    pub(crate) fn disrupted_controls(
        &self,
        slot: usize,
        controls: oag_physics::ShipControls,
        ai_driven: bool,
    ) -> oag_physics::ShipControls {
        let disruption = self.sim.world.ships[slot].disruption;
        let mut controls = disruption.filter(controls, ai_driven);
        if let Some(scale) = disruption.autopilot_thrust_scale() {
            controls.thrust *= scale;
        }
        controls
    }
}
