//! What a `Weapon Pad` hands out, and what a craft does with it.
//!
//! The pad's *trigger* is recovered and lives in the composition root beside the
//! speed pad's, because it needs the track's volumes. What is here is the two
//! halves that are gameplay rather than geometry: **which** weapon a crossing
//! grants, and the inventory that holds it.
//!
//! # What is recovered and what is ours
//!
//! Stated at the top because this module is unusually mixed, and
//! `docs/gameplay/pickups.md` carries the same split with its evidence.
//!
//! **Recovered.** The odds themselves - `<Pickupodds class="...">` weights every
//! weapon per speed class and separately for `ai`, `human`, `front` and `back`
//! of the grid, at confidence 92 (`docs/formats/weapon-stats.md`). That table
//! *is* the pickup design, and this module spends it rather than inventing a
//! distribution.
//!
//! **Also recovered as of 2026-08-17**, and these three rows used to say "ours"
//! on the grounds that no grant existed to read. `WeaponPickup_Grant`
//! (`0x08861d20`) does exist - `Weapons_DispatchFire` calls it on the
//! pad-crossing flag, right after a `WEAPONPICKUP` cue:
//!
//! - **That a crossing grants anything at all**, confidence 85.
//! - **The draw**: `rand() % total` then a cumulative walk over `<Pickupodds>`,
//!   with the `ai` column spent flat and the `human` column blended with
//!   `front`/`back` by race position, and a refusal to hand out the same weapon
//!   twice running. Confidence 92; see [`draw`] and [`Driver`].
//! - **The inventory being one slot**, confidence 88 - `craft+0x1bc` holds a
//!   weapon id and `-1` means empty, which is exactly [`Held`].
//!
//! **Ours, and there is no way for it not to be.**
//!
//! - **The sequence of draws.** The original's PRNG is an open question on the
//!   roadmap (`docs/overview/roadmap.md`), so which weapon comes out *when*
//!   cannot match even with a byte-exact algorithm. Only the *distribution* can
//!   be checked, which is what [`tests`] does.
//! - **The bounded retry** behind the no-repeat rule; the original's loop is
//!   unbounded. See [`REDRAW_ATTEMPTS`].
//! - **What an unplaced craft draws.** The original always has a place; this
//!   spends the `human` column alone. See [`Driver::descent`].
//!
//! # Only what has an effect is handed out
//!
//! [`IMPLEMENTED`] is the pool a pad draws from: Turbo, Shield, Rocket, Missile,
//! Autopilot, Mine, Bomb, Plasma, Shuriken and, as of 2026-09-07, the Cannon.
//!
//! Of the three still out, the Quake needs its wave's own per-frame travel
//! along the track's spline (not deformation - that reading was wrong), the
//! LeachBeam needs its two drain-rate functions, and the Repulser needs a
//! field the craft *is in* rather than a projectile.
//!
//! **The Cannon left the list the same day**, and it is the odd one out among
//! everything built here so far: it does not fire through
//! `Weapon_RequestFire`'s bit system at all. `craft+0x1bc == 3` (holding it)
//! is the only gate `Cannon_UpdateReload` (`0x0883f424`) reads - not a press -
//! so a picked-up Cannon fires itself, on a per-craft reload countdown built
//! from its own authored `rate`, until its own authored `rounds` runs out.
//! `Race::advance_cannons` is the port of that countdown; see
//! [`crate::projectile::cannon`] for the round itself and
//! `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md` for the
//! whole reading, including the address correction it made to the
//! `0x088537ac` candidate this doc used to cite.
//!
//! **The Plasma left that list on 2026-09-02**, and it is the cheapest weapon
//! since the Bomb for the mirror-image reason: the Bomb reused the Mine's whole
//! module and the Plasma reuses the Rocket's whole flight model.
//! `Plasma_Update` (`0x0885c6cc`) is `Rocket_Update`'s floor follower - the same
//! 12-unit probe along the carried surface normal, the same speed-preserving
//! redirect, the same fall, the same detonate on a wall - and
//! `Weapon_FirePlasma` (`0x0886a868`) spawns exactly one where the Rocket
//! spawns three. See `docs/ghidra/functions/psp-pulse-usa/plasma.md`.
//!
//! **The Shuriken followed it the same day, and the reason it could is worth
//! recording because the first estimate said otherwise.** Judged from its
//! constructor and its bounce alone it looked like a session's work - a
//! reflection unlike the Missile's, two damage numbers, a fuse, a coin flip.
//! Then `Shuriken_Update` (`0x08877bdc`) turned out to be the *same floor
//! follower* the Rocket and the Plasma already share, differing only in that a
//! wall bounces a blade instead of ending it. **The trajectory is the expensive
//! part of a weapon here, and the trajectory has to be read rather than
//! inferred from the shape of a constructor.** See
//! `docs/ghidra/functions/psp-pulse-usa/shuriken.md`.
//!
//! **The Mine left that list on 2026-08-26**, and it is the third in a row that
//! a *reading* let through rather than a mechanic - and the first where the
//! reading was of somebody else's mistake. Two pages had this weapon's fire
//! handler down as the Cannon's and the Bomb's handler down as the Mine's,
//! because `Weapon_RequestFire`'s jump table holds its entries for those two
//! weapons out of address order. `Mine_Init` (`0x08859ac8`) plays `MINELAUNCH`
//! and loads `Data\\Weapons\\Pulse_Mine.vex`, which settles it without any
//! ordering argument at all. See
//! `docs/ghidra/functions/psp-pulse-usa/mine.md` and
//! [`crate::projectile::mine`], which carries this weapon's own split.
//!
//! **The Bomb followed it the same day**, and it is the cheapest weapon this
//! project has added: one bigger charge out of the same rear anchor, sharing
//! every line of [`crate::projectile::mine`] except a count. Its `<Stats>` are
//! the Mine's six one size up on both shipped tables, `Weapon_FireBomb`
//! (`0x08863a20`) spawns once where `Weapon_DropMines` reloads and spawns
//! again, and a maintainer who plays Pulse describes it as "a single big mine"
//! - three independent things saying the same shape.
//!
//! **It is also the first weapon whose firing is not instantaneous.** A drop is
//! [`crate::projectile::mine::CLUSTER`] mines laid one every
//! [`crate::projectile::mine::DROP_INTERVAL`], and the craft goes on holding the
//! pickup until the last one is out - which is the original's own arrangement
//! and is why [`Held`] carries the two counters rather than the fire path
//! spending the slot outright.
//!
//! **The Autopilot left that list on 2026-08-24**, and like the Missile before
//! it what let it was a reading rather than a mechanic: `Autopilot_Fire`
//! (`0x088613bc`) and `Autopilot_Update` (`0x08861404`) give the duration's
//! source, the running bit, the countdown, the one-second `disengaging` warning
//! and the fact that **pressing fire cancels it**. See
//! `docs/ghidra/functions/psp-pulse-usa/autopilot.md`. What is still ours is the
//! *takeover itself* - the original hands the craft to the driver
//! `Ai_Construct` names `"autopilot input"`, and which code makes that swap was
//! not found - so this engine reuses [`oag_ai::Driver`] the way
//! `Race::set_autopilot` already did.
//!
//! **The Missile left that list on 2026-08-17**, and what let it was not a
//! mechanic but a reading: its lock is recovered whole from `Ship_AcquireLock`
//! (`0x08844784`) and its guidance from `Missile_Update` (`0x0885a918`). See
//! [`crate::projectile::missile`], which carries the split for that half.
//!
//! **What Shield and Rocket do is ours**, more so than the Turbo's effect was:
//! the Turbo at least has a recovered magnitude in `<Engine turbo>`, while
//! Shield's `time` joins to no recovered code path at all and no
//! projectile-flight call site has been found anywhere. The durations, speeds,
//! radii and damage are the disc's; what they drive is this project's reading.
//! See [`oag_physics::ShipState::shield_pickup_timer`], [`crate::projectile`]
//! and `docs/gameplay/pickups.md`, which carries the split.
//!
//! **A rocket has targets now**: `oag_race::Mode::has_opponents` is `true` for
//! [`oag_race::Mode::SingleRace`], so a single race fields seven driven craft
//! as well as the track. What is still missing is *aiming* - nothing picks a
//! target, so a hit is a matter of where the craft was pointed.
//!
//! **This is a departure and it is deliberate**: the authored table weights
//! thirteen weapons and this draws from a subset, so the distribution a player
//! sees is the authored one *conditioned on* the implemented set. It narrows to
//! nothing as weapons land - adding one to [`IMPLEMENTED`] is the whole change -
//! and it is preferred to handing out a mine that cannot be dropped.

