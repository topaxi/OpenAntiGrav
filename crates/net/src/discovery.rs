//! Finding a LAN host: a query on UDP and the host's answer.
//!
//! **Query and reply, not an announcement.** Only the host binds [`PORT`]; a
//! browser asks from an ephemeral port, so a host and a joiner on one machine do
//! not fight over the socket. The query goes to the limited broadcast address
//! and to loopback (a machine with no broadcast route still finds a host on
//! itself), or, for a typed address, straight to that address. The host answers
//! the sender with an [`Advert`]: the game port, the certificate hash a client
//! pins, the title and the circuit. The host's address is the answer's source
//! address, so the host never has to guess its own.
//!
//! LAN hosts are trusted (decision 3 of `docs/networking/README.md`), which is
//! why the certificate hash travels in the clear. Everything here is **chosen,
//! not measured**: the port, the cadence, the message bytes.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

use web_time::{Duration, Instant};

/// The UDP port a host answers queries on. Chosen: unassigned, next to the game's.
pub const PORT: u16 = 4434;

/// How often a browser asks again. Chosen.
pub const ASK_EVERY: Duration = Duration::from_secs(1);

/// How long a host that has stopped answering stays in the list. Chosen: three asks.
pub const FORGET_AFTER: Duration = Duration::from_secs(3);

const QUERY: [u8; 4] = *b"OAGQ";
const REPLY: [u8; 4] = *b"OAGA";

/// What a host says about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advert {
    /// [`crate::wire::PROTOCOL`] the host speaks.
    pub protocol: u16,
    /// The game port (WebTransport) on the host's address.
    pub port: u16,
    /// The SHA-256 of the host's certificate, which a client pins.
    pub hash: [u8; 32],
    /// What the host calls itself.
    pub name: String,
    /// The title it races (`Title::name`).
    pub title: String,
    /// The circuit, as the host's menu names it (a display label, not an id).
    pub track: String,
    /// People in the lobby now, the host included.
    pub people: u8,
}

fn put_str(out: &mut Vec<u8>, text: &str) {
    let bytes = text.as_bytes();
    let len = bytes.len().min(48);
    out.push(u8::try_from(len).unwrap_or(48));
    out.extend_from_slice(&bytes[..len]);
}

impl Advert {
    /// The reply's bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = REPLY.to_vec();
        out.extend_from_slice(&self.protocol.to_le_bytes());
        out.extend_from_slice(&self.port.to_le_bytes());
        out.extend_from_slice(&self.hash);
        out.push(self.people);
        put_str(&mut out, &self.name);
        put_str(&mut out, &self.title);
        put_str(&mut out, &self.track);
        out
    }

    /// Parses a reply; `None` for anything that is not one.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let rest = bytes.strip_prefix(&REPLY)?;
        let (head, mut rest) = rest.split_at_checked(2 + 2 + 32 + 1)?;
        let protocol = u16::from_le_bytes([head[0], head[1]]);
        let port = u16::from_le_bytes([head[2], head[3]]);
        let mut hash = [0; 32];
        hash.copy_from_slice(&head[4..36]);
        let people = head[36];
        let mut take = || -> Option<String> {
            let (&len, tail) = rest.split_first()?;
            let (text, tail) = tail.split_at_checked(usize::from(len))?;
            rest = tail;
            Some(String::from_utf8_lossy(text).into_owned())
        };
        let name = take()?;
        let title = take()?;
        let track = take()?;
        Some(Self {
            protocol,
            port,
            hash,
            name,
            title,
            track,
            people,
        })
    }
}

/// A host's side: answers queries with the advert it is given.
#[derive(Debug)]
pub struct Responder {
    socket: UdpSocket,
}

impl Responder {
    /// Listens on [`PORT`], every interface.
    ///
    /// # Errors
    ///
    /// The port is taken (another host on this machine).
    pub fn bind() -> io::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, PORT))?;
        socket.set_nonblocking(true)?;
        Ok(Self { socket })
    }

    /// Answers every query that has arrived since the last call; how many.
    pub fn answer(&self, advert: &Advert) -> usize {
        let reply = advert.encode();
        let mut buf = [0u8; 64];
        let mut answered = 0;
        while let Ok((len, from)) = self.socket.recv_from(&mut buf) {
            if buf[..len] == QUERY {
                let _ = self.socket.send_to(&reply, from);
                answered += 1;
            }
        }
        answered
    }
}

/// A host a browser has heard from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// Where to connect: the answer's source address and the advert's game port.
    pub addr: SocketAddr,
    /// What it said.
    pub advert: Advert,
}

/// A joiner's side: asks, and keeps the list of who answered.
#[derive(Debug)]
pub struct Browser {
    socket: UdpSocket,
    found: Vec<(Found, Instant)>,
    asked: Option<Instant>,
    /// Hosts to ask directly (typed addresses), besides the broadcast.
    direct: Vec<SocketAddr>,
    /// The port queries are sent to; [`PORT`] except in a test.
    port: u16,
}

impl Browser {
    /// A browser on an ephemeral port.
    ///
    /// # Errors
    ///
    /// No UDP socket could be made.
    pub fn new() -> io::Result<Self> {
        Self::on(PORT)
    }

