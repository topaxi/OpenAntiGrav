//! The online lobby's messages: rooms, a ready and start gate, the queue for the one game
//! slot, and the hand-off to a race.
//!
//! They ride the same reliable stream as [`super::Control`] (a lobby is a WebTransport
//! endpoint like a race), with tag bytes of their own (`0x20..`) that collide with neither
//! [`super::Control`] (1 to 8) nor [`super::Datagram`] (`0x10`, `0x11`), so a client that
//! dials the wrong port is refused by name by either end. Chosen, not measured: the
//! shapes follow Omega's lobby (`docs/networking/online-lobby.md`), the bytes are ours.

use super::board::Board;
use super::{Malformed, PROTOCOL, RaceParams, Reader, put_string};

/// The first four bytes of a lobby hello, so a race client dialling a lobby is refused by name.
pub const MAGIC: [u8; 4] = *b"OAGL";

/// The fewest people a room can start with. Omega's lobby says "at least %d players"
/// (`ONL_LOB_STARTFAILED`) without the number; two is chosen.
pub const MIN_PLAYERS: u8 = 2;

/// The most people a room seats (Omega's lobby draws eight rows).
pub const MAX_PLAYERS: u8 = 8;

/// What a machine says it can race, per title: enough for the lobby to show it only rooms it
/// can join. The same facts LAN discovery compares ([`crate::discovery::differs`]).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Edition {
    /// The title (`oag_title::Title::name`).
    pub title: String,
    /// The platform of the image (`oag_disc::Platform`'s display): a PSP and a PS2 pressing of
    /// one title are told apart, as in discovery.
    pub platform: String,
    /// The mods this machine runs, fingerprinted (0: none). Mods may change the sim.
    pub content: u64,
    /// The build ([`crate::BUILD_VERSION`]): a different build may simulate differently, and
    /// the lobby refuses it by name at the hello rather than at the race's tick-0 hash.
    pub build: String,
}

/// What the owner of a room decides, after Omega's `MP Select` table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoomSettings {
    /// The title (`oag_title::Title::name`): the server loads this title's image for the race.
    pub title: String,
    /// The circuit as `--track` spells it, empty for the title's default.
    pub track: String,
    /// The speed class, as the title spells it.
    pub class: String,
    /// [`oag_race::Mode::name`].
    pub mode: String,
    /// How many people the room seats, `2..=8`.
    pub max_players: u8,
    /// Whether the AI fills the rest of the grid.
    pub opponents: bool,
    /// Laps the race runs, `0` for the mode's own, else `1..=20` (Omega's slider is 3 to 20;
    /// fewer is chosen, for a short test race).
    pub laps: u8,
    /// Whether barrel rolls are off (Omega's `Barrel rolls` row, inverted so the default is the
    /// originals' race).
    pub no_barrel_rolls: bool,
    /// Whether people may fly with Pilot Assist (Omega's `Pilot assist allowed`). Off, every
    /// seat flies without it whatever its owner has set.
    pub pilot_assist: bool,
    /// Whether weapons are on (Omega's `Weapons` row, `RB_WEAP`, default on). An Eliminator room
    /// is always on: the mode is about them.
    pub weapons: bool,
    /// Zone's ending in zones: `0` for none, else `15..=55` in fives (Omega's `NitroTarget`
    /// slider, default 30). Zeroed outside Zone.
    pub zone_target: u8,
}

/// One person in a room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// The chosen name.
    pub name: String,
    /// The team (ship) picked, empty before a pick.
    pub team: String,
    /// Whether the pick is made and the person is ready for START.
    pub ready: bool,
    /// Whether this person owns the room: edits settings, presses START.
    pub owner: bool,
}

/// Where a room is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Gathering people; joinable.
    Open,
    /// START was pressed and the one game slot is busy: this room is `position`th in line
    /// (1 is next).
    Queued {
        /// Rooms ahead, plus one.
        position: u16,
    },
    /// The slot is ours: the race starts in `millis`.
    Countdown {
        /// Milliseconds left.
        millis: u32,
    },
    /// The race is being brought up or run.
    Racing,
}