use oag_core::Rng;
use oag_formats::weapons::{PickupTable, Weapon, WeaponStats};

/// The weapons a pad in this engine can hand out.
///
/// See the module docs: the shipped table weights thirteen and this is the
/// subset with an effect. A slice rather than a fixed-size array precisely
/// because it is expected to grow, which is the opposite of
/// [`oag_race::Mode::ALL`]'s reason for being one.
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
];

/// Which column of `<Pickupodds>` a craft draws from, and how its place bends it.
///
/// **All four authored columns are reachable now**, which they were not before
/// 2026-08-17: `ai` is spent flat, and `human` is blended with `front` and `back`
/// by where the craft is in the race. That is the original's own arrangement -
/// `WeaponPickup_Grant` (`0x08861d20`) has exactly these two paths - and it reads
/// the opposite way round from the obvious guess, so it is worth stating twice:
/// **the column pair that rubber-bands is the *player's*, not the AI's.**
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
    /// The player, unplaced. Matches what this enum defaulted to before it
    /// carried a place, so a caller that never set one keeps the `human` column
    /// alone rather than silently acquiring a blend against place zero.
    fn default() -> Self {
        Self::HUMAN_UNPLACED
    }
}

impl Driver {
    /// The player, unplaced - the `human` column with no blend.
    pub const HUMAN_UNPLACED: Self = Self::Human { place: 0, field: 0 };

