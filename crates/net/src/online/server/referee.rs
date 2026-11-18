//! The lobby as the referee of player-hosted races: a client's case against its host arrives in
//! parts, is bounded, checked against the host's signed checkpoint, and then either
//! re-simulated from the job the lobby issued (a state the host's own inputs do not give) or
//! put to the host as a challenge (an input its seat never sent). A proven case is a strike on
//! the host's key (`crate::online::moderation`, which bans on enough of them) and closes its
//! room with the reason
//! (`docs/architecture/adr/0092-a-player-hosted-race-is-a-room-whose-owner-runs-the-race-host.md`,
//! "The anti-cheat path").
//!
//! Everything a reporter controls is bounded: one report a seat a race, [`MAX_ASSEMBLING`]
//! assembling at once, a proof no longer than the ticks the race can have run, one
//! re-simulation at a time on its own thread with [`JUDGE_TIME`] to finish, and
//! [`MAX_QUEUED`] waiting. Every bound is **chosen, not measured**.

use std::collections::{BTreeMap, VecDeque};
use std::net::SocketAddr;
use std::sync::mpsc;

use web_time::{Duration, Instant};

use super::hosting::short;
use super::{ClientId, Server};
use crate::fair::{Claim, MAX_PROOF_BYTES, PART_BYTES, Proof, Verdict};
use crate::online::moderation::{STRIKES_TO_BAN, Strike, race_id, span};
use crate::online::race_proc::Loader;
use crate::wire::online::{Message, Refusal};
use crate::wire::{Confirmed, Held, RaceParams};

/// Reports being received at once; the next is refused until one is whole.
pub const MAX_ASSEMBLING: usize = 2;

/// Desync cases waiting for the re-simulation; the next is refused.
pub const MAX_QUEUED: usize = 4;

/// How long one re-simulation may run before it is given up as not checkable.
pub const JUDGE_TIME: Duration = Duration::from_secs(60);

/// How long a host has to show a challenged input's signed datagram.
pub const CHALLENGE_TIMEOUT: Duration = Duration::from_secs(10);

/// How long after its race ended a report is still taken: the end's last checkpoint can
/// cover a desync near the finish.
const REPORT_GRACE: Duration = Duration::from_secs(60);

/// Ticks a proof may run past the lobby's own clock of the race: the host's lead and the
/// countdown before its first tick.
const TICK_SLACK: u32 = 600;

/// Rulings kept for [`Server::rulings`].
const MAX_RULINGS: usize = 256;

/// The most proof bytes the lobby holds at once, reports being received and decoded ones
/// waiting or being judged together (a decoded proof counted at twice its bytes): with one
/// race being re-simulated, about 100 MB at the worst.
pub const MAX_HELD_BYTES: usize = 64 << 20;

/// What became of one report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ruling {
    /// Refused before anything was judged: malformed, oversized, unsigned, one too many.
    Refused {
        /// The room.
        room: u32,
        /// The reporter's name.
        reporter: String,
        /// Why.
        why: String,
    },
    /// Judged.
    Judged {
        /// The room.
        room: u32,
        /// The reporter's name.
        reporter: String,
        /// The verdict.
        verdict: Verdict,
    },
}

/// What a player-hosted race is judged against, kept past its end for [`REPORT_GRACE`].
#[derive(Clone)]
pub(super) struct RaceCase {
    pub(super) host: ClientId,
    pub(super) host_key: [u8; 32],
    pub(super) host_addr: SocketAddr,
    pub(super) params: RaceParams,
    pub(super) hash: [u8; 32],
    /// The build the host said at its hello (`Edition::build`), empty when it named none.
    pub(super) host_build: String,
    /// `(client, name, player key)` by seat.
    pub(super) seats: Vec<(ClientId, String, [u8; 32])>,
    pub(super) launched: Instant,
    /// When the host said its race was loaded: no race has run more ticks than the time since.
    pub(super) up: Option<Instant>,
    ended: Option<Instant>,
    reported: Vec<ClientId>,
}

