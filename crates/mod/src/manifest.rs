//! `oag-mod.toml`: who a mod is, and under what licence it travels.

use serde::Deserialize;

/// The manifest's name at a mod's root.
pub const MANIFEST: &str = "oag-mod.toml";
/// The schema kind the header names.
pub const SCHEMA_KIND: &str = "mod";
/// The one schema major this build reads.
pub const SCHEMA_MAJOR: u32 = 1;

/// What is wrong with a manifest.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Not TOML, or not shaped like a manifest.
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
    /// No `[toml-schema]` header, or one naming another kind or major.
    #[error("{0}")]
    Schema(#[from] oag_schema::Error),
    /// A key with a value that cannot be right.
    #[error("[mod] {key}: {problem}")]
    Bad {
        /// The key.
        key: &'static str,
        /// What is wrong.
        problem: String,
    },
}

/// A mod's licence, as stated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum License {
    /// `license = "<SPDX expression>"`.
    Spdx(String),
    /// `license-file = "<path in the mod>"`.
    File(String),
    /// Neither: the mod loads, and says so wherever it is listed.
    None,
}

impl License {
    /// The words a mods list and the load report use.
    #[must_use]
    pub fn report(&self) -> String {
        match self {
            Self::Spdx(expression) => expression.clone(),
            Self::File(path) => format!("see {path}"),
            Self::None => "no license stated".to_string(),
        }
    }
}

/// A checked manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Unique and stable: `topaxi.example`.
    pub id: String,
    /// The name a player reads.
    pub name: String,
    /// The mod's own semver release.
    pub version: semver::Version,
    /// Who made it.
    pub authors: Vec<String>,
    /// The licence.
    pub license: License,
    /// One or two sentences.
    pub description: String,
    /// Where the source lives.
    pub repository: Option<String>,
    /// A page about it.
    pub homepage: Option<String>,
    /// Search words.
    pub keywords: Vec<String>,
    /// The titles it is for (their lowercased hyphenated names); empty means any.
    pub titles: Vec<String>,
    /// The game versions it works with.
    pub oag: Option<semver::VersionReq>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(rename = "toml-schema")]
    header: Option<oag_schema::Header>,
    #[serde(rename = "mod")]
    module: Table,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Table {
    id: String,
    name: String,
    version: String,
    authors: Vec<String>,
    license: Option<String>,
    #[serde(rename = "license-file")]
    license_file: Option<String>,
    description: String,
    repository: Option<String>,
    homepage: Option<String>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    titles: Vec<String>,
    oag: Option<String>,
}

fn bad(key: &'static str, problem: impl Into<String>) -> Error {
    Error::Bad {
        key,
        problem: problem.into(),
    }
}

impl Manifest {
    /// Reads and checks a manifest. `has` says whether the mod holds a path,
    /// for `license-file`.
    ///
    /// # Errors
    ///
    /// Anything in [`Error`].
    pub fn parse(text: &str, has: &dyn Fn(&str) -> bool) -> Result<Self, Error> {
        let file: File = toml::from_str(text)?;
        oag_schema::check(file.header.as_ref(), SCHEMA_KIND, SCHEMA_MAJOR)?;
        let t = file.module;

        if !valid_id(&t.id) {
            return Err(bad(
                "id",
                format!(
                    "{:?} is not an id: lowercase letters, digits, '.', '-' and '_', starting with a letter",
                    t.id
                ),
            ));
        }
        if t.name.trim().is_empty() {
            return Err(bad("name", "empty"));
        }
        let version = semver::Version::parse(&t.version).map_err(|e| {
            bad(
                "version",
                format!("{:?} is not a semver release: {e}", t.version),
            )
        })?;
        if t.authors.is_empty() || t.authors.iter().any(|a| a.trim().is_empty()) {
            return Err(bad("authors", "at least one non-empty author"));
        }
        if t.description.trim().is_empty() {
            return Err(bad("description", "empty"));
        }
        let license = match (t.license, t.license_file) {
            (Some(_), Some(_)) => {
                return Err(bad("license", "name `license` or `license-file`, not both"));
            }
            (Some(expression), None) => {
                spdx_shape(&expression).map_err(|p| bad("license", p))?;
                License::Spdx(expression)
            }
            (None, Some(path)) => {
                if !has(&path) {
                    return Err(bad("license-file", format!("{path:?} is not in the mod")));
                }
                License::File(path)
            }
            (None, None) => License::None,
        };
        let oag = t
            .oag
            .map(|req| {
                semver::VersionReq::parse(&req)
                    .map_err(|e| bad("oag", format!("{req:?} is not a version requirement: {e}")))
            })
            .transpose()?;
        Ok(Self {
            id: t.id,
            name: t.name,
            version,
            authors: t.authors,
            license,
            description: t.description,
            repository: t.repository,
            homepage: t.homepage,
            keywords: t.keywords,
            titles: t.titles,
            oag,
        })
    }

    /// Whether this build of the game is one the mod says it works with.
    #[must_use]
    pub fn works_with(&self, game: &semver::Version) -> bool {
        self.oag.as_ref().is_none_or(|req| req.matches(game))
    }
}

pub(crate) fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_'))
}

/// The shape of an SPDX licence expression: identifiers joined by `AND`, `OR`
/// and `WITH`, with parentheses. The identifier list is not checked.
fn spdx_shape(expression: &str) -> Result<(), String> {
    let spaced = expression.replace('(', " ( ").replace(')', " ) ");
    let mut depth = 0_i32;
    let mut want_operand = true;
    let mut seen = false;
    for token in spaced.split_whitespace() {
        match token {
            "(" if want_operand => depth += 1,
            ")" if !want_operand => {
                depth -= 1;
                if depth < 0 {
                    return Err(format!("{expression:?}: unbalanced ')'"));
                }
            }
            "AND" | "OR" | "WITH" if !want_operand => want_operand = true,
            id if want_operand
                && id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
                && !matches!(id, "AND" | "OR" | "WITH" | "(" | ")") =>
            {
                want_operand = false;
                seen = true;
            }
            other => {
                return Err(format!(
                    "{expression:?}: unexpected {other:?} (an SPDX expression such as \"MIT OR Apache-2.0\")"
                ));
            }
        }
    }
    if !seen || want_operand || depth != 0 {
        return Err(format!("{expression:?} is not a complete SPDX expression"));
    }
    Ok(())
}
