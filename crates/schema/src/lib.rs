//! The `[toml-schema]` header of every TOML file this project owns, and the one
//! place that reads it.
//!
//! A document carries
//!
//! ```toml
//! [toml-schema]
//! location = "https://oag.topaxi.com/mods/schemas/v1/pilot.tosd"
//! version = "1.0.0"
//! ```
//!
//! and nothing else for versioning. The **kind** (`pilot`) and the **major**
//! (`1`) are the last two path segments of `location`; `version` is the TOML
//! Schema *language* version, not ours. There is no minor: a compatible
//! addition goes into the same `v1` schema, a breaking change gets `v2/`.
//! ADR-0064 has the reasoning.
//!
//! A loader calls [`check`] with the kind it reads and the one major it
//! understands, and gets an [`Error`] that names what was found.

use serde::{Deserialize, Serialize};

#[cfg(feature = "validate")]
pub mod validate;

/// Where the schemas are deployed, up to and excluding `v<major>/`.
pub const BASE: &str = "https://oag.topaxi.com/mods/schemas";
/// The TOML Schema language version the schemas are written in.
pub const LANGUAGE_VERSION: &str = "1.0.0";
/// The table's name in a document.
pub const TABLE: &str = "toml-schema";

/// The `[toml-schema]` table as a document spells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Header {
    /// The schema's address.
    pub location: String,
    /// The schema language version the document expects.
    pub version: String,
}

/// What a header names once its `location` is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// `pilot`, `menu`, `track`.
    pub kind: String,
    /// The schema's major version.
    pub major: u32,
}

/// Why a document's header was not accepted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// No `[toml-schema]` table.
    #[error("no [toml-schema] table; add `{}`", inline(kind, *major))]
    Missing {
        /// What the loader reads.
        kind: String,
        /// The major it reads.
        major: u32,
    },
    /// `location` does not end in `v<major>/<kind>.tosd`.
    #[error("[toml-schema] location {0:?} does not end in v<major>/<kind>.tosd")]
    BadLocation(String),
    /// A file of another kind.
    #[error("[toml-schema] location names a {found:?} file, but this is read as {expected:?}")]
    WrongKind {
        /// What the header says.
        found: String,
        /// What the loader reads.
        expected: String,
    },
    /// A major this build does not read.
    #[error("{kind} schema v{found} is not supported; this build reads v{supported}")]
    UnknownMajor {
        /// The kind.
        kind: String,
        /// What the header says.
        found: u32,
        /// What the loader reads.
        supported: u32,
    },
    /// `version` is not the language version the schemas use.
    #[error("[toml-schema] version {found:?}, but the schemas are TOML Schema {LANGUAGE_VERSION}")]
    LanguageVersion {
        /// What the header says.
        found: String,
    },
}

/// The `location` of the schema for `kind` at `major`.
#[must_use]
pub fn location(kind: &str, major: u32) -> String {
    format!("{BASE}/v{major}/{kind}.tosd")
}

/// The header text for `kind` at `major`, ready to paste into a file.
#[must_use]
pub fn header(kind: &str, major: u32) -> String {
    format!(
        "[{TABLE}]\nlocation = {:?}\nversion = {LANGUAGE_VERSION:?}\n",
        location(kind, major)
    )
}

/// The same header as one line, for a file whose top level is plain keys: a
/// `[toml-schema]` table there would have to be the file's last thing, because
/// every key after it belongs to it. Parsed, it is the same table.
#[must_use]
pub fn inline(kind: &str, major: u32) -> String {
    format!(
        "{TABLE} = {{ location = {:?}, version = {LANGUAGE_VERSION:?} }}",
        location(kind, major)
    )
}

/// Reads the kind and major out of a `location`.
///
/// # Errors
///
/// [`Error::BadLocation`] when it does not end in `v<digits>/<kind>.tosd`.
pub fn parse_location(location: &str) -> Result<Reference, Error> {
    let bad = || Error::BadLocation(location.to_owned());
    let mut segments = location.rsplit('/');
    let file = segments.next().ok_or_else(bad)?;
    let dir = segments.next().ok_or_else(bad)?;
    let kind = file
        .strip_suffix(".tosd")
        .filter(|k| !k.is_empty())
        .ok_or_else(bad)?;
    let digits = dir.strip_prefix('v').ok_or_else(bad)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    let major = digits.parse().map_err(|_| bad())?;
    Ok(Reference {
        kind: kind.to_owned(),
        major,
    })
}

/// Checks a document's header for a loader that reads `kind` at `major`.
///
/// `header` is what the document's `[toml-schema]` deserialised to, `None` when
/// the table is absent; a caller whose file is allowed to lack it (a pilot a
/// player wrote before the header existed) matches [`Error::Missing`] itself.
///
/// # Errors
///
/// By name: a missing table, a malformed `location`, another kind, another major,
/// another language version.
pub fn check(header: Option<&Header>, kind: &str, major: u32) -> Result<(), Error> {
    let header = header.ok_or_else(|| Error::Missing {
        kind: kind.to_owned(),
        major,
    })?;
    let found = parse_location(&header.location)?;
    if found.kind != kind {
        return Err(Error::WrongKind {
            found: found.kind,
            expected: kind.to_owned(),
        });
    }
    if found.major != major {
        return Err(Error::UnknownMajor {
            kind: found.kind,
            found: found.major,
            supported: major,
        });
    }
    if header.version != LANGUAGE_VERSION {
        return Err(Error::LanguageVersion {
            found: header.version.clone(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
