//! A PPSSPP texture pack's `textures.ini`.
//!
//! The grammar is the subset both Wipeout packs use, read the way PPSSPP's
//! `TextureReplacer::LoadIniValues` reads it: `[options]`, `[games]` (serial to
//! ini name) and `[hashes]` (`key = file`), with every other section kept by
//! name so a pack that depends on one (`[hashranges]`, `[reducehashranges]`)
//! can be refused rather than misread. See `docs/formats/ppsspp-texture-pack.md`.
//!
//! A `[hashes]` key is PPSSPP's `sscanf("%16llx%8x_%d")`: sixteen hex digits of
//! cache key, optionally eight of data hash, optionally `_<mip level>`. A value
//! may be empty: PPSSPP then never replaces (and, when saving, never re-dumps)
//! that texture.

use std::collections::BTreeMap;

use crate::replacement::Key;

/// One `[hashes]` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// The first sixteen hex digits: address in the high half, `dim` XOR palette
    /// hash in the low.
    pub cache_key: u64,
    /// The eight digits after them, absent for a bare cache-key wildcard.
    pub data_hash: Option<u32>,
    /// The `_N` mip level, 0 when omitted.
    pub level: u32,
    /// The replacement file, relative to the pack. Empty means "leave this
    /// texture alone".
    pub target: String,
}

impl Line {
    /// The key as the ini spells it when it has all 24 digits.
    #[must_use]
    pub fn key(&self) -> Option<Key> {
        Some(Key {
            dim_clut: self.cache_key as u32,
            data: self.data_hash?,
        })
    }

    /// Whether PPSSPP would leave a texture matching this line unreplaced.
    #[must_use]
    pub fn is_blacklist(&self) -> bool {
        self.target.is_empty()
    }
}

/// A parsed `textures.ini`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pack {
    /// `[options]`, key to value as written.
    pub options: BTreeMap<String, String>,
    /// `[games]`: game serial to ini file name.
    pub games: Vec<(String, String)>,
    /// `[hashes]`, in file order.
    pub hashes: Vec<Line>,
    /// `[hashes]` lines whose key is not `%16x%8x_%d`, as written.
    pub unparsed: Vec<String>,
    /// Every other section name with its line count.
    pub other_sections: BTreeMap<String, usize>,
}

impl Pack {
    /// Parses ini text. Never fails: what cannot be read is listed in
    /// [`Self::unparsed`].
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut pack = Self::default();
        let mut section = String::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.split(']').next()) {
                section = name.to_ascii_lowercase();
                continue;
            }
            let (key, value) = match line.split_once('=') {
                Some((k, v)) => (k.trim(), v.trim()),
                None => (line, ""),
            };
            match section.as_str() {
                "options" => {
                    pack.options
                        .insert(key.to_ascii_lowercase(), value.to_string());
                }
                "games" => pack.games.push((key.to_string(), value.to_string())),
                "hashes" => match parse_key(key) {
                    Some((cache_key, data_hash, level)) => pack.hashes.push(Line {
                        cache_key,
                        data_hash,
                        level,
                        target: value.to_string(),
                    }),
                    None => pack.unparsed.push(line.to_string()),
                },
                other => *pack.other_sections.entry(other.to_string()).or_insert(0) += 1,
            }
        }
        pack
    }

    /// The `hash =` option, lower-cased.
    #[must_use]
    pub fn hash_kind(&self) -> Option<String> {
        self.options.get("hash").map(|h| h.to_ascii_lowercase())
    }

    /// A boolean option, PPSSPP's `true`/`false` spelling.
    #[must_use]
    pub fn flag(&self, name: &str) -> bool {
        self.options
            .get(name)
            .is_some_and(|v| v.eq_ignore_ascii_case("true") || v == "1")
    }
}

/// `%16llx%8x_%d`: sixteen hex digits, then up to eight, then `_` and a level.
fn parse_key(key: &str) -> Option<(u64, Option<u32>, u32)> {
    let (digits, level) = match key.split_once('_') {
        Some((d, l)) => (d, l.parse::<u32>().ok()?),
        None => (key, 0),
    };
    if digits.len() < 16 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let cache_key = u64::from_str_radix(&digits[..16], 16).ok()?;
    let rest = &digits[16..];
    let data_hash = if rest.is_empty() {
        None
    } else if rest.len() <= 8 {
        Some(u32::from_str_radix(rest, 16).ok()?)
    } else {
        return None;
    };
    Some((cache_key, data_hash, level))
}

#[cfg(test)]
mod tests;