    /// How far down the field this driver is, `0.0` at the front.
    ///
    /// `None` when there is no meaningful place to blend against, in which case
    /// [`Self::weight`] spends the `human` column alone. **That case is ours** -
    /// the original always has a placed craft - and it is the conservative
    /// reading rather than picking an end of the blend arbitrarily.
    ///
    /// The divisor is the **whole field**, not `field - 1`, which is the
    /// original's own arithmetic and has a visible consequence: the craft in last
    /// place gets `(field - 1) / field` of the way to `back`, never all of it.
    #[must_use]
    fn descent(place: u8, field: u8) -> Option<f32> {
        if place == 0 || field == 0 {
            return None;
        }
        Some(f32::from(place - 1) / f32::from(field))
    }

    /// This driver's weight for one weapon, from one class's table.
    ///
    /// Zero for a weapon the class authors no odds for, which is not an error:
    /// a table need not weight every weapon.
    ///
    /// # The blend, and what it does with the shipped numbers
    ///
    /// `human + back * t + front * (1 - t)`, recovered whole. `t` is `0` for the
    /// leader, so **the leader gets `front` added and the tail gets `back`** -
    /// and the shipped Venom table gives Shield `front="2" back="0"` and Turbo
    /// `back="2" front="0"`, so a player in front draws more Shields and a player
    /// at the back more Turbos. The design is catch-up, and it is aimed at the
    /// player rather than at the field.
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
/// One slot. The original's inventory is a flag word with room for more, and
/// whether it can hold two at once is unread - `SubWeapon` exists as a HUD
/// widget, which suggests it can, and no code has been read that fills it. One
/// slot is the conservative reading and the one the HUD can draw.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Held {
    /// The weapon in the slot, or `None`.
    pub weapon: Option<Weapon>,
    /// The last weapon this craft was handed, whether or not it still has it.
    ///
    /// **Recovered as a concept.** The original keeps two copies of the held
    /// weapon id: `craft+0x1bc`, which every fire handler clears to `-1`, and
    /// `craft+0x1c0`, which it does not. The grant reads the second to refuse
    /// handing out the same weapon twice running - see [`draw`].
    ///
    /// Set by [`Self::grant`] and never cleared, so a craft that fires and
    /// crosses another pad still remembers. It is world state and is hashed.
    pub last: Option<Weapon>,
    /// How many mines are still to be laid from the drop in progress.
    ///
    /// **Recovered as a mechanism**, at confidence 90: the original keeps the
    /// same counter on the craft at `+0x1ac`, `Weapon_DropMines`
    /// (`0x088675cc`) decrements it once per spawn, and the craft goes on
    /// holding the weapon until it reaches zero. What is *not* recovered is the
    /// value it starts at - see [`crate::projectile::mine::CLUSTER`], which is
    /// the one invented number on this weapon.
    ///
    /// Zero for every craft that is not mid-drop, which is every craft almost
    /// all of the time.
    pub dropping: u8,
    /// Seconds until the next mine of a drop in progress leaves.
    ///
    /// The original's `craft+0x1b0`, reloaded with a literal `0.1` after every
    /// spawn. Meaningless when [`Self::dropping`] is zero, and held at zero
    /// there rather than left stale so that two worlds with no drop in flight
    /// hash the same.
    pub drop_reload: f32,
    /// Rounds still to leave the barrel from the Cannon's own reload
    /// countdown, or `0` for a craft not holding one.
    ///
    /// **Recovered as a mechanism**, at confidence 85: the original's
    /// `craft+0x154` starts at `<Weapon type="Cannon"><Stats rounds>` and
    /// counts down one per round; `Cannon_UpdateReload` clears the held
    /// slot in the same branch that sees it reach zero. See
    /// [`Self::advance_cannon_reload`] and
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
    ///
    /// **Self-arming rather than armed at grant time - chosen, not
    /// measured.** A pad crossing, `--give` and a test setting
    /// [`Self::weapon`] directly are three different ways this project fills
    /// the slot, and only one of them (the pad) is a call site this module
    /// controls. Reading `0` as "not yet armed" the first time
    /// [`Self::advance_cannon_reload`] runs after a grant, rather than
    /// requiring every grant site to call a separate initialiser, is what
    /// makes all three arm it the same way. Safe because a *genuine* empty
    /// magazine clears [`Self::weapon`] in the same call that reaches zero -
    /// see below - so this can never observe `weapon == Some(Cannon)` with a
    /// counter that is honestly spent.
    pub cannon_rounds: u8,
    /// Seconds until the Cannon's own next round leaves.
    ///
    /// The original's `craft+0x158`, decremented every tick and reloaded
    /// with `<Weapon type="Cannon"><Stats rate>` on the tick it goes
    /// negative. Meaningless when [`Self::cannon_rounds`] is zero, and held
    /// at zero there for [`Self::drop_reload`]'s own reason: so two worlds
    /// with no Cannon in hand hash the same regardless of how each got
    /// there.
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
    /// **The first leaves immediately rather than after one interval.** The
    /// original's reload timer is decremented *before* it is tested and its
    /// initial value is one of the things `weapon-fire.md` lists as unread, so
    /// which of the two it does is genuinely open. Immediately is the reading
    /// that makes a fire press visibly do something on the tick it is pressed.
    ///
    /// **A no-op while a cluster is already coming out - chosen, not measured,
    /// and deliberately given no confidence score.** A mashed fire button used
    /// to call this on every press with no guard, which reset
    /// [`Self::drop_reload`] to zero on a craft already mid-drop; the next
    /// [`Self::advance_drop`] then fired immediately rather than waiting out
    /// the remainder of the current interval, and [`Self::dropping`] never
    /// reached zero, so [`Self::weapon`] never cleared either - one extra mine
    /// per press-edge, indefinitely, three times the authored ceiling of one
    /// every [`crate::projectile::mine::DROP_INTERVAL`].
    ///
    /// There is real evidence here and it is suggestive rather than
    /// dispositive, which is why this stays a choice: the fire-request path
    /// (`Weapon_RequestFire`'s bit-8 case, `docs/ghidra/functions/psp-pulse-usa/mine.md`)
    /// `ori`s its bit into `craft+0x1b8` and nothing more, so a re-press while
    /// that bit is already `1` is a no-op there, and the same page's
    /// `search_instructions` sweep of every `sw` to `craft+0x1ac` (the round
    /// counter) finds exactly one store in the whole binary, the handler's own
    /// decrement. Neither closes the question: that sweep also finds no writer
    /// that *arms* the counter to its starting value in the first place - the
    /// same page lists that write as unread - and without it, whether that
    /// unlocated write would fire again on a re-press is unknown rather than
    /// ruled out. So the guard is this project's own reading of what a
    /// press-and-hold should do, not the original's.
    pub const fn begin_drop(&mut self, count: u8) {
        if self.is_dropping() {
            return;
        }
        self.dropping = count;
        self.drop_reload = 0.0;
    }

