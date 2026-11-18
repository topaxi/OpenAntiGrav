//! The lobby's side of a player-hosted race: the signed offer, the ban list, the probe of the
//! offered port, the launch sent to the owner instead of a race child, and the hand-off to the
//! owner's address
//! (`docs/architecture/adr/0092-a-player-hosted-race-is-a-room-whose-owner-runs-the-race-host.md`).

use std::net::{IpAddr, SocketAddr};
use std::sync::mpsc;

use web_time::{Duration, Instant};

use super::{Server, token};
use crate::online::hostkey;
use crate::online::moderation::canonical;
use crate::online::race_proc::QUIET_TIMEOUT;
use crate::online::rooms::{ClientId, HOST_LEFT_RACING, Launch};
use crate::wire::board::Board;
use crate::wire::online::{HostOffer, Message, RaceJob, Refusal, RoomSettings};

/// Player-hosted rooms the lobby lists at once, probes under way included. Chosen.
pub const MAX_HOSTED_ROOMS: usize = 32;

/// Player-hosted rooms one address may hold at once. Chosen.
pub const MAX_HOSTED_PER_ADDRESS: usize = 2;

/// Probes of offered ports under way at once. Chosen: each is a thread for at most 5 s.
pub const MAX_PROBES: usize = 4;

/// The least time between two hosting requests of one connection. Chosen.
pub const HOST_ASK_INTERVAL: Duration = Duration::from_secs(5);

/// How long a host has to load the race it was sent before the room is given up. Chosen: the
/// race child's own load timeout.
const HOST_LOAD_TIMEOUT: Duration = Duration::from_secs(120);

/// How long after its launch a hosted race the lobby hears nothing of is declared over.
/// Chosen: the 30 minutes a server race may run, and its join time.
const HOSTED_RACE_LIMIT: Duration = Duration::from_secs(35 * 60);

/// A player whose offer verified and whose port answered.
pub(super) struct HostRec {
    pub(super) key: [u8; 32],
    pub(super) addr: SocketAddr,
    cert_hash: [u8; 32],
}

/// An offered port being dialled, off the serving thread.
pub(super) struct Probe {
    client: ClientId,
    settings: RoomSettings,
    offer: HostOffer,
    addr: SocketAddr,
    result: mpsc::Receiver<Result<(), String>>,
}

/// A race a player's machine runs for a room.
pub(super) struct HostedRace {
    pub(super) host: ClientId,
    /// `(client, name, seat token)` by seat.
    pub(super) seats: Vec<(ClientId, String, String)>,
    /// `crate::fair::params_hash` of the job: what the hand-off carries.
    pub(super) hash: [u8; 32],
    up: bool,
    pub(super) launched: Instant,
    /// When the host last said it is up: it says so once a second while the race runs.
    heard: Instant,
}

/// An address as the host part of `host:port`.
fn host_part(addr: IpAddr) -> String {
    match addr {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => format!("[{v6}]"),
    }
}

/// A seed from the system's random source.
fn random_seed() -> u64 {
    use ring::rand::SecureRandom;
    let mut bytes = [0u8; 8];
    ring::rand::SystemRandom::new()
        .fill(&mut bytes)
        .expect("the system's random source works");
    u64::from_le_bytes(bytes)
}

pub(super) fn short(key: &[u8; 32]) -> String {
    crate::wt::hex(&key[..8])
}

impl Server {
    fn refuse_host(&mut self, id: ClientId, code: Refusal, text: String) {
        log::info!(
            "host-log: refused client {id} ({:?}): {code:?}: {text}",
            self.rooms.name(id).unwrap_or_default()
        );
        if let Some(client) = self.clients.get_mut(&id) {
            client.link.send_lobby(&Message::Rejected { code, text });
        }
    }

