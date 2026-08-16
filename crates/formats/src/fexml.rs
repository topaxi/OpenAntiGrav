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
//!
//! # Two layers
//!
//! [`expand`] undoes the shortening and hands back text. [`parse`] turns that
//! text into a [`Node`] tree. Both are here because every consumer needs both:
//! the front end reads screens out of it, and
//! [`handling`](crate::handling) reads `handlingstats.xml`, which is the same
//! format with a different schema. A second hand-rolled tree walker in each
//! consumer is how a `Values` child or a `>` inside an attribute value comes to
//! be handled correctly in one place and not the other.

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
    let end = tag_end(&xml[start..]).ok_or(Error::NoDictionary)? + start;
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

/// A blob's XML text, expanded when it is shortened and decoded when it is not.
///
/// **Shortening is per file, not per platform**, and the PS2 uses it much less:
/// its `Skin.xml` is shortened, its five `Data\XML\*_HUD.xml` layouts are plain
/// `<?xml`. A caller that reaches straight for [`expand`] therefore fails on a
/// perfectly good file with "no `<code>` dictionary element", which is what the
/// PS2 in-race HUD did - the layout never loaded and the whole HUD went with
/// it. Deciding from the blob rather than from the source is what stops that:
/// [`is_fexml`] is a two-way test on the first bytes, so neither form can be
/// mistaken for the other.
///
/// # Errors
///
/// [`Error::NotText`] when the blob is not UTF-8, and everything [`expand`] can
/// raise when it *is* shortened. A plain file with no dictionary is not an
/// error here, which is the whole difference from [`expand`].
pub fn text(data: &[u8]) -> Result<String> {
    if is_fexml(data) {
        expand(data)
    } else {
        std::str::from_utf8(data)
            .map(str::to_owned)
            .map_err(|_| Error::NotText)
    }
}

/// Expands a blob's short names using its own dictionary.
///
/// The `<code>` element itself is dropped: it is metadata, not content.
///
/// Names absent from the dictionary are left as they are, so a partially
/// shortened file still round-trips rather than losing data.
///
/// Refuses a blob that carries no dictionary. Callers reading a file that may
/// be either form want [`text`] instead.
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

        let Some(close) = tag_end(rest) else {
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

/// Index of the `>` that closes the tag at the front of `rest`.
///
/// A quoted attribute value may contain `>`. The front end's own XML is full of
/// values like `x="FEGlobals->MenuXOffset"`, and stopping at the first `>`
/// truncates the tag, dropping every attribute after it and emitting the
/// remainder as text.
///
/// Comments and processing instructions end at **their own** terminator rather
/// than at the first unquoted `>`, and that is not pedantry. The PS2 ship files
/// open with a malformed declaration, `<?xml version="1.0" encoding=utf-81"?>`:
/// the unquoted `encoding` leaves an odd number of `"` in the tag, which inverts
/// quote tracking for the whole rest of the file and swallows the entire document
/// into one unterminated tag. Ending the declaration where XML says it ends costs
/// two lines and makes eight shipped files readable.
fn tag_end(rest: &str) -> Option<usize> {
    if let Some(body) = rest.strip_prefix("<!--") {
        return body.find("-->").map(|at| at + "<!--".len() + 2);
    }
    if rest.starts_with("<?") {
        return rest.find("?>").map(|at| at + 1);
    }

    let mut quoted = false;
    for (index, byte) in rest.bytes().enumerate() {
        match byte {
            b'"' => quoted = !quoted,
            b'>' if !quoted => return Some(index),
            _ => {}
        }
    }
    None
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

/// A parsed XML element.
///
/// Text content is discarded: no schema in this format carries any. Attributes
/// keep document order, because some of them are positional lists and a
/// `HashMap` would reorder them differently on every run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Node {
    /// Element name, as it appears after [`expand`] has run.
    pub name: String,
    /// Attributes in document order.
    pub attrs: Vec<(String, String)>,
    /// Child elements in document order.
    pub children: Vec<Node>,
}

impl Node {
    /// An attribute by name, case-insensitively.
    ///
    /// The XML is inconsistent about case (`font="menu"` and `font="Menu"` both
    /// appear, as do `color` and `Color`), so matching exactly would silently
    /// drop attributes.
    #[must_use]
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// An attribute of this element or of its `Values` child.
    ///
    /// `Values` is an **attribute carrier for its parent**, a convention of the
    /// format rather than of any one schema: `<Movie><Values src="..."/></Movie>`
    /// and `<Movie src="..."/>` mean the same thing, and both spellings occur.
    /// The element's own attribute wins, so a carrier cannot shadow it.
    #[must_use]
    pub fn value(&self, name: &str) -> Option<&str> {
        self.attr(name).or_else(|| {
            self.children
                .iter()
                .filter(|c| c.name.eq_ignore_ascii_case("Values"))
                .find_map(|c| c.attr(name))
        })
    }

    /// A boolean attribute. True for `"true"` or `"1"`, as
    /// `Movie_ParseAttributes` has it.
    #[must_use]
    pub fn flag(&self, name: &str) -> Option<bool> {
        self.value(name)
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
    }

    /// Children with this element name, case-insensitively.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> {
        self.children
            .iter()
            .filter(move |c| c.name.eq_ignore_ascii_case(name))
    }
}

