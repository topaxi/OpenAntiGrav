//! Front-end screen definitions: XML with per-file name shortening.
//!
//! The front end is data-driven. Screens, widgets, 3D model previews and menu
//! transitions are all XML, and the same format appears on PSP and PS2.
//!
//! To save space, element and attribute names are replaced by one- or two-letter
//! codes, with a `<code>` element at the top of each file giving the mapping:
//!
//! ```xml
//! <code as="Values" bs="Screen" cs="Mode3D" ds="name" es="Model" ls="Src"></code>
//! <b d="MPLobby Info">
//!   <c>
//!     <e d="Ship"><a l="Data\Ships\AG_Systems\ship_FE.vex"></a></e>
//!   </c>
//! </b>
//! ```
//!
//! An attribute `Xs="Name"` on `<code>` maps the short name `X` to `Name`, so
//! the above expands to `<Screen name="MPLobby Info"><Mode3D><Model name="Ship">
//! <Values Src="..."/>`.
//!
//! **The dictionary is per file.** `a` means `Values` in one screen and `Class`
//! in another, so a global table would silently corrupt most files. Across the
//! 64 blobs in `FEData.wad`, all 18 short codes carry more than one meaning.
//!
//! This is a size optimisation, not encryption.

use std::collections::HashMap;

/// Something wrong with a front-end XML blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The blob does not start with a `<code>` dictionary.
    NoDictionary,
    /// The blob is not valid UTF-8.
    NotText,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDictionary => f.write_str("no <code> dictionary element"),
            Self::NotText => f.write_str("not valid UTF-8"),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// Whether `data` looks like a shortened front-end XML blob.
#[must_use]
pub fn is_fexml(data: &[u8]) -> bool {
    data.starts_with(b"<code")
}

/// Reads the `<code>` dictionary, mapping short names to full ones.
pub fn dictionary(xml: &str) -> Result<HashMap<String, String>> {
    let start = xml.find("<code").ok_or(Error::NoDictionary)?;
    let end = xml[start..].find('>').ok_or(Error::NoDictionary)? + start;
    let body = &xml[start + 5..end];

    let mut map = HashMap::new();
    for (key, value) in attributes(body) {
        // Keys are the short name with a trailing `s`. Anything else is not
        // part of the mapping and is skipped rather than guessed at.
        if let Some(short) = key.strip_suffix('s') {
            map.insert(short.to_string(), value);
        }
    }

    if map.is_empty() {
        return Err(Error::NoDictionary);
    }
    Ok(map)
}

/// Expands a blob's short names using its own dictionary.
///
/// The `<code>` element itself is dropped: it is metadata, not content.
///
/// Names absent from the dictionary are left as they are, so a partially
/// shortened file still round-trips rather than losing data.
pub fn expand(data: &[u8]) -> Result<String> {
    let xml = std::str::from_utf8(data).map_err(|_| Error::NotText)?;
    let map = dictionary(xml)?;

    // Everything after the dictionary element.
    let start = xml.find("</code>").map_or_else(
        || xml.find('>').map_or(0, |i| i + 1),
        |i| i + "</code>".len(),
    );

    let mut out = String::with_capacity(xml.len() * 2);
    let body = &xml[start..];
    let mut rest = body;

    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        rest = &rest[open..];

        let Some(close) = rest.find('>') else {
            // Unterminated tag: emit the remainder verbatim rather than
            // dropping it, so nothing is silently lost.
            out.push_str(rest);
            return Ok(out);
        };

        out.push_str(&expand_tag(&rest[..=close], &map));
        rest = &rest[close + 1..];
    }
    out.push_str(rest);

    Ok(out)
}

/// Rewrites one `<...>` tag through the dictionary.
fn expand_tag(tag: &str, map: &HashMap<String, String>) -> String {
    let inner = &tag[1..tag.len() - 1];

    // Comments, declarations and processing instructions carry no names.
    if inner.starts_with('!') || inner.starts_with('?') {
        return tag.to_string();
    }

    let (closing, inner) = match inner.strip_prefix('/') {
        Some(rest) => (true, rest),
        None => (false, inner),
    };
    let self_closing = inner.ends_with('/');
    let inner = inner.trim_end_matches('/');

    let name_end = inner
        .find(|c: char| c.is_whitespace())
        .unwrap_or(inner.len());
    let name = &inner[..name_end];
    let full = map.get(name).map_or(name, String::as_str);

    let mut out = String::with_capacity(tag.len() * 2);
    out.push('<');
    if closing {
        out.push('/');
    }
    out.push_str(full);

    for (key, value) in attributes(&inner[name_end..]) {
        let key = map.get(&key).map_or(key.as_str(), String::as_str);
        out.push_str(&format!(" {key}=\"{value}\""));
    }

    if self_closing {
        out.push('/');
    }
    out.push('>');
    out
}

