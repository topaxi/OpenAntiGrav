//! Catching a player host that cheats: the pieces both ends and the lobby share
//! (`docs/architecture/adr/0092-a-player-hosted-race-is-a-room-whose-owner-runs-the-race-host.md`,
//! "The anti-cheat path").
//!
//! - [`params_hash`]: the race the lobby issued, as one hash the hand-off carries and a client
//!   compares the host's welcome with.
//! - [`Chain`]: a running SHA-256 over the confirmed tick records (inputs, away mask, state
//!   hash). Host and client fold the same records, so the host's signed [`Checkpoint`] once a
//!   second commits it to exactly the records every client was sent.
//! - [`input_statement`]: what a client signs each input datagram over, so the host can show
//!   the lobby the signed datagram behind an input it confirmed.
//! - [`Proof`]: a client's case against the host, the records from tick 0 to a checkpoint and
//!   the checkpoint; [`Proof::decode`] refuses a malformed or oversized one before anything
//!   is allocated for it.
//! - [`judge`]: the lobby's re-simulation of a proof's records from the job it issued.
//!
//! Every constant here is **chosen, not measured**.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use oag_bytes::{Le, Reader};
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::Input;
use oag_raceplay::Race;
use ring::digest::{Context, SHA256};
use web_time::Instant;

use crate::online::hostkey::{PlayerKey, signed_by};
use crate::wire::{Confirmed, Datagram, Held, Malformed, RaceParams, TickRecord};

/// Ticks between two signed checkpoints: one a second.
pub const CHECKPOINT_EVERY: u32 = 60;

/// Ticks of applied inputs a host keeps the signed datagram of, to show the lobby on a
/// challenge: two minutes, about 2 MB a seat.
pub const LEDGER_TICKS: u32 = 7200;

/// Whether a forgery claim on `tick` is too old to put to the host, `ticks_since_up` ticks of
/// wall clock after the host said its race was up (no race has run more ticks than that). An
/// honest client claims within a second or two of the tick; past half the ledger, an honest host
/// may no longer hold the datagram by the time the challenge's answer is due, and a claim it
/// cannot answer would ban it wrongly.
#[must_use]
pub fn too_old_to_challenge(ticks_since_up: u32, tick: u32) -> bool {
    ticks_since_up > tick.saturating_add(LEDGER_TICKS / 2)
}

pub use crate::wire::online::REPORT_PART_BYTES as PART_BYTES;

/// The most bytes a proof may have whatever the race: 30 minutes of twelve seats.
pub const MAX_PROOF_BYTES: usize = 28 << 20;

/// The ticks a proof may carry at most: the 30 minutes a hosted race may run.
pub const MAX_PROOF_TICKS: u32 = 60 * 60 * 30;

const PROOF_MAGIC: &[u8; 4] = b"OAGP";

mod watch;
pub use watch::{Watch, WatchStats};

fn sha256(parts: &[&[u8]]) -> [u8; 32] {
    let mut context = Context::new(&SHA256);
    for part in parts {
        context.update(part);
    }
    let mut out = [0; 32];
    out.copy_from_slice(context.finish().as_ref());
    out
}

/// The hash of the race the lobby issued: SHA-256 over its wire bytes.
#[must_use]
pub fn params_hash(params: &RaceParams) -> [u8; 32] {
    let mut bytes = Vec::new();
    params.put(&mut bytes);
    sha256(&[b"OAG-PARAMS-1", &bytes])
}

/// The running hash over the confirmed tick records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chain {
    value: [u8; 32],
    ticks: u32,
}

impl Chain {
    /// The chain before tick 0, bound to the race and the state it starts from.
    #[must_use]
    pub fn new(params: &[u8; 32], fingerprint: u64) -> Self {
        Self {
            value: sha256(&[b"OAG-CHAIN-1", params, &fingerprint.to_le_bytes()]),
            ticks: 0,
        }
    }

    /// Folds in the next tick's record.
    pub fn push(&mut self, record: &TickRecord) {
        let mut bytes = Vec::with_capacity(Confirmed::record_bytes(record.inputs.len()));
        bytes.extend_from_slice(&record.hash.to_le_bytes());
        bytes.push(record.retired);
        for held in &record.inputs {
            held.put(&mut bytes);
        }
        self.value = sha256(&[&self.value, &bytes]);
        self.ticks += 1;
    }

    /// The hash after [`Self::ticks`] records.
    #[must_use]
    pub fn value(&self) -> [u8; 32] {
        self.value
    }