impl RaceCase {
    pub(super) fn new(
        host: ClientId,
        host_key: [u8; 32],
        host_addr: SocketAddr,
        host_build: String,
        params: RaceParams,
        seats: Vec<(ClientId, String, [u8; 32])>,
    ) -> Self {
        Self {
            host,
            host_key,
            host_addr,
            hash: crate::fair::params_hash(&params),
            params,
            host_build,
            seats,
            launched: Instant::now(),
            up: None,
            ended: None,
            reported: Vec::new(),
        }
    }

    /// Ticks of wall clock since the host said the race was up, 0 before.
    fn ticks_since_up(&self) -> u32 {
        self.up.map_or(0, |up| {
            u32::try_from(up.elapsed().as_millis() * 60 / 1000).unwrap_or(u32::MAX)
        })
    }

    /// The most ticks the race can have run.
    fn ticks_so_far(&self) -> u32 {
        let ran = self
            .ended
            .unwrap_or_else(Instant::now)
            .saturating_duration_since(self.launched);
        u32::try_from(ran.as_millis() * 60 / 1000)
            .unwrap_or(u32::MAX)
            .saturating_add(TICK_SLACK)
    }
}

/// One report: who made it about which race.
#[derive(Clone)]
struct Case {
    room: u32,
    race: RaceCase,
    reporter: String,
    seat: u8,
}

struct Assembling {
    room: u32,
    from: ClientId,
    case: Case,
    parts: u16,
    next: u16,
    bytes: Vec<u8>,
    max_bytes: usize,
}

struct Challenge {
    case: Case,
    tick: u32,
    expected: Held,
    deadline: Instant,
}

struct Judging {
    case: Case,
    verdict: mpsc::Receiver<Verdict>,
    bytes: usize,
}

/// The lobby's reports, challenges and re-simulations.
#[derive(Default)]
pub(super) struct Referee {
    load: Option<Loader>,
    pub(super) races: BTreeMap<u32, RaceCase>,
    assembling: Vec<Assembling>,
    /// Desync cases waiting, each with the bytes of its proof.
    queue: VecDeque<(Case, Proof, usize)>,
    judging: Option<Judging>,
    challenges: Vec<Challenge>,
    judgements: u32,
    rulings: Vec<Ruling>,
}

impl Server {
    /// Lets the lobby re-simulate a reported race with `load`: the same race a race child loads
    /// for a job. Without it a desync report is evidence only.
    pub fn set_referee(&mut self, load: Loader) {
        self.referee.load = Some(load);
    }

    /// How many re-simulations the lobby has started.
    #[must_use]
    pub fn judgements(&self) -> u32 {
        self.referee.judgements
    }

    /// What became of the reports so far, oldest first.
    #[must_use]
    pub fn rulings(&self) -> &[Ruling] {
        &self.referee.rulings
    }

    fn rule(&mut self, ruling: Ruling) {
        let referee = &mut self.referee;
        if referee.rulings.len() >= MAX_RULINGS {
            referee.rulings.remove(0);
        }
        referee.rulings.push(ruling);
    }

    fn refuse_report(&mut self, id: ClientId, room: u32, reporter: &str, why: String) {
        log::info!("host-log: room {room}: a report by {reporter:?} is refused: {why}");
        if let Some(client) = self.clients.get_mut(&id) {
            client.link.send_lobby(&Message::Rejected {
                code: Refusal::BadHost,
                text: format!("your report was refused: {why}"),
            });
        }
        self.rule(Ruling::Refused {
            room,
            reporter: reporter.to_string(),
            why,
        });
    }

