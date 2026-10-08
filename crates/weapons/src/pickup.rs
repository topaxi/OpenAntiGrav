//! What a `Weapon Pad` hands out, and what a craft does with it.
//!
//! The pad's trigger is recovered and lives in the composition root beside the
//! speed pad's (it needs the track's volumes). Here: which weapon a crossing
//! grants, and the inventory that holds it. `docs/gameplay/pickups.md` carries
//! the split below with its evidence.
//!
//! # What is recovered and what is ours
//!
//! **Recovered.** `<Pickupodds class="...">` weights every weapon per speed
//! class and separately for `ai`, `human`, `front` and `back` (confidence 92,
//! `docs/formats/weapon-stats.md`). `WeaponPickup_Grant` (`0x08861d20`, called by
//! `Weapons_DispatchFire` on the pad-crossing flag after a `WEAPONPICKUP` cue):
//!
//! - A crossing grants anything at all, confidence 85.
//! - The draw: `rand() % total` then a cumulative walk, `ai` spent flat, `human`
//!   blended with `front`/`back` by race position, and no same weapon twice
//!   running. Confidence 92; see [`draw`] and [`Driver`].
//! - One inventory slot, confidence 88: `craft+0x1bc` holds a weapon id, `-1`
//!   empty, which is [`Held`].
//!
//! **Ours.**
//!
//! - The sequence of draws. The original's PRNG is open
//!   (`docs/overview/roadmap.md`), so only the distribution can be checked, as
//!   [`tests`] does.
//! - The bounded retry behind the no-repeat rule. See [`REDRAW_ATTEMPTS`].
//! - What an unplaced craft draws (the `human` column alone). See
//!   [`Driver::descent`].
//! - What Shield and Rocket do. The durations, speeds, radii and damage are the
//!   disc's; Shield's `time` joins to no recovered code path. See
//!   [`oag_physics::ShipState::shield_pickup_timer`], [`crate::projectile`] and
//!   `docs/gameplay/pickups.md`.
//! - Aiming: nothing picks a target, so a hit depends on where the craft points.
//!
//! # Only what has an effect is handed out
//!
//! [`IMPLEMENTED`] is the pool a pad draws from. Every weapon a shipped table
//! weights is drawable. Pure's Disruptor is in it, but a Pulse or HD pad never
//! draws it: [`Driver::weight`] answers `0.0` for a weapon a table does not
//! author. The shipped tables give the Repulser zero odds outside Eliminator.
//! The distribution a player sees is the authored one conditioned on the
//! implemented set; adding a weapon to [`IMPLEMENTED`] is the whole change.
//!
//! What let each weapon in was usually a reading, not a mechanic:
//!
//! - **Cannon.** Not driven by `Weapon_RequestFire`'s bit system but by the fire
//!   button being **held**: `Cannon_UpdateReload` (`0x0883f424`) advances a
//!   per-craft countdown built from the authored `rate` on each frame the button
//!   is down and arms the spawn bit at zero. A first reading took
//!   `*(*(entity+0x94)+0x78) + 0x16` for a track weapon-pad flag and had it
//!   self-firing; it is the craft's control record (`player_input`) and `+0x16`
//!   is the held state of `OPT_CTRL_FIRE`. `Race::advance_cannons` ports the
//!   countdown; see [`crate::projectile::cannon`] and
//!   `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
//! - **Plasma.** `Plasma_Update` (`0x0885c6cc`) is `Rocket_Update`'s floor
//!   follower (12-unit probe, speed-preserving redirect, fall, detonate on a
//!   wall); `Weapon_FirePlasma` (`0x0886a868`) spawns one where the Rocket spawns
//!   three. See `docs/ghidra/functions/psp-pulse-usa/plasma.md`.
//! - **Shuriken.** `Shuriken_Update` (`0x08877bdc`) is the same floor follower,
//!   except a wall bounces the blade. The trajectory is the expensive part of a
//!   weapon and has to be read, not inferred from a constructor. See
//!   `docs/ghidra/functions/psp-pulse-usa/shuriken.md`.
//! - **Mine.** Two pages had the Mine's fire handler down as the Cannon's and
//!   the Bomb's as the Mine's, because `Weapon_RequestFire`'s jump table holds
//!   those entries out of address order. `Mine_Init` (`0x08859ac8`) plays
//!   `MINELAUNCH` and loads `Data\\Weapons\\Pulse_Mine.vex`, which settles it. See
//!   `docs/ghidra/functions/psp-pulse-usa/mine.md` and
//!   [`crate::projectile::mine`].
//! - **Bomb.** One bigger charge out of the Mine's rear anchor; its `<Stats>`
//!   are the Mine's six one size up, and `Weapon_FireBomb` (`0x08863a20`) spawns
//!   once where `Weapon_DropMines` reloads. A drop is
//!   [`crate::projectile::mine::CLUSTER`] mines one every
//!   [`crate::projectile::mine::DROP_INTERVAL`], and the craft keeps the pickup
//!   until the last is out, which is why [`Held`] carries two counters.
//! - **Autopilot.** `Autopilot_Fire` (`0x088613bc`) and `Autopilot_Update`
//!   (`0x08861404`) give the duration, running bit, countdown, the one-second
//!   `disengaging` warning, and that pressing fire cancels it
//!   (`docs/ghidra/functions/psp-pulse-usa/autopilot.md`). Ours: the takeover
//!   itself (the original hands the craft to the driver `Ai_Construct` names
//!   `"autopilot input"`; the swap was not found), so this reuses
//!   `oag_ai::Driver` as `Race::set_autopilot` did.
//! - **Missile.** Lock from `Ship_AcquireLock` (`0x08844784`), guidance from
//!   `Missile_Update` (`0x0885a918`); see [`crate::projectile::missile`].

