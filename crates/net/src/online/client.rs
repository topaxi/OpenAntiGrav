//! A lobby client over any [`Link`]: says hello, keeps the list of rooms, the room it is
//! in and the hand-off to a race, and sends what a person does on the lobby screens.
//!
//! A frame at a time like [`crate::client::Client`]: [`LobbyClient::poll`] reads the link and
//! updates the fields, and the screens (or a test) read the fields.

use crate::link::Link;
use crate::wire::board::Board;
use crate::wire::online::{
    Edition, HostOffer, Message, RaceJob, Refusal, RoomInfo, RoomSettings, Snapshot,
};

/// Where a race is, as the lobby told this client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handoff {
    /// The address the race is on: empty for the host the lobby was dialled on, else a player
    /// host's own.
    pub host: String,
    /// The race's UDP port.
    pub port: u16,
    /// SHA-256 of the race server's certificate.
    pub cert_hash: [u8; 32],
    /// The seat token for the race hello.
    pub token: String,
    /// The name the seat is under.
    pub name: String,
    /// The hash of the race the lobby issued (`crate::fair::params_hash`); all zero for none.
    pub params: [u8; 32],
    /// A player host's public key, all zero for a server race.
    pub host_key: [u8; 32],
}

/// A connection to a lobby.
pub struct LobbyClient {
    link: Box<dyn Link>,
    /// The name the lobby accepted, once welcomed.
    pub name: Option<String>,
    /// The session token, once welcomed.
    pub token: Option<String>,
    /// The titles the server races, once welcomed.
    pub served: Vec<String>,
    /// The newest room list.
    pub rooms: Vec<RoomInfo>,
    /// The room this client is in, as of the newest push.
    pub room: Option<Snapshot>,
    /// A race to go to; taken with [`Self::take_handoff`].
    handoff: Option<Handoff>,
    /// Why this client is no longer in a room, from the newest `Left`.
    pub left: Option<String>,
    /// The newest refusal.
    pub refused: Option<(Refusal, String)>,
    /// A race the lobby asked this machine to host; taken with [`Self::take_launch`].
    launch: Option<(u32, RaceJob)>,
    /// Inputs the lobby asked this machine, as a room's host, to show: `(room, seat, tick)`.
    challenges: Vec<(u32, u8, u32)>,
    /// Report parts still to send, one a poll, so a long proof never fills the link's queue.
    outgoing: std::collections::VecDeque<Message>,
}

/// Challenges a host keeps unanswered at most; past it the oldest is dropped.
const MAX_CHALLENGES: usize = 64;

impl LobbyClient {
    /// Says hello on `link` as `name`, able to race `titles` (each with its platform and content); `token` is an earlier
    /// session's, empty for a new one.
    pub fn connect(link: Box<dyn Link>, name: &str, titles: &[Edition], token: &str) -> Self {
        Self::connect_keyed(link, name, titles, token, [0; 32])
    }

    /// [`Self::connect`] naming the player's public `key`, which a player-hosted race checks
    /// this person's signed inputs against.
    pub fn connect_keyed(
        mut link: Box<dyn Link>,
        name: &str,
        titles: &[Edition],
        token: &str,
        key: [u8; 32],
    ) -> Self {
        link.send_lobby(&Message::hello(name, titles, token, key));
        Self {
            link,
            name: None,
            token: None,
            served: Vec::new(),
            rooms: Vec::new(),
            room: None,
            handoff: None,
            left: None,
            refused: None,
            launch: None,
            challenges: Vec::new(),
            outgoing: std::collections::VecDeque::new(),
        }
    }

    /// Reads the link. `Err` when the connection ended.
    ///
    /// # Errors
    ///
    /// The link closed, and why. A refusal read in the same poll is held back one call, in `refused`.
    pub fn poll(&mut self) -> Result<(), String> {
        // Read before the queue: a message that lands after this is read on the next poll,
        // never lost behind a close seen after it.
        let closed = self.link.closed();
        // The next part only once the link has written most of the last: a long proof on a slow
        // uplink must not fill the link's send queue, which closes the link.
        if self
            .link
            .unsent()
            .is_none_or(|bytes| bytes < crate::wire::online::REPORT_PART_BYTES)
            && let Some(part) = self.outgoing.pop_front()
        {
            self.link.send_lobby(&part);
        }
        while let Some(message) = self.link.recv_lobby() {
            match message {
                Ok(Message::Welcome {
                    token,
                    name,
                    titles,
                }) => {
                    self.token = Some(token);
                    self.name = Some(name);
                    self.served = titles;
                }
                Ok(Message::Rooms(rooms)) => self.rooms = rooms,
                Ok(Message::Room(room)) => {
                    self.left = None;
                    self.room = Some(room);
                }
                Ok(Message::Left { reason }) => {
                    self.room = None;
                    self.left = Some(reason);
                }
                Ok(Message::Launch { room, job }) => self.launch = Some((room, job)),
                Ok(Message::Handoff {
                    host,
                    port,
                    cert_hash,
                    token,
                    name,
                    params,
                    host_key,
                }) => {
                    self.handoff = Some(Handoff {
                        host,
                        port,
                        cert_hash,
                        token,
                        name,
                        params,
                        host_key,
                    });
                }
                Ok(Message::ShowInput { room, seat, tick }) => {
                    if self.challenges.len() >= MAX_CHALLENGES {
                        self.challenges.remove(0);
                    }
                    self.challenges.push((room, seat, tick));
                }
                Ok(Message::Rejected { code, text }) => self.refused = Some((code, text)),
                Ok(other) => log::warn!("lobby: unexpected {other:?}"),
                Err(e) => log::warn!("lobby: {e}"),
            }
        }
        match closed {
            // A refused hello is followed by a hang-up that can be read in this same poll: hand
            // the refusal over first, and report the close on the next.
            Some(_) if self.refused.is_some() => Ok(()),
            Some(why) => Err(why),
            None => Ok(()),
        }
    }

