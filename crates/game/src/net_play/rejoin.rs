//! The way back to a seat: where the race is and who this person is in it, kept by a
//! [`super::NetPlay`] so it can dial again after the link closed ([`oag_net::redial`]), and
//! the dialler that does it off the frame.

use oag_net::link::Link;
use oag_net::redial::{Dialed, Dialer};

/// Where a race is and which seat is this person's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejoin {
    /// The race server, `host:port`.
    pub addr: String,
    /// The server certificate's SHA-256.
    pub cert_hash: [u8; 32],
    /// The lobby room the race came from, `0` when it did not come from one.
    pub room: u32,
    /// The team asked for.
    pub team: String,
    /// The name the seat is under.
    pub name: String,
    /// The seat token the server reseats this person on.
    pub token: String,
}

/// Connects with [`oag_net::wt::connect`] on a thread of its own, one attempt at a time.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct WtDialer {
    addr: String,
    cert_hash: [u8; 32],
    shim: Option<oag_net::link::Conditions>,
    task: Option<oag_thread::Task<anyhow::Result<oag_net::wt::WtLink>>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl WtDialer {
    /// A dialler for `rejoin`'s server, behind `shim` when `--net-shim` asked for one.
    #[must_use]
    pub fn new(rejoin: &Rejoin, shim: Option<oag_net::link::Conditions>) -> Self {
        Self {
            addr: rejoin.addr.clone(),
            cert_hash: rejoin.cert_hash,
            shim,
            task: None,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Dialer for WtDialer {
    fn begin(&mut self) {
        let (addr, hash) = (self.addr.clone(), self.cert_hash);
        self.task = Some(oag_thread::Task::spawn("net-redial", move || {
            oag_net::wt::connect(&addr, hash)
        }));
    }

    fn poll(&mut self) -> Dialed {
        let Some(task) = self.task.as_mut() else {
            return Dialed::Pending;
        };
        if !task.is_finished() {
            return Dialed::Pending;
        }
        let done = task.join();
        self.task = None;
        match done {
            Some(Ok(Ok(link))) => Dialed::Link(match self.shim {
                Some(conditions) => Box::new(oag_net::link::Shim::new(link, conditions)),
                None => Box::new(link) as Box<dyn Link>,
            }),
            Some(Ok(Err(e))) => Dialed::Failed(format!("{e:#}")),
            _ => Dialed::Failed("the connection thread stopped".to_string()),
        }
    }
}