    /// One part of a report on `room`'s hosted race from `id`.
    pub(super) fn report_part(
        &mut self,
        id: ClientId,
        room: u32,
        part: u16,
        parts: u16,
        bytes: Vec<u8>,
    ) {
        let reporter = self.rooms.name(id).unwrap_or_default().to_string();
        let open = self
            .referee
            .assembling
            .iter()
            .position(|a| a.from == id && a.room == room);
        if part == 0 {
            if let Some(stale) = open {
                self.referee.assembling.remove(stale);
            }
            let Some(race) = self.referee.races.get(&room).cloned() else {
                return self.refuse_report(id, room, &reporter, "no player-hosted race".into());
            };
            if race.ended.is_some_and(|at| at.elapsed() > REPORT_GRACE) {
                return self.refuse_report(id, room, &reporter, "the race is long over".into());
            }
            let Some(seat) = race.seats.iter().position(|(seat, _, _)| *seat == id) else {
                return self.refuse_report(id, room, &reporter, "you had no seat in it".into());
            };
            if id == race.host {
                return self.refuse_report(id, room, &reporter, "the host's own seat".into());
            }
            if race.reported.contains(&id) {
                return self.refuse_report(id, room, &reporter, "one report a race".into());
            }
            let held = self
                .referee
                .assembling
                .iter()
                .map(|a| a.max_bytes)
                .sum::<usize>()
                + 2 * self.referee.queue.iter().map(|(_, _, b)| b).sum::<usize>()
                + 2 * self.referee.judging.as_ref().map_or(0, |j| j.bytes);
            if self.referee.assembling.len() >= MAX_ASSEMBLING || held >= MAX_HELD_BYTES {
                let why = "the lobby is receiving other reports: try later".into();
                return self.refuse_report(id, room, &reporter, why);
            }
            let max_bytes = (200
                + race.ticks_so_far() as usize * Confirmed::record_bytes(race.seats.len()))
            .min(MAX_PROOF_BYTES)
            .min(MAX_HELD_BYTES - held);
            if usize::from(parts.max(1) - 1) * PART_BYTES >= max_bytes {
                let why = format!("{parts} parts is more than the race can have run");
                return self.refuse_report(id, room, &reporter, why);
            }
            if let Some(race) = self.referee.races.get_mut(&room) {
                race.reported.push(id);
            }
            self.referee.assembling.push(Assembling {
                room,
                from: id,
                case: Case {
                    room,
                    race,
                    reporter: reporter.clone(),
                    seat: u8::try_from(seat).unwrap_or(u8::MAX),
                },
                parts,
                next: 0,
                bytes: Vec::new(),
                max_bytes,
            });
        }
        let Some(at) = self
            .referee
            .assembling
            .iter()
            .position(|a| a.from == id && a.room == room)
        else {
            return self.refuse_report(id, room, &reporter, "a part of no report".into());
        };
        let assembling = &mut self.referee.assembling[at];
        if part != assembling.next || parts != assembling.parts {
            self.referee.assembling.remove(at);
            return self.refuse_report(id, room, &reporter, "parts out of order".into());
        }
        if bytes.len() > PART_BYTES || assembling.bytes.len() + bytes.len() > assembling.max_bytes {
            self.referee.assembling.remove(at);
            let why = "longer than the race can have run".into();
            return self.refuse_report(id, room, &reporter, why);
        }
        assembling.bytes.extend_from_slice(&bytes);
        assembling.next += 1;
        if assembling.next < assembling.parts {
            return;
        }
        let Assembling { case, bytes, .. } = self.referee.assembling.remove(at);
        self.weigh(id, case, &bytes);
    }

    /// A whole report: parsed against its race, its records checked against the host's signed
    /// checkpoint, then judged.
    fn weigh(&mut self, id: ClientId, case: Case, bytes: &[u8]) {
        let (room, race) = (case.room, &case.race);
        let proof = match Proof::decode(bytes, race.seats.len(), race.ticks_so_far()) {
            Ok(proof) => proof,
            Err(e) => return self.refuse_report(id, room, &case.reporter, e.to_string()),
        };
        if let Err(why) = proof.check_signed(&race.hash, &race.host_key) {
            return self.refuse_report(id, room, &case.reporter, why);
        }
        log::info!(
            "host-log: room {room}: {:?} reports the host (key {}): {:?} up to tick {}",
            case.reporter,
            short(&race.host_key),
            proof.claim,
            proof.checkpoint.ticks
        );
        match proof.claim {
            Claim::Desync => {
                // A re-simulation by this build says nothing about a host running another one.
                let ours = self.rooms.own_build();
                if race.host_build != ours {
                    let why = format!(
                        "the host runs another build ({} vs {ours})",
                        if race.host_build.is_empty() {
                            "unknown"
                        } else {
                            &race.host_build
                        }
                    );
                    return self.decide(&case, Verdict::NotCheckable(why));
                }
                if self.referee.queue.len() >= MAX_QUEUED {
                    let why = "the lobby is judging other reports: try later".into();
                    return self.refuse_report(id, room, &case.reporter, why);
                }
                self.referee.queue.push_back((case, proof, bytes.len()));
            }
            Claim::Forged { tick, seat } => self.challenge(case, &proof, seat, tick),
        }
    }

