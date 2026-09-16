//! The whole of gameplay state, in one struct of plain data.
//!
//! Fixed-size arrays and no allocation, so a snapshot is one `memcpy`-shaped
//! operation and a replay or a golden test needs no traversal. See
//! `docs/architecture/adr/0003-no-ecs.md` for why this is not an ECS.

use oag_core::Rng;
use oag_physics::{DamageRules, Handling, ShipState};
use oag_race::{Mode, RaceState};

/// The most ships a race can hold.
///
/// Eight, which is what Pulse grids. A hard array bound rather than a `Vec`
/// capacity: the limit is real, and making it visible is what keeps the world
/// snapshot a fixed size.
pub const MAX_SHIPS: usize = 8;

/// The most players a race can hold, which is the whole grid.
///
/// **Equal to [`MAX_SHIPS`] on purpose, not by coincidence.** A player occupies
/// a grid slot, so the player cap can never usefully exceed the grid and there
/// is no case that wants it smaller: split screen, multiple windows and network
/// play all target a full eight, which is exactly a race with no AI in it. Two
/// constants that are always the same number would be two things to keep in
/// step and one place for them to drift, so this is defined *as* [`MAX_SHIPS`]
/// rather than written out again - `race[i]` and `ships[i]` are the same racer,
/// and that has to stay true by construction.
pub const MAX_PLAYERS: usize = MAX_SHIPS;

/// Who flies a grid slot.
///
/// **Alongside [`World::race`], not instead of it.** A slot's *timing* and a
/// slot's *pilot* are two different questions: every slot is timed - that is
/// what widening `race` to an array buys - but only some are flown by a person,
/// and the rest are flown by [`oag_ai`] exactly as they are today. Collapsing
/// the two would mean "has a `RaceState`" implied "is human", which would make
/// the array useless for the thing it is for, namely ranking eight timed craft
/// against each other.
///
/// **Local and remote are distinguished here rather than left to the caller**,
/// because they differ in what the simulation is handed, not in what it does
/// with it: a local slot's [`crate::InputSnapshot`] comes off a device this
/// frame, a remote slot's comes off the wire and may be a prediction that gets
/// replayed. The tick treats both as "a human's input arrived for this slot",
/// which is why the distinction is a variant and not a second array - but a
/// reconciliation pass has to know which slots it is allowed to re-derive, and
/// it cannot ask the input layer, which the simulation may not depend on.
///
/// Hashed, for the reason [`Ship::autopilot_timer`] gives about itself: a slot
/// being flown by a person is a different race from one being flown by a
/// driver, and a replay that lost which was which would diverge the moment the
/// two disagreed about a control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Controller {
    /// Flown by [`oag_ai::Driver`]. Every slot but slot 0 today.
    ///
    /// The default, so a freshly defaulted array is a grid of opponents and a
    /// human slot has to be asked for rather than assumed.
    #[default]
    Ai,
    /// Flown by a person at this machine, off a device this process polls.
    Local,
    /// Flown by a person somewhere else, off inputs that arrive as data.
    Remote,
}

impl Controller {
    /// Whether a person flies this slot, wherever they are sitting.
    ///
    /// The question nearly every caller actually has - "does this slot take an
    /// [`crate::InputSnapshot`] rather than a driver's output" - and the one
    /// that must be asked instead of comparing against slot 0.
    #[must_use]
    pub fn is_human(self) -> bool {
        matches!(self, Self::Local | Self::Remote)
    }
}