use oag_core::Rng;
use oag_tables::weapons::{PickupTable, Weapon, WeaponStats};

/// The weapons a pad in this engine can hand out.
///
/// The shipped table weights thirteen; this is the subset with an effect. A
/// slice because it grows, unlike [`oag_race::Mode::ALL`].
pub const IMPLEMENTED: &[Weapon] = &[
    Weapon::Turbo,
    Weapon::Shield,
    Weapon::Rocket,
    Weapon::Missile,
    Weapon::Autopilot,
    Weapon::Mine,
    Weapon::Bomb,
    Weapon::Plasma,
    Weapon::Shuriken,
    Weapon::Cannon,
    Weapon::Quake,
    Weapon::LeachBeam,
    Weapon::Disruptor,
    Weapon::Repulser,
];

/// Which column of `<Pickupodds>` a craft draws from, and how its place bends it.
///
/// `ai` is spent flat; `human` is blended with `front` and `back` by race place,
/// as `WeaponPickup_Grant` (`0x08861d20`) does. The pair that rubber-bands is
/// the player's, not the AI's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Driver {
    /// The player, whose odds bend with their place in the race.
    Human {
        /// One-based place, or `0` for a craft not yet placed.
        place: u8,
        /// How many craft are racing.
        field: u8,
    },
    /// An AI-driven craft, which draws the `ai` column flat.
    Ai,
}

impl Default for Driver {
    /// The player, unplaced. A caller that never set a place keeps the `human`
    /// column alone rather than blending against place zero.
    fn default() -> Self {
        Self::HUMAN_UNPLACED
    }
}

impl Driver {
    /// The player, unplaced - the `human` column with no blend.
    pub const HUMAN_UNPLACED: Self = Self::Human { place: 0, field: 0 };

    /// How far down the field this driver is, `0.0` at the front.
    ///
    /// `None` when there is no place to blend against, in which case
    /// [`Self::weight`] spends `human` alone (**ours**; the original always has a
    /// placed craft). The divisor is the whole field, not `field - 1`, as in the
    /// original: last place gets `(field - 1) / field` of the way to `back`.
    #[must_use]
    fn descent(place: u8, field: u8) -> Option<f32> {
        if place == 0 || field == 0 {
            return None;
        }
        Some(f32::from(place - 1) / f32::from(field))
    }

    /// This driver's weight for one weapon, from one class's table. Zero for a
    /// weapon the class authors no odds for.
    ///
    /// The blend is `human + back * t + front * (1 - t)`, recovered whole, `t` being
    /// `0` for the leader. The shipped Venom table gives Shield `front="2" back="0"`
    /// and Turbo `back="2" front="0"`: catch-up aimed at the player.
    #[must_use]
    fn weight(self, table: &PickupTable, weapon: Weapon) -> f32 {
        let Some(odds) = table.get(weapon) else {
            return 0.0;
        };
        match self {
            Self::Ai => odds.ai,
            Self::Human { place, field } => match Self::descent(place, field) {
                Some(t) => odds.human + odds.back * t + odds.front * (1.0 - t),
                None => odds.human,
            },
        }
    }
}

