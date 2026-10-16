//! The wire messages, version 1, and their byte encoding.
//!
//! Two channel kinds, two message families: [`Control`] goes on the one
//! reliable, ordered stream (join, start, resend, end) and [`Datagram`] on the
//! unreliable datagrams (each tick's input up, each tick's confirmed inputs and
//! state hash down). Every message is little-endian, hand-encoded, and refused by
//! name when it does not parse; an `f32` crosses as its exact bits, so the value a
//! server applies is the value the client predicted with.
//!
//! **The server sends inputs, not state.** A `RaceSim` snapshot is about 1.7 MiB
//! and a datagram about 1,400 bytes. The simulation is deterministic
//! (ADR-0002), so a client that starts from the same content and seed and applies
//! the same inputs arrives at the same state; the server sends, per tick, the
//! inputs it *applied* to every person's craft (its choice, after its own buffering
//! and repeat-on-empty, which is where its authority lies) and the state hash it
//! arrived at, which the client checks. A hash that disagrees is a determinism bug
//! or different content, never a misprediction. Chosen, not measured.

use oag_gameplay::InputSnapshot;
use oag_gameplay::input::Input;

/// The protocol version. A peer speaking another one is refused at the hello.
pub const PROTOCOL: u16 = 1;

/// The first four bytes of a hello, so a stray connection is refused by name.
pub const MAGIC: [u8; 4] = *b"OAGN";

/// One tick of a person's input as it crosses the wire: the held buttons and the
/// four axes. No edges: the server re-derives press and release from the held
/// stream it advances itself (decision 2 of `docs/networking/README.md`), and the
/// client derives its own prediction the same way, through [`Self::snapshot`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Held {
    /// The held button mask.
    pub buttons: u32,
    /// [`InputSnapshot::stick_x`].
    pub stick_x: f32,
    /// [`InputSnapshot::stick_y`].
    pub stick_y: f32,
    /// [`InputSnapshot::airbrake_left`].
    pub airbrake_left: f32,
    /// [`InputSnapshot::airbrake_right`].
    pub airbrake_right: f32,
}

impl Held {
    /// Bytes on the wire.
    pub const BYTES: usize = 20;

    /// What a device's snapshot holds, its axes clamped first.
    #[must_use]
    pub fn of(snapshot: &InputSnapshot) -> Self {
        let snapshot = snapshot.sanitised();
        Self {
            buttons: snapshot.buttons.held_mask(),
            stick_x: snapshot.stick_x,
            stick_y: snapshot.stick_y,
            airbrake_left: snapshot.airbrake_left,
            airbrake_right: snapshot.airbrake_right,
        }
    }

    /// The snapshot this tick applies, its edges advanced from `edges`: the one
    /// derivation both ends use.
    #[must_use]
    pub fn snapshot(self, edges: &mut Input) -> InputSnapshot {
        edges.begin_frame(self.buttons);
        InputSnapshot {
            buttons: *edges,
            stick_x: self.stick_x,
            stick_y: self.stick_y,
            airbrake_left: self.airbrake_left,
            airbrake_right: self.airbrake_right,
        }
        .sanitised()
    }

    fn put(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.buttons.to_le_bytes());
        for axis in [
            self.stick_x,
            self.stick_y,
            self.airbrake_left,
            self.airbrake_right,
        ] {
            out.extend_from_slice(&axis.to_bits().to_le_bytes());
        }
    }

    fn take(reader: &mut Reader<'_>) -> Result<Self, Malformed> {
        Ok(Self {
            buttons: reader.u32()?,
            stick_x: f32::from_bits(reader.u32()?),
            stick_y: f32::from_bits(reader.u32()?),
            airbrake_left: f32::from_bits(reader.u32()?),
            airbrake_right: f32::from_bits(reader.u32()?),
        })
    }
}