/// One racer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ship {
    /// Body and suspension state.
    pub physics: ShipState,
    /// The tunables for this ship in the race's speed class, read from the
    /// player's own disc.
    pub handling: Handling,
    /// Which spline segment the ship was last nearest to.
    ///
    /// Carried between ticks so the search for the nearest segment is local
    /// rather than over the whole track. Lap counting is **not** built on this:
    /// where the original counts laps is an open question, and `gate` has no
    /// runtime class at all. See `docs/formats/track.md`.
    ///
    /// **Not the same index as [`Self::driver`]**, which is a position in the
    /// racing line's own point table and is what an opponent actually steers by.
    /// This one is a per-path control-point segment and is still read by nothing.
    pub segment: u16,
    /// What this craft is carrying, if anything.
    ///
    /// Filled by crossing a `Weapon Pad` and emptied by firing or absorbing;
    /// see [`crate::pickup`], which carries the recovered-versus-ours split for
    /// the whole system. Always empty in a mode that races with weapons off,
    /// because such a race has no armed pads at all - the original does not
    /// merely ignore a crossing, it empties the trigger list.
    ///
    /// **Here rather than on `ShipState`**, because it is an
    /// `oag_tables::weapons::Weapon` and `oag-physics` depends on nothing but
    /// `oag-core`. The consequence is that it is *not* covered by
    /// `oag_physics::probe::hash_state`'s compile-time enforcement - what a held
    /// pickup does to the simulation reaches the hash through
    /// `ShipState::turbo_timer` instead, and the inventory itself changes no
    /// force.
    pub pickup: crate::pickup::Held,
    /// Where this craft's driver last found itself on the racing line.
    ///
    /// Only an opponent uses it - slot 0 is flown by the player - and it is the
    /// seed for the next windowed search rather than a lap-counting index.
    ///
    /// **Here rather than beside the world**, because a driver whose state lived
    /// in the composition root would be invisible to a snapshot, and two replays
    /// of one race would not be the same race. It is one `u32` and stays `Copy`,
    /// so the array is still a `memcpy`. The line it indexes into is *track*
    /// data and is not in the world at all; see [`oag_ai::Line`].
    pub driver: oag_ai::Driver,
    /// Where this craft is in the race: its lap, its place on the circuit and
    /// whether it has finished.
    ///
    /// **Every craft, the player included**, and it was the *only* per-craft
    /// race tracking there was until [`World::race`] widened to an array on
    /// 2026-09-16: this is the smaller thing the whole field needs so that
    /// eight of them can be ordered against each other, and it is deliberately
    /// still that. `race[i]` is slot `i`'s clock, its Zone counters and its
    /// best lap; this is slot `i`'s place in the field. A human slot's `lap` is
    /// assigned from here into its `race[i].lap` rather than counted twice, so
    /// there is one lap rule and not two.
    ///
    /// [`World::race`]: World::race
    pub standing: oag_race::Standing,
    /// Seconds left on an Autopilot pickup, or zero.
    ///
    /// `Autopilot_Fire` (`0x088613bc`) writes
    /// `<Weapon type="Autopilot"><Stats time>` into `craft+0x148` and sets a
    /// running bit; `Autopilot_Update` (`0x08861404`) counts it down and clears
    /// the bit at zero. See
    /// `docs/ghidra/functions/psp-pulse-usa/autopilot.md`.
    ///
    /// **Here rather than on [`ShipState`]**, for the reason [`Self::pickup`]
    /// gives about itself: `oag-physics` depends on nothing but `oag-core`, and
    /// no force reads this. What it changes is *which controls the craft is
    /// handed*, which the composition root decides - so it belongs beside the
    /// pickup that arms it and the driver that flies it, not beside the body.
    ///
    /// Hashed all the same: a craft being driven for is a different race from
    /// one being driven, and a replay that lost this would diverge.
    pub autopilot_timer: f32,
    /// Seconds of weapon slowdown owed to this craft but not yet applied.
    ///
    /// The original's `entity+0x130`. A weapon impact adds its
    /// `<Stats slowdown_time>` here - nine writers, all in the weapon
    /// subsystems - and exactly one consumer drains it, once a tick, into
    /// [`oag_physics::ShipState::slowdown_timer`]. That is
    /// [`crate::slowdown::drain`], and it carries the shield gate: a shielded
    /// craft takes no slowdown, **and this slot is cleared anyway** rather than
    /// banked, so a hit landed one tick before a shield expires is simply lost.
    ///
    /// **Here rather than on [`oag_physics::ShipState`]**, for the reason
    /// [`Self::pickup`] and [`Self::autopilot_timer`] both give about
    /// themselves: what fills it is a figure out of `<WeaponStats>`, and
    /// `oag-physics` depends on `oag-core` and nothing else. It is also where
    /// the original puts it - on the *entity*, not on the craft, which is the
    /// one field of this mechanic that is not a `craft+` offset.
    ///
    /// Hashed: a craft owing slowdown is about to be a craft that has lost its
    /// engine, and a replay that dropped this would diverge one tick later.
    ///
    /// See `oag_physics::slowdown` for the whole law and
    /// `docs/ghidra/functions/psp-pulse-usa/engine.md` for the evidence.
    pub pending_slowdown: f32,
    /// The one-shot thrust scale a LeachBeam armed, `craft+0x31c`, until the
    /// next step consumes it.
    ///
    /// `Ship_ApplyPendingWeaponDamage` (`0x0883f13c`) writes the beam's
    /// `slowShipFactor` here on every tick the beam drains an unshielded,
    /// racing victim, and `Ship_UpdateEngine` reads it once and writes `1.0`
    /// back. The composition root hands it to
    /// `oag_physics::Environment::thrust_scale` at the top of the next step and
    /// resets it in the same move - the drain runs after the step, so a beam
    /// throttles from the tick after it connects to the tick after it breaks,
    /// the same ordering [`Self::pending_slowdown`] has. `1.0` is the neutral
    /// value, not `0.0`.
    ///
    /// Hashed, for [`Self::pending_slowdown`]'s reason: a craft about to lose a
    /// fifth of its thrust is a different craft one tick later. See
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
    pub pending_thrust_scale: f32,
    /// The Disruptor effect this craft is under, if any.
    ///
    /// Pure's `craft+0x134`/`+0x138`/`+0x13c` - a flag, a kind and the seconds
    /// left - as one [`crate::disruption::Disruption`]. **Here rather than on
    /// [`ShipState`]**, for [`Self::autopilot_timer`]'s reason exactly: what
    /// it changes is which controls the craft is handed, and the composition
    /// root decides that. Hashed, because a craft that cannot thrust is a
    /// different race from one that can.
    pub disruption: crate::disruption::Disruption,
    /// Whether this slot holds a ship at all.
    pub active: bool,
}