    /// Whether the lobby has welcomed this client.
    #[must_use]
    pub fn welcomed(&self) -> bool {
        self.token.is_some()
    }

    /// The race to go to, once the lobby has one up; once only.
    pub fn take_handoff(&mut self) -> Option<Handoff> {
        self.handoff.take()
    }

    /// Whether a hand-off is waiting to be taken.
    #[must_use]
    pub fn has_handoff(&self) -> bool {
        self.handoff.is_some()
    }

    /// Asks for the room list.
    pub fn list(&mut self) {
        self.link.send_lobby(&Message::ListRooms);
    }

    /// Opens a room.
    pub fn create(&mut self, settings: RoomSettings) {
        self.link.send_lobby(&Message::CreateRoom(settings));
    }

    /// Opens a room whose race this machine hosts ([`Message::HostRoom`]).
    pub fn host_room(&mut self, settings: RoomSettings, offer: HostOffer) {
        self.link.send_lobby(&Message::HostRoom { settings, offer });
    }

    /// The race the lobby asked this machine to host, once; the room and its job.
    pub fn take_launch(&mut self) -> Option<(u32, RaceJob)> {
        self.launch.take()
    }

    /// Tells the lobby the hosted race of `room` is loaded and its seats may dial.
    pub fn host_up(&mut self, room: u32) {
        self.link.send_lobby(&Message::HostUp { room });
    }

    /// The inputs the lobby asked this machine to show, as `(room, seat, tick)`; taken.
    pub fn take_challenges(&mut self) -> Vec<(u32, u8, u32)> {
        std::mem::take(&mut self.challenges)
    }

    /// Answers a challenge: the signed datagram behind the input, empty for none.
    pub fn show_input(&mut self, room: u32, seat: u8, tick: u32, datagram: Vec<u8>) {
        self.link.send_lobby(&Message::InputShown {
            room,
            seat,
            tick,
            datagram,
        });
    }

    /// Sends the lobby a case against the host of `room`'s race (a `crate::fair::Proof`'s
    /// bytes), one part each [`Self::poll`].
    pub fn queue_report(&mut self, room: u32, proof: &[u8]) {
        let parts: Vec<&[u8]> = proof
            .chunks(crate::wire::online::REPORT_PART_BYTES)
            .collect();
        let count = u16::try_from(parts.len()).unwrap_or(u16::MAX);
        for (part, bytes) in parts.into_iter().enumerate().take(usize::from(count)) {
            self.outgoing.push_back(Message::Report {
                room,
                part: u16::try_from(part).unwrap_or(u16::MAX),
                parts: count,
                bytes: bytes.to_vec(),
            });
        }
    }

    /// Whether report parts are still waiting to be sent.
    #[must_use]
    pub fn reporting(&self) -> bool {
        !self.outgoing.is_empty()
    }

    /// Sends the lobby one part of a case against the host of `room`'s race.
    pub fn report_part(&mut self, room: u32, part: u16, parts: u16, bytes: Vec<u8>) {
        self.link.send_lobby(&Message::Report {
            room,
            part,
            parts,
            bytes,
        });
    }

    /// Tells the lobby the hosted race of `room` is over.
    pub fn host_over(&mut self, room: u32, reason: &str, board: Board) {
        self.link.send_lobby(&Message::HostOver {
            room,
            reason: reason.to_string(),
            board,
        });
    }

    /// Joins a room.
    pub fn join(&mut self, room: u32) {
        self.link.send_lobby(&Message::JoinRoom { room });
    }

    /// Changes the room's settings (the owner).
    pub fn edit(&mut self, settings: RoomSettings) {
        self.link.send_lobby(&Message::Edit(settings));
    }

    /// Picks a team and says ready, or takes it back.
    ///
    /// `assist` is the Pilot Assist level this person flies with if the room allows it.
    pub fn ready(&mut self, team: &str, ready: bool, assist: oag_physics::pilot_assist::Level) {
        self.link.send_lobby(&Message::Ready {
            team: team.to_string(),
            ready,
            assist: assist_index(assist),
        });
    }

    /// Presses START (the owner).
    pub fn start(&mut self) {
        self.link.send_lobby(&Message::Start);
    }

    /// Asks the room for another race with the same settings (see [`Message::Rematch`]).
    pub fn rematch(&mut self) {
        self.link.send_lobby(&Message::Rematch);
    }

    /// Removes the named person from the room (the owner).
    pub fn kick(&mut self, name: &str) {
        self.link.send_lobby(&Message::Kick {
            name: name.to_string(),
        });
    }

    /// Leaves the room.
    pub fn leave(&mut self) {
        self.link.send_lobby(&Message::Leave);
    }
}

impl std::fmt::Debug for LobbyClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LobbyClient")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

/// The wire's spelling of a Pilot Assist level: its index in `Level::ALL`.
#[must_use]
pub fn assist_index(level: oag_physics::pilot_assist::Level) -> u8 {
    oag_physics::pilot_assist::Level::ALL
        .iter()
        .position(|&l| l == level)
        .map_or(0, |i| u8::try_from(i).unwrap_or(0))
}