/// What a client must load to race on this server: the race's options as names,
/// so a client resolves them against its own copy of the title.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RaceParams {
    /// The title the server opened (`oag_title::Title::name`), checked by name.
    pub title: String,
    /// The circuit as `--track` names it.
    pub track: String,
    /// The team a seat flies when [`Self::teams`] names none for it, as `--team` names it.
    pub team: String,
    /// The team each person flies, in seat order (the first is slot 0); an empty or missing
    /// entry is [`Self::team`]. The hosting game fills it from each person's hello.
    pub teams: Vec<String>,
    /// The speed class, as the title spells it.
    pub class: String,
    /// [`oag_race::Mode::name`].
    pub mode: String,
    /// [`oag_ai::Difficulty::name`].
    pub difficulty: String,
    /// [`oag_gameplay::ControlScheme::name`]: how every snapshot maps to controls.
    /// One per race in this engine (`Race::set_control_scheme`), so a client flies
    /// the server's, whatever its own settings say.
    pub scheme: String,
    /// The world seed.
    pub seed: u64,
    /// How many people race (each in grid slot `0..people`).
    pub people: u8,
    /// Whether the AI fills the rest of the grid.
    pub opponents: bool,
    /// Whether weapons are on. Version 1 servers always say no: weapons are not
    /// server events yet (ADR-0069, condition 2).
    pub weapons: bool,
    /// Laps the race runs, or 0 for the mode's own (`Options::laps_override`). Chosen: only
    /// the modes that end on laps read it.
    pub laps: u8,
}

/// The reliable, ordered stream's messages.
#[derive(Debug, Clone, PartialEq)]
pub enum Control {
    /// Client to server, first: who is asking, in which protocol.
    Hello {
        /// [`PROTOCOL`] as the client speaks it.
        protocol: u16,
        /// A name for the server's log.
        name: String,
        /// The team this person wants to fly (`--team`'s spelling), empty for the host's choice.
        team: String,
        /// The seat token a lobby handed this person for this race, empty on a LAN host or
        /// a plain `oag-server`. A server started with seat tokens seats a hello by it.
        token: String,
    },
    /// Server to client: the seat (grid slot) this client flies, the race, and the
    /// state hash at tick 0 after the race starts, which the client must reproduce.
    Welcome {
        /// The grid slot this client's person flies.
        seat: u8,
        /// The race to load.
        params: RaceParams,
        /// `RaceSim::state_hash` at tick 0, every person's slot marked remote.
        fingerprint: u64,
    },
    /// Server to client: refused, and why. The connection closes after it.
    Reject {
        /// Why.
        reason: String,
    },
    /// Client to server: loaded, and arrived at this tick-0 hash.
    Ready {
        /// The client's own tick-0 hash.
        fingerprint: u64,
    },
    /// Server to client: the race clock starts. The server runs its tick 0
    /// `lead_ticks` ticks after sending this; a client starts at once, so it runs
    /// ahead of the server by that lead less half a round trip.
    Start {
        /// The head start, in ticks.
        lead_ticks: u16,
    },
    /// Client to server: confirmed ticks from `from` went missing (more datagrams
    /// lost in a row than the redundancy covers); send them on this stream.
    Resend {
        /// The first tick missing.
        from: u32,
    },
    /// Server to client: confirmed ticks, in reply to [`Self::Resend`].
    Ticks(Confirmed),
    /// Either way: the race is over or the peer is leaving.
    End {
        /// Why.
        reason: String,
    },
}

/// A run of consecutive confirmed ticks: what the server applied and arrived at.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Confirmed {
    /// The tick the first record is for: its inputs were applied with the world at
    /// this tick, and its hash is of the world after (at `first + 1`).
    pub first: u32,
    /// One record per tick from `first`.
    pub ticks: Vec<TickRecord>,
}

/// One confirmed tick.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TickRecord {
    /// `RaceSim::state_hash` after the tick.
    pub hash: u64,
    /// The input the server applied to each person's slot, `0..people` in order.
    pub inputs: Vec<Held>,
    /// The seats whose person has left, one bit a seat (bit 0 is seat 0), cumulative. From
    /// the tick it first appears in, both ends hand that craft to the AI driver before the
    /// tick runs ([`crate::race::retire`]), so a leaver cannot hold the race open. Chosen, not
    /// measured.
    pub retired: u8,
}

/// The unreliable datagrams' messages.
#[derive(Debug, Clone, PartialEq)]
pub enum Datagram {
    /// Client to server: this seat's inputs for the ticks from `first`, oldest
    /// first. Several ticks in each, so a lost datagram is covered by the next.
    Inputs {
        /// The sender's seat.
        seat: u8,
        /// The tick the first input is for.
        first: u32,
        /// One input per tick from `first`.
        inputs: Vec<Held>,
    },
    /// Server to client: the newest confirmed ticks (several, for the same
    /// reason), and how far ahead of the server this client's input is arriving.
    Confirmed {
        /// The newest input tick the server holds for the receiver's seat, less the
        /// tick the server is about to run: how many ticks of input are buffered.
        /// Negative when the client's input is arriving late.
        lead: i16,
        /// The ticks.
        confirmed: Confirmed,
    },
}