/// A room as its members see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// The room's id.
    pub id: u32,
    /// The settings.
    pub settings: RoomSettings,
    /// Everyone in it, owner first, then in join order.
    pub members: Vec<Member>,
    /// Where it is.
    pub phase: Phase,
    /// A line for the screen (why a start was refused, who was made owner), possibly empty.
    pub note: String,
    /// The last race's final board, empty until the room has raced.
    pub board: Board,
}

/// A room as the list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomInfo {
    /// The room's id.
    pub id: u32,
    /// The owner's name.
    pub owner: String,
    /// The settings.
    pub settings: RoomSettings,
    /// How many are in it.
    pub members: u8,
    /// Whether a join would be accepted right now.
    pub joinable: bool,
}

/// Why the lobby said no.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The room is full.
    Full,
    /// The room has pressed START: "This game has already started".
    Started,
    /// This client lacks the room's title or speaks another build.
    Content,
    /// The lobby speaks another protocol.
    Version,
    /// No such room.
    NoSuchRoom,
    /// Only the room's owner can do that.
    NotOwner,
    /// START needs more people in the room.
    NeedPlayers,
    /// START needs every person's pick made.
    NotReady,
    /// The name is empty, too long, or in use by another session.
    BadName,
    /// This client is not in a room.
    NotInRoom,
    /// This client is already in a room.
    InRoom,
    /// The settings name something this server cannot race.
    BadSettings,
    /// The room's owner removed this person from it, and the room remembers.
    Kicked,
}

impl Refusal {
    const ALL: [Self; 13] = [
        Self::Full,
        Self::Started,
        Self::Content,
        Self::Version,
        Self::NoSuchRoom,
        Self::NotOwner,
        Self::NeedPlayers,
        Self::NotReady,
        Self::BadName,
        Self::NotInRoom,
        Self::InRoom,
        Self::BadSettings,
        Self::Kicked,
    ];

    fn from_byte(byte: u8) -> Result<Self, Malformed> {
        Self::ALL
            .get(usize::from(byte))
            .copied()
            .ok_or(Malformed("unknown refusal code"))
    }

    fn byte(self) -> u8 {
        Self::ALL
            .iter()
            .position(|&code| code == self)
            .map_or(0, |i| u8::try_from(i).unwrap_or(0))
    }
}

/// The lobby's messages, both ways.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// Client to lobby, first: the protocol, a chosen name, the titles this machine can
    /// race (so the list shows only rooms it can join), and a session token to take a
    /// session over (empty for a new one).
    Hello {
        /// [`PROTOCOL`] as the client speaks it.
        protocol: u16,
        /// The chosen name.
        name: String,
        /// What this machine can race, one per title it has an image for.
        titles: Vec<Edition>,
        /// A token an earlier [`Self::Welcome`] issued, empty for none.
        token: String,
    },
    /// Client to lobby: send [`Self::Rooms`].
    ListRooms,
    /// Client to lobby: open a room and own it.
    CreateRoom(RoomSettings),
    /// Client to lobby: join a room.
    JoinRoom {
        /// The room.
        room: u32,
    },
    /// Owner to lobby: change the settings (everyone's pick is cleared, as in Omega's lobby).
    Edit(RoomSettings),
    /// Client to lobby: pick a team and say ready (or take the ready back).
    Ready {
        /// The team, as `--team` spells it.
        team: String,
        /// Ready or not.
        ready: bool,
        /// The Pilot Assist level this person flies with if the room allows it, as the index
        /// into `oag_physics::pilot_assist::Level::ALL`.
        assist: u8,
    },
    /// Owner to lobby: press START.
    Start,
    /// Client to lobby: leave the room.
    Leave,
    /// Client to lobby: race again with the same settings. The sender is ready with the team
    /// last picked (also while the race is still running, taking effect when it ends); from
    /// the owner it also arms the start, which fires once everyone is ready.
    Rematch,
    /// Owner to lobby: remove the named person from the room. The room refuses them back.
    Kick {
        /// The person's name.
        name: String,
    },
    /// Lobby to client: the session, with the token that takes it over after a dropped link.
    Welcome {
        /// The session token.
        token: String,
        /// The name as the lobby accepted it.
        name: String,
        /// The titles this server races.
        titles: Vec<String>,
    },
    /// Lobby to client: the rooms this client can join (or see).
    Rooms(Vec<RoomInfo>),
    /// Lobby to client: the room the client is in, pushed on every change.
    Room(Snapshot),
    /// Lobby to client: no longer in a room, and why (a race ended, the room closed).
    Left {
        /// Why.
        reason: String,
    },
    /// Lobby to client: the race is up. Reconnect to the lobby's host on `port`, trusting
    /// only `cert_hash`, and say the seat `token` in the race hello. The lobby link stays open.
    Handoff {
        /// The race's UDP port, on the host the lobby was dialled on.
        port: u16,
        /// SHA-256 of the race server's certificate.
        cert_hash: [u8; 32],
        /// The seat token for [`super::Control::Hello`].
        token: String,
        /// The name the seat is under.
        name: String,
    },
    /// Lobby to client: refused, and why.
    Rejected {
        /// The named reason.
        code: Refusal,
        /// A line for the screen.
        text: String,
    },
}