    /// Puts a forgery claim to the host: show the seat's signed datagram behind the input.
    fn challenge(&mut self, case: Case, proof: &Proof, seat: u8, tick: u32) {
        let verdict = if seat != case.seat {
            Some(Verdict::Dismissed(
                "a seat reports only its own inputs".into(),
            ))
        } else if case.race.seats[usize::from(seat)].2 == [0; 32] {
            Some(Verdict::NotCheckable("the seat signs no inputs".into()))
        } else if !self.clients.contains_key(&case.race.host) {
            Some(Verdict::NotCheckable("the host has left the lobby".into()))
        } else if crate::fair::too_old_to_challenge(case.race.ticks_since_up(), tick) {
            Some(Verdict::NotCheckable(format!(
                "tick {tick} is older than the host is asked to keep its inputs"
            )))
        } else {
            match proof.forged_input(seat, tick) {
                Err(why) => Some(Verdict::Dismissed(why)),
                Ok(expected) => {
                    if let Some(client) = self.clients.get_mut(&case.race.host) {
                        client.link.send_lobby(&Message::ShowInput {
                            room: case.room,
                            seat,
                            tick,
                        });
                    }
                    self.referee.challenges.push(Challenge {
                        case: case.clone(),
                        tick,
                        expected,
                        deadline: Instant::now() + CHALLENGE_TIMEOUT,
                    });
                    None
                }
            }
        };
        if let Some(verdict) = verdict {
            self.decide(&case, verdict);
        }
    }

    /// The host's answer to a challenge.
    pub(super) fn input_shown(
        &mut self,
        id: ClientId,
        room: u32,
        seat: u8,
        tick: u32,
        datagram: &[u8],
    ) {
        let Some(at) = self.referee.challenges.iter().position(|c| {
            c.case.room == room && c.case.race.host == id && c.case.seat == seat && c.tick == tick
        }) else {
            return;
        };
        let challenge = self.referee.challenges.remove(at);
        let race = &challenge.case.race;
        let key = race.seats[usize::from(seat)].2;
        let verdict = if crate::fair::answer_holds(
            datagram,
            &race.hash,
            seat,
            tick,
            challenge.expected,
            &key,
        ) {
            Verdict::Dismissed(format!(
                "the host showed seat {seat}'s signed input of tick {tick}"
            ))
        } else {
            Verdict::Proven(format!(
                "the host confirmed an input for seat {seat} at tick {tick} that the seat never \
                 signed"
            ))
        };
        self.decide(&challenge.case, verdict);
    }

