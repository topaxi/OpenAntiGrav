//! Which pattern each event of a race plays: the map from our events to
//! pattern names (`docs/gameplay/rumble.md`, "Triggers").
//!
//! Keyed by **our events** and by pattern **name**, never by title or by id:
//! the player resolves a name to its place in whatever [`Library`] the title
//! loaded, so a title that ships its own patterns (or our fallback TOML ones)
//! reuses the rows. What differs per title is which rows fire ([`Set`]) and how
//! the small motor reads ([`SmallMotor`](super::SmallMotor)), both title data.
//!
//! [`Library`]: oag_tables::rumble::Library

use oag_gameplay::cue::Cue;

/// The trigger set a row belongs to, oldest first. A set fires its own rows
/// and every earlier set's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Set {
    /// HD/Fury's call sites (`RUMBLE` = `PS3`).
    Ps3,
    /// Omega's: HD's, plus weapon impacts and explosions (`RUMBLE` = `PS4`).
    Ps4,
}

impl Set {
    /// Whether a row of set `row` fires under `self`.
    #[must_use]
    pub fn fires(self, row: Self) -> bool {
        row <= self
    }
}

/// What ends a held voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// Set the release flag: the voice runs to the end of its track.
    Finish,
    /// Silent at once.
    Stop,
}

/// How a row plays its pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Once through.
    OneShot,
    /// Held while the row's [`Level`] stays up, then ended as `End` says.
    Held(End),
}

/// A condition that stays true over several ticks. The playing side recomputes
/// it every tick, so a seek or a restart cannot leave a motor running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Over a mag strip.
    MagStrip,
    /// Autopilot engaged.
    Autopilot,
    /// A Leech beam drains the local ship.
    LeechBeam,
    /// The wrong-way warning is up.
    WrongWay,
    /// The ship is blowing up.
    Exploding,
    /// The weapon reticle has taken its lock.
    LockedOn,
}

/// Which side of a condition a row watches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// The tick the level comes up.
    Rise,
    /// The tick it goes down.
    Fall,
}

/// What raises a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// A one-shot edge the simulation already raises for sound
    /// ([`oag_gameplay::cue::CueEvent`]).
    Cue(Cue),
    /// A [`Level`] coming up or going down.
    Edge(Level, Edge),
    /// A [`Level`] staying up: starts a held voice on the rise, ends it on the
    /// fall.
    Level(Level),
}

/// Whose pad a row reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The local player's own ship only; a rival's event reaches no motor.
    OwnShip,
    /// A world event at a point: every local pad, positional.
    World,
}

/// One line of the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// What raises it.
    pub trigger: Trigger,
    /// The pattern's name in the title's library.
    pub pattern: &'static str,
    /// Once through, or held.
    pub shape: Shape,
    /// Whose pad.
    pub scope: Scope,
    /// The set that first fires it.
    pub from: Set,
}

const fn own(trigger: Trigger, pattern: &'static str, from: Set) -> Row {
    Row {
        trigger,
        pattern,
        shape: Shape::OneShot,
        scope: Scope::OwnShip,
        from,
    }
}

const fn world(cue: Cue, pattern: &'static str, from: Set) -> Row {
    Row {
        trigger: Trigger::Cue(cue),
        pattern,
        shape: Shape::OneShot,
        scope: Scope::World,
        from,
    }
}

const fn held(level: Level, pattern: &'static str, end: End) -> Row {
    Row {
        trigger: Trigger::Level(level),
        pattern,
        shape: Shape::Held(end),
        scope: Scope::OwnShip,
        from: Set::Ps3,
    }
}

/// Every wired row. The disc ids are on `docs/formats/rumble-xml.md`
/// ("Every pattern"); the call sites are on `docs/gameplay/rumble.md`.
pub const ROWS: &[Row] = &[
    own(Trigger::Cue(Cue::SpeedupPad), "pad_speed", Set::Ps3),
    own(
        Trigger::Cue(Cue::Absorb),
        "absorb_any_pickup_rumble",
        Set::Ps3,
    ),
    own(
        Trigger::Edge(Level::LockedOn, Edge::Rise),
        "lockon_rumble",
        Set::Ps3,
    ),
    own(Trigger::Cue(Cue::Rocket), "rockets_fired_rumble", Set::Ps3),
    own(Trigger::Cue(Cue::Missile), "missile_fired_rumble", Set::Ps3),
    own(Trigger::Cue(Cue::Cannon), "cannon_fired_rumble", Set::Ps3),
    own(Trigger::Cue(Cue::Plasma), "plasma_fired_rumble", Set::Ps3),
    own(Trigger::Cue(Cue::MineLaunch), "mine_fired_rumble", Set::Ps3),
    own(
        Trigger::Cue(Cue::QuakeLaunch),
        "quake_fired_rumble",
        Set::Ps3,
    ),
    own(Trigger::Cue(Cue::Leach), "leech_fired_rumble", Set::Ps3),
    own(
        Trigger::Cue(Cue::ShurikenLaunch),
        "shuriken_fired_rumble",
        Set::Ps3,
    ),
    own(
        Trigger::Cue(Cue::Repulsor),
        "repulser_fired_rumble",
        Set::Ps3,
    ),
    world(Cue::PlasmaHitWall, "plasma_hits_wall", Set::Ps3),
    world(Cue::RocketHitWall, "rocket_hits_wall", Set::Ps3),
    world(Cue::MissileHitWall, "missile_rebound_wall_rumble", Set::Ps4),
    own(
        Trigger::Edge(Level::MagStrip, Edge::Rise),
        "enter_mag_rumble",
        Set::Ps3,
    ),
    own(
        Trigger::Edge(Level::MagStrip, Edge::Fall),
        "exit_mag_rumble",
        Set::Ps3,
    ),
    own(
        Trigger::Edge(Level::WrongWay, Edge::Rise),
        "wrong_way",
        Set::Ps3,
    ),
    own(
        Trigger::Edge(Level::Exploding, Edge::Rise),
        "ship_exploding_player_rumble",
        Set::Ps4,
    ),
    held(Level::MagStrip, "travel_mag_rumble", End::Finish),
    held(Level::Autopilot, "autopilot_active_rumble", End::Finish),
    held(Level::LeechBeam, "leeching_energy_rumble", End::Stop),
];

