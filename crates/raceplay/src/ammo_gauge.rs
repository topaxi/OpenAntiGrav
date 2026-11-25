//! What the Detonator craft's cannon gauge shows: the fill and colour `Detonator_UpdateHud` writes
//! into the hull's `detonator_bulletgaugesegments` block
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-ammo-gauge.md`).

use oag_title::DetonatorAmmoGauge;
use oag_weapons::detonator::cannon::Cannon;

/// The gauge's two constants as the block holds them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shown {
    /// `StandardColour`.
    pub colour: [f32; 3],
    /// `StandardBullets`: the lit share of the segments, `0..=1`.
    pub fill: f32,
}

impl Shown {
    /// What the block holds from the hull's bind until the first update.
    #[must_use]
    pub fn bind(gauge: &DetonatorAmmoGauge) -> Self {
        Self {
            colour: gauge.bind.0,
            fill: gauge.bind.1,
        }
    }

    /// The next frame's block. At or below the speed gate the update does not run and the block
    /// keeps what it held. Locked (`lockout > 0`), the count is the reload's refill,
    /// `trunc((1 - lockout / lockout_total) * rounds)`, in the locked colour; otherwise the clip, in
    /// the ready colour. The fill is the count times `1 / rounds`.
    #[must_use]
    pub fn step(self, gauge: &DetonatorAmmoGauge, cannon: &Cannon, speed: f32) -> Self {
        if speed <= gauge.speed_gate {
            return self;
        }
        let (count, colour) = if cannon.lockout > 0.0 {
            let refill = (1.0 - cannon.lockout / cannon.lockout_total) * gauge.rounds;
            (refill.trunc(), gauge.locked)
        } else {
            (f32::from(cannon.clip), gauge.ready)
        };
        Self {
            colour,
            fill: count * (1.0 / gauge.rounds),
        }
    }
}

impl crate::Race {
    /// Makes the gauge's speed gate read `speed` instead of the craft's (`--detonator-gauge-speed`): a
    /// comparison that holds the craft at rest still shows the gauge the original's moving craft
    /// kept updating.
    pub fn pin_ammo_gauge_speed(&self, speed: f32) {
        *crate::drawable::lock(&self.view.ammo_gauge_speed) = Some(speed);
    }

    /// The speed the gauge's gate reads: the pinned one, else the player's own.
    fn gauge_speed(&self) -> f32 {
        (*crate::drawable::lock(&self.view.ammo_gauge_speed))
            .unwrap_or_else(|| self.core.ship().physics.body.linear_velocity.length())
    }

    /// This frame's cannon gauge on the player's craft, or `None` where the title draws none (off
    /// Detonator, or a title whose law is not read). Steps the block the last frame left.
    pub(crate) fn ammo_gauge(&self) -> Option<Shown> {
        let gauge = self.view.detonator_ammo_gauge?;
        let cannon = self.core.sim.detonator_cannon()?;
        let speed = self.gauge_speed();
        let mut held = crate::drawable::lock(&self.view.ammo_gauge_shown);
        let shown = held
            .unwrap_or_else(|| Shown::bind(&gauge))
            .step(&gauge, cannon, speed);
        *held = Some(shown);
        Some(shown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HD: DetonatorAmmoGauge = oag_hd::race::DETONATOR_AMMO_GAUGE;

    fn cannon(clip: u8, lockout: f32, total: f32) -> Cannon {
        let mut cannon = Cannon::new();
        cannon.clip = clip;
        cannon.lockout = lockout;
        cannon.lockout_total = total;
        cannon
    }

    /// The grid: full and red until the craft first passes the gate (live, RPCS3, the grid state).
    #[test]
    fn the_grid_shows_the_bind_state_until_the_gate() {
        let shown = Shown::bind(&HD).step(&HD, &cannon(15, 0.0, 0.0), 3.3);
        assert_eq!(
            shown,
            Shown {
                colour: [1.0, 0.0, 0.0],
                fill: 1.0
            }
        );
    }

    /// The clip over fifteen in blue, the block's exact words as read live (RPCS3): `12` reads
    /// `0x3f4cccce`, `4` reads `0x3e888889`.
    #[test]
    fn a_ready_cannon_shows_its_clip_in_blue() {
        for (clip, bits) in [
            (15u8, 0x3f80_0000u32),
            (12, 0x3f4c_ccce),
            (4, 0x3e88_8889),
            (0, 0),
        ] {
            let shown = Shown::bind(&HD).step(&HD, &cannon(clip, 0.0, 0.0), 100.0);
            assert_eq!(shown.colour, [0.0, 0.1, 1.0]);
            assert_eq!(shown.fill.to_bits(), bits, "clip {clip}");
        }
    }

    /// A reload refills in red, in whole fifteenths, while the clip already reads full.
    #[test]
    fn a_reload_refills_in_red() {
        let shown = Shown::bind(&HD).step(&HD, &cannon(15, 0.5, 1.0), 100.0);
        assert_eq!(shown.colour, [1.0, 0.0, 0.0]);
        assert_eq!(shown.fill, 7.0 * (1.0 / 15.0));
        let start = Shown::bind(&HD).step(&HD, &cannon(15, 0.75, 0.75), 100.0);
        assert_eq!(start.fill, 0.0);
    }

    /// A seek back to the grid draws the bind state, not the gauge the race had reached.
    #[test]
    fn a_seek_forgets_the_held_gauge() {
        let mut race = crate::tests::race_with_a_grid();
        let snapshot = race.core.sim.clone();
        *crate::drawable::lock(&race.view.ammo_gauge_shown) = Some(Shown {
            colour: HD.ready,
            fill: 0.4,
        });
        race.restore_for_viewing(&snapshot);
        assert!(crate::drawable::lock(&race.view.ammo_gauge_shown).is_none());
    }

    /// A craft held at rest reads speed 0 and so never passes the gate; a comparison that pins the
    /// original's speed makes it pass.
    #[test]
    fn a_pinned_speed_replaces_a_resting_craft_s() {
        let race = crate::tests::race_with_a_grid();
        let at_rest = race.gauge_speed();
        assert!(at_rest <= HD.speed_gate, "{at_rest}");
        race.pin_ammo_gauge_speed(102.3);
        assert_eq!(race.gauge_speed(), 102.3);
    }

    /// Below the gate the block holds what the last update wrote.
    #[test]
    fn a_slow_craft_freezes_the_gauge() {
        let moving = Shown::bind(&HD).step(&HD, &cannon(7, 0.0, 0.0), 100.0);
        let stopped = moving.step(&HD, &cannon(2, 0.0, 0.0), 10.0);
        assert_eq!(stopped, moving);
    }
}
