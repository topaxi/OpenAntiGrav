//! [`Link`] over WebTransport (HTTP/3 over QUIC), native: the server's listener,
//! the client's connect, and the certificate a listener presents.
//!
//! **One certificate per run, pinned by hash.** A listener mints a self-signed
//! ECDSA P-256 certificate valid for 13 days (chosen: a browser's
//! `serverCertificateHashes` refuses one valid for more than 14), and a client
//! accepts the server whose certificate has the SHA-256 it was told. A wrong hash
//! fails the handshake with the TLS error, within milliseconds. Self-signed
//! certificates on a LAN are the maintainer's decision (`docs/networking/README.md`);
//! a public server would present a CA-signed one instead (not built).
//!
//! The crate is `web-transport-quinn` on rustls' `ring` provider, chosen in a
//! spike against `wtransport` (both worked on this workspace's MSRV;
//! [ADR-0070](../../../docs/architecture/adr/0070-the-network-transport-is-webtransport.md)).
//! The async half runs on a tokio runtime of its own, two worker threads, shared
//! by every link in the process; a [`WtLink`] is a set of channels to it.

use std::net::SocketAddr;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{Context, Result, anyhow};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use web_time::Duration;

use crate::link::Link;

/// How long a client waits for the handshake. Chosen.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// The largest reliable message accepted, so a broken peer cannot ask for 4 GiB.
const MAX_RELIABLE: usize = 1 << 20;

fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("oag-net")
            .enable_all()
            .build()
            .expect("the network runtime starts")
    })
}

/// A listener's certificate and key, and the hash a client pins.
pub struct Identity {
    chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
    hash: [u8; 32],
}

impl Identity {
    /// A fresh self-signed certificate for `localhost`, `127.0.0.1` and `::1`, and
    /// any extra names given (a LAN address, a host name).
    ///
    /// # Errors
    ///
    /// The certificate could not be generated.
    pub fn self_signed(extra_names: &[String]) -> Result<Self> {
        let key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256)?;
        let mut names = vec!["localhost".to_string(), "127.0.0.1".into(), "::1".into()];
        names.extend(extra_names.iter().cloned());
        let mut params = rcgen::CertificateParams::new(names)?;
        let now = std::time::SystemTime::now();
        params.not_before = (now - std::time::Duration::from_secs(3_600)).into();
        params.not_after = (now + std::time::Duration::from_secs(13 * 86_400)).into();
        let cert = params.self_signed(&key)?;
        let der = cert.der().clone();
        let digest = ring::digest::digest(&ring::digest::SHA256, der.as_ref());
        let mut hash = [0; 32];
        hash.copy_from_slice(digest.as_ref());
        Ok(Self {
            chain: vec![der],
            key: PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der())),
            hash,
        })
    }

    /// The SHA-256 of the certificate, what a client pins.
    #[must_use]
    pub fn hash(&self) -> [u8; 32] {
        self.hash
    }

    /// [`Self::hash`] as 64 lowercase hex digits, as `--cert-hash` takes it.
    #[must_use]
    pub fn hash_hex(&self) -> String {
        hex(&self.hash)
    }
}

/// Bytes as lowercase hex.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Parses 64 hex digits (colons allowed between bytes) into a certificate hash.
///
/// # Errors
///
/// Anything that is not 32 bytes of hex.
pub fn parse_hash(text: &str) -> Result<[u8; 32]> {
    let digits: String = text.chars().filter(|c| *c != ':').collect();
    if digits.len() != 64 {
        return Err(anyhow!(
            "a certificate hash is 64 hex digits, got {}",
            digits.len()
        ));
    }
    let mut hash = [0; 32];
    for (i, byte) in hash.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&digits[i * 2..i * 2 + 2], 16)
            .with_context(|| format!("certificate hash: {text:?} is not hex"))?;
    }
    Ok(hash)
}

/// How long a link may go without sending before it sends a keep-alive. Chosen: well inside
/// QUIC's default idle timeout.
const KEEP_ALIVE: std::time::Duration = std::time::Duration::from_secs(5);

enum Out {
    Datagram(Vec<u8>),
    Reliable(Vec<u8>),
}

/// One end of a WebTransport session.
pub struct WtLink {
    datagrams: mpsc::Receiver<Vec<u8>>,
    reliable: mpsc::Receiver<Vec<u8>>,
    out: tokio::sync::mpsc::UnboundedSender<Out>,
    closed: Arc<Mutex<Option<String>>>,
    max_datagram: usize,
    peer: SocketAddr,
}

impl WtLink {
    /// The peer's address.
    #[must_use]
    pub fn peer(&self) -> SocketAddr {
        self.peer
    }