    /// How many records are folded in.
    #[must_use]
    pub fn ticks(&self) -> u32 {
        self.ticks
    }
}

/// The host's signed word on the first `ticks` records: their chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checkpoint {
    /// How many records the chain covers (ticks `0..ticks`).
    pub ticks: u32,
    /// [`Chain::value`] after them.
    pub chain: [u8; 32],
    /// The host's player key over [`Self::statement`].
    pub signature: [u8; 64],
}

impl Checkpoint {
    /// The bytes the host signs.
    #[must_use]
    pub fn statement(params: &[u8; 32], ticks: u32, chain: &[u8; 32]) -> Vec<u8> {
        let mut out = b"OAG-CHECK-1".to_vec();
        out.extend_from_slice(params);
        out.extend_from_slice(&ticks.to_le_bytes());
        out.extend_from_slice(chain);
        out
    }

    /// `key`'s checkpoint of `chain`.
    #[must_use]
    pub fn sign(key: &PlayerKey, params: &[u8; 32], chain: &Chain) -> Self {
        let value = chain.value();
        Self {
            ticks: chain.ticks(),
            chain: value,
            signature: key.sign(&Self::statement(params, chain.ticks(), &value)),
        }
    }

    /// Whether the host whose public key is `key` signed this for the race `params`.
    #[must_use]
    pub fn verify(&self, key: &[u8; 32], params: &[u8; 32]) -> bool {
        signed_by(
            key,
            &Self::statement(params, self.ticks, &self.chain),
            &self.signature,
        )
    }
}

/// What a client signs an input datagram over: the race, the seat and the inputs from `first`.
#[must_use]
pub fn input_statement(params: &[u8; 32], seat: u8, first: u32, inputs: &[Held]) -> Vec<u8> {
    let mut out = b"OAG-INPUT-1".to_vec();
    out.extend_from_slice(params);
    out.push(seat);
    out.extend_from_slice(&first.to_le_bytes());
    out.push(u8::try_from(inputs.len()).unwrap_or(u8::MAX));
    for held in inputs.iter().take(usize::from(u8::MAX)) {
        held.put(&mut out);
    }
    out
}

/// Whether `datagram` is a signed input datagram of `seat` for the race `params`, under `key`.
#[must_use]
pub fn inputs_signed(datagram: &Datagram, params: &[u8; 32], seat: u8, key: &[u8; 32]) -> bool {
    match datagram {
        Datagram::Inputs {
            seat: from,
            first,
            inputs,
            signature: Some(signature),
        } if *from == seat => signed_by(
            key,
            &input_statement(params, seat, *first, inputs),
            signature,
        ),
        _ => false,
    }
}

/// Whether two inputs are the same to the bit.
#[must_use]
pub fn same(a: Held, b: Held) -> bool {
    let (mut x, mut y) = (
        Vec::with_capacity(Held::BYTES),
        Vec::with_capacity(Held::BYTES),
    );
    a.put(&mut x);
    b.put(&mut y);
    x == y
}

/// The signed datagram behind each input a host applied, by seat and tick, for the last
/// [`LEDGER_TICKS`]: what it shows the lobby when a seat says it never sent an input.
#[derive(Debug, Default)]
pub struct Ledger {
    seats: Vec<BTreeMap<u32, Arc<[u8]>>>,
    count: u64,
}

/// A [`Ledger`] the race thread writes and the hosting game reads.
pub type SharedLedger = Arc<Mutex<Ledger>>;

impl Ledger {
    /// Notes that the input `seat` flew at `tick` came in `datagram` (its encoded bytes).
    pub fn applied(&mut self, seat: usize, tick: u32, datagram: Arc<[u8]>) {
        if self.seats.len() <= seat {
            self.seats.resize_with(seat + 1, BTreeMap::new);
        }
        self.count += 1;
        let book = &mut self.seats[seat];
        book.insert(tick, datagram);
        let keep = tick.saturating_sub(LEDGER_TICKS);
        while book.first_key_value().is_some_and(|(&t, _)| t < keep) {
            book.pop_first();
        }
    }

    /// How many signed inputs were applied.
    #[must_use]
    pub fn count(&self) -> u64 {
        self.count
    }

    /// The datagram behind `seat`'s input at `tick`, if it is still kept.
    #[must_use]
    pub fn find(&self, seat: usize, tick: u32) -> Option<Vec<u8>> {
        self.seats.get(seat)?.get(&tick).map(|bytes| bytes.to_vec())
    }
}