/// What the lobby tells a race process it brought up: the race, and who sits where with
/// the token each person must say in their race hello. Written to the child's standard
/// input as one hex line (not argv, where any local user can read the tokens).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaceJob {
    /// The race.
    pub params: RaceParams,
    /// `(name, seat token)` by seat.
    pub seats: Vec<(String, String)>,
}

impl RaceJob {
    /// The job as bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.params.put(&mut out);
        out.push(u8::try_from(self.seats.len()).unwrap_or(u8::MAX));
        for (name, token) in self.seats.iter().take(usize::from(u8::MAX)) {
            put_string(&mut out, name);
            put_string(&mut out, token);
        }
        out
    }

    /// Parses the job.
    ///
    /// # Errors
    ///
    /// [`Malformed`] for truncated bytes or bytes left over.
    pub fn decode(bytes: &[u8]) -> Result<Self, Malformed> {
        let mut r = Reader { bytes };
        let params = RaceParams::take(&mut r)?;
        let count = r.u8()?;
        let seats = (0..count)
            .map(|_| Ok((r.string()?, r.string()?)))
            .collect::<Result<Vec<_>, Malformed>>()?;
        r.end()?;
        Ok(Self { params, seats })
    }

    /// The job as one line of hex.
    #[must_use]
    pub fn to_line(&self) -> String {
        super::to_hex(&self.encode())
    }

    /// Parses [`Self::to_line`].
    ///
    /// # Errors
    ///
    /// [`Malformed`] for odd or non-hex text, or bytes that are not a job.
    pub fn from_line(line: &str) -> Result<Self, Malformed> {
        Self::decode(&super::from_hex(line)?)
    }
}

fn put_editions(out: &mut Vec<u8>, list: &[Edition]) {
    out.push(u8::try_from(list.len()).unwrap_or(u8::MAX));
    for e in list.iter().take(usize::from(u8::MAX)) {
        put_string(out, &e.title);
        put_string(out, &e.platform);
        out.extend_from_slice(&e.content.to_le_bytes());
        put_string(out, &e.build);
    }
}

fn take_editions(r: &mut Reader<'_>) -> Result<Vec<Edition>, Malformed> {
    let count = r.u8()?;
    (0..count)
        .map(|_| {
            Ok(Edition {
                title: r.string()?,
                platform: r.string()?,
                content: r.u64()?,
                build: r.string()?,
            })
        })
        .collect()
}

fn put_settings(out: &mut Vec<u8>, s: &RoomSettings) {
    for text in [&s.title, &s.track, &s.class, &s.mode] {
        put_string(out, text);
    }
    out.push(s.max_players);
    out.push(u8::from(s.opponents));
    out.push(s.laps);
    out.push(u8::from(s.no_barrel_rolls));
    out.push(u8::from(s.pilot_assist));
    out.push(u8::from(s.weapons));
    out.push(s.zone_target);
}

