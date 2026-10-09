//! Front-end screen definitions: XML with per-file name shortening.
//!
//! The front end is data-driven: screens, widgets, 3D model previews and menu
//! transitions are XML, on PSP and PS2 alike.
//!
//! To save space, element and attribute names are replaced by one- or two-letter
//! codes mapped by a `<code>` element at the top of each file:
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
//! **The dictionary is per file.** `a` means `Values` in one screen and `Class` in
//! another; across the 64 blobs in `FEData.wad` all 18 short codes carry more
//! than one meaning, so a global table would corrupt most files. A size
//! optimisation, not encryption.
//!
//! # Two layers
//!
//! [`expand`] undoes the shortening to text; [`parse`] builds a [`Node`] tree.
//! Every consumer needs both (the front end, and [`handling`](crate::handling)),
//! and a second hand-rolled walker per consumer is how a `Values` child or a `>`
//! inside an attribute value gets handled in one place and not the other.

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
        // Keys are the short name plus a trailing `s`; anything else is skipped.
        if let Some(short) = key.strip_suffix('s') {
            map.insert(short.to_string(), value);
        }
    }

    if map.is_empty() {
        return Err(Error::NoDictionary);
    }
    Ok(map)
}

/// A blob's XML text, expanded when shortened and decoded when not.
///
/// **Shortening is per file, not per platform**: PS2's `Skin.xml` is shortened,
/// its five `Data\XML\*_HUD.xml` layouts are plain `<?xml`. Calling [`expand`]
/// directly fails on a plain file ("no `<code>` dictionary element"), which once
/// kept the whole PS2 in-race HUD from loading. [`is_fexml`] decides from the
/// first bytes.
///
/// # Errors
///
/// [`Error::NotText`] for non-UTF-8, and anything [`expand`] raises on a
/// shortened blob. A plain file without a dictionary is not an error here.
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
/// The `<code>` element is dropped as metadata. Names absent from the dictionary
/// stay as they are, so a partly shortened file round-trips. Refuses a blob with
/// no dictionary; for either form use [`text`].
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
            // Unterminated tag: emit the rest verbatim so nothing is lost.
            out.push_str(rest);
            return Ok(out);
        };

        out.push_str(&expand_tag(&rest[..=close], &map));
        rest = &rest[close + 1..];
    }
    out.push_str(rest);

    Ok(out)
}

/// Index of the `>` that closes the tag at the front of `rest`, real or implied.
///
/// A quoted value may contain `>` (`x="FEGlobals->MenuXOffset"`); stopping at the
/// first `>` truncates the tag.
///
/// Comments and processing instructions end at **their own** terminator. The PS2
/// ship files open with a malformed `<?xml version="1.0" encoding=utf-81"?>`
/// whose unquoted `encoding` leaves an odd number of `"`, inverting quote
/// tracking and swallowing the document into one tag; this makes eight shipped
/// files readable.
///
/// **An unquoted `<` also ends a tag, one character short of it.** HD/Fury's
/// `grid_04.xml` (`DATA02`/`DATA04`/`DATA06`, identical) authors `<Values
/// RequiredPoints="22" ... RotY="-0.5"</Values>`, a start tag missing its `>`.
/// Untreated, `Values` never closes and every following sibling (`<Unlock>`,
/// all ten `<PI_Cell>`) becomes its child, so `PI_Grid.children_named("PI_Cell")`
/// finds none: `crates/game/src/campaign.rs` logged `grid4 cells=0` where the
/// raw blob names ten, while RPCS3's campaign-select frame reads `HD 0/87`, the
/// sum of all eight HD grids including `grid4`'s ten. The original's parser
/// tolerates it. The same shape recurs in `stats_definition.xml` and
/// `endrace_definition.xml` (`<Values ... RotY="-0.5"</Values>` trophy/rank
/// blocks, all three HD archives); `rg --no-ignore -n '="[^"]*"</[A-Za-z]'` over
/// a scratch directory, not kept, found no other shape and no false
/// positive.
///
/// The tag ends at `index - 1`, one short of the `<`: callers' `close + 1`
/// resumes at the next tag, and [`attributes`] treats end-of-input as the
/// unterminated value's closing quote, so `"-0.5"` still reads whole. Guarded
/// at `index > 1`, not `> 0`: callers slice `rest[1..close]`, which panics for
/// `close == 0`, so a second-character `<` (a stray `<<`, not on any disc) and
/// the tag's own opening `<` must not end it.
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
            b'<' if !quoted && index > 1 => return Some(index - 1),
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
/// Text content is discarded (no schema here carries any). Attributes keep
/// document order, since some are positional lists.
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
    /// An attribute by name, case-insensitively: the XML mixes `font="menu"` with
    /// `font="Menu"` and `color` with `Color`.
    #[must_use]
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// An attribute of this element or of its `Values` child.
    ///
    /// `Values` is an **attribute carrier for its parent**, a format convention:
    /// `<Movie><Values src="..."/></Movie>` and `<Movie src="..."/>` both occur.
    /// The element's own attribute wins.
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

/// Parses expanded front-end XML into a synthetic `"#document"` root (the files
/// have several top-level elements).
///
/// Deliberately forgiving: an unterminated document yields everything it opened
/// and a stray close tag is ignored. The schema on top decides what is broken;
/// see [`handling::parse`](crate::handling::parse), which errors on a missing
/// attribute because a silent zero would look like a tuning problem.
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
/// Whitespace uses the **ASCII** predicate: `char`'s also matches `U+0085` and
/// `U+00A0`, whose bytes inside a `&str` are only UTF-8 continuation bytes, so a
/// name containing one would be sliced mid-character and panic.
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
/// Front-end XML is the richest source of real asset names, since most are built
/// at runtime; feeding them to `oag_formats::wad::hash_name` resolves entries
/// binary strings cannot. A value counts as a path if it has a separator and an
/// extension, loosely: a false positive costs one wasted hash.
#[must_use]
pub fn asset_paths(expanded: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = expanded;

    while let Some(open) = rest.find('<') {
        rest = &rest[open..];
        let Some(close) = tag_end(rest) else { break };

        // Skip the element name: the attribute scanner stops at the first
        // non-`name="value"` token.
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
