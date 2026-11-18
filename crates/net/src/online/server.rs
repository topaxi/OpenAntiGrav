//! The online lobby as a service: a WebTransport listener, the people who connected, the
//! rooms ([`super::rooms`]) and the one race they share ([`super::race_proc`]).
//!
//! [`Server::poll`] is one non-blocking pass; [`Server::serve`] repeats it. Identity is a
//! chosen name plus a session token this server issues; there are no accounts. A person's
//! lobby link stays open while their race runs (the hand-off tells them where the race is
//! and they open a second link there), so the lobby can push the queue and the end of the
//! race, and everyone is back in their room when it ends. Chosen, not measured.

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};

use anyhow::Result;
use web_time::{Duration, Instant};

use super::moderation::BanFile;
use super::race_proc::{Handle, Runner, Status};
use super::rooms::{ClientId, Config as RoomsConfig, Rooms};
use crate::limits::LogLimit;
use crate::link::Link;
use crate::wire::online::{Edition, Message, RaceJob, Refusal};
use crate::wire::{Control, PROTOCOL};
use crate::wt::{Identity, LinkLimits, Listener, WtLink};

/// How long a new connection has to say hello. Chosen.
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
use crate::host::SEAT_GRACE;

/// Connections the lobby keeps at once, joining or joined, by default. Chosen, not measured: a
/// lobby link costs about 2.6 MiB at its worst (`docs/networking/server-roadmap.md`, "V1:
/// survive the network", the memory arithmetic), so 64 stay inside a 1 GB host with the race beside them.
pub const MAX_CONNECTIONS: usize = 64;

/// Connections one address may hold at once, by default. Chosen: a household or a LAN party
/// behind one NAT, with room for a full race of twelve to reconnect.
pub const MAX_PER_ADDRESS: usize = 16;

mod hosting;
mod referee;
pub use hosting::{HOST_ASK_INTERVAL, MAX_HOSTED_PER_ADDRESS, MAX_HOSTED_ROOMS, MAX_PROBES};
pub use referee::{CHALLENGE_TIMEOUT, JUDGE_TIME, MAX_ASSEMBLING, MAX_HELD_BYTES, Ruling};

/// What the service is built from.
#[derive(Debug, Clone)]
pub struct Config {
    /// Where the lobby listens.
    pub listen: SocketAddr,
    /// More names the certificate is valid for.
    pub cert_names: Vec<String>,
    /// The rooms' rules and the titles served.
    pub rooms: RoomsConfig,
    /// Connections held at once; the next is refused by name ([`MAX_CONNECTIONS`]).
    pub max_connections: usize,
    /// Connections one address may hold at once ([`MAX_PER_ADDRESS`]).
    pub max_per_address: usize,
    /// The ban list for players who host ([`super::moderation`]), if the operator keeps one.
    pub ban_file: Option<std::path::PathBuf>,
}

struct Pending {
    link: WtLink,
    since: Instant,
}

struct Connected {
    link: Box<dyn Link>,
    ip: IpAddr,
    /// One warning a second about this person's messages.
    log: LogLimit,
    token: String,
    /// When the link was first seen closed, for a person held by `SEAT_GRACE`.
    dropped: Option<Instant>,
    /// When this connection last asked to host.
    host_asked: Option<Instant>,
    /// The player's public key its hello named, all zero for none.
    key: [u8; 32],
}

struct Running {
    room: u32,
    handle: Box<dyn Handle>,
    /// `(client, name, seat token)` by seat.
    seats: Vec<(ClientId, String, String)>,
    /// Where the race listens once it is up: what a hand-off carries.
    up: Option<(u16, [u8; 32])>,
    handed_off: bool,
    /// `crate::fair::params_hash` of the job: what a hand-off carries.
    params: [u8; 32],
}

