//! A client's eye on a player host: the race it was issued, the records it was sent, the host's
//! signed checkpoints of them, and the inputs it confirmed for this client's seat. What it
//! finds becomes a [`Proof`] for the lobby.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::{Chain, Checkpoint, Claim, Proof, input_statement, params_hash, same};
use crate::online::hostkey::PlayerKey;
use crate::wire::{Held, RaceParams, TickRecord};

/// Received checkpoints held until the client's confirmed records reach them, at most.
const MAX_PENDING_CHECKPOINTS: usize = 64;

/// Chain values kept for checkpoints that arrive after the records they cover.
const MAX_KEPT_CHAINS: usize = 64;

/// Own inputs remembered until their tick is confirmed, at most: far past the
/// client's own window.
const MAX_SENT: usize = 1024;

/// What the watch has seen, for a test or a log line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WatchStats {
    /// Checkpoints whose signature and chain matched this client's records.
    pub checkpoints_verified: u32,
    /// Checkpoints whose signature was not the host's: dropped.
    pub checkpoints_unsigned: u32,
    /// Checkpoints signed by the host over records other than the ones this client was sent:
    /// evidence only, since the client cannot prove what it was sent.
    pub checkpoints_differ: u32,
    /// Input datagrams this client signed.
    pub inputs_signed: u32,
    /// Proofs built for the lobby.
    pub reports: u32,
}

/// One client's watch on its race's host.
pub struct Watch {
    params: [u8; 32],
    host: [u8; 32],
    signer: Option<Arc<PlayerKey>>,
    seat: u8,
    fingerprint: u64,
    chain: Chain,
    /// Every confirmed record from tick 0, kept only when the host signs checkpoints.
    records: Vec<TickRecord>,
    kept: BTreeMap<u32, [u8; 32]>,
    pending: BTreeMap<u32, Checkpoint>,
    sent: BTreeMap<u32, Held>,
    /// The first tick this client sent an input for: before it (a game that came back to its
    /// race in a new process), what was sent is unknown and nothing is judged.
    first_sent: Option<u32>,
    last_own: Held,
    mismatch: Option<u32>,
    forged: Option<u32>,
    report: Option<Proof>,
    reported: bool,
    stats: WatchStats,
    /// A lying client (tests only): claims the first input of its own from this tick on that
    /// is not a repeat, which it did send.
    #[cfg(feature = "test-cheats")]
    lie_from: Option<u32>,
}

impl Watch {
    /// A watch for the race whose [`params_hash`] is `params` (all zero: none issued, nothing
    /// checked), hosted by the player whose key is `host` (all zero: a server, no checkpoints),
    /// signing this client's inputs with `signer` when there is one.
    #[must_use]
    pub fn new(params: [u8; 32], host: [u8; 32], signer: Option<Arc<PlayerKey>>) -> Self {
        Self {
            params,
            host,
            signer,
            seat: 0,
            fingerprint: 0,
            chain: Chain::new(&params, 0),
            records: Vec::new(),
            kept: BTreeMap::new(),
            pending: BTreeMap::new(),
            sent: BTreeMap::new(),
            first_sent: None,
            last_own: Held::default(),
            mismatch: None,
            forged: None,
            report: None,
            reported: false,
            stats: WatchStats::default(),
            #[cfg(feature = "test-cheats")]
            lie_from: None,
        }
    }

    /// Makes this client lie (tests only): from `tick` on, the first input of its own that is
    /// not a repeat is claimed forged, though it sent it.
    #[cfg(feature = "test-cheats")]
    pub fn lie_from(&mut self, tick: u32) {
        self.lie_from = Some(tick);
    }

    fn player_hosted(&self) -> bool {
        self.host != [0; 32]
    }

    /// Whether the host's welcome names the race the lobby issued. With none issued (a game
    /// that came back to its race from its record), the welcome's race is the one its inputs
    /// are signed for.
    ///
    /// # Errors
    ///
    /// The line a player reads: the host offered another race.
    pub fn check_welcome(&mut self, params: &RaceParams) -> Result<(), String> {
        if self.params == [0; 32] {
            self.params = params_hash(params);
            return Ok(());
        }
        if params_hash(params) == self.params {
            return Ok(());
        }
        Err(format!(
            "the host offered a race the lobby did not issue (circuit {:?}, seed {}): refused",
            params.track, params.seed
        ))
    }

    /// The race starts: this client flies `seat` from the tick-0 state `fingerprint`.
    pub fn start(&mut self, seat: usize, fingerprint: u64) {
        self.seat = u8::try_from(seat).unwrap_or(u8::MAX);
        self.fingerprint = fingerprint;
        self.chain = Chain::new(&self.params, fingerprint);
    }

