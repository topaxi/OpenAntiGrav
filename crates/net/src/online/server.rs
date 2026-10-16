//! The online lobby as a service: a WebTransport listener, the people who connected, the
//! rooms ([`super::rooms`]) and the one race they share ([`super::race_proc`]).
//!
//! [`Server::poll`] is one non-blocking pass; [`Server::serve`] repeats it. Identity is a
//! chosen name plus a session token this server issues; there are no accounts. A person's
//! lobby link stays open while their race runs (the hand-off tells them where the race is
//! and they open a second link there), so the lobby can push the queue and the end of the
//! race, and everyone is back in their room when it ends. Chosen, not measured.

use std::collections::BTreeMap;
use std::net::SocketAddr;

use anyhow::Result;
use web_time::{Duration, Instant};

use super::race_proc::{Handle, Runner, Status};
use super::rooms::{ClientId, Config as RoomsConfig, Rooms};
use crate::link::Link;
use crate::wire::online::{Message, RaceJob, Refusal};
use crate::wire::{Control, PROTOCOL};
use crate::wt::{Identity, Listener, WtLink};

/// How long a new connection has to say hello. Chosen.
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);

/// What the service is built from.
#[derive(Debug, Clone)]
pub struct Config {
    /// Where the lobby listens.
    pub listen: SocketAddr,
    /// More names the certificate is valid for.
    pub cert_names: Vec<String>,
    /// The rooms' rules and the titles served.
    pub rooms: RoomsConfig,
}

struct Pending {
    link: WtLink,
    since: Instant,
}

struct Connected {
    link: Box<dyn Link>,
    token: String,
}

struct Running {
    room: u32,
    handle: Box<dyn Handle>,
    /// `(client, name, seat token)` by seat.
    seats: Vec<(ClientId, String, String)>,
    handed_off: bool,
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
        let listener = Listener::bind(config.listen, &identity)?;
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
        while let Some(link) = self.listener.accept() {
            log::info!("lobby: connection from {}", link.peer());
            self.joining.push(Pending {
                link,
                since: Instant::now(),
            });
        }
        for mut pending in std::mem::take(&mut self.joining) {
            let Some(bytes) = pending.link.recv_reliable() else {
                if pending.link.closed().is_none() && pending.since.elapsed() < HELLO_TIMEOUT {
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
                }) => self.hello(pending.link, protocol, &name, titles, &token),
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
        titles: Vec<String>,
        token_in: &str,
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
            log::info!("lobby: {name:?} took over their session");
            self.clients.insert(
                id,
                Connected {
                    link: Box::new(link),
                    token: token_in.to_string(),
                },
            );
            self.welcome(id);
            return;
        }
        let id = self.next_client;
        match self.rooms.connect(id, name, titles) {
            Err(rejected) => link.send_lobby(&rejected),
            Ok(()) => {
                self.next_client += 1;
                self.clients.insert(
                    id,
                    Connected {
                        link: Box::new(link),
                        token: token(),
                    },
                );
                log::info!("lobby: {:?} joined as client {id}", self.rooms.name(id));
                self.welcome(id);
            }
        }
    }

    fn welcome(&mut self, id: ClientId) {
        let (Some(client), Some(name)) = (self.clients.get_mut(&id), self.rooms.name(id)) else {
            return;
        };
        let message = Message::Welcome {
            token: client.token.clone(),
            name: name.to_string(),
            titles: self.rooms.titles().to_vec(),
        };
        client.link.send_lobby(&message);
    }

    fn read(&mut self) {
        let ids: Vec<ClientId> = self.clients.keys().copied().collect();
        for id in ids {
            let mut gone = None;
            let mut incoming = Vec::new();
            if let Some(client) = self.clients.get_mut(&id) {
                while let Some(message) = client.link.recv_lobby() {
                    incoming.push(message);
                }
                gone = client.link.closed();
            }
            for message in incoming {
                match message {
                    Ok(message) => self.rooms.handle(id, message),
                    Err(e) => log::warn!("lobby: client {id}: {e}"),
                }
            }
            if let Some(why) = gone {
                log::info!("lobby: client {id} left: {why}");
                self.clients.remove(&id);
                self.rooms.disconnect(id);
            }
        }
    }

    fn launch(&mut self) {
        let Some(launch) = self.rooms.take_launch() else {
            return;
        };
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
        };
        match self.runner.launch(&job) {
            Ok(handle) => {
                log::info!("lobby: room {} races: {:?}", launch.room, job.params);
                self.race = Some(Running {
                    room: launch.room,
                    handle,
                    seats,
                    handed_off: false,
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
                if !running.handed_off {
                    running.handed_off = true;
                    for (id, name, token) in &running.seats {
                        if let Some(client) = self.clients.get_mut(id) {
                            client.link.send_lobby(&Message::Handoff {
                                port,
                                cert_hash,
                                token: token.clone(),
                                name: name.clone(),
                            });
                        }
                    }
                }
            }
            Status::Over(reason) => {
                let room = running.room;
                log::info!("lobby: room {room}'s race is over: {reason}");
                self.race = None;
                self.rooms.race_over(room, &reason);
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