    /// A player asks to host a room's race: the signature, the ban list, the limits, then the
    /// probe. The room is made when the probe passes ([`Self::poll_probes`]).
    pub(super) fn host_room(&mut self, id: ClientId, settings: RoomSettings, offer: HostOffer) {
        let Some(client) = self.clients.get_mut(&id) else {
            return;
        };
        let (ip, session) = (canonical(client.ip), client.token.clone());
        let asked = client.host_asked.replace(Instant::now());
        if asked.is_some_and(|at| at.elapsed() < HOST_ASK_INTERVAL) {
            let text = format!(
                "one hosting request every {} s",
                HOST_ASK_INTERVAL.as_secs()
            );
            return self.refuse_host(id, Refusal::BadHost, text);
        }
        if self.probes.iter().any(|probe| probe.client == id) {
            return self.refuse_host(id, Refusal::BadHost, "already checking your port".into());
        }
        if offer.port == 0 || !hostkey::verify(&offer, &self.identity.hash(), &session) {
            let text = "the hosting offer is not signed for this lobby and session".into();
            return self.refuse_host(id, Refusal::BadHost, text);
        }
        if let Some(reason) = self.bans.bans().banned(&offer.key, ip) {
            let text = format!("you may not host on this lobby: {reason}");
            return self.refuse_host(id, Refusal::Banned, text);
        }
        if let Err((code, text)) = self.rooms.vet_hosted(id, &settings) {
            return self.refuse_host(id, code, text);
        }
        if self.rooms.hosted_rooms() + self.probes.len() >= MAX_HOSTED_ROOMS {
            let text = format!("this lobby lists {MAX_HOSTED_ROOMS} player-hosted games already");
            return self.refuse_host(id, Refusal::Full, text);
        }
        let from_here = self.hosts.values().filter(|h| h.addr.ip() == ip).count()
            + self.probes.iter().filter(|p| p.addr.ip() == ip).count();
        if from_here >= MAX_HOSTED_PER_ADDRESS {
            let text =
                format!("{MAX_HOSTED_PER_ADDRESS} games are hosted from your address already");
            return self.refuse_host(id, Refusal::Full, text);
        }
        if self.probes.len() >= MAX_PROBES {
            let text = "the lobby is checking other hosts: try again in a moment".into();
            return self.refuse_host(id, Refusal::Full, text);
        }
        // Only ever the address the offer came from: an offer cannot aim the lobby elsewhere.
        let addr = SocketAddr::new(ip, offer.port);
        let (tx, result) = mpsc::channel();
        let hash = offer.cert_hash;
        let spawned = std::thread::Builder::new()
            .name("oag-host-probe".into())
            .spawn(move || {
                // One byte on the stream completes the listener's side of the handshake; the
                // host reads nothing from a probe.
                let reached = crate::wt::connect(&addr.to_string(), hash)
                    .map(|mut link| crate::link::Link::send_reliable(&mut link, vec![0]))
                    .map_err(|e| format!("{e:#}"));
                let _ = tx.send(reached);
            });
        if spawned.is_err() {
            return self.refuse_host(
                id,
                Refusal::Full,
                "the lobby could not check your port".into(),
            );
        }
        self.probes.push(Probe {
            client: id,
            settings,
            offer,
            addr,
            result,
        });
    }

    /// Probes that have answered: the room is made, or the host is told its port is closed.
    pub(super) fn poll_probes(&mut self) {
        for probe in std::mem::take(&mut self.probes) {
            let reached = match probe.result.try_recv() {
                Ok(reached) => reached,
                Err(mpsc::TryRecvError::Empty) => {
                    self.probes.push(probe);
                    continue;
                }
                Err(mpsc::TryRecvError::Disconnected) => Err("the check stopped".into()),
            };
            let id = probe.client;
            if !self.clients.contains_key(&id) {
                continue;
            }
            match reached {
                Ok(()) => {
                    self.hosts.insert(
                        id,
                        HostRec {
                            key: probe.offer.key,
                            addr: probe.addr,
                            cert_hash: probe.offer.cert_hash,
                        },
                    );
                    self.rooms.create_hosted(id, probe.settings.clone());
                    log::info!(
                        "host-log: key {} addr {} name {:?} hosts room {:?}: {:?}",
                        short(&probe.offer.key),
                        probe.addr,
                        self.rooms.name(id).unwrap_or_default(),
                        self.rooms.hosted_by(id),
                        probe.settings
                    );
                }
                Err(why) => {
                    let text = format!(
                        "the lobby could not reach you on UDP port {}: open or forward that port \
                         to this machine ({why})",
                        probe.addr.port()
                    );
                    self.refuse_host(id, Refusal::Unreachable, text);
                }
            }
        }
    }

