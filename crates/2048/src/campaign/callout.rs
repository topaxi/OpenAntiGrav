//! The weapon callout of an event card (`FUN_810626ce`, `0x810626ce`): the
//! row of weapon icons the rules page and the objective page show, or the
//! single "weapons off" style icon when the event restricts the pickups.
//!
//! Evidence and confidence: `docs/ghidra/functions/vita-2048-eu-v104/
//! weapon-callout.md`. [`law`] copies the executable's predicates literally;
//! [`for_event`] feeds it what `SP.xml` authors and the constructor defaults
//! for what it does not.

use oag_tables::mjolnir::campaign::{Event, typedef, weapon_set_for};
use oag_tables::mjolnir::{Document, Field};

/// The eleven weapons in `WeaponType` bit order, as the file stem under
/// `NewImages\weapons\` (`FUN_8105dcb8` loads them into `DAT_816c8858..` in
/// this order, which is the bit order `weapon-type-bits.md` derived from the
/// data, so the two corroborate each other).
pub const ICONS: [&str; 11] = [
    "rocket",
    "missile",
    "quake",
    "speedup",
    "shield",
    "cannon",
    "autopilot",
    "plasma",
    "bomb",
    "mine",
    "leach",
];

/// The string id of each weapon's name, in the same order (`0x8151caf4`).
pub const NAME_IDS: [&str; 11] = [
    "FE_ROCKETS",
    "FE_MISSILE",
    "FE_QUAKE",
    "FE_TURBO",
    "FE_SHIELD",
    "FE_CANNON",
    "FE_AUTOPILOT",
    "FE_PLASMA",
    "FE_BOMB",
    "FE_MINES",
    "FE_LEECHBEAM",
];

/// The separator the names are joined with (`DAT_8142a998`).
pub const SEPARATOR: &str = " + ";

/// `GameModeBase`'s constructor sets bits `0..=10` of `m_weaponSet`
/// (`FUN_812afd9e`: the loop skips only `11` and `12`): an event that
/// authors no set offers every weapon.
pub const DEFAULT_BITS: i64 = 0x7ff;

/// `m_activeWeaponPads` is `3` in both `FUN_812bb914` (`GameMode_ArcadeRace`)
/// and `FUN_812be628` (`GameMode_EliminatorRace`).
pub const DEFAULT_PADS: i64 = 3;

/// What the callout shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Callout {
    /// Nothing: every weapon is on offer, so there is nothing to say.
    Nothing,
    /// `weapons/weapons_off` with `Event_Variable_Weapons_Off_0`.
    WeaponsOff,
    /// `weapons/offensive_off` with `Event_Variable_Offensive_1`.
    OffensiveOff,
    /// `weapons/defensive_off` with `Event_Variable_Defensive_1`.
    DefensiveOff,
    /// The weapons on offer, as indices into [`ICONS`], ascending.
    Weapons(Vec<usize>),
}

/// The weapons the executable calls offensive: Rocket, Missile, Quake,
/// Cannon and Plasma (`FUN_810626ce`'s `bVar8`).
const OFFENSIVE: i64 = 0b1010_0111;
/// The defensive six for a race (Turbo, Shield, Autopilot, Bomb, Mine and
/// Leach Beam), and the three an elimination needs (`bVar7`).
const DEFENSIVE_RACE: i64 = 0b111_0101_1000;
const DEFENSIVE_ELIMINATION: i64 = 0b111_0000_0000;

/// `FUN_810626ce`'s decision tree, copied as it reads: `bits` is `m_weaponSet`,
/// `pads` is `m_activeWeaponPads`, `elimination` is `+0x190 == 1`, and
/// `disabled` is the `+0x64` slot (`m_bDisableWeapons`, read by
/// `GameMode_ArcadeRace` alone).
#[must_use]
pub fn law(bits: i64, pads: i64, elimination: bool, disabled: bool) -> Callout {
    let all_offensive = bits & OFFENSIVE == OFFENSIVE;
    let defensive = if elimination {
        DEFENSIVE_ELIMINATION
    } else {
        DEFENSIVE_RACE
    };
    let all_defensive = bits & defensive == defensive;
    let pads_a = pads & 0b0101 != 0;
    let pads_b = pads & 0b1010 != 0;
    if disabled || !(pads_a || pads_b) {
        return Callout::WeaponsOff;
    }
    if !pads_a && all_defensive {
        return Callout::OffensiveOff;
    }
    if pads_b {
        if all_offensive && all_defensive {
            return Callout::Nothing;
        }
    } else if all_offensive {
        return Callout::DefensiveOff;
    }
    let listed: Vec<usize> = (0..ICONS.len())
        .filter(|bit| bits & (1 << bit) != 0)
        .collect();
    if listed.is_empty() {
        Callout::WeaponsOff
    } else {
        Callout::Weapons(listed)
    }
}

/// The callout for `event`, `None` when the card has no callout item at all:
/// the original skips a Zone event and a Speed Lap event (`+0x190` of `0` or
/// `3`) before it asks.
#[must_use]
pub fn for_event(document: &Document, event: &Event) -> Option<Callout> {
    let elimination = match event.typedef_id {
        typedef::RACE_B => false,
        typedef::ELIMINATION => true,
        _ => return None,
    };
    let instance = document.instance(event.instance_id)?;
    let bits = event
        .weapon_set
        .and_then(|reference| weapon_set_for(document, reference))
        .and_then(|set| set.available_bits)
        .unwrap_or(DEFAULT_BITS);
    let pads = instance
        .field("M_ACTIVEWEAPONPADS")
        .and_then(Field::int)
        .unwrap_or(DEFAULT_PADS);
    let disabled = event.typedef_id == typedef::RACE_B
        && instance
            .field("M_BDISABLEWEAPONS")
            .and_then(Field::bool)
            .unwrap_or(false);
    Some(law(bits, pads, elimination, disabled))
}

/// The texture for the one-icon outcomes and each weapon, spelled the way
/// the card spells every texture.
#[must_use]
pub fn texture(stem: &str) -> String {
    format!(r"Data\FE\NewImages\weapons\{stem}.gtf")
}
