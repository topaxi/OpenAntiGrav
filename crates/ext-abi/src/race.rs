//! The question a module is asked once, before the race is built: does it run in this one?
//!
//! A guest may export `oag_wants_race() -> u32`. The host calls it once at race load, on a
//! fresh instance, with the race described at the start of the I/O buffer ([`RaceInfo`]),
//! and the guest answers [`RUN`] or [`SKIP`]. A module that answers [`SKIP`] is dropped
//! from that race as if its mod were off: no `tick`, no fuel, no state, no pickup kind, no
//! HUD, sound, effect or shader of its own, and the race is not marked modded on its
//! account. A module that exports nothing runs in every race, as before.
//!
//! The description is its own layout, not the tick observation's: the call happens before
//! a race has a tick, a craft or a track.

use crate::{ABI, IO_LEN, OPTIONS_LEN, get_u32, put_u32};

/// What `oag_wants_race` returns for "run in this race".
pub const RUN: u32 = 1;
/// What `oag_wants_race` returns for "do not run in this race". Any other value, a trap or
/// running out of fuel leaves the module out of the race by name, so a broken check never
/// runs a module the guest meant to withhold.
pub const SKIP: u32 = 0;

/// Where the ABI major is written.
pub const RACE_ABI_AT: usize = 0;
/// Where the title's key is written: [`crate::channel_key`] of its lowercased hyphenated
/// name (`wipeout-hd`), 0 when unknown.
pub const RACE_TITLE_AT: usize = 4;
/// Where the speed class's key is written: [`crate::channel_key`] of its lowercased name
/// (`venom`), 0 when the race has none.
pub const RACE_CLASS_AT: usize = 8;
/// Where the mode code is written, one byte ([`RaceMode::code`]).
pub const RACE_MODE_AT: usize = 12;
/// Where the flags byte is written ([`RACE_WEAPONS`], [`RACE_CAMPAIGN`]).
pub const RACE_FLAGS_AT: usize = 13;
/// Where the number of people flying is written, one byte.
pub const RACE_PEOPLE_AT: usize = 14;
/// Where the number of craft on the grid is written, one byte.
pub const RACE_CRAFTS_AT: usize = 15;
/// Where the module's options are written ([`crate::OPTIONS_LEN`] bytes, as in a tick).
pub const RACE_OPTIONS_AT: usize = 16;
/// The description's length.
pub const RACE_LEN: usize = RACE_OPTIONS_AT + OPTIONS_LEN;

/// A flag: the race has weapons (pads hand out pickups, hits do damage).
pub const RACE_WEAPONS: u8 = 1;
/// A flag: the race is an event of a campaign (a title's downloadable one or a mod's).
pub const RACE_CAMPAIGN: u8 = 2;

const _: () = assert!(RACE_LEN <= IO_LEN);

coded! {
    /// The race's mode, in the engine's `oag_race::Mode` order from 1 (0 is "unknown").
    RaceMode: u8, first 1 {
        /// Laps against the clock; no weapons.
        TimeTrial,
        /// One fast lap, unlimited laps; no weapons.
        SpeedLap,
        /// Escalating auto-speed; no weapons.
        Zone,
        /// A lap race against a full grid, weapons optional.
        SingleRace,
        /// Race for kills.
        Eliminator,
        /// One leg of a series of single races.
        Tournament,
        /// Two craft.
        Head2Head,
        /// A detonator race.
        Detonator,
    }
}

/// The race a module is asked about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaceInfo {
    /// The title's key (`channel_key` of its slug); 0 when unknown.
    pub title: u32,
    /// The class's key (`channel_key` of its lowercased name); 0 when the race has none.
    pub class: u32,
    /// The mode.
    pub mode: RaceMode,
    /// Weapons are on.
    pub weapons: bool,
    /// The race is an event of a campaign.
    pub campaign: bool,
    /// People flying, across every window.
    pub people: u8,
    /// Craft on the grid, people included.
    pub crafts: u8,
    /// The module's options, as a tick would give them.
    pub options: [u8; OPTIONS_LEN],
}

impl RaceInfo {
    /// Writes the description at the start of a whole buffer.
    pub fn write(&self, io: &mut [u8; IO_LEN]) {
        put_u32(io, RACE_ABI_AT, ABI);
        put_u32(io, RACE_TITLE_AT, self.title);
        put_u32(io, RACE_CLASS_AT, self.class);
        io[RACE_MODE_AT] = self.mode.code();
        io[RACE_FLAGS_AT] = if self.weapons { RACE_WEAPONS } else { 0 }
            | if self.campaign { RACE_CAMPAIGN } else { 0 };
        io[RACE_PEOPLE_AT] = self.people;
        io[RACE_CRAFTS_AT] = self.crafts;
        io[RACE_OPTIONS_AT..RACE_OPTIONS_AT + OPTIONS_LEN].copy_from_slice(&self.options);
    }

    /// The description a buffer holds, or `None` when its mode code is not one of
    /// [`RaceMode::ALL`].
    #[must_use]
    pub fn read(io: &[u8; IO_LEN]) -> Option<Self> {
        let mut options = [0; OPTIONS_LEN];
        options.copy_from_slice(&io[RACE_OPTIONS_AT..RACE_OPTIONS_AT + OPTIONS_LEN]);
        Some(Self {
            title: get_u32(io, RACE_TITLE_AT),
            class: get_u32(io, RACE_CLASS_AT),
            mode: RaceMode::from_code(io[RACE_MODE_AT])?,
            weapons: io[RACE_FLAGS_AT] & RACE_WEAPONS != 0,
            campaign: io[RACE_FLAGS_AT] & RACE_CAMPAIGN != 0,
            people: io[RACE_PEOPLE_AT],
            crafts: io[RACE_CRAFTS_AT],
            options,
        })
    }
}