impl Default for Ship {
    fn default() -> Self {
        Self {
            physics: ShipState::default(),
            handling: Handling::ZERO,
            segment: 0,
            pickup: crate::pickup::Held::empty(),
            driver: oag_ai::Driver::default(),
            standing: oag_race::Standing::default(),
            autopilot_timer: 0.0,
            pending_slowdown: 0.0,
            pending_thrust_scale: 1.0,
            disruption: crate::disruption::Disruption::default(),
            active: false,
        }
    }
}

/// The `Weapons` and `Damage` race options a mode runs with.
///
/// **Recovered, not chosen.** `Race_ReadSetupOptions` (`0x08896b84`) takes a
/// per-mode default before consulting the setup strings: `g_weapons_enabled` is
/// cleared for game modes `{5, 9, 10, 0xf, 0x11}` and `g_damage_enabled` for
/// `{5, 7, 10}`. Modes **5 and 10** are in both sets *and* share one constructor
/// in `Race_CreateModeObject` (`0x0882112c`), which is exactly the relationship
/// this project's time trial and speed lap have - speed lap is the time trial
/// without the lap target.
///
/// The disc's own manual text closes it independently: `MAN_P3_PG1_ENER` reads
/// *"The Time Trial and Speed Lap events will recover your ship energy
/// automatically"*, and recovering energy automatically is precisely what
/// clearing `g_damage_enabled` does - `oag_physics::damage::regenerate`, 4 a
/// second with a floor of 20. A shipped string table and a switch statement
/// agreeing is what puts this at confidence **85** rather than the 84 a
/// decompile alone caps at.
///
/// Zone is game mode 6 and is in neither set, so its *damage* default stands
/// and its shield depletes - which is what its unimplemented "destroyed" ending
/// depends on. See `docs/ghidra/functions/psp-pulse-usa/shield.md`. Its
/// **weapons** default does not stand: mode 6 defaults `g_weapons_enabled` to
/// `1` and the front end then always supplies an explicit
/// `<Weapons>Off</Weapons>` for it, which is why selecting ZONE greys the
/// `WEAPONS` row to `OFF` on the running original (confirmed live 2026-08-10,
/// PPSSPP v1.20.4). That is the measurement [`Mode::weapons_enabled`] carries.
///
/// A single race is in neither set either, and unlike Zone the front end does
/// not override it: `MSC_EVENT_SR` calls weapons "optional" and the Custom Race
/// screen leaves both rows selectable, so both defaults stand. That makes it the
/// only mode here whose contact damage is *not* halved - see
/// [`oag_physics::damage::NO_WEAPONS_DAMAGE_SCALE`], which a time trial does get.
///
/// **The weapons flag is read from [`Mode::weapons_enabled`] rather than
/// restated here.** Restating it is how the two disagreed about Zone until the
/// 2026-08-18 review (finding D2): this function said weapons on, `mode.rs`
/// said off with the stronger evidence, and because the flag halves *all*
/// damage rather than only weapon damage, Zone was taking roughly twice the
/// original's contact damage. Survival time is the whole of Zone's score, so
/// that is a scoreboard divergence and not a detail. One source of truth
/// cannot drift from itself.
///
/// **Here rather than on [`oag_race::Mode`]**, because `oag-race` deliberately
/// does not depend on `oag-physics` (see that crate's `Cargo.toml`) and this
/// returns a physics type. This crate is the bridge between the two, which is
/// what it is for.
#[must_use]
pub fn damage_rules(mode: Mode) -> DamageRules {
    DamageRules {
        weapons: mode.weapons_enabled(),
        // `g_damage_enabled` is cleared for modes `{5, 7, 10}`: time trial and
        // speed lap, of the four this engine runs.
        damage: !matches!(mode, Mode::TimeTrial | Mode::SpeedLap),
    }
}