/// The name of every pattern a row fires, once each: what a title's loader
/// reads from its archive.
pub fn pattern_names() -> impl Iterator<Item = &'static str> {
    let rows = ROWS
        .iter()
        .enumerate()
        .filter(|(i, row)| ROWS[..*i].iter().all(|r| r.pattern != row.pattern))
        .map(|(_, row)| row.pattern);
    rows.chain(super::contact::WALL)
        .chain(super::contact::LANDING)
}

/// Patterns the disc loads and no row fires yet, each with the reason.
/// Together with [`ROWS`] they name every one of the 64 ids
/// (`docs/gameplay/rumble.md`, "Wired and unwired").
pub const UNFIRED: &[(&str, &str)] = &[
    ("energy_critical_rumble", "the original never fires it"),
    ("shield_active_rumble", "the original never fires it"),
    ("repulser_impact_rumble", "the original never fires it"),
    ("shuriken_impact_rumble", "the original never fires it"),
    ("quake_incoming_rumble", "the original never fires it"),
    ("repulser_out_of_range", "the original never fires it"),
    ("shuriken_hits_wall", "the original never fires it"),
    ("lock_on_warning", "the original never fires it"),
    ("weapon_impact_shield_rumble", "the original never fires it"),
    ("photo_click", "the original never fires it"),
    ("heartbeat", "HD's counter in FUN_00055848 is unread"),
    (
        "pad_weapon",
        "collecting a weapon pad raises no event to read",
    ),
    (
        "rlr_barrell_roll",
        "the roll gesture arms timers; no single edge says which way",
    ),
    (
        "lrl_barrell_roll",
        "the roll gesture arms timers; no single edge says which way",
    ),
    (
        "turbo_start_rumble",
        "the boost-start event is unread (confidence 60) and Cue::Turbo also fires for the perfect start, a pad and a roll payout",
    ),
    ("fire_engine", "no afterburner-ignition event"),
    (
        "turbo_fired_rumble",
        "Cue::Turbo does not say the pickup fired it",
    ),
    (
        "ship_exploding_rumble",
        "a rival's explosion needs a per-craft edge: only the local ship's is read",
    ),
    (
        "side_shift_left",
        "the gesture machines arm timers; no single edge says a shift started",
    ),
    (
        "side_shift_right",
        "the gesture machines arm timers; no single edge says a shift started",
    ),
    (
        "bomb_fired_rumble",
        "no bomb cue: Weapon_FireBomb raises none",
    ),
    ("mine_explosion_rumble", "no mine-explosion cue"),
    ("bomb_explosion_rumble", "no bomb cue"),
    ("bomb_impact_rumble", "a hit cue names no victim"),
    ("cannon_impact_rumble", "a hit cue names no victim"),
    ("leech_impact_rumble", "a hit cue names no victim"),
    ("mine_impact_rumble", "a hit cue names no victim"),
    ("missile_impact_rumble", "a hit cue names no victim"),
    ("plasma_impact_rumble", "a hit cue names no victim"),
    ("quake_impact_rumble", "a hit cue names no victim"),
    ("rocket_impact_rumble", "a hit cue names no victim"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_set_fires_its_own_rows_and_the_earlier_ones() {
        assert!(Set::Ps4.fires(Set::Ps3) && Set::Ps4.fires(Set::Ps4));
        assert!(Set::Ps3.fires(Set::Ps3) && !Set::Ps3.fires(Set::Ps4));
    }

    #[test]
    fn no_pattern_is_both_fired_and_unfired() {
        for (name, _) in UNFIRED {
            assert!(ROWS.iter().all(|r| r.pattern != *name), "{name}");
        }
    }

    #[test]
    fn a_held_row_is_always_a_level() {
        for row in ROWS {
            assert_eq!(
                matches!(row.shape, Shape::Held(_)),
                matches!(row.trigger, Trigger::Level(_)),
                "{}",
                row.pattern
            );
        }
    }
}