/// What a client claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Claim {
    /// A state hash the host sent cannot be reproduced from the inputs it sent with it.
    Desync,
    /// The host confirmed an input for `seat` at `tick` that the seat never sent.
    Forged {
        /// The tick.
        tick: u32,
        /// The reporter's own seat.
        seat: u8,
    },
}

/// A client's case against its host.
#[derive(Debug, Clone, PartialEq)]
pub struct Proof {
    /// What is claimed.
    pub claim: Claim,
    /// The tick-0 state hash the race started from (the welcome's).
    pub fingerprint: u64,
    /// The host's signed checkpoint the records end at.
    pub checkpoint: Checkpoint,
    /// The confirmed records from tick 0, as many as the checkpoint covers.
    pub records: Vec<TickRecord>,
}

/// The bytes before the records.
const PROOF_HEADER: usize = 4 + 1 + 4 + 1 + 8 + 4 + 32 + 64 + 1;

impl Proof {
    /// The proof's bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let people = self.records.first().map_or(0, |r| r.inputs.len());
        let mut out =
            Vec::with_capacity(PROOF_HEADER + self.records.len() * Confirmed::record_bytes(people));
        out.extend_from_slice(PROOF_MAGIC);
        let (kind, tick, seat) = match self.claim {
            Claim::Desync => (0, 0, 0),
            Claim::Forged { tick, seat } => (1, tick, seat),
        };
        out.push(kind);
        out.extend_from_slice(&tick.to_le_bytes());
        out.push(seat);
        out.extend_from_slice(&self.fingerprint.to_le_bytes());
        out.extend_from_slice(&self.checkpoint.ticks.to_le_bytes());
        out.extend_from_slice(&self.checkpoint.chain);
        out.extend_from_slice(&self.checkpoint.signature);
        out.push(u8::try_from(people).unwrap_or(u8::MAX));
        for record in &self.records {
            out.extend_from_slice(&record.hash.to_le_bytes());
            out.push(record.retired);
            for held in record.inputs.iter().take(people) {
                held.put(&mut out);
            }
        }
        out
    }

    /// Parses a proof of a race of `people` seats that may have run `max_ticks` ticks so far.
    /// The length is checked against the header before any record is read.
    ///
    /// # Errors
    ///
    /// [`Malformed`] for a wrong magic or claim, another seat count, more ticks than the race
    /// has run, a length that is not exactly the header's records, or a checkpoint of no tick.
    pub fn decode(bytes: &[u8], people: usize, max_ticks: u32) -> Result<Self, Malformed> {
        let mut r = Reader::<Le>::new(bytes);
        if r.take(4)? != PROOF_MAGIC {
            return Err(Malformed("a proof without the OAGP magic"));
        }
        let kind = r.u8()?;
        let tick = r.u32()?;
        let seat = r.u8()?;
        let claim = match kind {
            0 => Claim::Desync,
            1 => Claim::Forged { tick, seat },
            _ => return Err(Malformed("unknown claim")),
        };
        let fingerprint = r.u64()?;
        let ticks = r.u32()?;
        let mut checkpoint = Checkpoint {
            ticks,
            chain: [0; 32],
            signature: [0; 64],
        };
        checkpoint.chain.copy_from_slice(r.take(32)?);
        checkpoint.signature.copy_from_slice(r.take(64)?);
        if usize::from(r.u8()?) != people {
            return Err(Malformed("a proof of another race's seats"));
        }
        if ticks == 0 || ticks > max_ticks.min(MAX_PROOF_TICKS) {
            return Err(Malformed("a proof of more ticks than the race has run"));
        }
        let expected = PROOF_HEADER + ticks as usize * Confirmed::record_bytes(people);
        if bytes.len() != expected || expected > MAX_PROOF_BYTES {
            return Err(Malformed("a proof whose length is not its records'"));
        }
        let mut records = Vec::with_capacity(ticks as usize);
        for _ in 0..ticks {
            let hash = r.u64()?;
            let retired = r.u8()?;
            let inputs = (0..people)
                .map(|_| Held::take(&mut r))
                .collect::<Result<_, _>>()?;
            records.push(TickRecord {
                hash,
                inputs,
                retired,
            });
        }
        Ok(Self {
            claim,
            fingerprint,
            checkpoint,
            records,
        })
    }

    /// Whether the host whose key is `host` signed exactly these records for the race `params`:
    /// the checkpoint's signature, then the chain folded again from the records.
    ///
    /// # Errors
    ///
    /// Why the records are not the host's.
    pub fn check_signed(&self, params: &[u8; 32], host: &[u8; 32]) -> Result<(), String> {
        if !self.checkpoint.verify(host, params) {
            return Err("the checkpoint is not signed by the host for this race".into());
        }
        if self.records.len() != self.checkpoint.ticks as usize {
            return Err("the records do not end at the checkpoint".into());
        }
        let mut chain = Chain::new(params, self.fingerprint);
        for record in &self.records {
            chain.push(record);
        }
        if chain.value() != self.checkpoint.chain {
            return Err("the records are not the ones the host signed".into());
        }
        Ok(())
    }

    /// The input a forgery claim says the host confirmed for `seat` at `tick`, when the
    /// records show one that is not a repeat of the tick before (a host repeats a seat's last
    /// input when none arrived, which is its right).
    ///
    /// # Errors
    ///
    /// The records do not reach the tick or seat, or the input is a repeat.
    pub fn forged_input(&self, seat: u8, tick: u32) -> Result<Held, String> {
        let seat = usize::from(seat);
        let at = |t: usize| {
            self.records
                .get(t)
                .and_then(|r| r.inputs.get(seat))
                .copied()
        };
        let held = at(tick as usize).ok_or("the records do not reach the claimed input")?;
        let before = match tick.checked_sub(1) {
            Some(t) => at(t as usize).ok_or("the records do not reach the claimed input")?,
            None => Held::default(),
        };
        if same(held, before) {
            return Err("the input is a repeat of the tick before".into());
        }
        Ok(held)
    }
}