/// Extracts `name="value"` pairs from a tag body.
fn attributes(body: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        while i < bytes.len() && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        let key_start = i;
        while i < bytes.len() && bytes[i] != b'=' && !(bytes[i] as char).is_whitespace() {
            i += 1;
        }
        if key_start == i || i >= bytes.len() || bytes[i] != b'=' {
            break;
        }
        let key = body[key_start..i].to_string();

        i += 1; // '='
        if i >= bytes.len() || bytes[i] != b'"' {
            break;
        }
        i += 1; // opening quote
        let value_start = i;
        while i < bytes.len() && bytes[i] != b'"' {
            i += 1;
        }
        out.push((key, body[value_start..i].to_string()));
        i += 1; // closing quote
    }

    out
}

/// Collects asset paths referenced by a blob.
///
/// The front-end XML is the richest source of real asset names in the game,
/// because most names are built at runtime and never appear in the executable.
/// Feeding these to [`crate::wad::hash_name`] resolves archive entries that
/// binary strings alone cannot.
///
/// Values are recognised as paths by containing a separator and an extension,
/// which is deliberately loose: a false positive costs one wasted hash.
#[must_use]
pub fn asset_paths(expanded: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = expanded;

    while let Some(open) = rest.find('<') {
        rest = &rest[open..];
        let Some(close) = rest.find('>') else { break };

        // Skip the element name: the attribute scanner stops at the first
        // token that is not `name="value"`, so starting on the element name
        // finds nothing at all.
        let inner = rest[1..close].trim_start_matches('/');
        let name_end = inner
            .find(|c: char| c.is_whitespace())
            .unwrap_or(inner.len());

        for (_, value) in attributes(&inner[name_end..]) {
            if looks_like_path(&value) {
                out.push(value);
            }
        }
        rest = &rest[close + 1..];
    }

    out.sort();
    out.dedup();
    out
}

fn looks_like_path(value: &str) -> bool {
    if !value.contains('\\') && !value.contains('/') {
        return false;
    }
    value.rsplit('.').next().is_some_and(|ext| {
        !ext.is_empty() && ext.len() <= 5 && ext.chars().all(char::is_alphanumeric)
    }) && value.contains('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = concat!(
        r#"<code as="Values" bs="Screen" ds="name" es="Model" ls="Src"></code>"#,
        "\n",
        r#"<b d="Top"><e d="Ship"><a l="Data\Ships\Feisar\ship_FE.vex" x="1.0"></a></e></b>"#
    );

    #[test]
    fn reads_the_dictionary() {
        let map = dictionary(SAMPLE).unwrap();
        assert_eq!(map["a"], "Values");
        assert_eq!(map["b"], "Screen");
        assert_eq!(map["l"], "Src");
        assert_eq!(map.len(), 5);
    }

    #[test]
    fn expands_elements_and_attributes() {
        let out = expand(SAMPLE.as_bytes()).unwrap();
        assert!(out.contains(r#"<Screen name="Top">"#), "{out}");
        assert!(out.contains(r#"<Model name="Ship">"#), "{out}");
        assert!(
            out.contains(r#"Src="Data\Ships\Feisar\ship_FE.vex""#),
            "{out}"
        );
        assert!(out.contains("</Screen>"), "closing tags expand too: {out}");
        assert!(!out.contains("<code"), "the dictionary itself is dropped");
    }

    #[test]
    fn leaves_unmapped_names_alone() {
        // `x` is not in the dictionary, so it must survive untouched rather
        // than being dropped or renamed.
        let out = expand(SAMPLE.as_bytes()).unwrap();
        assert!(out.contains(r#"x="1.0""#), "{out}");
    }

    #[test]
    fn the_dictionary_is_per_file() {
        // The same short name means different things in different files, which
        // is why a shared table would corrupt most of them.
        let other = r#"<code as="Class" bs="Difficulty"></code><a b="Hard"></a>"#;
        let out = expand(other.as_bytes()).unwrap();
        assert!(out.contains(r#"<Class Difficulty="Hard">"#), "{out}");
    }

    #[test]
    fn extracts_asset_paths() {
        let expanded = expand(SAMPLE.as_bytes()).unwrap();
        assert_eq!(
            asset_paths(&expanded),
            vec![r"Data\Ships\Feisar\ship_FE.vex"]
        );
    }

    #[test]
    fn does_not_mistake_plain_values_for_paths() {
        assert!(!looks_like_path("1.0"));
        assert!(!looks_like_path("Top"));
        assert!(!looks_like_path("true"));
        assert!(looks_like_path(r"Data\Ships\Feisar\ship_FE.vex"));
        assert!(looks_like_path("data/plugins/grids/grid_00.xml"));
    }

    #[test]
    fn handles_self_closing_tags() {
        let src = r#"<code as="Values"></code><a x="1"/>"#;
        let out = expand(src.as_bytes()).unwrap();
        assert!(out.contains(r#"<Values x="1"/>"#), "{out}");
    }

    #[test]
    fn rejects_a_blob_with_no_dictionary() {
        assert_eq!(expand(b"<Screen/>"), Err(Error::NoDictionary));
    }

    #[test]
    fn identifies_candidates_by_their_leading_bytes() {
        assert!(is_fexml(b"<code as=\"Values\">"));
        assert!(!is_fexml(b"<?xml version=\"1.0\"?>"));
    }
}
