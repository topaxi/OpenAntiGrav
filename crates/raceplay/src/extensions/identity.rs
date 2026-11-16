//! A race's simulation modules as a replay names them, and the check that a rebuild runs the
//! same ones (`docs/architecture/wasm-extensions/determinism-and-state.md`, "Replays").
//!
//! A replay re-simulates, so a module's code is part of the recording. The header's
//! [`ModUse`] list carries each module's SHA-256; [`restrict_to`] makes a load run exactly the
//! recorded modules (and hand them the recorded picks), and [`check_recorded`] compares what
//! the load compiled with the list and names every difference.

use oag_replay::header::ModUse;

use super::Source;

/// SHA-256 of `bytes` as sixty-four lower-case hex digits.
#[cfg(feature = "extensions")]
pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    oag_disc::ps4_pkg::crypto::sha256(bytes).iter().fold(
        String::with_capacity(64),
        |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        },
    )
}

/// `source`'s module as a replay names it, once the load has hashed its bytes and compiled it.
#[cfg(feature = "extensions")]
pub(super) fn used(source: &Source, sha256: String, state_bytes: u32) -> ModUse {
    ModUse {
        id: source.mod_id.clone(),
        version: source.version.clone(),
        module: source.module.clone(),
        sha256,
        abi: oag_ext_abi::ABI,
        revision: oag_ext_abi::REVISION,
        state_bytes,
        picks: source.options.sim.to_vec(),
    }
}

/// The `sources` a rebuild of a race that ran `recorded` loads: those the list names (by mod id
/// and module path), each with the recorded picks. An empty list is a vanilla race and keeps
/// none. A module the list names that is not among `sources` is [`check_recorded`]'s to name.
#[must_use]
pub fn restrict_to(sources: Vec<Source>, recorded: &[ModUse]) -> Vec<Source> {
    sources
        .into_iter()
        .filter_map(|mut source| {
            let entry = recorded
                .iter()
                .find(|m| m.id == source.mod_id && m.module == source.module)?;
            source.options = source.options.with_sim_picks(&entry.picks);
            Some(source)
        })
        .collect()
}

/// Why a race that runs `running` is not the one `recorded` ran, every difference named, or
/// `Ok` when they are the same modules in the same order.
///
/// # Errors
///
/// The differences, joined: a recorded module that is missing, another build of it (hash or
/// version), other picks, a module the recording did not have.
pub fn check_recorded(recorded: &[ModUse], running: &[ModUse]) -> Result<(), String> {
    let mut why = Vec::new();
    for want in recorded {
        let Some(have) = running
            .iter()
            .find(|m| m.id == want.id && m.module == want.module)
        else {
            why.push(format!(
                "the recording ran {} {} and this machine does not load it (is the mod installed and switched on, for this title?)",
                want.label(),
                want.version
            ));
            continue;
        };
        if have.sha256 != want.sha256 {
            why.push(format!(
                "{} is another build: the recording ran sha256 {} (version {}), this machine has {} (version {})",
                want.label(),
                &want.sha256[..want.sha256.len().min(12)],
                want.version,
                &have.sha256[..have.sha256.len().min(12)],
                have.version
            ));
        } else if have.version != want.version {
            why.push(format!(
                "{} is version {} here and {} in the recording, so its manifest tables may differ",
                want.label(),
                have.version,
                want.version
            ));
        }
        if have.state_bytes != want.state_bytes
            || have.abi != want.abi
            || have.revision != want.revision
        {
            why.push(format!(
                "{}'s layout differs: ABI {} rev {} with {} state bytes recorded, ABI {} rev {} with {} here",
                want.label(),
                want.abi,
                want.revision,
                want.state_bytes,
                have.abi,
                have.revision,
                have.state_bytes
            ));
        }
        if have.picks != want.picks {
            why.push(format!(
                "{} runs with picks {:?} here and {:?} in the recording",
                want.label(),
                have.picks,
                want.picks
            ));
        }
    }
    for have in running {
        if !recorded
            .iter()
            .any(|m| m.id == have.id && m.module == have.module)
        {
            why.push(format!(
                "this race also runs {}, which the recording did not have",
                have.label()
            ));
        }
    }
    if why.is_empty()
        && recorded
            .iter()
            .map(ModUse::label)
            .ne(running.iter().map(ModUse::label))
    {
        why.push("the modules run in another order".to_string());
    }
    if why.is_empty() {
        Ok(())
    } else {
        Err(why.join("; "))
    }
}
