//! Making a screenshot's race again from the record its PNG carries (`oag_replay::shot`).
//!
//! [`resimulate`] rebuilds the race from the record's header, re-drives the recorded inputs to
//! the shot's tick and reports whether the state hash is the one the shot was taken at. It builds
//! without the `view` feature, like [`super::rebuild`].

use anyhow::{Context, Result};
use oag_replay::{Desync, ShotRecord, Verifier};

use crate::Race;

/// A race brought to a shot's tick, and what its hash said.
#[derive(Debug)]
pub struct Resimulated {
    /// The race at the shot's tick.
    pub race: Race,
    /// The state hash the shot was taken at.
    pub expected: u64,
    /// The state hash the re-driven race arrived at.
    pub found: u64,
    /// The first stored hash (a second apart) that disagreed on the way, if one did.
    pub desync: Option<Desync>,
}

impl Resimulated {
    /// Whether this build reproduced the shot's race to the bit.
    #[must_use]
    pub fn matches(&self) -> bool {
        self.desync.is_none() && self.expected == self.found
    }
}

/// The race of `record` rebuilt over `source` and driven to the shot's tick.
///
/// A mismatch is a result ([`Resimulated::matches`]), not an error: the race is still there to
/// look at. The run stops at the first stored hash that disagrees, so a build that no longer
/// reproduces the run names the second it stopped in.
///
/// # Errors
///
/// A header that cannot be rebuilt ([`super::rebuild`]'s), or an event this build does not apply.
pub fn resimulate(record: &ShotRecord, source: &str) -> Result<Resimulated> {
    let replay = &record.replay;
    let mut race = super::rebuild(&replay.header, source)?;
    let mut verifier = match Verifier::new(replay, race.sim.state_hash()) {
        Ok(verifier) => verifier,
        Err(desync) => {
            return Ok(Resimulated {
                found: desync.found,
                expected: record.hash,
                desync: Some(desync),
                race,
            });
        }
    };
    let mut desync = None;
    while verifier.tick() < record.tick {
        let Some(inputs) = verifier.next_inputs() else {
            break;
        };
        for event in verifier.events_due() {
            race.apply_event(event)
                .with_context(|| format!("an event before tick {}", verifier.tick()))?;
        }
        race.tick(&inputs);
        race.drain_cues();
        if let Err(found) = verifier.after_tick(|| race.sim.state_hash()) {
            desync = Some(found);
            break;
        }
    }
    let found = race.sim.state_hash();
    Ok(Resimulated {
        expected: record.hash,
        found,
        desync,
        race,
    })
}