fn take_settings(r: &mut Reader<'_>) -> Result<RoomSettings, Malformed> {
    Ok(RoomSettings {
        title: r.string()?,
        track: r.string()?,
        class: r.string()?,
        mode: r.string()?,
        max_players: r.u8()?,
        opponents: r.u8()? != 0,
        laps: r.u8()?,
        no_barrel_rolls: r.u8()? != 0,
        pilot_assist: r.u8()? != 0,
        weapons: r.u8()? != 0,
        zone_target: r.u8()?,
    })
}

fn put_strings(out: &mut Vec<u8>, list: &[String]) {
    out.push(u8::try_from(list.len()).unwrap_or(u8::MAX));
    for text in list.iter().take(usize::from(u8::MAX)) {
        put_string(out, text);
    }
}

fn take_strings(r: &mut Reader<'_>) -> Result<Vec<String>, Malformed> {
    let count = r.u8()?;
    (0..count).map(|_| r.string()).collect()
}

fn put_phase(out: &mut Vec<u8>, phase: Phase) {
    match phase {
        Phase::Open => out.push(0),
        Phase::Queued { position } => {
            out.push(1);
            out.extend_from_slice(&position.to_le_bytes());
        }
        Phase::Countdown { millis } => {
            out.push(2);
            out.extend_from_slice(&millis.to_le_bytes());
        }
        Phase::Racing => out.push(3),
    }
}

fn take_phase(r: &mut Reader<'_>) -> Result<Phase, Malformed> {
    Ok(match r.u8()? {
        0 => Phase::Open,
        1 => Phase::Queued { position: r.u16()? },
        2 => Phase::Countdown { millis: r.u32()? },
        3 => Phase::Racing,
        _ => return Err(Malformed("unknown room phase")),
    })
}