    /// A browser asking `port` instead of [`PORT`].
    ///
    /// # Errors
    ///
    /// No UDP socket could be made.
    pub fn on(port: u16) -> io::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        socket.set_nonblocking(true)?;
        // A machine with no broadcast route refuses this on send, not here.
        let _ = socket.set_broadcast(true);
        Ok(Self {
            socket,
            found: Vec::new(),
            asked: None,
            direct: Vec::new(),
            port,
        })
    }

    /// Also asks `ip` itself on every round: a host the broadcast does not reach (another
    /// subnet, a wired-to-wireless bridge that drops it).
    pub fn ask_also(&mut self, ip: IpAddr) {
        let target = SocketAddr::new(ip, self.port);
        if !self.direct.contains(&target) {
            self.direct.push(target);
        }
        self.asked = None;
    }

    /// Records an answer. A host answers once for each leg of a round that reached it (the
    /// broadcast and the loopback or a typed address), each from a different source address:
    /// the same certificate and port are one host, listed once at its first address that is
    /// not loopback.
    fn hear(&mut self, from: SocketAddr, advert: Advert, now: Instant) {
        let found = Found {
            addr: SocketAddr::new(from.ip(), advert.port),
            advert,
        };
        let same =
            |f: &Found| f.advert.hash == found.advert.hash && f.advert.port == found.advert.port;
        match self.found.iter_mut().find(|(f, _)| same(f)) {
            Some((known, at)) => {
                let better = known.addr.ip().is_loopback() && !found.addr.ip().is_loopback();
                if better {
                    known.addr = found.addr;
                }
                known.advert = found.advert;
                *at = now;
            }
            None => self.found.push((found, now)),
        }
    }

    /// Asks when a round is due, reads every answer, forgets hosts that went quiet; the
    /// hosts now known, in the order they were first heard. Call once a frame.
    pub fn poll(&mut self) -> Vec<Found> {
        let now = Instant::now();
        if self
            .asked
            .is_none_or(|at| now.duration_since(at) >= ASK_EVERY)
        {
            self.asked = Some(now);
            let targets = [
                SocketAddr::new(Ipv4Addr::BROADCAST.into(), self.port),
                SocketAddr::new(Ipv4Addr::LOCALHOST.into(), self.port),
            ];
            for target in targets.iter().chain(&self.direct) {
                // An unreachable network is an empty list, not an error to show.
                let _ = self.socket.send_to(&QUERY, target);
            }
        }
        let mut buf = [0u8; 256];
        while let Ok((len, from)) = self.socket.recv_from(&mut buf) {
            let Some(advert) = Advert::decode(&buf[..len]) else {
                continue;
            };
            self.hear(from, advert, now);
        }
        self.found
            .retain(|(_, at)| now.duration_since(*at) < FORGET_AFTER);
        self.found.iter().map(|(f, _)| f.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn advert() -> Advert {
        Advert {
            protocol: 1,
            port: 5555,
            hash: [7; 32],
            name: "Ada".into(),
            title: "pulse".into(),
            track: "Vineta K".into(),
            people: 2,
        }
    }

    #[test]
    fn an_advert_survives_its_bytes_and_a_stranger_is_refused() {
        let advert = advert();
        assert_eq!(Advert::decode(&advert.encode()), Some(advert.clone()));
        let mut bytes = advert.encode();
        bytes.truncate(bytes.len() - 3);
        assert_eq!(Advert::decode(&bytes), None, "a cut-short reply");
        assert_eq!(Advert::decode(b"GET / HTTP/1.1"), None);
    }

    /// A host that answers on two legs from two addresses is one host, at the address that is
    /// not loopback.
    #[test]
    fn one_host_heard_on_two_addresses_is_listed_once_at_the_lan_one() {
        let mut browser = Browser::on(1).expect("browser");
        let now = Instant::now();
        let lan = SocketAddr::new(Ipv4Addr::new(192, 168, 1, 20).into(), PORT);
        let local = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), PORT);
        browser.hear(local, advert(), now);
        browser.hear(lan, advert(), now);
        browser.hear(local, advert(), now);
        assert_eq!(browser.found.len(), 1);
        assert_eq!(browser.found[0].0.addr, SocketAddr::new(lan.ip(), 5555));
        let mut other = advert();
        other.hash = [8; 32];
        browser.hear(lan, other, now);
        assert_eq!(
            browser.found.len(),
            2,
            "another certificate is another host"
        );
    }

    /// The real sockets on loopback: a responder on a test port, a browser that finds it
    /// through the loopback leg of its round (no broadcast route is needed), and the
    /// address it reports being the answer's source plus the advert's game port.
    #[test]
    fn a_browser_finds_a_responder_on_loopback() {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).expect("a free port");
        let port = socket.local_addr().expect("bound").port();
        socket.set_nonblocking(true).expect("nonblocking");
        let responder = Responder { socket };
        let mut browser = Browser::on(port).expect("browser");
        let started = Instant::now();
        let found = loop {
            responder.answer(&advert());
            let list = browser.poll();
            if let Some(first) = list.into_iter().next() {
                break first;
            }
            assert!(started.elapsed() < Duration::from_secs(5), "never found");
            std::thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(
            found.addr,
            SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 5555)
        );
        assert_eq!(found.advert, advert());
    }
}
