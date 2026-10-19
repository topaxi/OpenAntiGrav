//! A lobby client over any [`Link`]: says hello, keeps the list of rooms, the room it is
//! in and the hand-off to a race, and sends what a person does on the lobby screens.
//!
//! A frame at a time like [`crate::client::Client`]: [`LobbyClient::poll`] reads the link and
//! updates the fields, and the screens (or a test) read the fields.

use crate::link::Link;
use crate::wire::online::{Edition, Message, Refusal, RoomInfo, RoomSettings, Snapshot};

/// Where a race is, as the lobby told this client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handoff {
    /// The race's UDP port, on the host the lobby was dialled on.
    pub port: u16,
    /// SHA-256 of the race server's certificate.
    pub cert_hash: [u8; 32],
    /// The seat token for the race hello.
    pub token: String,
    /// The name the seat is under.
    pub name: String,
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
}

impl LobbyClient {
    /// Says hello on `link` as `name`, able to race `titles` (each with its platform and content); `token` is an earlier
    /// session's, empty for a new one.
    pub fn connect(mut link: Box<dyn Link>, name: &str, titles: &[Edition], token: &str) -> Self {
        link.send_lobby(&Message::hello(name, titles, token));
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
        }
    }

    /// Reads the link. `Err` when the connection ended.
    ///
    /// # Errors
    ///
    /// The link closed, and why. A refusal read in the same poll is held back one call, in `refused`.
    pub fn poll(&mut self) -> Result<(), String> {
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
                Ok(Message::Handoff {
                    port,
                    cert_hash,
                    token,
                    name,
                }) => {
                    self.handoff = Some(Handoff {
                        port,
                        cert_hash,
                        token,
                        name,
                    });
                }
                Ok(Message::Rejected { code, text }) => self.refused = Some((code, text)),
                Ok(other) => log::warn!("lobby: unexpected {other:?}"),
                Err(e) => log::warn!("lobby: {e}"),
            }
        }
        match self.link.closed() {
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