/// What a craft is carrying.
///
/// One slot, the conservative reading: the original's inventory is a flag word
/// and whether it holds two is unread (`SubWeapon` exists as a HUD widget).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Held {
    /// The weapon in the slot, or `None`.
    pub weapon: Option<Weapon>,
    /// The last weapon this craft was handed, whether or not it still has it.
    ///
    /// The original keeps `craft+0x1bc`, which every fire handler clears to `-1`,
    /// and `craft+0x1c0`, which it does not; the grant reads the second to refuse
    /// repeats (see [`draw`]). Set by [`Self::grant`], never cleared, and hashed.
    pub last: Option<Weapon>,
    /// How many mines are still to be laid from the drop in progress.
    ///
    /// **Recovered**, confidence 90: `craft+0x1ac`, decremented once per spawn by
    /// `Weapon_DropMines` (`0x088675cc`); the craft holds the weapon until zero. The
    /// start value is [`crate::projectile::mine::CLUSTER`]. The original arms it at
    /// grant time (`WeaponPickup_ArmMine`), this engine at press time; nothing reads
    /// it between. Zero when not mid-drop.
    pub dropping: u8,
    /// Seconds until the next mine of a drop leaves: the original's `craft+0x1b0`,
    /// reloaded with a literal `0.1` after every spawn. Held at zero when
    /// [`Self::dropping`] is zero so two worlds with no drop hash the same.
    pub drop_reload: f32,
    /// Rounds still to leave the barrel, or `0` for a craft not holding a Cannon.
    ///
    /// **Recovered as a mechanism**, confidence 85: `craft+0x154` starts at
    /// `<Weapon type="Cannon"><Stats rounds>` and counts down per round;
    /// `Cannon_UpdateReload` clears the held slot as it reaches zero. See
    /// [`Self::advance_cannon_reload`] and
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
    ///
    /// **Self-arming rather than armed at grant time: chosen, not measured.** A pad
    /// crossing, `--give` and a test setting [`Self::weapon`] are three ways the slot
    /// fills, so reading `0` as "not yet armed" on the first
    /// [`Self::advance_cannon_reload`] arms all three alike. Safe because a spent
    /// magazine clears [`Self::weapon`] in the same call.
    pub cannon_rounds: u8,
    /// Seconds until the Cannon's next round: the original's `craft+0x158`,
    /// decremented every tick and reloaded with `<Stats rate>` when negative. Held at
    /// zero when [`Self::cannon_rounds`] is, so two worlds hash the same.
    pub cannon_reload: f32,
}