/// Everything the simulation carries from one tick to the next.
#[derive(Debug, Clone)]
pub struct World {
    /// Ticks elapsed since the race began.
    pub tick: u64,
    /// The seeded generator. Every random draw in the simulation comes from
    /// here; nothing reads OS entropy. See `oag_core::rng`.
    pub rng: Rng,
    /// The grid. Slots past [`Self::ship_count`] are inactive, not absent, so
    /// the array's size never depends on the race.
    pub ships: [Ship; MAX_SHIPS],
    /// How many of [`Self::ships`] are in play.
    pub ship_count: u8,
    /// Lap, timing and mode state, one per grid slot.
    ///
    /// [ADR-0003] sketched this field when the world was first laid out, and
    /// this is it. It was a *single* `RaceState` until 2026-09-16, on the
    /// reasoning that the modes which need a clock - time trial, speed lap,
    /// Zone - have one ship on the track, so per-opponent timing could arrive
    /// with the grid rather than before it. It arrives now, because three
    /// separate features need it at once: split screen, multiple windows and
    /// network play are all "N people racing in one simulation", and every one
    /// of them needs N clocks before it needs anything else.
    ///
    /// **A fixed array and not a `Vec`**, for [ADR-0003]'s own reason rather
    /// than a new one: a world snapshot is one `memcpy`-shaped operation, and a
    /// heap pointer in it would make a replay's frames uncomparable by memory
    /// layout. Eight `RaceState`s is the cost of a field that never changes
    /// size, paid once, against a race that could change the size of its own
    /// snapshot.
    ///
    /// **Per *slot*, not per human.** Index `i` is the racer in `ships[i]`,
    /// including the AI ones, so the two arrays are read with the same index
    /// and there is no mapping table to keep in step. An AI slot's clock is
    /// simply not read by a HUD; it costs nothing to advance and it is what
    /// makes ranking eight timed craft against each other a lookup rather than
    /// a search. Which slots a *person* flies is [`Self::controllers`], a
    /// separate question with a separate array.
    ///
    /// **[`RaceState::mode`] and [`RaceState::laps_target`] are replicated
    /// eight times and only slot 0's is authoritative.** They describe the
    /// race, not a racer, and every slot carries the same value - a known
    /// redundancy, kept rather than hoisted out because splitting `RaceState`
    /// into a per-race half and a per-racer half is a change to `oag-race`'s
    /// own recovered type, which this widening deliberately does not touch.
    /// Read them through [`Self::mode`] rather than indexing, so the day a mode
    /// genuinely differs per player there is one place to fix.
    ///
    /// [ADR-0003]: ../../../docs/architecture/adr/0003-no-ecs.md
    /// [`RaceState::mode`]: oag_race::RaceState::mode
    /// [`RaceState::laps_target`]: oag_race::RaceState::laps_target
    pub race: [RaceState; MAX_PLAYERS],
    /// Who flies each grid slot: a person here, a person elsewhere, or the AI.
    ///
    /// **Alongside [`Self::race`] rather than folded into it** - see
    /// [`Controller`] for why the two questions are separate. One `Controller`
    /// is a byte and the array is `Copy`, so the world is still a `memcpy`.
    ///
    /// Slot 0 is [`Controller::Local`] and the rest are [`Controller::Ai`] in
    /// every race this engine currently starts, which is exactly the shape the
    /// field hard-coded before it existed. The point of writing it down is that
    /// the hard-coding is now *data*: a caller that wants two local players
    /// marks slot 1 as well, and nothing in the tick has to learn a second rule
    /// to make that work.
    pub controllers: [Controller; MAX_PLAYERS],
    /// Everything a weapon has put in the air.
    ///
    /// **The field that grows the snapshot**, and it grows it by a constant:
    /// [`crate::projectile::MAX_PROJECTILES`] slots of plain `Copy` data, sized
    /// the same way `[Ship; MAX_SHIPS]` is and for the same reason. A race must
    /// not change the size of a snapshot, so the array holds free slots rather
    /// than shrinking - see [ADR-0003].
    ///
    /// Unlike [`Ship::pickup`] this **is** simulation state that moves every
    /// tick, so it is covered by [`crate::hash::hash_world`] rather than left to
    /// reach the determinism gate indirectly.
    pub projectiles: crate::projectile::Projectiles,
    /// The single travelling Quake wave, or `None` when none is in flight.
    ///
    /// Not part of [`Self::projectiles`] - see `crate::projectile::quake`'s
    /// module doc comment for why: the original's own pool is a single slot,
    /// not an array, and it carries a travelling *distance* rather than a
    /// position and a velocity. Simulation state all the same, so it is
    /// covered by [`crate::hash::hash_world`] exactly as [`Self::projectiles`]
    /// is.
    pub quake: Option<crate::projectile::quake::Wave>,
    /// The single LeachBeam link, or `None` when none is in flight.
    ///
    /// Beside [`Self::quake`] and for the same reason - see
    /// `crate::projectile::leach_beam`'s module doc comment: the original's own
    /// pool cursor allows exactly one beam **in the whole race** at a time, a
    /// stricter gate than any other weapon has, and a beam carries a link
    /// between two craft rather than a position and a velocity. Simulation
    /// state, so [`crate::hash::hash_world`] covers it.
    pub leach_beam: Option<crate::projectile::leach_beam::Beam>,
}

