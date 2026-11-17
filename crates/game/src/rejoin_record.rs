//! The seat of an online race, kept on disk while the race runs so a game started again after
//! a crash can offer to take it back: `rejoin.toml` in the state directory.
//!
//! Written when the race starts and refreshed every [`REFRESH_SECONDS`] while it runs, each
//! write moving the expiry [`HOLD_SECONDS`] on, so a record left by a killed process goes
//! stale on its own. Deleted when the race ends or the player leaves it, and when the offer
//! is declined. Whether the server still holds the seat is asked by dialling it before the
//! offer is made (`session::online`), because a fresh record only says the race was running a
//! little while ago.
//!
//! The write is atomic (temp file, then rename), like `music.toml`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use log::warn;
use serde::{Deserialize, Serialize};

use crate::net_play::rejoin::Rejoin;

/// The schema kind this file is written as (`schemas/v1/rejoin.tosd`).
pub const KIND: &str = "rejoin";

/// How often a race refreshes the record. Chosen, not measured.
pub const REFRESH_SECONDS: u64 = 10;

/// How long after its last refresh a record is still offered: the time to start the game again
/// and open ONLINE, well inside what a race lasts. Chosen, not measured.
pub const HOLD_SECONDS: u64 = 180;

#[derive(Debug, Default, Serialize, Deserialize)]
struct File {
    #[serde(rename = "toml-schema", skip_serializing_if = "Option::is_none")]
    schema: Option<oag_schema::Header>,
    seat: Option<Seat>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Seat {
    title: String,
    addr: String,
    cert_hash: String,
    room: u32,
    name: String,
    team: String,
    token: String,
    expires: u64,
}

/// `rejoin.toml` in the state directory, or [`None`] where there is none (the web).
#[must_use]
pub fn path() -> Option<PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        dirs::state_dir()
            .or_else(dirs::data_local_dir)
            .map(|dir| dir.join("oag").join("rejoin.toml"))
    }
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
}

fn hex(bytes: &[u8; 32]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(64), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
}

fn parse_hash(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 || !text.is_ascii() {
        return None;
    }
    let mut hash = [0u8; 32];
    for (byte, pair) in hash.iter_mut().zip(text.as_bytes().chunks(2)) {
        *byte = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(hash)
}

/// Seconds since the Unix epoch.
#[must_use]
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

fn read(path: &Path) -> Option<Seat> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            warn!(
                "could not read {}: {e:#}; no seat to rejoin",
                path.display()
            );
            return None;
        }
    };
    let parsed = toml::from_str::<File>(&text)
        .map_err(anyhow::Error::from)
        .and_then(|file| {
            oag_schema::check(file.schema.as_ref(), KIND, 1)?;
            Ok(file)
        });
    match parsed {
        Ok(file) => file.seat,
        Err(e) => {
            warn!("{} is unusable ({e:#}); no seat to rejoin", path.display());
            None
        }
    }
}

/// Writes `rejoin` as the seat of `title`'s race, good until [`HOLD_SECONDS`] after `now`.
///
/// # Errors
/// A directory or file that cannot be written.
pub fn store_to(path: &Path, title: &str, rejoin: &Rejoin, now: u64) -> Result<()> {
    let file = File {
        schema: None,
        seat: Some(Seat {
            title: title.to_owned(),
            addr: rejoin.addr.clone(),
            cert_hash: hex(&rejoin.cert_hash),
            room: rejoin.room,
            name: rejoin.name.clone(),
            team: rejoin.team.clone(),
            token: rejoin.token.clone(),
            expires: now + HOLD_SECONDS,
        }),
    };
    let text = format!(
        "# OpenAntiGrav online seat: where an online race this machine is in is, so a game\n\
         # started again can take the seat back. This project's own file.\n\
         {}\n\n{}",
        oag_schema::inline(KIND, 1),
        toml::to_string_pretty(&file).context("serialising the seat")?
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let temp = path.with_extension("toml.tmp");
    std::fs::write(&temp, text).with_context(|| format!("writing {}", temp.display()))?;
    std::fs::rename(&temp, path).with_context(|| format!("replacing {}", path.display()))
}

/// The seat saved for `title` in the file at `path`, if it has not expired by `now`. A record
/// that has expired, is another title's or cannot be read is deleted and none is offered.
#[must_use]
pub fn load_from(path: &Path, title: &str, now: u64) -> Option<Rejoin> {
    let seat = read(path)?;
    let hash = parse_hash(&seat.cert_hash);
    let (Some(cert_hash), true) = (hash, title_fits(&seat.title, title) && seat.expires > now)
    else {
        clear_at(path);
        return None;
    };
    Some(Rejoin {
        addr: seat.addr,
        cert_hash,
        room: seat.room,
        name: seat.name,
        team: seat.team,
        token: seat.token,
    })
}

/// The rule `oag_net::race::check_title` applies: a server that names no title fits any.
fn title_fits(recorded: &str, opened: &str) -> bool {
    recorded.is_empty() || recorded == opened
}

/// Deletes the record at `path`; no record is not an error.
pub fn clear_at(path: &Path) {
    if let Err(e) = std::fs::remove_file(path)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        warn!("could not delete {}: {e:#}", path.display());
    }
}

#[cfg(test)]
mod tests;