    /// Wires a session and its reliable stream to channels, on the runtime.
    fn spawn(
        session: web_transport_quinn::Session,
        send: web_transport_quinn::SendStream,
        recv: web_transport_quinn::RecvStream,
        peer: SocketAddr,
    ) -> Self {
        let closed = Arc::new(Mutex::new(None));
        let close = {
            let closed = closed.clone();
            move |why: String| {
                let mut slot = closed
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if slot.is_none() {
                    *slot = Some(why);
                }
            }
        };
        let (datagram_in, datagrams) = mpsc::channel();
        let (reliable_in, reliable) = mpsc::channel();
        let (out, mut outgoing) = tokio::sync::mpsc::unbounded_channel::<Out>();
        let max_datagram = session.max_datagram_size();
        let rt = runtime();

        let reader = session.clone();
        let on_close = close.clone();
        rt.spawn(async move {
            loop {
                match reader.read_datagram().await {
                    // The keep-alive of the other end's writer.
                    Ok(bytes) if bytes.is_empty() => {}
                    Ok(bytes) => {
                        if datagram_in.send(bytes.to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        on_close(format!("{e}"));
                        break;
                    }
                }
            }
        });

        let on_close = close.clone();
        rt.spawn(async move {
            let mut recv = recv;
            loop {
                let mut len = [0u8; 4];
                if let Err(e) = recv.read_exact(&mut len).await {
                    on_close(format!("reliable stream: {e}"));
                    break;
                }
                let len = u32::from_le_bytes(len) as usize;
                if len > MAX_RELIABLE {
                    on_close(format!("reliable message of {len} bytes refused"));
                    break;
                }
                let mut bytes = vec![0; len];
                if let Err(e) = recv.read_exact(&mut bytes).await {
                    on_close(format!("reliable stream: {e}"));
                    break;
                }
                if reliable_in.send(bytes).is_err() {
                    break;
                }
            }
        });

        let writer = session;
        rt.spawn(async move {
            let mut send = send;
            loop {
                // A lobby can sit for minutes with nothing to say, and QUIC closes a quiet
                // connection: an empty datagram is the keep-alive (the other end drops it).
                let message = tokio::select! {
                    message = outgoing.recv() => message,
                    () = tokio::time::sleep(KEEP_ALIVE) => {
                        let _ = writer.send_datagram(Vec::new().into());
                        continue;
                    }
                };
                let Some(message) = message else { break };
                match message {
                    // A datagram that will not go (too large, congestion) is lost,
                    // which the protocol's redundancy covers.
                    Out::Datagram(bytes) => {
                        let _ = writer.send_datagram(bytes.into());
                    }
                    Out::Reliable(bytes) => {
                        let len = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
                        if let Err(e) = send.write_all(&len.to_le_bytes()).await {
                            close(format!("reliable stream: {e}"));
                            break;
                        }
                        if let Err(e) = send.write_all(&bytes).await {
                            close(format!("reliable stream: {e}"));
                            break;
                        }
                    }
                }
            }
            writer.close(0, b"bye");
        });

        Self {
            datagrams,
            reliable,
            out,
            closed,
            max_datagram,
            peer,
        }
    }
}

impl Link for WtLink {
    fn send_datagram(&mut self, bytes: Vec<u8>) {
        let _ = self.out.send(Out::Datagram(bytes));
    }
    fn recv_datagram(&mut self) -> Option<Vec<u8>> {
        self.datagrams.try_recv().ok()
    }
    fn send_reliable(&mut self, bytes: Vec<u8>) {
        let _ = self.out.send(Out::Reliable(bytes));
    }
    fn recv_reliable(&mut self) -> Option<Vec<u8>> {
        self.reliable.try_recv().ok()
    }
    fn closed(&self) -> Option<String> {
        self.closed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
    fn max_datagram(&self) -> usize {
        self.max_datagram
    }
}

/// A server's listening socket. New sessions arrive through [`Self::accept`].
pub struct Listener {
    incoming: mpsc::Receiver<WtLink>,
    local: SocketAddr,
}

impl Listener {
    /// Listens on `addr` with `identity`'s certificate.
    ///
    /// # Errors
    ///
    /// The address could not be bound, or the certificate was refused.
    pub fn bind(addr: SocketAddr, identity: &Identity) -> Result<Self> {
        let rt = runtime();
        let _guard = rt.enter();
        let mut server = web_transport_quinn::ServerBuilder::new()
            .with_addr(addr)
            .with_certificate(identity.chain.clone(), identity.key.clone_key())
            .with_context(|| format!("listening on {addr}"))?;
        let local = server.local_addr()?;
        let (found, incoming) = mpsc::channel();
        rt.spawn(async move {
            while let Some(request) = server.accept().await {
                let found = found.clone();
                tokio::spawn(async move {
                    let peer = request.conn().remote_address();
                    let session = match request.ok().await {
                        Ok(session) => session,
                        Err(e) => {
                            log::warn!("net: {peer}: session refused: {e}");
                            return;
                        }
                    };
                    match session.accept_bi().await {
                        Ok((send, recv)) => {
                            let _ = found.send(WtLink::spawn(session, send, recv, peer));
                        }
                        Err(e) => log::warn!("net: {peer}: no reliable stream: {e}"),
                    }
                });
            }
        });
        Ok(Self { incoming, local })
    }