/// The lobby service.
pub struct Server {
    listener: Listener,
    identity: Identity,
    rooms: Rooms,
    joining: Vec<Pending>,
    clients: BTreeMap<ClientId, Connected>,
    runner: Box<dyn Runner>,
    race: Option<Running>,
    next_client: ClientId,
    started: Instant,
    max_connections: usize,
    max_per_address: usize,
    /// One warning a second about refused connections.
    refusals: LogLimit,
    /// The players whose machines run a listed room's race, by client.
    hosts: BTreeMap<ClientId, hosting::HostRec>,
    /// Offered ports being checked.
    probes: Vec<hosting::Probe>,
    /// The player-hosted races under way, by room.
    hosted: BTreeMap<u32, hosting::HostedRace>,
    bans: BanFile,
    /// Reports against player hosts, and their judging.
    referee: referee::Referee,
}

/// A fresh 128-bit token, hex.
fn token() -> String {
    use ring::rand::SecureRandom;
    let mut bytes = [0u8; 16];
    ring::rand::SystemRandom::new()
        .fill(&mut bytes)
        .expect("the system's random source works");
    crate::wt::hex(&bytes)
}

impl Server {
    /// Binds the lobby and mints its certificate.
    ///
    /// # Errors
    ///
    /// The address could not be bound.
    pub fn bind(config: Config, runner: Box<dyn Runner>) -> Result<Self> {
        let identity = Identity::self_signed(&config.cert_names)?;
        let listener = Listener::bind(config.listen, &identity, LinkLimits::LOBBY)?;
        Ok(Self {
            listener,
            identity,
            rooms: Rooms::new(config.rooms),
            joining: Vec::new(),
            clients: BTreeMap::new(),
            runner,
            race: None,
            next_client: 1,
            started: Instant::now(),
            max_connections: config.max_connections,
            max_per_address: config.max_per_address,
            refusals: LogLimit::default(),
            hosts: BTreeMap::new(),
            probes: Vec::new(),
            hosted: BTreeMap::new(),
            bans: BanFile::new(config.ban_file),
            referee: referee::Referee::default(),
        })
    }

