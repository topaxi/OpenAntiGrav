//! One tick of a race, as the sound effects see it: a plain-data snapshot the
//! host builds after `Race::tick` (cue queues already drained), so the sound
//! crate does not depend on the composition root. Values or borrows of decoded
//! sound data only; nothing here can step the simulation.

use oag_core::math::Vec3;
use oag_weapons::projectile::leach_beam::Beam;
use oag_weapons::projectile::{MAX_PROJECTILES, Projectile};

use super::{Announcer, Banks, ClassAnnouncer, CueEvent, TrackEmitters};

/// What the effects need to know about one tick of a race.
#[derive(Debug)]
pub struct RaceFrame<'a> {
    /// The race's decoded effect banks.
    pub banks: &'a Banks,
    /// The circuit's authored ambience emitters.
    pub emitters: &'a TrackEmitters,
    /// The Zone milestone announcer.
    pub announcer: &'a Announcer,
    /// The Zone speed-class announcer.
    pub class_announcer: &'a ClassAnnouncer,
    /// The cues the race raised this tick, drained.
    pub cues: Vec<CueEvent>,
    /// The Zone milestones raised this tick, drained.
    pub announcements: Vec<u16>,
    /// The Zone speed-class stages raised this tick, drained.
    pub class_announcements: Vec<u32>,
    /// The ears, off the camera the frame is drawn from.
    pub listener: oag_audio::Listener,
    /// Every live craft's position and speed by grid slot; [`None`] for a slot
    /// the race did not field.
    pub craft: [Option<(Vec3, f32)>; oag_gameplay::MAX_SHIPS],
    /// Each grid slot's team name, for HD's crossfaded engine table.
    pub slot_teams: [Option<&'a str>; oag_gameplay::MAX_SHIPS],
    /// The local player's throttle, `0..=100`.
    pub throttle: f32,
    /// Every projectile slot, read after the tick has run.
    pub projectiles: [Projectile; MAX_PROJECTILES],
    /// Whether the race is still being run, which is false once it finishes.
    pub running: bool,
    /// Whether the player's shield pickup is up.
    pub shielded: bool,
    /// Whether the player's craft is mid-explosion.
    pub exploding: bool,
    /// Whether the player's own Autopilot pickup is active.
    pub autopilot_active: bool,
    /// Whether the grid is still held on the countdown (thrust gated).
    pub thrust_gated: bool,
    /// What the lock-on reticle is doing.
    pub sight: oag_race::sight::State,
    /// The quake wave's road midpoint, while one travels.
    pub quake_point: Option<Vec3>,
    /// The world's one Leach beam, while one exists.
    pub leach_beam: Option<Beam>,
}
