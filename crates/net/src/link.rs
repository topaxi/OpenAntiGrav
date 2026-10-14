//! The transport, as the protocol sees it: two channel kinds and nothing else.
//!
//! [`Link`] is one peer's end of a connection: unreliable datagrams (each tick's
//! inputs and confirmed ticks) and one reliable, ordered stream of whole messages
//! (join, start, resend, end). It is polled, never awaited, so the game's frame
//! loop and the server's tick loop drive it without an async runtime of their own;
//! the WebTransport implementation (`crate::wt`) keeps its runtime on threads of
//! its own behind it.
//!
//! Besides WebTransport there are two implementations here: [`Loopback`], a pair of
//! ends in one process for tests that need no socket, and [`Shim`], which wraps any
//! link and delays and drops what crosses it, so a run on localhost can be judged at
//! a round trip and a loss rate a real network has.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use web_time::{Duration, Instant};

use crate::wire::{Control, Datagram, Malformed};

/// One end of a connection.
pub trait Link: Send {
    /// Sends a datagram. Never blocks; a datagram that cannot be sent is lost,
    /// which the protocol already covers.
    fn send_datagram(&mut self, bytes: Vec<u8>);
    /// The next datagram that has arrived, if any.
    fn recv_datagram(&mut self) -> Option<Vec<u8>>;
    /// Sends one message on the reliable stream.
    fn send_reliable(&mut self, bytes: Vec<u8>);
    /// The next whole message off the reliable stream, if any.
    fn recv_reliable(&mut self) -> Option<Vec<u8>>;
    /// Why the connection ended, once it has.
    fn closed(&self) -> Option<String>;
    /// The largest datagram this link carries.
    fn max_datagram(&self) -> usize;

    /// Sends a [`Control`] message.
    fn send_control(&mut self, message: &Control) {
        self.send_reliable(message.encode());
    }

    /// The next [`Control`] message, if one has arrived.
    fn recv_control(&mut self) -> Option<Result<Control, Malformed>> {
        self.recv_reliable().map(|bytes| Control::decode(&bytes))
    }

    /// Sends a [`Datagram`] message.
    fn send(&mut self, message: &Datagram) {
        self.send_datagram(message.encode());
    }

    /// The next [`Datagram`] message, if one has arrived.
    fn recv(&mut self) -> Option<Result<Datagram, Malformed>> {
        self.recv_datagram().map(|bytes| Datagram::decode(&bytes))
    }
}

impl<L: Link + ?Sized> Link for Box<L> {
    fn send_datagram(&mut self, bytes: Vec<u8>) {
        (**self).send_datagram(bytes);
    }
    fn recv_datagram(&mut self) -> Option<Vec<u8>> {
        (**self).recv_datagram()
    }
    fn send_reliable(&mut self, bytes: Vec<u8>) {
        (**self).send_reliable(bytes);
    }
    fn recv_reliable(&mut self) -> Option<Vec<u8>> {
        (**self).recv_reliable()
    }
    fn closed(&self) -> Option<String> {
        (**self).closed()
    }
    fn max_datagram(&self) -> usize {
        (**self).max_datagram()
    }
}

#[derive(Default)]
struct Queues {
    datagrams: VecDeque<Vec<u8>>,
    reliable: VecDeque<Vec<u8>>,
}

/// One end of an in-process connection. [`Loopback::pair`] makes both.
pub struct Loopback {
    inbox: Arc<Mutex<Queues>>,
    outbox: Arc<Mutex<Queues>>,
}

impl Loopback {
    /// Two connected ends.
    #[must_use]
    pub fn pair() -> (Self, Self) {
        let a = Arc::new(Mutex::new(Queues::default()));
        let b = Arc::new(Mutex::new(Queues::default()));
        (
            Self {
                inbox: a.clone(),
                outbox: b.clone(),
            },
            Self {
                inbox: b,
                outbox: a,
            },
        )
    }