    /// Counts one tick off the drop and says whether a mine leaves now.
    ///
    /// Clears the held slot when the last one is out, which is the recovered
    /// coupling: `Weapon_DropMines` writes `-1` into `craft+0x1bc` and drops its
    /// own fire bit in the same branch that sees the counter reach zero.
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

    /// Counts one tick off the Cannon's own reload timer and says whether a
    /// round leaves now, and the rounds left in the magazine after it does.
    ///
    /// `full` is `<Weapon type="Cannon"><Stats rounds>`, passed on every call
    /// rather than cached at grant time - see [`Self::cannon_rounds`]'s own
    /// doc comment for why a self-arming counter is what lets a pad
    /// crossing, `--give` and a test all fill this the same way. `rate` is
    /// the same block's `<Stats rate>`, read the literal way
    /// `oag_formats::weapons::CannonStats::rate` argues for.
    ///
    /// The returned count is the magazine **after** this round is spent,
    /// because that is the order `Weapon_FireCannon` reads it in:
    /// `Cannon_UpdateReload` decrements the counter and arms the fire bit in
    /// the same branch, and only then does the dispatch loop read
    /// `craft->shots & 1` to pick a muzzle - see
    /// `oag_gameplay::projectile::cannon::launch`, which is what that bit
    /// feeds.
    ///
    /// **The first round leaves immediately rather than after one `rate`-
    /// second wait - chosen, not measured, and deliberately given no
    /// confidence score.** [`Self::begin_drop`]'s doc comment makes the same
    /// choice for the same reason and it applies unchanged here: a
    /// picked-up weapon should visibly do something on the tick it arrives
    /// rather than sit silent for a full reload first.
    pub fn advance_cannon_reload(&mut self, dt: f32, rate: f32, full: u8) -> Option<u8> {
        if full == 0 {
            // Degenerate authored data - a Cannon with no rounds at all.
            // Nothing to arm and nothing to fire; clear the slot rather than
            // spin forever re-arming a zero-round magazine every call.
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

    /// Fills the slot and remembers what went in it.
    ///
    /// The pairing is the point: [`draw`] needs the previous grant and would
    /// silently stop refusing repeats if a caller set `weapon` directly. Tests
    /// that only want a craft to be carrying something still assign the field.
    pub const fn grant(&mut self, weapon: Weapon) {
        self.weapon = Some(weapon);
        self.last = Some(weapon);
    }

    /// Takes what is held, leaving the slot empty.
    ///
    /// [`Self::last`] survives, which is what makes the no-repeat rule outlast
    /// firing.
    ///
    /// **Also ends a drop in progress, and that part is chosen, not
    /// measured.** Absorbing (`CIRCLE`) a Mine or a Bomb while its cluster is
    /// still coming out used to clear only [`Self::weapon`], leaving
    /// [`Self::dropping`] and [`Self::drop_reload`] stuck at whatever they were.
    /// `Race::lay_mines` sees `is_dropping()` still true and `weapon` gone, so
    /// it does nothing every tick after, forever. The counter then survives
    /// into the *next* pickup this craft is granted: a later Mine grant reads
    /// as still-dropping from the old cluster, so [`Self::begin_drop`]'s own
    /// mid-cluster guard silently swallows the new press too. Nothing pins what
    /// the original does when `CIRCLE` is pressed mid-drop - this project picks
    /// "ends the drop", the reading that cannot leave the slot in a state no
    /// future press can escape.
    pub const fn take(&mut self) -> Option<Weapon> {
        self.dropping = 0;
        self.drop_reload = 0.0;
        // Same reasoning for the Cannon's own two fields: absorbing one
        // mid-burst must not leave a live round count behind for the *next*
        // weapon this craft is granted to inherit, and two worlds that gave
        // up a Cannon at different points in its magazine must hash the
        // same once both hold nothing.
        self.cannon_rounds = 0;
        self.cannon_reload = 0.0;
        self.weapon.take()
    }
}

/// Draws one weapon from a class's authored odds, restricted to [`IMPLEMENTED`].
///
/// `None` when the table weights none of the implemented weapons above zero,
/// which is a real state rather than a failure: a class that authored no Turbo
/// odds would hand out nothing until a second weapon lands.
///
/// The walk is over [`IMPLEMENTED`]'s order rather than the table's document
/// order, so the sequence depends only on this crate and the seed - a table that
/// reordered its `<Weapon>` elements would otherwise change every draw.
///
/// # It will not hand out `last` twice running
///
/// **Recovered.** `WeaponPickup_Grant` (`0x08861d20`) compares every draw against
/// the craft's previous grant and, on a match, rolls again rather than handing it
/// over. Pass [`Held::last`].
///
/// **The retry is bounded here and is not in the original**, which loops until it
/// draws something different. Unbounded is fine when thirteen weapons are
/// weighted and is not fine here: [`IMPLEMENTED`] is ten, so a table weighting
/// only one of them would spin for ever, and a simulation that can hang on a
/// table is worse than one that occasionally repeats a pickup. After
/// [`REDRAW_ATTEMPTS`] the repeat is accepted.
///
/// So the rule is **best-effort, and how good the effort is depends on the
/// odds**: a weapon holding a share `p` of the live weight repeats with
/// probability `p^REDRAW_ATTEMPTS`. On the shipped Venom weights the worst case
/// is Turbo at 14 of 48, or about one repeat in twenty thousand grants. A table
/// that gave one weapon nearly all the weight would repeat often, and
/// `an_overwhelming_weight_terminates_and_may_repeat` pins that rather than
/// pretending otherwise.
#[must_use]
pub fn draw(
    rng: &mut Rng,
    table: &PickupTable,
    driver: Driver,
    last: Option<Weapon>,
) -> Option<Weapon> {
    for _ in 0..REDRAW_ATTEMPTS {
        let drawn = draw_once(rng, table, driver)?;
        if Some(drawn) != last {
            return Some(drawn);
        }
    }
    // Every attempt came back the same weapon. Hand it over rather than hand over
    // nothing: a pad that silently grants nothing reads as a broken pad.
    draw_once(rng, table, driver)
}

/// How many times [`draw`] re-rolls to avoid repeating the last pickup.
///
/// **Ours.** See [`draw`] on why the original's unbounded loop is not reproduced,
/// and on what the bound costs. Eight is enough that the shipped odds repeat
/// about once in twenty thousand grants, and small enough that the worst case
/// spends nine generator draws rather than hanging.
pub const REDRAW_ATTEMPTS: usize = 8;

/// One weighted draw, with no regard for what came before it.
#[must_use]
fn draw_once(rng: &mut Rng, table: &PickupTable, driver: Driver) -> Option<Weapon> {
    let total: f32 = IMPLEMENTED
        .iter()
        .map(|&weapon| driver.weight(table, weapon).max(0.0))
        .sum();
    if total <= 0.0 || !total.is_finite() {
        return None;
    }

    let mut roll = rng.next_f32() * total;
    for &weapon in IMPLEMENTED {
        let weight = driver.weight(table, weapon).max(0.0);
        if weight <= 0.0 {
            continue;
        }
        roll -= weight;
        if roll < 0.0 {
            return Some(weapon);
        }
    }
    // Only reachable when `next_f32` returns something that rounds the walk past
    // the end - it is half-open on `1.0`, so this is float slack rather than a
    // logic hole. The last weighted weapon is the right answer either way.
    IMPLEMENTED
        .iter()
        .rev()
        .copied()
        .find(|&weapon| driver.weight(table, weapon) > 0.0)
}

/// The class's own table out of a whole weapon file.
///
/// # The two files spell a speed class differently, and this is where that lands
///
/// `HandlingStats.xml` authors `<GlobalClass name="VENOM">` and
/// `WeaponStats_Race.xml` authors `<Pickupodds class="Venom">` - measured on the
/// shipped USA disc, all four classes, both files. So a caller holding a
/// [`SpeedClass`] cannot reach the pickup table through
/// [`WeaponStats::pickups_for`] and [`SpeedClass::as_str`] together, because the
/// latter is the *handling* file's spelling.
///
/// Matched case-insensitively here rather than by adding a second spelling to
/// [`SpeedClass`]: the difference is between two documents, not between two
/// concepts, and the parser deliberately keeps `PickupTable::class` a `String`.
///
/// **The rung arrives as a name**, which is what lets this answer for a ladder
/// that is not Pulse's. Wipeout Pure authors a `<Pickupodds class="Vector">`
/// beside its four, and this reaches it with no change beyond the parameter
/// type; Pulse authors four and no `Vector`, so a `Vector` lookup there is
/// `None` - the same honest absence any unauthored rung gets.
///
/// [`SpeedClass`]: oag_formats::handling::SpeedClass
/// [`SpeedClass::as_str`]: oag_formats::handling::SpeedClass::as_str
#[must_use]
pub fn table_for<'a>(stats: &'a WeaponStats, class: &str) -> Option<&'a PickupTable> {
    stats
        .pickups
        .iter()
        .find(|table| table.class.eq_ignore_ascii_case(class))
}

#[cfg(test)]
mod tests;
