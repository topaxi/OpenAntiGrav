//! A host's waiting room: connections that have said hello and are waiting for the head
//! count to be fixed.
//!
//! A fixed-size server ([`crate::host::Host`] as `oag-server` runs it) welcomes a hello at
//! once, because the race it carries already has its head count. A LAN host does not know
//! it until its player presses START, and the welcome carries the race, so the welcomes are
//! held here. Joiners wait on their reliable stream meanwhile; [`Lobby::take`] hands the
//! links over, in the order they arrived, to be seated with [`crate::host::Host::seat`].

use crate::link::Link;
use crate::wire::{Control, PROTOCOL};

struct Waiting {
    name: String,
    team: String,
    link: Box<dyn Link>,
}

/// The connections waiting to be seated.
#[derive(Default)]
pub struct Lobby {
    /// Connected, hello not read yet.
    joining: Vec<Box<dyn Link>>,
    waiting: Vec<Waiting>,
}

impl Lobby {
    /// A lobby with nobody in it.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A new connection. It waits when its hello arrives.
    pub fn join(&mut self, link: Box<dyn Link>) {
        self.joining.push(link);
    }

    /// Reads hellos, refuses a protocol this build does not speak, and forgets a
    /// connection that closed while it waited.
    pub fn poll(&mut self) {
        let mut still = Vec::new();
        for mut link in std::mem::take(&mut self.joining) {
            let Some(message) = link.recv_control() else {
                if link.closed().is_none() {
                    still.push(link);
                }
                continue;
            };
            let reason = match message {
                Ok(Control::Hello {
                    protocol,
                    name,
                    team,
                }) if protocol == PROTOCOL => {
                    self.waiting.push(Waiting { name, team, link });
                    continue;
                }
                Ok(Control::Hello { protocol, .. }) => {
                    format!("this host speaks protocol {PROTOCOL}, you speak {protocol}")
                }
                Ok(other) => format!("expected a hello, got {other:?}"),
                Err(e) => format!("{e}"),
            };
            log::warn!("net: refused a connection: {reason}");
            link.send_control(&Control::Reject { reason });
        }
        self.joining = still;
        self.waiting
            .retain(|waiting| waiting.link.closed().is_none());
    }

    /// Who is waiting, in the order they will be seated.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.waiting.iter().map(|w| w.name.clone()).collect()
    }

    /// How many are waiting.
    #[must_use]
    pub fn len(&self) -> usize {
        self.waiting.len()
    }

    /// Whether nobody is waiting.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.waiting.is_empty()
    }

    /// The team each waiting person asked for, in the order they will be seated.
    #[must_use]
    pub fn teams(&self) -> Vec<String> {
        self.waiting.iter().map(|w| w.team.clone()).collect()
    }

    /// Hands over everyone waiting, name and link, first arrival first.
    pub fn take(&mut self) -> Vec<(String, Box<dyn Link>)> {
        std::mem::take(&mut self.waiting)
            .into_iter()
            .map(|w| (w.name, w.link))
            .collect()
    }
}

impl std::fmt::Debug for Lobby {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lobby").finish_non_exhaustive()
    }
}