    /// A player-hosted room's countdown ran out: its owner is sent the race to run.
    pub(super) fn launch_hosted(&mut self, launch: Launch) {
        let room = launch.room;
        let host = launch.seats.first().map(|(id, _)| *id);
        let (Some(host), true) = (host, host.is_some_and(|id| self.hosts.contains_key(&id))) else {
            self.rooms
                .launch_failed(room, "the room's host is not registered");
            return;
        };
        let seats: Vec<(ClientId, String, String)> = launch
            .seats
            .iter()
            .map(|(id, name)| (*id, name.clone(), token()))
            .collect();
        // The lobby draws the seed, so the host cannot pick one that suits it (ADR-0092, "The
        // anti-cheat path", item 1).
        let mut params = launch.params;
        params.seed = random_seed();
        let job = RaceJob {
            params,
            seats: seats
                .iter()
                .map(|(_, n, t)| (n.clone(), t.clone()))
                .collect(),
            keys: seats
                .iter()
                .map(|(id, _, _)| self.clients.get(id).map_or([0; 32], |c| c.key))
                .collect(),
        };
        let hash = crate::fair::params_hash(&job.params);
        let race_params = job.params.clone();
        if let Some(rec) = self.hosts.get(&host) {
            let seated = seats
                .iter()
                .zip(&job.keys)
                .map(|((id, name, _), key)| (*id, name.clone(), *key))
                .collect();
            self.referee.races.insert(
                room,
                super::referee::RaceCase::new(host, rec.key, rec.addr, race_params, seated),
            );
        }
        log::info!(
            "host-log: room {room} races on its host's machine: {:?}",
            job.params
        );
        if let Some(client) = self.clients.get_mut(&host) {
            client.link.send_lobby(&Message::Launch { room, job });
        }
        self.hosted.insert(
            room,
            HostedRace {
                host,
                seats,
                hash,
                up: false,
                launched: Instant::now(),
                heard: Instant::now(),
            },
        );
    }

    /// The hand-off of `room`'s hosted race for `seat`, once the host has said it is up.
    fn hosted_handoff(&self, room: u32, seat: ClientId) -> Option<Message> {
        let race = self.hosted.get(&room).filter(|race| race.up)?;
        let host = self.hosts.get(&race.host)?;
        let (_, name, token) = race.seats.iter().find(|(id, _, _)| *id == seat)?;
        Some(Message::Handoff {
            host: host_part(host.addr.ip()),
            port: host.addr.port(),
            cert_hash: host.cert_hash,
            token: token.clone(),
            name: name.clone(),
            params: race.hash,
            host_key: host.key,
        })
    }

    /// The host has loaded the race: every seat is handed its address. Said again, it is the
    /// host's sign of life.
    pub(super) fn host_up(&mut self, id: ClientId, room: u32) {
        let Some(race) = self.hosted.get_mut(&room).filter(|race| race.host == id) else {
            return;
        };
        race.heard = Instant::now();
        if race.up {
            return;
        }
        race.up = true;
        let seats: Vec<ClientId> = race.seats.iter().map(|(seat, _, _)| *seat).collect();
        for seat in seats {
            if let (Some(message), Some(client)) =
                (self.hosted_handoff(room, seat), self.clients.get_mut(&seat))
            {
                client.link.send_lobby(&message);
            }
        }
    }

