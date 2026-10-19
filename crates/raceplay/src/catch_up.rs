//! Showing late what a correction's replay first revealed.
//!
//! A network client predicts a remote person on their last held input, which carries no button
//! edges, so a remote fire is not in this client's present until a correction replays the
//! confirmed tick. [`Race::tick_sim_only`] drops the cues and the one-shot effects of the ticks
//! it replays, because for an event the client already presented that is right. For one it did
//! not, it means a missile that exists mid-flight with no launch sound and a blast that never
//! plays.
//!
//! The replay therefore *journals* the weapon events it raises instead of discarding them
//! ([`Race::note_cues`], [`Race::note_blast`], [`Race::note_shield_hit`]), a drawn tick logs the
//! same events as presented, and [`Race::catch_up`] matches the two after the replay. An event the
//! replay raised that the client had not presented is started now, at its current age. One it
//! had presented is left alone, so nothing fires twice.
//!
//! None of this is simulation state: the journal lives on [`Race`], outside [`RaceSim`], and no
//! hash reads it. The cues it adds go on the per-tick output queue (ADR-0018) after the replay,
//! where the next drain finds them.
//!
//! Chosen, not measured: every constant below.

use std::collections::VecDeque;

use oag_gameplay::cue::{Cue, CueEvent};
use oag_tables::weapons::Weapon;

use super::*;

/// Ticks of presented events kept to match a replay against: longer than any prediction window.
const KEEP_TICKS: u32 = 240;
/// How many ticks apart a replayed event and a presented one may be and still be the same event:
/// a corrected input can move a fire by a tick or two.
const MATCH_TICKS: u32 = 4;
/// A sound older than this when a correction reveals it is not played: half a second late reads
/// as a wrong sound, not a late one.
const CUE_MAX_AGE: u32 = 30;
/// A blast older than this is over, or nearly, and is not started.
const BLAST_MAX_AGE: u32 = 90;
/// A shield bulge older than this has faded.
const SHIELD_MAX_AGE: u32 = 20;
/// Entries of the journal kept when nothing reconciles (a replay test, a lean client).
const MISSED_CAP: usize = 256;

/// One thing a drawn tick presented, or a replayed tick would have.
#[derive(Debug, Clone, Copy)]
struct Entry {
    tick: u32,
    what: What,
}

#[derive(Debug, Clone, Copy)]
enum What {
    Cue(CueEvent),
    Blast {
        kind: Weapon,
        point: Vec3,
        struck: Option<usize>,
        orientation: Quat,
    },
    ShieldHit(usize),
}

impl What {
    /// Whether two events are the same one: what and whose, never where, since a corrected
    /// position moves.
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Cue(a), Self::Cue(b)) => a.cue == b.cue && a.slot == b.slot,
            (
                Self::Blast {
                    kind: a, struck: s, ..
                },
                Self::Blast {
                    kind: b, struck: t, ..
                },
            ) => a == b && s == t,
            (Self::ShieldHit(a), Self::ShieldHit(b)) => a == b,
            _ => false,
        }
    }
}

/// The cues that announce a weapon: its launch, its landing and its ending. The pad, lap,
/// countdown and magstrip cues a replay also drops are not here: this lane measured only weapons.
fn is_weapon_cue(cue: Cue) -> bool {
    matches!(
        cue,
        Cue::Turbo
            | Cue::Absorb
            | Cue::Shield
            | Cue::MineLaunch
            | Cue::Plasma
            | Cue::PlasmaHitWall
            | Cue::PlasmaHitShip
            | Cue::Rocket
            | Cue::RocketHitWall
            | Cue::RocketHitShip
            | Cue::Missile
            | Cue::MissileHitWall
            | Cue::MissileHitShip
            | Cue::MissileExpire
            | Cue::Cannon
            | Cue::CannonHitWall
            | Cue::CannonHitShip
            | Cue::QuakeLaunch
            | Cue::QuakeHit
            | Cue::Leach
            | Cue::LeachFail
            | Cue::LeachAttach
            | Cue::ShurikenLaunch
            | Cue::ShurikenHit
            | Cue::Repulsor
            | Cue::RepulsorHit
    )
}

/// What the late presentations of one or more corrections were.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LateReport {
    /// Weapon sounds started late.
    pub cues: u32,
    /// Blasts started late.
    pub blasts: u32,
    /// Shield bulges started late.
    pub shield_hits: u32,
    /// The greatest age, in ticks, of anything started late.
    pub oldest_age: u32,
    /// Events the replay raised that the client had already presented, left alone.
    pub already_presented: u32,
}

