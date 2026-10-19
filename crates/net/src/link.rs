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

use crate::stats::{PathStats, ShimStats};
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

    /// The transport's cumulative counters (round trip, bytes, losses), for the
    /// performance overlay. `None` on a link that keeps none; a wrapper must forward
    /// it, or the number is lost behind it.
    fn path(&self) -> Option<PathStats> {
        None
    }

    /// Sends a [`Control`] message.
    fn send_control(&mut self, message: &Control) {
        self.send_reliable(message.encode());
    }

    /// The next [`Control`] message, if one has arrived.
    fn recv_control(&mut self) -> Option<Result<Control, Malformed>> {
        self.recv_reliable().map(|bytes| Control::decode(&bytes))
    }

    /// Sends an online-lobby [`online::Message`](crate::wire::online::Message).
    fn send_lobby(&mut self, message: &crate::wire::online::Message) {
        self.send_reliable(message.encode());
    }

    /// The next online-lobby message, if one has arrived.
    fn recv_lobby(&mut self) -> Option<Result<crate::wire::online::Message, Malformed>> {
        self.recv_reliable()
            .map(|bytes| crate::wire::online::Message::decode(&bytes))
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
    fn path(&self) -> Option<PathStats> {
        (**self).path()
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
    /// Each message is delayed by a further amount drawn evenly from `0..jitter`, so
    /// datagrams can overtake one another. The reliable stream keeps its order (QUIC
    /// delivers it in order), taking the later of its own draw and the one before.
    pub jitter: Duration,
}

impl Conditions {
    /// Parses `--net-shim`'s `RTT_MS[:LOSS_PERCENT[:JITTER_MS]]`, e.g. `100`, `100:2` or
    /// `100:2:30`.
    ///
    /// # Errors
    ///
    /// A message naming the part that is not a number.
    pub fn parse(spec: &str) -> Result<Self, String> {
        let (rtt, rest) = spec.split_once(':').unwrap_or((spec, "0"));
        let (loss, jitter) = rest.split_once(':').unwrap_or((rest, "0"));
        let rtt: u64 = rtt
            .parse()
            .map_err(|_| format!("--net-shim: round trip {rtt:?} is not whole milliseconds"))?;
        let loss: f32 = loss
            .parse()
            .map_err(|_| format!("--net-shim: loss {loss:?} is not a percentage"))?;
        let jitter: u64 = jitter
            .parse()
            .map_err(|_| format!("--net-shim: jitter {jitter:?} is not whole milliseconds"))?;
        Ok(Self {
            one_way: Duration::from_micros(rtt * 500),
            loss: (loss / 100.0).clamp(0.0, 1.0),
            jitter: Duration::from_millis(jitter),
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
    /// Datagrams dropped so far, both directions.
    drops: u64,
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
            drops: 0,
        }
    }

    /// The next roll of the network's dice, `0.0..1.0`.
    fn roll(&mut self) -> f32 {
        self.dice ^= self.dice << 13;
        self.dice ^= self.dice >> 7;
        self.dice ^= self.dice << 17;
        #[expect(
            clippy::cast_precision_loss,
            reason = "a probability, not a value the race reads"
        )]
        let roll = (self.dice >> 40) as f32 / (1u64 << 24) as f32;
        roll
    }

    /// When a message sent or received at `now` is due: the one-way delay and a draw of the
    /// jitter.
    fn due(&mut self, now: Instant) -> Instant {
        let extra = if self.conditions.jitter.is_zero() {
            Duration::ZERO
        } else {
            self.conditions.jitter.mul_f32(self.roll())
        };
        now + self.conditions.one_way + extra
    }

    fn dropped(&mut self) -> bool {
        if self.conditions.loss <= 0.0 {
            return false;
        }
        let dropped = self.roll() < self.conditions.loss;
        self.drops += u64::from(dropped);
        dropped
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
        while let Some(bytes) = self.inner.recv_datagram() {
            if !self.dropped() {
                let due = self.due(now);
                let at = self.incoming_datagrams.partition_point(|(d, _)| *d <= due);
                self.incoming_datagrams.insert(at, (due, bytes));
            }
        }
        while let Some(bytes) = self.inner.recv_reliable() {
            let behind = self.incoming_reliable.back().map(|(d, _)| *d);
            let due = self.due(now).max(behind.unwrap_or(now));
            self.incoming_reliable.push_back((due, bytes));
        }
    }
}