    /// The address actually bound (a port of 0 resolved).
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.local
    }

    /// A session that has connected since the last call, if any.
    pub fn accept(&mut self) -> Option<WtLink> {
        self.incoming.try_recv().ok()
    }
}

/// Connects to the server at `addr` (`host:port`), accepting only the certificate
/// whose SHA-256 is `hash`. Blocks for at most five seconds.
///
/// # Errors
///
/// The handshake failed (a wrong hash names the TLS error) or timed out.
pub fn connect(addr: &str, hash: [u8; 32]) -> Result<WtLink> {
    let url = url::Url::parse(&format!("https://{addr}/oag"))
        .with_context(|| format!("--connect {addr:?} is not host:port"))?;
    let rt = runtime();
    rt.block_on(async move {
        let client = web_transport_quinn::ClientBuilder::new()
            .with_server_certificate_hashes(vec![hash.to_vec()])?;
        let session = tokio::time::timeout(CONNECT_TIMEOUT, client.connect(url))
            .await
            .map_err(|_| anyhow!("connecting to {addr}: no answer in {CONNECT_TIMEOUT:?}"))?
            .with_context(|| format!("connecting to {addr}"))?;
        let peer = session.remote_address();
        let (send, recv) = session.open_bi().await?;
        Ok(WtLink::spawn(session, send, recv, peer))
    })
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Identity")
            .field("hash", &hex(&self.hash))
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for WtLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WtLink")
            .field("peer", &self.peer)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for Listener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Listener")
            .field("local", &self.local)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::{Control, PROTOCOL};

    fn wait<T>(mut poll: impl FnMut() -> Option<T>) -> T {
        let start = std::time::Instant::now();
        loop {
            if let Some(value) = poll() {
                return value;
            }
            assert!(
                start.elapsed() < std::time::Duration::from_secs(5),
                "timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    /// The spike's claim, kept: both channel kinds both ways over a real socket on
    /// localhost, the certificate pinned, and a wrong pin refused by the handshake
    /// rather than by a timeout.
    #[test]
    fn a_session_carries_both_kinds_and_a_wrong_pin_is_refused() {
        let identity = Identity::self_signed(&[]).expect("certificate");
        let mut listener =
            Listener::bind("127.0.0.1:0".parse().expect("addr"), &identity).expect("bind");
        let addr = listener.local_addr().to_string();

        let wrong = connect(&addr, [7; 32]).expect_err("a wrong pin connected");
        let why = format!("{wrong:#}");
        assert!(why.contains("certificate"), "{why}");

        let mut client = connect(&addr, identity.hash()).expect("connect");
        client.send_control(&Control::Hello {
            protocol: PROTOCOL,
            name: "test".into(),
        });
        let mut server = wait(|| listener.accept());
        let hello = wait(|| server.recv_control()).expect("parses");
        assert!(matches!(
            hello,
            Control::Hello {
                protocol: PROTOCOL,
                ..
            }
        ));
        server.send_reliable(vec![5; 3000]);
        assert_eq!(wait(|| client.recv_reliable()), vec![5; 3000]);
        client.send_datagram(vec![1, 2, 3]);
        assert_eq!(wait(|| server.recv_datagram()), vec![1, 2, 3]);
        server.send_datagram(vec![4]);
        assert_eq!(wait(|| client.recv_datagram()), vec![4]);
        assert!(client.max_datagram() >= 1_200, "{}", client.max_datagram());
    }

    #[test]
    fn a_hash_round_trips_through_hex() {
        let hash: [u8; 32] = std::array::from_fn(|i| i as u8 * 7);
        assert_eq!(parse_hash(&hex(&hash)).expect("parses"), hash);
        let colons: Vec<String> = hash.iter().map(|b| format!("{b:02X}")).collect();
        assert_eq!(parse_hash(&colons.join(":")).expect("parses"), hash);
        assert!(parse_hash("abc").is_err());
    }
}