impl World {
    /// An empty world with the given seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            tick: 0,
            rng: Rng::new(seed),
            ships: [Ship::default(); MAX_SHIPS],
            ship_count: 0,
            race: [RaceState::default(); MAX_PLAYERS],
            controllers: Self::SINGLE_PLAYER,
            projectiles: crate::projectile::Projectiles::new(),
            quake: None,
            leach_beam: None,
        }
    }

    /// One person in slot 0 and seven drivers behind them.
    ///
    /// The only grid this engine starts today, and named rather than spelled
    /// out at each of its uses so "the default is one local player" is a thing
    /// the reader can find, not a pattern they have to notice.
    pub const SINGLE_PLAYER: [Controller; MAX_PLAYERS] = {
        let mut controllers = [Controller::Ai; MAX_PLAYERS];
        controllers[0] = Controller::Local;
        controllers
    };

    /// The active ships, in slot order.
    ///
    /// Slot order, not race position: iteration order feeds simulation state, so
    /// it has to be something that cannot vary between runs.
    pub fn active_ships(&self) -> impl Iterator<Item = &Ship> {
        self.ships.iter().take(self.ship_count as usize)
    }

    /// The slots a person flies, in slot order.
    ///
    /// **Slot order, and over the whole array rather than the active prefix**,
    /// for the reason [`Self::active_ships`] gives about itself: this iteration
    /// reaches simulation state - it is what decides which slots consume an
    /// input snapshot - so it must not vary between runs. Whole array because a
    /// slot marked human but inactive is a caller's mistake to surface, not one
    /// to silently skip; nothing marks one today.
    ///
    /// This is what replaces `0` wherever the composition root used to mean
    /// "the player". With the default grid it yields exactly `0`, once, which
    /// is why that substitution is behaviour-preserving.
    pub fn human_slots(&self) -> impl Iterator<Item = usize> + '_ {
        (0..MAX_PLAYERS).filter(|&slot| self.controllers[slot].is_human())
    }

    /// The lowest slot a person flies, or slot 0 when nobody does.
    ///
    /// The single view a full-screen HUD draws and a single-camera render
    /// follows, until a viewport per player exists to draw the rest into. Slot
    /// 0 as the fallback rather than `None`, because a race with no human in it
    /// is an attract-mode demo that still has to point a camera somewhere, and
    /// every caller here already had slot 0 written into it.
    #[must_use]
    pub fn primary_slot(&self) -> usize {
        self.human_slots().next().unwrap_or(0)
    }

    /// [`Self::primary_slot`]'s race: the clock a full-screen HUD draws.
    ///
    /// **Not `race[0]`, even though it resolves to `race[0]` today.** The two
    /// are the same value for as long as slot 0 is the only human, and the
    /// whole point of the widening is that this will stop being true - a reader
    /// that spelled the index out would keep working and keep showing the wrong
    /// player. Anything that means "the view being drawn" comes through here;
    /// anything that means "every racer" iterates [`Self::race`] directly.
    #[must_use]
    pub fn primary_race(&self) -> &RaceState {
        &self.race[self.primary_slot()]
    }

    /// [`Self::primary_race`], to write to.
    pub fn primary_race_mut(&mut self) -> &mut RaceState {
        let slot = self.primary_slot();
        &mut self.race[slot]
    }

    /// The race's own mode, which every slot carries a copy of.
    ///
    /// Read through here rather than by indexing [`Self::race`], so that the
    /// replication that field's doc comment records has one reader and not
    /// sixty. See that comment for why the copies exist at all.
    #[must_use]
    pub fn mode(&self) -> Mode {
        self.race[0].mode
    }

    /// The race's lap target, which every slot carries a copy of.
    ///
    /// [`Self::mode`]'s argument exactly, for the other race-wide field.
    #[must_use]
    pub fn laps_target(&self) -> Option<u32> {
        self.race[0].laps_target
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The recovered table, pinned per mode rather than per rule, because the
    /// two flags come from two different sets in `Race_ReadSetupOptions` and a
    /// test that only checked "weapons implies damage" would pass on a table
    /// that had them the wrong way round.
    #[test]
    fn each_mode_races_with_the_flags_the_original_gives_it() {
        for (mode, weapons, damage) in [
            (Mode::TimeTrial, false, false),
            (Mode::SpeedLap, false, false),
            (Mode::Zone, false, true),
            (Mode::SingleRace, true, true),
        ] {
            let rules = damage_rules(mode);
            assert_eq!(rules.weapons, weapons, "{mode:?} weapons");
            assert_eq!(rules.damage, damage, "{mode:?} damage");
        }
    }

    /// Finding D2's own guard: the weapons flag here is the one `oag-race`
    /// measured, not a second opinion about it. They contradicted each other
    /// about Zone for as long as both were written out by hand.
    #[test]
    fn the_weapons_flag_is_the_one_oag_race_measured() {
        for mode in Mode::ALL {
            assert_eq!(
                damage_rules(mode).weapons,
                mode.weapons_enabled(),
                "{mode:?} disagrees with Mode::weapons_enabled"
            );
        }
    }

    #[test]
    fn a_new_world_has_no_ships() {
        let world = World::new(1);
        assert_eq!(world.tick, 0);
        assert_eq!(world.ship_count, 0);
        assert_eq!(world.active_ships().count(), 0);
        assert!(world.ships.iter().all(|ship| !ship.active));
    }

    #[test]
    fn a_new_world_is_on_lap_one_of_a_time_trial() {
        let world = World::new(1);
        assert_eq!(world.mode(), oag_race::Mode::TimeTrial);
        for (slot, race) in world.race.iter().enumerate() {
            assert_eq!(race.mode, oag_race::Mode::TimeTrial, "slot {slot}");
            assert_eq!(race.lap, 1, "slot {slot}");
            assert!(!race.finished, "slot {slot}");
            assert_eq!(race.progress, None, "slot {slot}");
        }
    }

    /// The pool starts empty because nothing fills it yet. When collision damage
    /// lands this becomes the ship's maximum and this test changes with it - it
    /// is here so that change is deliberate rather than incidental.
    #[test]
    fn a_default_ship_carries_no_shield_yet() {
        assert_eq!(Ship::default().physics.shield, 0.0);
    }

    /// The array is the whole point: a race must not change the size of a
    /// snapshot, or a replay's frames stop being comparable by memory layout.
    #[test]
    fn the_grid_is_always_eight_slots_wide() {
        let mut world = World::new(1);
        world.ship_count = 3;
        assert_eq!(world.ships.len(), MAX_SHIPS);
        assert_eq!(world.active_ships().count(), 3);
    }
}