impl Held {
    /// Nothing held, nothing remembered and nothing being laid.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            weapon: None,
            last: None,
            dropping: 0,
            drop_reload: 0.0,
            cannon_rounds: 0,
            cannon_reload: 0.0,
        }
    }

    /// Whether a cluster is still coming out of the back of this craft.
    #[must_use]
    pub const fn is_dropping(&self) -> bool {
        self.dropping > 0
    }

    /// Starts a drop of `count` mines, the first of them on this very tick.
    ///
    /// The first leaves immediately, recovered, confidence 92:
    /// `WeaponPickup_ArmMine` (`0x0886759c`) zeroes `craft+0x1b0` at grant, and the
    /// original's live trail shows the pool count going `0` to `1` on the frame
    /// `Weapon_DropMines` first runs (`docs/ghidra/functions/psp-pulse-usa/mine.md`,
    /// 2026-09-15 section).
    ///
    /// A no-op while a cluster is coming out, recovered, confidence 88: the only
    /// writer of the round counter besides the handler's decrement is that arm
    /// function, reachable only from `WeaponPickup_Grant`. Without the guard a
    /// mashed fire button reset [`Self::drop_reload`] on every press, so
    /// [`Self::dropping`] never reached zero and one extra mine left per press.
    /// A `search_instructions` sweep of `sw` to `craft+0x1ac` missed the arming
    /// store; a runtime write watch caught it on the grant path.
    pub const fn begin_drop(&mut self, count: u8) {
        if self.is_dropping() {
            return;
        }
        self.dropping = count;
        self.drop_reload = 0.0;
    }

    /// Counts one tick off the drop and says whether a mine leaves now.
    ///
    /// Clears the held slot when the last is out, as `Weapon_DropMines` writes
    /// `-1` into `craft+0x1bc` in the branch that sees the counter reach zero.
    pub fn advance_drop(&mut self, dt: f32, interval: f32) -> bool {
        if self.dropping == 0 {
            return false;
        }
        self.drop_reload -= dt;
        if self.drop_reload > 0.0 {
            return false;
        }
        self.drop_reload = interval;
        self.dropping -= 1;
        if self.dropping == 0 {
            self.weapon = None;
            self.drop_reload = 0.0;
        }
        true
    }

    /// Whether the slot can take a pickup.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.weapon.is_none()
    }

    /// Counts one tick off the Cannon's reload timer; says whether a round leaves
    /// now and the magazine left after it.
    ///
    /// `full` is `<Stats rounds>`, passed every call rather than cached (see
    /// [`Self::cannon_rounds`]); `rate` is `<Stats rate>`, read as
    /// `oag_tables::weapons::CannonStats::rate` argues. The count is after this
    /// round, the order `Weapon_FireCannon` reads it in, before the dispatch
    /// loop reads `craft->shots & 1` to pick a muzzle (see
    /// `oag_weapons::projectile::cannon::launch`).
    ///
    /// **The first round leaves immediately: chosen, not measured**, for
    /// [`Self::begin_drop`]'s reason.
    ///
    /// The caller (`Race::advance_cannons`) calls only while fire is held:
    /// `Cannon_UpdateReload` returns before touching `craft+0x158` when the
    /// button is up, so the countdown pauses rather than resets.
    pub fn advance_cannon_reload(&mut self, dt: f32, rate: f32, full: u8) -> Option<u8> {
        if full == 0 {
            // Degenerate authored data: a zero-round Cannon. Clear rather than
            // re-arm every call.
            self.weapon = None;
            self.cannon_rounds = 0;
            self.cannon_reload = 0.0;
            return None;
        }
        if self.cannon_rounds == 0 {
            self.cannon_rounds = full;
            self.cannon_reload = 0.0;
        }
        self.cannon_reload -= dt;
        if self.cannon_reload >= 0.0 {
            return None;
        }
        self.cannon_reload += rate;
        self.cannon_rounds -= 1;
        let remaining = self.cannon_rounds;
        if remaining == 0 {
            self.weapon = None;
            self.cannon_reload = 0.0;
        }
        Some(remaining)
    }

    /// Fills the slot and remembers what went in it. [`draw`] needs the previous
    /// grant, so a caller must not set `weapon` directly to be remembered.
    pub const fn grant(&mut self, weapon: Weapon) {
        self.weapon = Some(weapon);
        self.last = Some(weapon);
    }

    /// Takes what is held, leaving the slot empty. [`Self::last`] survives, so
    /// the no-repeat rule outlasts firing.
    ///
    /// **Also ends a drop in progress: chosen, not measured.** Absorbing
    /// (`CIRCLE`) a Mine mid-cluster would leave [`Self::dropping`] stuck, so
    /// `Race::lay_mines` does nothing forever and a later grant is swallowed by
    /// [`Self::begin_drop`]'s guard. Nothing pins what the original does.
    pub const fn take(&mut self) -> Option<Weapon> {
        self.dropping = 0;
        self.drop_reload = 0.0;
        // Same for the Cannon, so worlds that gave one up mid-magazine hash alike.
        self.cannon_rounds = 0;
        self.cannon_reload = 0.0;
        self.weapon.take()
    }
}