/// Whether `datagram`, the host's answer to a challenge, is a signed input datagram of `seat`
/// under `key` whose input at `tick` is `expected`.
#[must_use]
pub fn answer_holds(
    datagram: &[u8],
    params: &[u8; 32],
    seat: u8,
    tick: u32,
    expected: Held,
    key: &[u8; 32],
) -> bool {
    let Ok(message) = Datagram::decode(datagram) else {
        return false;
    };
    let Datagram::Inputs { first, inputs, .. } = &message else {
        return false;
    };
    let at = tick
        .checked_sub(*first)
        .and_then(|i| inputs.get(i as usize).copied());
    at.is_some_and(|held| same(held, expected)) && inputs_signed(&message, params, seat, key)
}

/// What the lobby concluded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The host cheated, and how: the lobby bans it.
    Proven(String),
    /// The report does not hold: the host did what it said.
    Dismissed(String),
    /// The lobby cannot decide (content it lacks, a start it cannot reproduce, out of time):
    /// evidence, never a ban.
    NotCheckable(String),
}

/// Re-simulates a desync proof's records on `race`, loaded and prepared from the job the lobby
/// issued, the way a client's confirmed timeline runs them: each record's away mask, its
/// inputs through the held-stream edges, one simulation tick, and the state hash compared.
/// Gives up as not checkable once `deadline` passes.
#[must_use]
pub fn judge(race: &mut Race, proof: &Proof, deadline: Instant) -> Verdict {
    let start = crate::race::fingerprint(race);
    if start != proof.fingerprint {
        return Verdict::NotCheckable(format!(
            "the lobby's copy of the race starts at state {start:#018x}, the host's at {:#018x}",
            proof.fingerprint
        ));
    }
    let people = proof.records.first().map_or(0, |r| r.inputs.len());
    let mut edges = vec![Input::new(); people];
    for (tick, record) in proof.records.iter().enumerate() {
        if tick % 600 == 0 && Instant::now() > deadline {
            return Verdict::NotCheckable(format!(
                "the re-simulation ran out of time at tick {tick}"
            ));
        }
        let mut inputs = PlayerInputs::none();
        for (slot, held) in record.inputs.iter().enumerate() {
            if let Some(edges) = edges.get_mut(slot) {
                inputs.set(slot, held.snapshot(edges));
            }
        }
        crate::race::presence(race, record.retired);
        race.tick_sim_only(&inputs);
        let ours = race.sim.state_hash();
        if ours != record.hash {
            return Verdict::Proven(format!(
                "after tick {tick} the host claimed state {:#018x}; its own inputs give \
                 {ours:#018x}",
                record.hash
            ));
        }
    }
    Verdict::Dismissed(format!(
        "the lobby reproduced all {} states the host sent",
        proof.records.len()
    ))
}

#[cfg(test)]
mod tests;