    fn with<T>(queues: &Mutex<Queues>, f: impl FnOnce(&mut Queues) -> T) -> T {
        f(&mut queues
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner))
    }
}

impl Link for Loopback {
    fn send_datagram(&mut self, bytes: Vec<u8>) {
        Self::with(&self.outbox, |q| q.datagrams.push_back(bytes));
    }
    fn recv_datagram(&mut self) -> Option<Vec<u8>> {
        Self::with(&self.inbox, |q| q.datagrams.pop_front())
    }
    fn send_reliable(&mut self, bytes: Vec<u8>) {
        Self::with(&self.outbox, |q| q.reliable.push_back(bytes));
    }
    fn recv_reliable(&mut self) -> Option<Vec<u8>> {
        Self::with(&self.inbox, |q| q.reliable.pop_front())
    }
    fn closed(&self) -> Option<String> {
        None
    }
    fn max_datagram(&self) -> usize {
        1_200
    }
}

/// How a [`Shim`] degrades a link. **Chosen, not measured**: a test condition,
/// not a model of any network.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Conditions {
    /// Added to every message in each direction, so the round trip grows by twice
    /// this.
    pub one_way: Duration,
    /// The fraction of datagrams dropped, each direction, `0.0..=1.0`. The reliable
    /// stream is never dropped (QUIC retransmits it), only delayed.
    pub loss: f32,
}

impl Conditions {
    /// Parses `--net-shim`'s `RTT_MS[:LOSS_PERCENT]`, e.g. `100` or `100:2`.
    ///
    /// # Errors
    ///
    /// A message naming the part that is not a number.
    pub fn parse(spec: &str) -> Result<Self, String> {
        let (rtt, loss) = spec.split_once(':').unwrap_or((spec, "0"));
        let rtt: u64 = rtt
            .parse()
            .map_err(|_| format!("--net-shim: round trip {rtt:?} is not whole milliseconds"))?;
        let loss: f32 = loss
            .parse()
            .map_err(|_| format!("--net-shim: loss {loss:?} is not a percentage"))?;
        Ok(Self {
            one_way: Duration::from_micros(rtt * 500),
            loss: (loss / 100.0).clamp(0.0, 1.0),
        })
    }
}

/// A link with a round trip and a loss rate added on this end, in both
/// directions. A verification aid for running on localhost.
pub struct Shim<L: Link> {
    inner: L,
    conditions: Conditions,
    outgoing: VecDeque<(Instant, bool, Vec<u8>)>,
    incoming_datagrams: VecDeque<(Instant, Vec<u8>)>,
    incoming_reliable: VecDeque<(Instant, Vec<u8>)>,
    /// xorshift64, for the drops: the network's dice, never the simulation's.
    dice: u64,
}

impl<L: Link> Shim<L> {
    /// `inner` with `conditions` on it.
    pub fn new(inner: L, conditions: Conditions) -> Self {
        Self {
            inner,
            conditions,
            outgoing: VecDeque::new(),
            incoming_datagrams: VecDeque::new(),
            incoming_reliable: VecDeque::new(),
            dice: 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn dropped(&mut self) -> bool {
        if self.conditions.loss <= 0.0 {
            return false;
        }
        self.dice ^= self.dice << 13;
        self.dice ^= self.dice >> 7;
        self.dice ^= self.dice << 17;
        #[expect(
            clippy::cast_precision_loss,
            reason = "a probability, not a value the race reads"
        )]
        let roll = (self.dice >> 40) as f32 / (1u64 << 24) as f32;
        roll < self.conditions.loss
    }

    /// Moves what is due across: sends whose delay has passed, and receives from
    /// the inner link into the delay queues.
    fn pump(&mut self) {
        let now = Instant::now();
        while self.outgoing.front().is_some_and(|(due, _, _)| *due <= now) {
            if let Some((_, reliable, bytes)) = self.outgoing.pop_front() {
                if reliable {
                    self.inner.send_reliable(bytes);
                } else {
                    self.inner.send_datagram(bytes);
                }
            }
        }
        let due = now + self.conditions.one_way;
        while let Some(bytes) = self.inner.recv_datagram() {
            if !self.dropped() {
                self.incoming_datagrams.push_back((due, bytes));
            }
        }
        while let Some(bytes) = self.inner.recv_reliable() {
            self.incoming_reliable.push_back((due, bytes));
        }
    }
}