    /// The address the lobby listens on.
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.listener.local_addr()
    }

    /// SHA-256 of the lobby's certificate: what a client pins.
    #[must_use]
    pub fn cert_hash(&self) -> [u8; 32] {
        self.identity.hash()
    }

    /// The certificate hash as hex.
    #[must_use]
    pub fn cert_hash_hex(&self) -> String {
        self.identity.hash_hex()
    }

    /// How many people are connected.
    #[must_use]
    pub fn connected(&self) -> usize {
        self.clients.len()
    }

    /// The room whose race holds the slot, if one does.
    #[must_use]
    pub fn racing_room(&self) -> Option<u32> {
        self.race.as_ref().map(|r| r.room)
    }

    /// One non-blocking pass: new connections, messages, the clock, the race, and the sends.
    pub fn poll(&mut self) {
        self.accept();
        self.read();
        let now = u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.rooms.tick(now);
        self.launch();
        self.watch_race();
        self.watch_hosted();
        self.watch_referee();
        self.flush();
    }

    /// Polls until `stop` says so.
    pub fn serve(&mut self, stop: &dyn Fn() -> bool) {
        while !stop() {
            self.poll();
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn accept(&mut self) {
        while let Some(mut link) = self.listener.accept() {
            log::debug!("lobby: connection from {}", link.peer());
            let ip = link.peer().ip();
            let held = self.joining.len() + self.clients.len();
            let held_by_ip = self
                .joining
                .iter()
                .filter(|p| p.link.peer().ip() == ip)
                .count()
                + self.clients.values().filter(|c| c.ip == ip).count();
            if held >= self.max_connections || held_by_ip >= self.max_per_address {
                let what = if held >= self.max_connections {
                    format!(
                        "this lobby holds {} connections already",
                        self.max_connections
                    )
                } else {
                    format!(
                        "{} connections from your address already",
                        self.max_per_address
                    )
                };
                self.refusals
                    .warn(|| format!("lobby: refused {ip}: {what}"));
                link.send_lobby(&Message::Rejected {
                    code: Refusal::Full,
                    text: what,
                });
                continue;
            }
            self.joining.push(Pending {
                link,
                since: Instant::now(),
            });
        }
        for mut pending in std::mem::take(&mut self.joining) {
            let closed = pending.link.closed();
            let Some(bytes) = pending.link.recv_reliable() else {
                if closed.is_none() && pending.since.elapsed() < HELLO_TIMEOUT {
                    self.joining.push(pending);
                }
                continue;
            };
            match Message::decode(&bytes) {
                Ok(Message::Hello {
                    protocol,
                    name,
                    titles,
                    token,
                    key,
                }) => self.hello(pending.link, protocol, &name, titles, &token, key),
                Ok(other) => {
                    let mut link = pending.link;
                    link.send_lobby(&Message::Rejected {
                        code: Refusal::BadName,
                        text: format!("expected a lobby hello, got {other:?}"),
                    });
                }
                Err(_) if matches!(Control::decode(&bytes), Ok(Control::Hello { .. })) => {
                    let mut link = pending.link;
                    link.send_control(&Control::Reject {
                        reason: "this is the online lobby, not a race: connect with --online \
                                 (the lobby hands you the race's address)"
                            .into(),
                    });
                }
                Err(e) => {
                    let mut link = pending.link;
                    link.send_lobby(&Message::Rejected {
                        code: Refusal::Version,
                        text: format!("{e}"),
                    });
                }
            }
        }
    }

    fn hello(
        &mut self,
        mut link: WtLink,
        protocol: u16,
        name: &str,
        titles: Vec<Edition>,
        token_in: &str,
        key: [u8; 32],
    ) {
        if protocol != PROTOCOL {
            link.send_lobby(&Message::Rejected {
                code: Refusal::Version,
                text: format!("this lobby speaks protocol {PROTOCOL}, you speak {protocol}"),
            });
            return;
        }
        // A session token takes over a session of the same name: a dropped link, a reconnect.
        let taken = (!token_in.is_empty())
            .then(|| {
                self.clients.iter().find_map(|(id, c)| {
                    (c.token == token_in && self.rooms.name(*id) == Some(name.trim()))
                        .then_some(*id)
                })
            })
            .flatten();
        if let Some(id) = taken {
            log::debug!("lobby: {name:?} took over their session");
            let ip = link.peer().ip();
            self.clients.insert(
                id,
                Connected {
                    link: Box::new(link),
                    ip,
                    log: LogLimit::default(),
                    token: token_in.to_string(),
                    dropped: None,
                    host_asked: None,
                    key,
                },
            );
            self.welcome(id);
            self.resend_handoff(id);
            self.resend_hosted_handoff(id);
            return;
        }
        let id = self.next_client;
        match self.rooms.connect(id, name, titles) {
            Err((code, text)) => link.send_lobby(&Message::Rejected { code, text }),
            Ok(()) => {
                self.next_client += 1;
                let ip = link.peer().ip();
                self.clients.insert(
                    id,
                    Connected {
                        link: Box::new(link),
                        ip,
                        log: LogLimit::default(),
                        token: token(),
                        dropped: None,
                        host_asked: None,
                        key,
                    },
                );
                log::debug!("lobby: {:?} joined as client {id}", self.rooms.name(id));
                self.welcome(id);
            }
        }
    }

    /// A person who took their session over after the hand-off went out gets it again: the
    /// first one went to the link that dropped.
    fn resend_handoff(&mut self, id: ClientId) {
        let Some(running) = self.race.as_ref().filter(|r| r.handed_off) else {
            return;
        };
        let (Some((port, cert_hash)), Some((_, name, token))) = (
            running.up,
            running.seats.iter().find(|(seat, _, _)| *seat == id),
        ) else {
            return;
        };
        let message = Message::Handoff {
            host: String::new(),
            port,
            cert_hash,
            token: token.clone(),
            name: name.clone(),
            params: running.params,
            host_key: [0; 32],
        };
        if let Some(client) = self.clients.get_mut(&id) {
            client.link.send_lobby(&message);
        }
    }

    fn welcome(&mut self, id: ClientId) {
        let (Some(client), Some(name)) = (self.clients.get_mut(&id), self.rooms.name(id)) else {
            return;
        };
        let message = Message::Welcome {
            token: client.token.clone(),
            name: name.to_string(),
            titles: self.rooms.titles(),
        };
        client.link.send_lobby(&message);
    }

    fn read(&mut self) {
        let ids: Vec<ClientId> = self.clients.keys().copied().collect();
        for id in ids {
            let mut gone = None;
            let mut incoming = Vec::new();
            if let Some(client) = self.clients.get_mut(&id) {
                gone = client.link.closed();
                while let Some(message) = client.link.recv_lobby() {
                    match message {
                        Ok(message) => incoming.push(message),
                        Err(e) => client.log.warn(|| format!("lobby: client {id}: {e}")),
                    }
                }
            }
            for message in incoming {
                match message {
                    Message::HostRoom { settings, offer } => self.host_room(id, settings, offer),
                    Message::HostUp { room } => self.host_up(id, room),
                    Message::HostOver {
                        room,
                        reason,
                        board,
                    } => self.host_over(id, room, &reason, board),
                    Message::Report {
                        room,
                        part,
                        parts,
                        bytes,
                    } => self.report_part(id, room, part, parts, bytes),
                    Message::InputShown {
                        room,
                        seat,
                        tick,
                        datagram,
                    } => self.input_shown(id, room, seat, tick, &datagram),
                    other => self.rooms.handle(id, other),
                }
            }
            if let Some(why) = gone {
                let seated = self
                    .race
                    .as_ref()
                    .is_some_and(|r| r.seats.iter().any(|(seat, _, _)| *seat == id))
                    || self.hosted_guest(id);
                if let Some(client) = self.clients.get_mut(&id).filter(|_| seated) {
                    let since = *client.dropped.get_or_insert_with(Instant::now);
                    if since.elapsed() < SEAT_GRACE {
                        continue;
                    }
                }
                log::debug!("lobby: client {id} left: {why}");
                self.clients.remove(&id);
                self.rooms.disconnect(id);
            }
        }
    }

    fn launch(&mut self) {
        let Some(launch) = self.rooms.take_launch() else {
            return;
        };
        if launch.hosted {
            self.launch_hosted(launch);
            return;
        }
        let seats: Vec<(ClientId, String, String)> = launch
            .seats
            .iter()
            .map(|(id, name)| (*id, name.clone(), token()))
            .collect();
        let job = RaceJob {
            params: launch.params,
            seats: seats
                .iter()
                .map(|(_, n, t)| (n.clone(), t.clone()))
                .collect(),
            keys: Vec::new(),
        };
        let params = crate::fair::params_hash(&job.params);
        match self.runner.launch(&job) {
            Ok(handle) => {
                log::info!("lobby: room {} races: {:?}", launch.room, job.params);
                self.race = Some(Running {
                    room: launch.room,
                    handle,
                    seats,
                    up: None,
                    handed_off: false,
                    params,
                });
            }
            Err(why) => {
                log::warn!("lobby: room {} could not race: {why}", launch.room);
                self.rooms.launch_failed(launch.room, &why);
            }
        }
    }

    fn watch_race(&mut self) {
        let Some(running) = self.race.as_mut() else {
            return;
        };
        match running.handle.poll() {
            Status::Starting => {}
            Status::Up { port, cert_hash } => {
                running.up = Some((port, cert_hash));
                if !running.handed_off {
                    running.handed_off = true;
                    for (id, name, token) in &running.seats {
                        if let Some(client) = self.clients.get_mut(id) {
                            client.link.send_lobby(&Message::Handoff {
                                host: String::new(),
                                port,
                                cert_hash,
                                token: token.clone(),
                                name: name.clone(),
                                params: running.params,
                                host_key: [0; 32],
                            });
                        }
                    }
                }
            }
            Status::Over(ended) => {
                let room = running.room;
                log::info!("lobby: room {room}'s race is over: {}", ended.reason);
                self.race = None;
                self.rooms.race_over(room, &ended.reason, ended.board);
            }
        }
    }

    fn flush(&mut self) {
        for (to, message) in self.rooms.take_outbox() {
            if let Some(client) = self.clients.get_mut(&to) {
                client.link.send_lobby(&message);
            }
        }
    }
}

impl std::fmt::Debug for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Server")
            .field("addr", &self.local_addr())
            .field("clients", &self.clients.len())
            .finish_non_exhaustive()
    }
}
