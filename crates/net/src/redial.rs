//! Getting back to a seat after the link closed: the retry schedule a client runs while its
//! phase is [`Phase::Lost`], with the dialling left to the caller ([`Dialer`]) so a window
//! never blocks on it and a test needs no socket.
//!
//! A redial says hello with the seat token ([`Client::reconnect`]); the host reseats the
//! token if the race still holds the seat, and refuses it otherwise, which ends the race for
//! this client with the host's reason. The schedule gives up after [`GIVE_UP`], the time a
//! lobby keeps a seat's person ([`crate::host::SEAT_GRACE`]), because past it nobody is
//! holding the seat.

use std::time::Duration;

use web_time::Instant;

use crate::client::{Client, Phase};
use crate::link::Link;

/// How long to wait before the next dial, by attempts so far; the last entry repeats.
/// Chosen, not measured.
pub const BACKOFF: [Duration; 5] = [
    Duration::from_millis(250),
    Duration::from_millis(500),
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];

/// How long the redial lasts before the client gives up: [`crate::host::SEAT_GRACE`].
pub const GIVE_UP: Duration = crate::host::SEAT_GRACE;

/// How a dial went, polled a frame at a time.
pub enum Dialed {
    /// Still connecting.
    Pending,
    /// Connected.
    Link(Box<dyn Link>),
    /// The attempt failed, and why.
    Failed(String),
}

/// Starts a connection to the server off the calling thread and reports it.
pub trait Dialer {
    /// Begins one attempt. Must not block.
    fn begin(&mut self);
    /// How the attempt begun by [`Self::begin`] is going.
    fn poll(&mut self) -> Dialed;
}

/// Who says hello again.
#[derive(Debug, Clone, Default)]
pub struct Identity {
    /// The name in the server's log.
    pub name: String,
    /// The team asked for, empty for the host's choice.
    pub team: String,
    /// The seat token.
    pub token: String,
}

/// The retry schedule of one client.
#[derive(Debug)]
pub struct Redial {
    identity: Identity,
    backoff: &'static [Duration],
    give_up: Duration,
    lost_at: Option<Instant>,
    next_at: Option<Instant>,
    dialing: bool,
    attempts: u32,
}

impl Redial {
    /// A schedule that dials as [`BACKOFF`] says and gives up after [`GIVE_UP`].
    #[must_use]
    pub fn new(identity: Identity) -> Self {
        Self::with(identity, &BACKOFF, GIVE_UP)
    }

    /// A schedule with its own spacing and patience.
    #[must_use]
    pub fn with(identity: Identity, backoff: &'static [Duration], give_up: Duration) -> Self {
        Self {
            identity,
            backoff,
            give_up,
            lost_at: None,
            next_at: None,
            dialing: false,
            attempts: 0,
        }
    }

    /// How many dials have been begun since the link was lost.
    #[must_use]
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    /// How much of the patience is left, while the link is lost.
    #[must_use]
    pub fn left(&self, now: Instant) -> Option<Duration> {
        let lost_at = self.lost_at?;
        Some(self.give_up.saturating_sub(now.duration_since(lost_at)))
    }

    /// Whether the client is between a loss and getting its seat back.
    #[must_use]
    pub fn active(&self) -> bool {
        self.lost_at.is_some()
    }

    /// One frame: starts a dial when one is due, hands a connected link to `client`, and gives
    /// up when the seat can no longer be held.
    pub fn step(&mut self, client: &mut Client, dialer: &mut dyn Dialer, now: Instant) {
        match client.phase() {
            Phase::Lost(_) => {}
            Phase::Racing => {
                *self = Self::with(self.identity.clone(), self.backoff, self.give_up);
                return;
            }
            _ => return,
        }
        let lost_at = *self.lost_at.get_or_insert(now);
        if now.duration_since(lost_at) > self.give_up {
            client.give_up(&format!(
                "the connection could not be restored in {} s",
                self.give_up.as_secs()
            ));
            return;
        }
        if self.dialing {
            match dialer.poll() {
                Dialed::Pending => {}
                Dialed::Link(link) => {
                    self.dialing = false;
                    self.next_at = Some(now + self.pause());
                    let who = &self.identity;
                    client.reconnect(link, &who.name, &who.team, &who.token);
                }
                Dialed::Failed(why) => {
                    log::debug!("net: redial {} failed: {why}", self.attempts);
                    self.dialing = false;
                    self.next_at = Some(now + self.pause());
                }
            }
        } else if self.next_at.is_none_or(|due| now >= due) {
            self.attempts += 1;
            self.dialing = true;
            dialer.begin();
        }
    }

    fn pause(&self) -> Duration {
        let n = usize::try_from(self.attempts.saturating_sub(1)).unwrap_or(usize::MAX);
        self.backoff
            .get(n.min(self.backoff.len().saturating_sub(1)))
            .copied()
            .unwrap_or(Duration::ZERO)
    }
}

impl std::fmt::Debug for Dialed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => f.write_str("Pending"),
            Self::Link(_) => f.write_str("Link"),
            Self::Failed(why) => f.debug_tuple("Failed").field(why).finish(),
        }
    }
}