/// Parses expanded front-end XML into a synthetic root node.
///
/// The files have several top-level elements, so the root is `"#document"`
/// rather than any element from the file.
///
/// This is deliberately forgiving. The goal is to read whatever the shipped
/// files contain, not to validate them: an unterminated document still yields
/// everything it opened, and a stray close tag is ignored rather than unwinding
/// past the root. A schema on top of this decides what counts as broken - see
/// [`handling::parse`](crate::handling::parse), which turns a missing element or
/// attribute into a typed error precisely because silently defaulting one to
/// zero would look like a tuning problem rather than a bug.
#[must_use]
pub fn parse(xml: &str) -> Node {
    let mut stack = vec![Node {
        name: "#document".to_string(),
        ..Node::default()
    }];
    let mut at = 0usize;

    while at < xml.len() {
        let Some(open) = xml[at..].find('<').map(|i| i + at) else {
            break;
        };
        let Some(close) = tag_end(&xml[open..]).map(|i| i + open) else {
            break;
        };
        at = close + 1;

        let inner = &xml[open + 1..close];
        // Comments, declarations and processing instructions carry no structure.
        if inner.starts_with('!') || inner.starts_with('?') {
            continue;
        }

        if let Some(name) = inner.strip_prefix('/') {
            let name = name.trim();
            // Tolerate a stray close tag rather than unwinding past the root.
            if stack.len() > 1
                && stack
                    .last()
                    .is_some_and(|n| n.name.eq_ignore_ascii_case(name))
            {
                let node = stack.pop().expect("checked above");
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                }
            }
            continue;
        }

        let self_closing = inner.ends_with('/');
        let inner = inner.trim_end_matches('/');
        let name_end = inner.find(char::is_whitespace).unwrap_or(inner.len());
        let node = Node {
            name: inner[..name_end].to_string(),
            attrs: attributes(&inner[name_end..]),
            children: Vec::new(),
        };

        if self_closing {
            if let Some(parent) = stack.last_mut() {
                parent.children.push(node);
            }
        } else {
            stack.push(node);
        }
    }

    // An unterminated document still yields everything it did open.
    while stack.len() > 1 {
        let node = stack.pop().expect("checked above");
        if let Some(parent) = stack.last_mut() {
            parent.children.push(node);
        }
    }
    stack.pop().unwrap_or_default()
}

/// Extracts `name="value"` pairs from a tag body.
///
/// Whitespace is tested with the **ASCII** predicate on purpose. `char`'s
/// version also matches `U+0085` and `U+00A0`, whose bytes inside a `&str` are
/// only ever UTF-8 continuation bytes, so a name containing one would be sliced
/// mid-character and panic.
fn attributes(body: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = body.as_bytes();
    let mut at = 0usize;

    while at < bytes.len() {
        while at < bytes.len() && bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        let key_at = at;
        while at < bytes.len() && bytes[at] != b'=' && !bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        if key_at == at || at >= bytes.len() || bytes[at] != b'=' {
            break;
        }
        let key = body[key_at..at].to_string();

        at += 1; // '='
        if at >= bytes.len() || bytes[at] != b'"' {
            break;
        }
        at += 1; // opening quote
        let value_at = at;
        while at < bytes.len() && bytes[at] != b'"' {
            at += 1;
        }
        out.push((key, body[value_at..at].to_string()));
        at += 1; // closing quote
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
        let Some(close) = tag_end(rest) else { break };

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
mod tests;