/// Draws one weapon from a class's authored odds, restricted to [`IMPLEMENTED`].
///
/// `None` when the table weights none of them above zero. The walk is over
/// [`IMPLEMENTED`]'s order, not the table's, so the sequence depends only on
/// this crate and the seed.
///
/// # It will not hand out `last` twice running
///
/// **Recovered.** `WeaponPickup_Grant` (`0x08861d20`) re-rolls a draw equal to
/// the craft's previous grant. Pass [`Held::last`].
///
/// **The retry is bounded here and not in the original**, which loops until it
/// draws something different; a table weighting one weapon would spin forever.
/// After [`REDRAW_ATTEMPTS`] the repeat is accepted. A weapon with share `p` of
/// the live weight repeats with probability `p^REDRAW_ATTEMPTS`: on the shipped
/// Venom weights, Turbo at 14 of 48, about one grant in twenty thousand.
/// `an_overwhelming_weight_terminates_and_may_repeat` pins the worst case.
///
/// `allowed` restricts the draw to a subset of [`IMPLEMENTED`]: `None` draws from
/// all, and `Some(&[])` hands out nothing rather than falling back to
/// unrestricted (authored absence is honest; `docs/formats/2048-campaign.md`'s
/// weapon-set gate). The one `Some` caller reads
/// `oag_tables::mjolnir::campaign::WeaponSet::allowed_weapons`.
#[must_use]
pub fn draw(
    rng: &mut Rng,
    table: &PickupTable,
    driver: Driver,
    last: Option<Weapon>,
    allowed: Option<&[Weapon]>,
) -> Option<Weapon> {
    for _ in 0..REDRAW_ATTEMPTS {
        let drawn = draw_once(rng, table, driver, allowed)?;
        if Some(drawn) != last {
            return Some(drawn);
        }
    }
    // Hand over the repeat rather than nothing: a silent pad reads as broken.
    draw_once(rng, table, driver, allowed)
}

/// How many times [`draw`] re-rolls to avoid repeating the last pickup.
///
/// **Ours.** Eight repeats about once in twenty thousand grants on the shipped
/// odds and spends at most nine draws.
pub const REDRAW_ATTEMPTS: usize = 8;

/// A weapon's weight for this draw: the driver/table's own, or `0.0` when `allowed`
/// is `Some` and does not list it.
#[must_use]
fn gated_weight(
    driver: Driver,
    table: &PickupTable,
    weapon: Weapon,
    allowed: Option<&[Weapon]>,
) -> f32 {
    if allowed.is_some_and(|allowed| !allowed.contains(&weapon)) {
        return 0.0;
    }
    driver.weight(table, weapon).max(0.0)
}

/// One weighted draw, with no regard for what came before it.
#[must_use]
fn draw_once(
    rng: &mut Rng,
    table: &PickupTable,
    driver: Driver,
    allowed: Option<&[Weapon]>,
) -> Option<Weapon> {
    let total: f32 = IMPLEMENTED
        .iter()
        .map(|&weapon| gated_weight(driver, table, weapon, allowed))
        .sum();
    if total <= 0.0 || !total.is_finite() {
        return None;
    }

    let mut roll = rng.next_f32() * total;
    for &weapon in IMPLEMENTED {
        let weight = gated_weight(driver, table, weapon, allowed);
        if weight <= 0.0 {
            continue;
        }
        roll -= weight;
        if roll < 0.0 {
            return Some(weapon);
        }
    }
    // Float slack only (`next_f32` is half-open on `1.0`); the last weighted
    // weapon is the right answer.
    IMPLEMENTED
        .iter()
        .rev()
        .copied()
        .find(|&weapon| gated_weight(driver, table, weapon, allowed) > 0.0)
}

/// The class's own table out of a whole weapon file.
///
/// `HandlingStats.xml` authors `<GlobalClass name="VENOM">` and
/// `WeaponStats_Race.xml` `<Pickupodds class="Venom">` (measured on the USA disc),
/// so [`SpeedClass::as_str`] cannot reach the table through
/// [`WeaponStats::pickups_for`]. Matched case-insensitively here, as the
/// difference is between two documents, not two concepts.
///
/// The rung arrives as a name, so a non-Pulse ladder works: Pure authors a
/// `Vector` class beside its four; Pulse has none, so that lookup is `None`.
///
///
/// [`SpeedClass`]: oag_tables::handling::SpeedClass
/// [`SpeedClass::as_str`]: oag_tables::handling::SpeedClass::as_str
#[must_use]
pub fn table_for<'a>(stats: &'a WeaponStats, class: &str) -> Option<&'a PickupTable> {
    stats
        .pickups
        .iter()
        .find(|table| table.class.eq_ignore_ascii_case(class))
}

#[cfg(test)]
mod tests;