    /// This client's signature over an input datagram, and notes the inputs as sent.
    pub fn sign(&mut self, first: u32, inputs: &[Held]) -> Option<[u8; 64]> {
        if let Some(held) = inputs.last() {
            let tick = first.saturating_add(u32::try_from(inputs.len() - 1).unwrap_or(0));
            self.sent.insert(tick, *held);
            self.first_sent.get_or_insert(tick);
            while self.sent.len() > MAX_SENT {
                self.sent.pop_first();
            }
        }
        // Signed whenever there is a key, so a game that came back to a player-hosted race from
        // its record (which keeps no host key) is still heard; a server ignores the signature.
        let key = self.signer.as_ref()?;
        self.stats.inputs_signed += 1;
        Some(key.sign(&input_statement(&self.params, self.seat, first, inputs)))
    }

    /// The host's confirmed record of `tick` was applied; `matched` says whether this client
    /// reached the same state hash.
    pub fn confirmed(&mut self, tick: u32, record: &TickRecord, matched: bool) {
        if !self.player_hosted() {
            return;
        }
        if !matched && self.mismatch.is_none() {
            self.mismatch = Some(tick);
        }
        if let Some(&own) = record.inputs.get(usize::from(self.seat)) {
            while self.sent.first_key_value().is_some_and(|(&t, _)| t < tick) {
                self.sent.pop_first();
            }
            // A host applies what this client sent for the tick, or repeats the tick before
            // when nothing arrived (or this client sent nothing: its present skipped the tick).
            let sent = self.sent.remove(&tick);
            #[cfg(feature = "test-cheats")]
            if self.lie_from.is_some_and(|from| tick >= from)
                && !same(own, self.last_own)
                && self.forged.is_none()
            {
                self.forged = Some(tick);
            }
            if self.first_sent.is_some_and(|first| tick >= first)
                && sent.is_none_or(|sent| !same(own, sent))
                && !same(own, self.last_own)
                && self.forged.is_none()
            {
                log::warn!(
                    "net: the host confirmed an input for this seat at tick {tick} that it \
                     never sent"
                );
                self.forged = Some(tick);
            }
            self.last_own = own;
        }
        self.chain.push(record);
        self.records.push(record.clone());
        let ticks = self.chain.ticks();
        if ticks.is_multiple_of(super::CHECKPOINT_EVERY) {
            self.kept.insert(ticks, self.chain.value());
            while self.kept.len() > MAX_KEPT_CHAINS {
                self.kept.pop_first();
            }
        }
        if let Some(checkpoint) = self.pending.remove(&ticks) {
            self.settle(checkpoint, self.chain.value());
        }
    }

    /// A checkpoint arrived from the host.
    pub fn checkpoint(&mut self, checkpoint: Checkpoint) {
        if !self.player_hosted() {
            return;
        }
        if !checkpoint.verify(&self.host, &self.params) {
            self.stats.checkpoints_unsigned += 1;
            log::warn!("net: a checkpoint not signed by the host: dropped");
            return;
        }
        let ticks = checkpoint.ticks;
        if ticks == self.chain.ticks() {
            self.settle(checkpoint, self.chain.value());
        } else if let Some(&chain) = self.kept.get(&ticks) {
            self.settle(checkpoint, chain);
        } else if ticks > self.chain.ticks() {
            self.pending.insert(ticks, checkpoint);
            while self.pending.len() > MAX_PENDING_CHECKPOINTS {
                self.pending.pop_first();
            }
        }
    }

    /// Compares a verified checkpoint with this client's chain at its tick, and builds the
    /// proof once a checkpoint covers what this client found.
    fn settle(&mut self, checkpoint: Checkpoint, ours: [u8; 32]) {
        if checkpoint.chain != ours {
            self.stats.checkpoints_differ += 1;
            log::warn!(
                "net: the host signed records other than the ones it sent this client, up to \
                 tick {}",
                checkpoint.ticks
            );
            return;
        }
        self.stats.checkpoints_verified += 1;
        if self.reported {
            return;
        }
        let claim = match (self.forged, self.mismatch) {
            (Some(tick), _) if tick < checkpoint.ticks => Claim::Forged {
                tick,
                seat: self.seat,
            },
            (_, Some(tick)) if tick < checkpoint.ticks => Claim::Desync,
            _ => return,
        };
        let records = self.records[..checkpoint.ticks as usize].to_vec();
        self.report = Some(Proof {
            claim,
            fingerprint: self.fingerprint,
            checkpoint,
            records,
        });
        self.reported = true;
        self.stats.reports += 1;
    }

    /// The proof for the lobby, once there is one; once only.
    pub fn take_report(&mut self) -> Option<Proof> {
        self.report.take()
    }

    /// What the watch has seen.
    #[must_use]
    pub fn stats(&self) -> WatchStats {
        self.stats
    }
}

impl std::fmt::Debug for Watch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watch")
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}
