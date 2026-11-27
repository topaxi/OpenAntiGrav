//! This player's key pair, kept in the profile: `identity.toml` in the configuration directory.
//!
//! Made the first time it is needed and never replaced: it is what an online lobby knows a
//! player who hosts a race by, and what a ban names
//! (`docs/architecture/adr/0092-a-player-hosted-race-is-a-room-whose-owner-runs-the-race-host.md`,
//! decision 3). The file holds the private key, so it is written for its owner alone where the
//! system has such a thing, and a file that cannot be read is an error, never a reason to make
//! another key over it.
//!
//! The write is atomic and owner-only (`profile::write_private`).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use oag_net::online::hostkey::PlayerKey;
use serde::{Deserialize, Serialize};

/// The schema kind this file is written as (`schemas/v1/identity.tosd`).
pub const KIND: &str = "identity";

/// The one algorithm a key is written under.
const ALGORITHM: &str = "ed25519";

#[derive(Debug, Default, Serialize, Deserialize)]
struct File {
    #[serde(rename = "toml-schema", skip_serializing_if = "Option::is_none")]
    schema: Option<oag_schema::Header>,
    key: Option<Key>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Key {
    algorithm: String,
    /// The key pair as PKCS#8, hex.
    pkcs8: String,
}

/// `identity.toml` in the configuration directory.
#[must_use]
pub fn path() -> Option<PathBuf> {
    crate::profile::config_dir().map(|dir| dir.join("oag").join("identity.toml"))
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return None;
    }
    text.as_bytes()
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

/// This profile's key pair, made and written on first use.
///
/// # Errors
///
/// No configuration directory, or what [`load_or_create_at`] names.
pub fn load_or_create() -> Result<PlayerKey> {
    let path = path().ok_or_else(|| anyhow!("no configuration directory to keep a key in"))?;
    load_or_create_at(&path)
}

/// The key pair in the file at `path`, made and written there when the file does not exist.
///
/// # Errors
///
/// A file that exists and cannot be read as a key (it is left as it is: delete it to make a new
/// identity), or a directory that cannot be written.
pub fn load_or_create_at(path: &Path) -> Result<PlayerKey> {
    match std::fs::read_to_string(path) {
        Ok(text) => read(&text).with_context(|| {
            format!(
                "{} is not a player key; delete it to make a new one",
                path.display()
            )
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let key = PlayerKey::generate()?;
            write(path, &key)?;
            log::info!("made this profile's player key ({})", path.display());
            Ok(key)
        }
        Err(e) => Err(anyhow!(e)).with_context(|| format!("reading {}", path.display())),
    }
}

fn read(text: &str) -> Result<PlayerKey> {
    let file: File = toml::from_str(text)?;
    oag_schema::check(file.schema.as_ref(), KIND, 1)?;
    let key = file.key.ok_or_else(|| anyhow!("no [key] table"))?;
    if key.algorithm != ALGORITHM {
        bail!(
            "a {} key, where this build knows {ALGORITHM}",
            key.algorithm
        );
    }
    let bytes = from_hex(&key.pkcs8).ok_or_else(|| anyhow!("pkcs8 is not hex"))?;
    PlayerKey::from_pkcs8(&bytes)
}

fn write(path: &Path, key: &PlayerKey) -> Result<()> {
    let file = File {
        schema: None,
        key: Some(Key {
            algorithm: ALGORITHM.to_owned(),
            pkcs8: oag_net::wt::hex(key.pkcs8()),
        }),
    };
    let text = format!(
        "# OpenAntiGrav player key: who this profile is to an online lobby when it hosts a race.\n\
         # SECRET: whoever has this file can host as you. Deleting it makes a new identity.\n\
         {}\n\n{}",
        oag_schema::inline(KIND, 1),
        toml::to_string_pretty(&file).context("serialising the key")?
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    crate::profile::write_private(path, text).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = oag_testdata::scratch::dir(&format!("identity-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("config").join("oag").join("identity.toml")
    }

    #[test]
    fn the_key_is_made_on_first_use_and_is_the_same_key_ever_after() {
        let path = scratch("first");
        assert!(!path.exists());
        let made = load_or_create_at(&path).expect("a key is made");
        let text = std::fs::read_to_string(&path).expect("the key was written");
        assert!(text.contains("schemas/v1/identity.tosd"), "{text}");
        let again = load_or_create_at(&path).expect("the key reads back");
        assert_eq!(made.public(), again.public());
        assert_eq!(
            std::fs::read_to_string(&path).expect("still there"),
            text,
            "a second use rewrites nothing"
        );
        // Another profile is another player.
        let other = load_or_create_at(&scratch("other")).expect("a key is made");
        assert_ne!(made.public(), other.public());
    }

    #[cfg(unix)]
    #[test]
    fn the_key_file_is_its_owners_alone() {
        use std::os::unix::fs::PermissionsExt;
        let path = scratch("mode");
        load_or_create_at(&path).expect("a key is made");
        let mode = std::fs::metadata(&path)
            .expect("written")
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0, "{mode:o}");
    }

    #[test]
    fn a_file_that_is_not_a_key_is_an_error_and_is_left_alone() {
        let path = scratch("broken");
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
        std::fs::write(&path, "not a key").expect("writes");
        let why = load_or_create_at(&path).expect_err("refused");
        assert!(format!("{why:#}").contains("delete it"), "{why:#}");
        assert_eq!(std::fs::read_to_string(&path).expect("kept"), "not a key");
    }

    #[test]
    fn the_shipped_fixture_is_a_key() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/identity.toml");
        let text = std::fs::read_to_string(fixture).expect("the fixture");
        read(&text).expect("the fixture is a key");
    }
}