    /// The host says its race is over.
    pub(super) fn host_over(&mut self, id: ClientId, room: u32, reason: &str, board: Board) {
        if self.hosted.get(&room).is_none_or(|race| race.host != id) {
            return;
        }
        self.hosted.remove(&room);
        log::info!("host-log: room {room}'s hosted race is over: {reason}");
        self.rooms.race_over(room, reason, board);
    }

    /// The hand-off again for a person who took their session over after it went out.
    pub(super) fn resend_hosted_handoff(&mut self, id: ClientId) {
        let room = self
            .hosted
            .iter()
            .find(|(_, race)| race.seats.iter().any(|(seat, _, _)| *seat == id))
            .map(|(room, _)| *room);
        if let Some(message) = room.and_then(|room| self.hosted_handoff(room, id))
            && let Some(client) = self.clients.get_mut(&id)
        {
            client.link.send_lobby(&message);
        }
    }

    /// Whether `id` sits in a hosted race as a guest: a seat the lobby holds through a
    /// dropped link, as it does a server race's. The host's own is not held: its race is gone
    /// with it.
    pub(super) fn hosted_guest(&self, id: ClientId) -> bool {
        self.hosted
            .values()
            .any(|race| race.host != id && race.seats.iter().any(|(seat, _, _)| *seat == id))
    }

    /// Every pass: probes, hosted races whose room or time is gone, and the ban list.
    pub(super) fn watch_hosted(&mut self) {
        self.poll_probes();
        let rooms: Vec<u32> = self.hosted.keys().copied().collect();
        for room in rooms {
            let Some(race) = self.hosted.get(&room) else {
                continue;
            };
            let why = if !self.rooms.exists(room) {
                // Its host left, and the rooms closed it and told its people.
                self.hosted.remove(&room);
                log::info!("host-log: room {room}'s hosted race ended with its host");
                continue;
            } else if race.up && race.heard.elapsed() > QUIET_TIMEOUT {
                // A machine that was killed closes no link; its silence is the sign.
                self.hosted.remove(&room);
                log::info!("host-log: room {room}'s host went quiet: closing the room");
                self.rooms.close(room, HOST_LEFT_RACING);
                continue;
            } else if !race.up && race.launched.elapsed() > HOST_LOAD_TIMEOUT {
                "the host did not bring the race up"
            } else if race.launched.elapsed() > HOSTED_RACE_LIMIT {
                "the player-hosted race ran past its time limit"
            } else {
                continue;
            };
            self.hosted.remove(&room);
            log::info!("host-log: room {room}'s hosted race is over: {why}");
            self.rooms.race_over(room, why, Board::default());
        }
        let hosting: Vec<ClientId> = self.hosts.keys().copied().collect();
        for id in hosting {
            if self.rooms.hosted_by(id).is_none() || !self.clients.contains_key(&id) {
                self.hosts.remove(&id);
            }
        }
        if self.bans.refresh() {
            let banned: Vec<(u32, String, [u8; 32], SocketAddr)> = self
                .hosts
                .iter()
                .filter_map(|(id, host)| {
                    let reason = self.bans.bans().banned(&host.key, host.addr.ip())?;
                    Some((
                        self.rooms.hosted_by(*id)?,
                        reason.to_string(),
                        host.key,
                        host.addr,
                    ))
                })
                .collect();
            for (room, reason, key, addr) in banned {
                log::info!(
                    "host-log: room {room} delisted: key {} addr {addr} is banned: {reason}",
                    short(&key)
                );
                self.hosted.remove(&room);
                self.rooms.close(
                    room,
                    &format!("the lobby closed this game: its host may not host here ({reason})"),
                );
            }
        }
    }

    /// How many player-hosted races are under way.
    #[must_use]
    pub fn hosted_races(&self) -> usize {
        self.hosted.len()
    }
}