    /// Every pass: races that ended, challenges that ran out, the re-simulation.
    pub(super) fn watch_referee(&mut self) {
        let hosted = &self.hosted;
        self.referee.races.retain(|room, race| {
            if race.ended.is_none() && hosted.get(room).is_none_or(|h| h.host != race.host) {
                race.ended = Some(Instant::now());
            }
            race.ended.is_none_or(|at| at.elapsed() <= REPORT_GRACE)
        });
        for challenge in std::mem::take(&mut self.referee.challenges) {
            if Instant::now() < challenge.deadline {
                self.referee.challenges.push(challenge);
                continue;
            }
            // A host whose link closed cannot answer: an honest one exits after its race.
            let verdict = if self.clients.contains_key(&challenge.case.race.host) {
                Verdict::Proven(format!(
                    "the host did not show seat {}'s signed input of tick {} within {} s",
                    challenge.case.seat,
                    challenge.tick,
                    CHALLENGE_TIMEOUT.as_secs()
                ))
            } else {
                Verdict::NotCheckable("the host left before it answered".into())
            };
            self.decide(&challenge.case, verdict);
        }
        if let Some(judging) = &self.referee.judging {
            let verdict = match judging.verdict.try_recv() {
                Ok(verdict) => verdict,
                Err(mpsc::TryRecvError::Empty) => return,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Verdict::NotCheckable("the re-simulation stopped".into())
                }
            };
            let case = judging.case.clone();
            self.referee.judging = None;
            self.decide(&case, verdict);
        }
        let Some((case, proof, bytes)) = self.referee.queue.pop_front() else {
            return;
        };
        let Some(load) = self.referee.load.clone() else {
            let why = format!(
                "the lobby has no copy of {} to re-simulate",
                case.race.params.title
            );
            return self.decide(&case, Verdict::NotCheckable(why));
        };
        let (tx, verdict) = mpsc::channel();
        let params = case.race.params.clone();
        let spawned = std::thread::Builder::new()
            .name("oag-referee".into())
            .spawn(move || {
                let deadline = Instant::now() + JUDGE_TIME;
                // A server running from an extraction answers a read it lacks with an absence,
                // which can change the race: a judgement over one is no proof. Only the referee
                // loads races in the lobby's process, so the misses are its own.
                let _ = oag_assets::record::take_misses();
                let verdict = match load(&params) {
                    Ok(mut race) => crate::fair::judge(&mut race, &proof, deadline),
                    Err(e) => {
                        Verdict::NotCheckable(format!("the lobby cannot load the race: {e:#}"))
                    }
                };
                let missed = oag_assets::record::take_misses();
                let verdict = match (verdict, missed.first()) {
                    (Verdict::Proven(_), Some(miss)) => Verdict::NotCheckable(format!(
                        "the lobby's copy lacks a read the race made ({miss})"
                    )),
                    (verdict, _) => verdict,
                };
                let _ = tx.send(verdict);
            });
        match spawned {
            Ok(_) => {
                self.referee.judgements += 1;
                self.referee.judging = Some(Judging {
                    case,
                    verdict,
                    bytes,
                });
            }
            Err(e) => self.decide(&case, Verdict::NotCheckable(format!("no thread: {e}"))),
        }
    }

    /// Acts on a verdict: a proven case strikes the host's key, which may ban it, and closes its
    /// room.
    fn decide(&mut self, case: &Case, verdict: Verdict) {
        let (room, race) = (case.room, &case.race);
        match &verdict {
            Verdict::Proven(why) => {
                let now = std::time::SystemTime::now();
                let strike = Strike {
                    key: race.host_key,
                    at: now,
                    race: race_id(&race.hash),
                    by: race
                        .seats
                        .get(usize::from(case.seat))
                        .map_or([0; 32], |seat| seat.2),
                    reason: why.clone(),
                };
                let standing = self.bans.strike(&strike, now);
                let banned = standing
                    .ban
                    .map(|length| format!(", banned for {}", span(length)))
                    .unwrap_or_default();
                log::info!(
                    "host-log: room {room}: {:?}'s report is proven: {why}: key {} addr {} gets a \
                     strike ({} of {STRIKES_TO_BAN} counted){banned}",
                    case.reporter,
                    short(&race.host_key),
                    race.host_addr,
                    standing.counted
                );
                if self.hosted.get(&room).is_some_and(|h| h.host == race.host) {
                    self.hosted.remove(&room);
                }
                if self.rooms.hosted_by(race.host) == Some(room) {
                    self.rooms.close(
                        room,
                        &format!(
                            "the lobby closed this game: its host was caught cheating ({why})"
                        ),
                    );
                }
            }
            Verdict::Dismissed(why) => log::info!(
                "host-log: room {room}: {:?}'s report is dismissed: {why}",
                case.reporter
            ),
            Verdict::NotCheckable(why) => log::info!(
                "host-log: room {room}: {:?}'s report cannot be checked, kept as evidence: {why}",
                case.reporter
            ),
        }
        self.rule(Ruling::Judged {
            room,
            reporter: case.reporter.clone(),
            verdict,
        });
    }
}