impl<L: Link> Link for Shim<L> {
    fn send_datagram(&mut self, bytes: Vec<u8>) {
        if !self.dropped() {
            let due = self.due(Instant::now());
            let at = self.outgoing.partition_point(|(d, _, _)| *d <= due);
            self.outgoing.insert(at, (due, false, bytes));
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
        let behind = self
            .outgoing
            .iter()
            .rev()
            .find(|(_, reliable, _)| *reliable)
            .map(|(d, _, _)| *d);
        let now = Instant::now();
        let due = self.due(now).max(behind.unwrap_or(now));
        let at = self.outgoing.partition_point(|(d, _, _)| *d <= due);
        self.outgoing.insert(at, (due, true, bytes));
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
    /// The inner link's counters with the shim's conditions on them: the round trip
    /// QUIC reports is the socket's alone, the delay being added above it.
    fn path(&self) -> Option<PathStats> {
        let mut path = self.inner.path().unwrap_or_default();
        path.shim = Some(ShimStats {
            conditions: self.conditions,
            dropped: self.drops,
        });
        Some(path)
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

    /// A link that keeps counters, to prove a wrapper forwards them.
    struct Counted(Loopback);
    impl Link for Counted {
        fn send_datagram(&mut self, bytes: Vec<u8>) {
            self.0.send_datagram(bytes);
        }
        fn recv_datagram(&mut self) -> Option<Vec<u8>> {
            self.0.recv_datagram()
        }
        fn send_reliable(&mut self, bytes: Vec<u8>) {
            self.0.send_reliable(bytes);
        }
        fn recv_reliable(&mut self) -> Option<Vec<u8>> {
            self.0.recv_reliable()
        }
        fn closed(&self) -> Option<String> {
            None
        }
        fn max_datagram(&self) -> usize {
            1_200
        }
        fn path(&self) -> Option<PathStats> {
            Some(PathStats {
                rtt: Some(Duration::from_millis(7)),
                ..PathStats::default()
            })
        }
    }

    #[test]
    fn a_boxed_link_and_a_shim_forward_the_path_counters() {
        let (a, _b) = Loopback::pair();
        let boxed: Box<dyn Link> = Box::new(Counted(a));
        assert_eq!(
            boxed.path().and_then(|p| p.rtt),
            Some(Duration::from_millis(7))
        );
        let conditions = Conditions {
            one_way: Duration::from_millis(50),
            loss: 0.5,
            ..Conditions::default()
        };
        let shim = Shim::new(boxed, conditions);
        let path = shim.path().expect("forwarded");
        assert_eq!(path.rtt, Some(Duration::from_millis(7)));
        assert_eq!(path.shim.map(|s| s.conditions), Some(conditions));
        let (plain, _) = Loopback::pair();
        assert!(plain.path().is_none(), "a link with no counters says so");
    }

    #[test]
    fn a_shim_counts_what_it_drops() {
        let (a, _b) = Loopback::pair();
        let mut shim = Shim::new(
            a,
            Conditions {
                one_way: Duration::ZERO,
                loss: 1.0,
                ..Conditions::default()
            },
        );
        for _ in 0..5 {
            shim.send_datagram(vec![1]);
        }
        assert_eq!(shim.path().and_then(|p| p.shim).map(|s| s.dropped), Some(5));
    }

    #[test]
    fn a_shim_delays_and_drops() {
        let (a, mut b) = Loopback::pair();
        let mut a = Shim::new(
            a,
            Conditions {
                one_way: Duration::from_millis(30),
                loss: 0.0,
                ..Conditions::default()
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
                ..Conditions::default()
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
                loss: 0.02,
                ..Conditions::default()
            })
        );
        assert_eq!(
            Conditions::parse("100:2:30").map(|c| c.jitter),
            Ok(Duration::from_millis(30))
        );
        assert_eq!(Conditions::parse("0").map(|c| c.loss), Ok(0.0));
        assert!(Conditions::parse("fast").is_err());
    }

    #[test]
    fn jitter_lets_datagrams_overtake_and_loses_none() {
        let (a, mut b) = Loopback::pair();
        let mut a = Shim::new(
            a,
            Conditions {
                jitter: Duration::from_millis(20),
                ..Conditions::default()
            },
        );
        for i in 0..200u8 {
            a.send_datagram(vec![i]);
        }
        let sent = Instant::now();
        let mut order = Vec::new();
        while order.len() < 200 {
            a.recv_datagram();
            while let Some(bytes) = b.recv_datagram() {
                order.push(bytes[0]);
            }
            assert!(
                sent.elapsed() < Duration::from_secs(2),
                "{} arrived",
                order.len()
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        let mut sorted = order.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..200u8).collect::<Vec<_>>(), "none lost");
        assert_ne!(order, sorted, "some overtook another");
    }

    #[test]
    fn jitter_never_reorders_the_reliable_stream() {
        let (a, mut b) = Loopback::pair();
        let mut a = Shim::new(
            a,
            Conditions {
                jitter: Duration::from_millis(20),
                ..Conditions::default()
            },
        );
        for i in 0..100u8 {
            a.send_reliable(vec![i]);
        }
        let sent = Instant::now();
        let mut order = Vec::new();
        while order.len() < 100 {
            a.recv_reliable();
            while let Some(bytes) = b.recv_reliable() {
                order.push(bytes[0]);
            }
            assert!(sent.elapsed() < Duration::from_secs(2));
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(order, (0..100u8).collect::<Vec<_>>());
    }
}