impl Message {
    /// The message's bytes, without the stream's length prefix.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            Self::Hello {
                protocol,
                name,
                titles,
                token,
            } => {
                out.push(0x20);
                out.extend_from_slice(&MAGIC);
                out.extend_from_slice(&protocol.to_le_bytes());
                put_string(&mut out, name);
                put_editions(&mut out, titles);
                put_string(&mut out, token);
            }
            Self::ListRooms => out.push(0x21),
            Self::CreateRoom(settings) => {
                out.push(0x22);
                put_settings(&mut out, settings);
            }
            Self::JoinRoom { room } => {
                out.push(0x23);
                out.extend_from_slice(&room.to_le_bytes());
            }
            Self::Edit(settings) => {
                out.push(0x24);
                put_settings(&mut out, settings);
            }
            Self::Ready {
                team,
                ready,
                assist,
            } => {
                out.push(0x25);
                put_string(&mut out, team);
                out.push(u8::from(*ready));
                out.push(*assist);
            }
            Self::Start => out.push(0x26),
            Self::Leave => out.push(0x27),
            Self::Rematch => out.push(0x28),
            Self::Kick { name } => {
                out.push(0x29);
                put_string(&mut out, name);
            }
            Self::Welcome {
                token,
                name,
                titles,
            } => {
                out.push(0x30);
                put_string(&mut out, token);
                put_string(&mut out, name);
                put_strings(&mut out, titles);
            }
            Self::Rooms(rooms) => {
                out.push(0x31);
                out.extend_from_slice(
                    &u16::try_from(rooms.len()).unwrap_or(u16::MAX).to_le_bytes(),
                );
                for room in rooms.iter().take(usize::from(u16::MAX)) {
                    out.extend_from_slice(&room.id.to_le_bytes());
                    put_string(&mut out, &room.owner);
                    put_settings(&mut out, &room.settings);
                    out.push(room.members);
                    out.push(u8::from(room.joinable));
                }
            }
            Self::Room(room) => {
                out.push(0x32);
                out.extend_from_slice(&room.id.to_le_bytes());
                put_settings(&mut out, &room.settings);
                out.push(u8::try_from(room.members.len()).unwrap_or(u8::MAX));
                for member in room.members.iter().take(usize::from(u8::MAX)) {
                    put_string(&mut out, &member.name);
                    put_string(&mut out, &member.team);
                    out.push(u8::from(member.ready));
                    out.push(u8::from(member.owner));
                }
                put_phase(&mut out, room.phase);
                put_string(&mut out, &room.note);
                room.board.put(&mut out);
            }
            Self::Left { reason } => {
                out.push(0x33);
                put_string(&mut out, reason);
            }
            Self::Handoff {
                port,
                cert_hash,
                token,
                name,
            } => {
                out.push(0x34);
                out.extend_from_slice(&port.to_le_bytes());
                out.extend_from_slice(cert_hash);
                put_string(&mut out, token);
                put_string(&mut out, name);
            }
            Self::Rejected { code, text } => {
                out.push(0x35);
                out.push(code.byte());
                put_string(&mut out, text);
            }
        }
        out
    }

    /// Parses one message.
    ///
    /// # Errors
    ///
    /// [`Malformed`] for a truncated or unknown message, a hello without the magic, or
    /// bytes left over.
    pub fn decode(bytes: &[u8]) -> Result<Self, Malformed> {
        let mut r = Reader { bytes };
        let message = match r.u8()? {
            0x20 => {
                if r.take(4)? != MAGIC {
                    return Err(Malformed("lobby hello without the OAGL magic"));
                }
                Self::Hello {
                    protocol: r.u16()?,
                    name: r.string()?,
                    titles: take_editions(&mut r)?,
                    token: r.string()?,
                }
            }
            0x21 => Self::ListRooms,
            0x22 => Self::CreateRoom(take_settings(&mut r)?),
            0x23 => Self::JoinRoom { room: r.u32()? },
            0x24 => Self::Edit(take_settings(&mut r)?),
            0x25 => Self::Ready {
                team: r.string()?,
                ready: r.u8()? != 0,
                assist: r.u8()?,
            },
            0x26 => Self::Start,
            0x27 => Self::Leave,
            0x28 => Self::Rematch,
            0x29 => Self::Kick { name: r.string()? },
            0x30 => Self::Welcome {
                token: r.string()?,
                name: r.string()?,
                titles: take_strings(&mut r)?,
            },
            0x31 => {
                let count = r.u16()?;
                let mut rooms = Vec::new();
                for _ in 0..count {
                    rooms.push(RoomInfo {
                        id: r.u32()?,
                        owner: r.string()?,
                        settings: take_settings(&mut r)?,
                        members: r.u8()?,
                        joinable: r.u8()? != 0,
                    });
                }
                Self::Rooms(rooms)
            }
            0x32 => {
                let id = r.u32()?;
                let settings = take_settings(&mut r)?;
                let count = r.u8()?;
                let mut members = Vec::new();
                for _ in 0..count {
                    members.push(Member {
                        name: r.string()?,
                        team: r.string()?,
                        ready: r.u8()? != 0,
                        owner: r.u8()? != 0,
                    });
                }
                Self::Room(Snapshot {
                    id,
                    settings,
                    members,
                    phase: take_phase(&mut r)?,
                    note: r.string()?,
                    board: Board::take(&mut r)?,
                })
            }
            0x33 => Self::Left {
                reason: r.string()?,
            },
            0x34 => {
                let port = r.u16()?;
                let mut cert_hash = [0; 32];
                cert_hash.copy_from_slice(r.take(32)?);
                Self::Handoff {
                    port,
                    cert_hash,
                    token: r.string()?,
                    name: r.string()?,
                }
            }
            0x35 => Self::Rejected {
                code: Refusal::from_byte(r.u8()?)?,
                text: r.string()?,
            },
            _ => return Err(Malformed("unknown lobby message")),
        };
        r.end()?;
        Ok(message)
    }

    /// The hello a client opens with.
    #[must_use]
    pub fn hello(name: &str, titles: &[Edition], token: &str) -> Self {
        Self::Hello {
            protocol: PROTOCOL,
            name: name.to_string(),
            titles: titles.to_vec(),
            token: token.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> RoomSettings {
        RoomSettings {
            title: "Wipeout Pulse".into(),
            track: "01_Track".into(),
            class: "VENOM".into(),
            mode: "single_race".into(),
            max_players: 4,
            opponents: true,
            laps: 3,
            no_barrel_rolls: true,
            pilot_assist: true,
            weapons: true,
            zone_target: 30,
        }
    }

    #[test]
    fn every_lobby_message_round_trips() {
        let messages = [
            Message::hello(
                "ann",
                &[
                    Edition {
                        title: "Wipeout Pulse".into(),
                        platform: "PSP".into(),
                        content: 0xABCD,
                        build: "0.1.0 (abc1234)".into(),
                    },
                    Edition::default(),
                ],
                "tok",
            ),
            Message::ListRooms,
            Message::CreateRoom(settings()),
            Message::JoinRoom { room: 9 },
            Message::Edit(settings()),
            Message::Ready {
                team: "Feisar".into(),
                ready: true,
                assist: 0,
            },
            Message::Start,
            Message::Leave,
            Message::Rematch,
            Message::Kick { name: "bob".into() },
            Message::Welcome {
                token: "abc".into(),
                name: "ann".into(),
                titles: vec!["Wipeout Pulse".into()],
            },
            Message::Rooms(vec![RoomInfo {
                id: 3,
                owner: "bob".into(),
                settings: settings(),
                members: 2,
                joinable: true,
            }]),
            Message::Room(Snapshot {
                id: 3,
                settings: settings(),
                members: vec![Member {
                    name: "bob".into(),
                    team: String::new(),
                    ready: false,
                    owner: true,
                }],
                phase: Phase::Queued { position: 2 },
                note: "bob is the owner".into(),
                board: Board {
                    rows: vec![crate::wire::board::BoardRow {
                        place: 1,
                        slot: 0,
                        name: "bob".into(),
                        laps: 3,
                        finish_ticks: Some(100),
                        best_lap_ticks: None,
                        eliminations: 1,
                    }],
                },
            }),
            Message::Left {
                reason: "race over".into(),
            },
            Message::Handoff {
                port: 60001,
                cert_hash: [7; 32],
                token: "seat".into(),
                name: "ann".into(),
            },
        ];
        for message in messages {
            assert_eq!(Message::decode(&message.encode()), Ok(message));
        }
        for phase in [
            Phase::Open,
            Phase::Countdown { millis: 3000 },
            Phase::Racing,
        ] {
            let room = Message::Room(Snapshot {
                id: 1,
                settings: settings(),
                members: Vec::new(),
                phase,
                note: String::new(),
                board: Board::default(),
            });
            assert_eq!(Message::decode(&room.encode()), Ok(room));
        }
        for code in Refusal::ALL {
            let message = Message::Rejected {
                code,
                text: "no".into(),
            };
            assert_eq!(Message::decode(&message.encode()), Ok(message));
        }
    }

    #[test]
    fn a_race_job_round_trips_through_its_stdin_line() {
        let job = RaceJob {
            params: RaceParams {
                title: "Wipeout Pulse".into(),
                track: "01_Track".into(),
                team: "Feisar".into(),
                teams: vec!["Feisar".into(), "Qirex".into()],
                class: "VENOM".into(),
                mode: "single_race".into(),
                difficulty: "elite".into(),
                scheme: "veteran".into(),
                seed: 0xFEED_0000_0000_0001,
                people: 2,
                opponents: false,
                weapons: false,
                laps: 3,
                no_barrel_rolls: true,
                assists: vec![0, 2],
                zone_target: 30,
            },
            seats: vec![("ann".into(), "t0".into()), ("bob".into(), "t1".into())],
        };
        assert_eq!(RaceJob::from_line(&job.to_line()), Ok(job));
        assert!(RaceJob::from_line("zz").is_err());
        assert!(RaceJob::from_line("abc").is_err());
    }

    #[test]
    fn a_race_message_and_a_lobby_message_never_parse_as_each_other() {
        let lobby = Message::ListRooms.encode();
        assert!(super::super::Control::decode(&lobby).is_err());
        let race = super::super::Control::Start { lead_ticks: 1 }.encode();
        assert!(Message::decode(&race).is_err());
    }

    #[test]
    fn a_truncated_or_unmagicked_hello_is_refused_by_name() {
        let hello = Message::hello("x", &[], "").encode();
        assert_eq!(
            Message::decode(&hello[..hello.len() - 1]),
            Err(Malformed("truncated"))
        );
        let mut wrong = hello;
        wrong[1] = b'X';
        assert!(Message::decode(&wrong).is_err());
    }
}