impl<L: Link> Link for Shim<L> {
    fn send_datagram(&mut self, bytes: Vec<u8>) {
        if !self.dropped() {
            let due = Instant::now() + self.conditions.one_way;
            self.outgoing.push_back((due, false, bytes));
        }
        self.pump();
    }
    fn recv_datagram(&mut self) -> Option<Vec<u8>> {
        self.pump();
        let now = Instant::now();
        if self
            .incoming_datagrams
            .front()
            .is_some_and(|(due, _)| *due <= now)
        {
            return self.incoming_datagrams.pop_front().map(|(_, bytes)| bytes);
        }
        None
    }
    fn send_reliable(&mut self, bytes: Vec<u8>) {
        let due = Instant::now() + self.conditions.one_way;
        self.outgoing.push_back((due, true, bytes));
        self.pump();
    }
    fn recv_reliable(&mut self) -> Option<Vec<u8>> {
        self.pump();
        let now = Instant::now();
        if self
            .incoming_reliable
            .front()
            .is_some_and(|(due, _)| *due <= now)
        {
            return self.incoming_reliable.pop_front().map(|(_, bytes)| bytes);
        }
        None
    }
    fn closed(&self) -> Option<String> {
        self.inner.closed()
    }
    fn max_datagram(&self) -> usize {
        self.inner.max_datagram()
    }
}

impl std::fmt::Debug for Loopback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Loopback").finish_non_exhaustive()
    }
}

impl<L: Link> std::fmt::Debug for Shim<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shim")
            .field("conditions", &self.conditions)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loopback_carries_both_kinds_in_order() {
        let (mut a, mut b) = Loopback::pair();
        a.send_datagram(vec![1]);
        a.send_reliable(vec![2]);
        a.send_reliable(vec![3]);
        assert_eq!(b.recv_datagram(), Some(vec![1]));
        assert_eq!(b.recv_reliable(), Some(vec![2]));
        assert_eq!(b.recv_reliable(), Some(vec![3]));
        assert_eq!(b.recv_reliable(), None);
        b.send_datagram(vec![4]);
        assert_eq!(a.recv_datagram(), Some(vec![4]));
    }

    #[test]
    fn a_shim_delays_and_drops() {
        let (a, mut b) = Loopback::pair();
        let mut a = Shim::new(
            a,
            Conditions {
                one_way: Duration::from_millis(30),
                loss: 0.0,
            },
        );
        let sent = Instant::now();
        a.send_reliable(vec![9]);
        while b.recv_reliable().is_none() {
            a.recv_reliable();
            assert!(sent.elapsed() < Duration::from_secs(2), "never arrived");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(sent.elapsed() >= Duration::from_millis(30));

        let (a, mut b) = Loopback::pair();
        let mut a = Shim::new(
            a,
            Conditions {
                one_way: Duration::ZERO,
                loss: 0.5,
            },
        );
        for i in 0..1000u32 {
            a.send_datagram(i.to_le_bytes().to_vec());
        }
        let mut arrived = 0;
        while b.recv_datagram().is_some() {
            arrived += 1;
        }
        assert!(
            (350..650).contains(&arrived),
            "{arrived} of 1000 at 50% loss"
        );
    }

    #[test]
    fn shim_conditions_parse() {
        assert_eq!(
            Conditions::parse("100:2"),
            Ok(Conditions {
                one_way: Duration::from_millis(50),
                loss: 0.02
            })
        );
        assert_eq!(Conditions::parse("0").map(|c| c.loss), Ok(0.0));
        assert!(Conditions::parse("fast").is_err());
    }
}