/// A message that would not parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed(pub &'static str);

impl std::fmt::Display for Malformed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "malformed message: {}", self.0)
    }
}

impl std::error::Error for Malformed {}

pub(crate) struct Reader<'a> {
    pub(crate) bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], Malformed> {
        if self.bytes.len() < n {
            return Err(Malformed("truncated"));
        }
        let (head, tail) = self.bytes.split_at(n);
        self.bytes = tail;
        Ok(head)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, Malformed> {
        Ok(self.take(1)?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16, Malformed> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, Malformed> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, Malformed> {
        let lo = u64::from(self.u32()?);
        let hi = u64::from(self.u32()?);
        Ok(lo | hi << 32)
    }

    pub(crate) fn string(&mut self) -> Result<String, Malformed> {
        let len = usize::from(self.u16()?);
        String::from_utf8(self.take(len)?.to_vec()).map_err(|_| Malformed("string is not UTF-8"))
    }

    pub(crate) fn end(&self) -> Result<(), Malformed> {
        if self.bytes.is_empty() {
            Ok(())
        } else {
            Err(Malformed("trailing bytes"))
        }
    }
}

pub(crate) fn put_string(out: &mut Vec<u8>, text: &str) {
    let bytes = &text.as_bytes()[..text.len().min(usize::from(u16::MAX))];
    out.extend_from_slice(&u16::try_from(bytes.len()).unwrap_or(u16::MAX).to_le_bytes());
    out.extend_from_slice(bytes);
}

impl Confirmed {
    fn put(&self, out: &mut Vec<u8>) {
        let people = self.ticks.first().map_or(0, |t| t.inputs.len());
        out.extend_from_slice(&self.first.to_le_bytes());
        out.push(u8::try_from(people).unwrap_or(u8::MAX));
        out.extend_from_slice(
            &u16::try_from(self.ticks.len())
                .unwrap_or(u16::MAX)
                .to_le_bytes(),
        );
        for tick in &self.ticks {
            out.extend_from_slice(&tick.hash.to_le_bytes());
            out.push(tick.retired);
            for held in tick.inputs.iter().take(people) {
                held.put(out);
            }
        }
    }

    fn take(reader: &mut Reader<'_>) -> Result<Self, Malformed> {
        let first = reader.u32()?;
        let people = usize::from(reader.u8()?);
        let count = usize::from(reader.u16()?);
        let mut ticks = Vec::with_capacity(count);
        for _ in 0..count {
            let hash = reader.u64()?;
            let retired = reader.u8()?;
            let inputs = (0..people)
                .map(|_| Held::take(reader))
                .collect::<Result<_, _>>()?;
            ticks.push(TickRecord {
                hash,
                inputs,
                retired,
            });
        }
        Ok(Self { first, ticks })
    }

    /// The bytes one tick record costs with `people` people.
    #[must_use]
    pub const fn record_bytes(people: usize) -> usize {
        9 + people * Held::BYTES
    }
}

impl RaceParams {
    fn put(&self, out: &mut Vec<u8>) {
        for text in [
            &self.title,
            &self.track,
            &self.team,
            &self.class,
            &self.mode,
            &self.difficulty,
            &self.scheme,
        ] {
            put_string(out, text);
        }
        out.push(u8::try_from(self.teams.len()).unwrap_or(u8::MAX));
        for team in self.teams.iter().take(usize::from(u8::MAX)) {
            put_string(out, team);
        }
        out.extend_from_slice(&self.seed.to_le_bytes());
        out.push(self.people);
        out.push(u8::from(self.opponents));
        out.push(u8::from(self.weapons));
        out.push(self.laps);
    }

    fn take(reader: &mut Reader<'_>) -> Result<Self, Malformed> {
        let title = reader.string()?;
        let track = reader.string()?;
        let team = reader.string()?;
        let class = reader.string()?;
        let mode = reader.string()?;
        let difficulty = reader.string()?;
        let scheme = reader.string()?;
        let count = reader.u8()?;
        let teams = (0..count)
            .map(|_| reader.string())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            title,
            track,
            team,
            teams,
            class,
            mode,
            difficulty,
            scheme,
            seed: reader.u64()?,
            people: reader.u8()?,
            opponents: reader.u8()? != 0,
            weapons: reader.u8()? != 0,
            laps: reader.u8()?,
        })
    }
}