/// The two lists a correction compares, and what it started late.
#[derive(Debug, Default)]
pub(crate) struct Journal {
    presented: VecDeque<Entry>,
    missed: Vec<Entry>,
    late: LateReport,
}

impl Race {
    fn record(&mut self, what: What) {
        let entry = Entry {
            tick: u32::try_from(self.sim.world.tick).unwrap_or(0),
            what,
        };
        if self.quiet {
            if self.journal.missed.len() >= MISSED_CAP {
                self.journal.missed.remove(0);
            }
            self.journal.missed.push(entry);
        } else {
            self.journal.presented.push_back(entry);
        }
    }

    /// Logs the weapon cues raised since the queue was `queued` long: presented on a drawn tick,
    /// missed on a replayed one.
    pub(super) fn note_cues(&mut self, queued: usize) {
        let raised: Vec<CueEvent> = self
            .sim
            .cues
            .get(queued..)
            .unwrap_or(&[])
            .iter()
            .copied()
            .filter(|event| is_weapon_cue(event.cue))
            .collect();
        for event in raised {
            self.record(What::Cue(event));
        }
    }

    /// A weapon's blast: played on a drawn tick, journalled on a replayed one.
    pub(super) fn note_blast(
        &mut self,
        kind: Weapon,
        point: Vec3,
        struck: Option<usize>,
        orientation: Quat,
    ) {
        if self.draws() {
            self.ignite_blast(kind, point, struck, orientation);
        }
        self.record(What::Blast {
            kind,
            point,
            struck,
            orientation,
        });
    }

    /// A shield bulge: journalled whichever kind of tick it was; [`Self::shield_hit`] draws it.
    pub(super) fn note_shield_hit(&mut self, slot: usize) {
        self.record(What::ShieldHit(slot));
    }

    /// After a replay from `start`: starts now whatever the replay raised that the client had not
    /// presented, at its age, and leaves what it had.
    pub(super) fn catch_up(&mut self, start: u32) {
        let now = u32::try_from(self.sim.world.tick).unwrap_or(0);
        let missed = std::mem::take(&mut self.journal.missed);
        let mut earlier: Vec<Entry> = Vec::new();
        self.journal.presented.retain(|entry| {
            let kept = entry.tick < start;
            if !kept {
                earlier.push(*entry);
            }
            kept
        });
        for entry in missed {
            let seen = earlier.iter().position(|e| {
                e.tick.abs_diff(entry.tick) <= MATCH_TICKS && e.what.same(&entry.what)
            });
            if let Some(index) = seen {
                earlier.swap_remove(index);
                self.journal.late.already_presented += 1;
            } else {
                self.present_late(entry, now.saturating_sub(entry.tick));
            }
            self.journal.presented.push_back(entry);
        }
        while self
            .journal
            .presented
            .front()
            .is_some_and(|e| e.tick + KEEP_TICKS < now)
        {
            self.journal.presented.pop_front();
        }
    }

    fn present_late(&mut self, entry: Entry, age: u32) {
        let late = &mut self.journal.late;
        match entry.what {
            What::Cue(event) if age <= CUE_MAX_AGE => {
                late.cues += 1;
                late.oldest_age = late.oldest_age.max(age);
                self.sim.cues.push(event);
            }
            What::Blast {
                kind,
                point,
                struck,
                orientation,
            } if age <= BLAST_MAX_AGE => {
                late.blasts += 1;
                late.oldest_age = late.oldest_age.max(age);
                if let Some(playing) = self.ignite_blast(kind, point, struck, orientation) {
                    let dt = self.sim.dt;
                    self.view
                        .stage
                        .pre_age(playing, age, dt, &mut self.view.stage_rng);
                }
            }
            What::ShieldHit(slot) if age <= SHIELD_MAX_AGE => {
                late.shield_hits += 1;
                late.oldest_age = late.oldest_age.max(age);
                let dt = self.sim.dt;
                if let Some(shell) = self.view.shield.get_mut(slot) {
                    shell.hit();
                    for _ in 0..age {
                        shell.advance(dt);
                    }
                }
            }
            _ => {}
        }
    }

    /// What the corrections since the last call started late, and resets it.
    pub fn take_late_report(&mut self) -> LateReport {
        std::mem::take(&mut self.journal.late)
    }
}
