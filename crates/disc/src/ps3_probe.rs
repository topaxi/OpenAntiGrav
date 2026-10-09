//! Whether a picked image needs a disc key, and whether the key the player
//! supplied opens it - one question the web page asks before it boots, and
//! the native launcher can ask of a dropped file.
//!
//! The key goes in as the bytes a player has (a `.dkey`'s sixteen raw bytes or
//! its hex text, see [`DiscKey::parse`]) and never comes back out except as
//! [`Probe::accepted`], for the caller to keep. Nothing here logs it.

use std::path::Path;

use crate::ps3_crypt::DiscKey;
use crate::{DiscImage, Ps3State, Result};

/// What the supplied key (or the lack of one) amounts to for this image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyCheck {
    /// A plain image, or a decrypted PS3 dump: no key is needed, and one
    /// supplied is ignored.
    NotNeeded,
    /// Encrypted, and no key was supplied.
    Missing,
    /// A key was supplied and is not sixteen bytes or 32 hex digits.
    Malformed,
    /// A well-formed key that does not open this image.
    Wrong,
    /// A key that opens this image (`Debug` redacts it; [`DiscKey::to_hex`]
    /// is what a caller keeps).
    Accepted(DiscKey),
}

/// The answer for one image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    /// The disc's serial, when the directory reads without the key (it does on
    /// a real PS3 disc: the directories sit in the plain regions).
    pub serial: Option<String>,
    /// The key's standing.
    pub check: KeyCheck,
}

/// Opens `path` with `supplied` as the only key (no search beside the image or
/// in the keys directory) and reports what happened.
///
/// # Errors
/// The image does not open at all.
pub fn probe(path: &Path, supplied: Option<&[u8]>) -> Result<Probe> {
    let parsed = supplied.map(DiscKey::parse);
    let keys: Vec<DiscKey> = parsed.iter().flatten().cloned().collect();
    let mut disc = DiscImage::open_with_keys(path, &keys)?;
    let serial = disc.identify().ok().and_then(|info| info.serial);
    let check = match (disc.ps3_state(), parsed) {
        (Ps3State::Locked, None) => KeyCheck::Missing,
        (Ps3State::Locked, Some(None)) => KeyCheck::Malformed,
        (Ps3State::Locked, Some(Some(_))) => KeyCheck::Wrong,
        (Ps3State::Decrypting, Some(Some(key))) => KeyCheck::Accepted(key),
        _ => KeyCheck::NotNeeded,
    };
    Ok(Probe { serial, check })
}