impl Control {
    /// The message's bytes, without the stream's length prefix.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            Self::Hello {
                protocol,
                name,
                team,
                token,
            } => {
                out.push(1);
                out.extend_from_slice(&MAGIC);
                out.extend_from_slice(&protocol.to_le_bytes());
                put_string(&mut out, name);
                put_string(&mut out, team);
                put_string(&mut out, token);
            }
            Self::Welcome {
                seat,
                params,
                fingerprint,
            } => {
                out.push(2);
                out.push(*seat);
                params.put(&mut out);
                out.extend_from_slice(&fingerprint.to_le_bytes());
            }
            Self::Reject { reason } => {
                out.push(3);
                put_string(&mut out, reason);
            }
            Self::Ready { fingerprint } => {
                out.push(4);
                out.extend_from_slice(&fingerprint.to_le_bytes());
            }
            Self::Start { lead_ticks } => {
                out.push(5);
                out.extend_from_slice(&lead_ticks.to_le_bytes());
            }
            Self::Resend { from } => {
                out.push(6);
                out.extend_from_slice(&from.to_le_bytes());
            }
            Self::Ticks(confirmed) => {
                out.push(7);
                confirmed.put(&mut out);
            }
            Self::End { reason } => {
                out.push(8);
                put_string(&mut out, reason);
            }
        }
        out
    }

    /// Parses one message.
    ///
    /// # Errors
    ///
    /// [`Malformed`] for a truncated or unknown message, a hello without the
    /// magic, or bytes left over.
    pub fn decode(bytes: &[u8]) -> Result<Self, Malformed> {
        let mut reader = Reader { bytes };
        let message = match reader.u8()? {
            1 => {
                if reader.take(4)? != MAGIC {
                    return Err(Malformed("hello without the OAGN magic"));
                }
                Self::Hello {
                    protocol: reader.u16()?,
                    name: reader.string()?,
                    team: reader.string()?,
                    token: reader.string()?,
                }
            }
            2 => Self::Welcome {
                seat: reader.u8()?,
                params: RaceParams::take(&mut reader)?,
                fingerprint: reader.u64()?,
            },
            3 => Self::Reject {
                reason: reader.string()?,
            },
            4 => Self::Ready {
                fingerprint: reader.u64()?,
            },
            5 => Self::Start {
                lead_ticks: reader.u16()?,
            },
            6 => Self::Resend {
                from: reader.u32()?,
            },
            7 => Self::Ticks(Confirmed::take(&mut reader)?),
            8 => Self::End {
                reason: reader.string()?,
            },
            _ => return Err(Malformed("unknown control message")),
        };
        reader.end()?;
        Ok(message)
    }
}

impl Datagram {
    /// The datagram's bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            Self::Inputs {
                seat,
                first,
                inputs,
            } => {
                out.push(0x10);
                out.push(*seat);
                out.extend_from_slice(&first.to_le_bytes());
                out.push(u8::try_from(inputs.len()).unwrap_or(u8::MAX));
                for held in inputs.iter().take(usize::from(u8::MAX)) {
                    held.put(&mut out);
                }
            }
            Self::Confirmed { lead, confirmed } => {
                out.push(0x11);
                out.extend_from_slice(&lead.to_le_bytes());
                confirmed.put(&mut out);
            }
        }
        out
    }

    /// Parses one datagram.
    ///
    /// # Errors
    ///
    /// [`Malformed`] for a truncated or unknown datagram, or bytes left over.
    pub fn decode(bytes: &[u8]) -> Result<Self, Malformed> {
        let mut reader = Reader { bytes };
        let message = match reader.u8()? {
            0x10 => {
                let seat = reader.u8()?;
                let first = reader.u32()?;
                let count = usize::from(reader.u8()?);
                let inputs = (0..count)
                    .map(|_| Held::take(&mut reader))
                    .collect::<Result<_, _>>()?;
                Self::Inputs {
                    seat,
                    first,
                    inputs,
                }
            }
            0x11 => {
                let lead = i16::from_le_bytes([reader.u8()?, reader.u8()?]);
                Self::Confirmed {
                    lead,
                    confirmed: Confirmed::take(&mut reader)?,
                }
            }
            _ => return Err(Malformed("unknown datagram")),
        };
        reader.end()?;
        Ok(message)
    }
}

pub mod online;

#[cfg(test)]
mod tests;
