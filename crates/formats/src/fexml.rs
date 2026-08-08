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
    fn keeps_attributes_after_an_angle_bracket_in_a_value() {
        // `FEGlobals->TitleColor` is how the real front-end XML references a
        // global. Treating its `>` as the end of the tag drops `color` and
        // spills the rest into the text content.
        let src = concat!(
            r#"<code as="Values" bs="Text" xs="x" cs="color"></code>"#,
            r#"<b><a x="FEGlobals->TitleXOffset" c="FEGlobals->TitleColor"></a></b>"#
        );
        let out = expand(src.as_bytes()).unwrap();
        assert!(out.contains(r#"x="FEGlobals->TitleXOffset""#), "{out}");
        assert!(out.contains(r#"color="FEGlobals->TitleColor""#), "{out}");
        assert!(!out.contains("TitleXOffset\">"), "no truncated tag: {out}");
    }

    #[test]
    fn finds_asset_paths_after_an_angle_bracket_in_a_value() {
        let expanded = r#"<Image y="FEGlobals->Top" src="Data\FE\Images\pulse_logo.mip"></Image>"#;
        assert_eq!(
            asset_paths(expanded),
            vec![r"Data\FE\Images\pulse_logo.mip"]
        );
    }

    #[test]
    fn rejects_a_blob_with_no_dictionary() {
        assert_eq!(expand(b"<Screen/>"), Err(Error::NoDictionary));
    }

    /// The PS2's `Data\XML\*_HUD.xml` shape: plain `<?xml`, no dictionary, and
    /// nothing to expand. `expand` refuses it and `text` must not.
    #[test]
    fn plain_xml_passes_through_text_untouched() {
        let plain = b"<?xml version=\"1.0\" ?>\n<Screen name=\"HUD\"></Screen>";
        assert_eq!(expand(plain), Err(Error::NoDictionary));
        assert_eq!(
            text(plain).as_deref(),
            Ok("<?xml version=\"1.0\" ?>\n<Screen name=\"HUD\"></Screen>")
        );
    }

    #[test]
    fn text_still_expands_a_shortened_blob() {
        let short = br#"<code as="Values" bs="Screen"></code><b><a d="1"></a></b>"#;
        assert_eq!(text(short), expand(short));
        assert!(text(short).expect("expands").contains("<Screen>"));
    }

    #[test]
    fn text_refuses_bytes_that_are_not_utf8() {
        assert_eq!(text(&[0xff, 0xfe, 0x00]), Err(Error::NotText));
    }

    #[test]
    fn identifies_candidates_by_their_leading_bytes() {
        assert!(is_fexml(b"<code as=\"Values\">"));
        assert!(!is_fexml(b"<?xml version=\"1.0\"?>"));
    }

    #[test]
    fn builds_a_tree_from_expanded_text() {
        let root = parse(&expand(SAMPLE.as_bytes()).unwrap());
        assert_eq!(root.name, "#document");
        let screen = &root.children[0];
        assert_eq!(screen.name, "Screen");
        assert_eq!(screen.attr("name"), Some("Top"));
        assert_eq!(screen.children[0].name, "Model");
    }

    #[test]
    fn an_angle_bracket_in_a_value_does_not_end_the_tag() {
        let node = parse(r#"<Text x="FEGlobals->X" y="2"></Text>"#);
        let text = &node.children[0];
        assert_eq!(text.attr("x"), Some("FEGlobals->X"));
        assert_eq!(text.attr("y"), Some("2"));
    }

    #[test]
    fn attributes_match_case_insensitively() {
        let node = parse(r#"<Text Color="0xFF00FF00"></Text>"#);
        assert_eq!(node.children[0].attr("color"), Some("0xFF00FF00"));
    }

    #[test]
    fn unterminated_markup_yields_what_it_opened() {
        let root = parse("<Screen name=\"Top\"><Text");
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].attr("name"), Some("Top"));
    }

    #[test]
    fn a_stray_close_tag_is_ignored() {
        let root = parse("</Screen><Screen name=\"Top\"></Screen>");
        assert_eq!(root.children.len(), 1);
    }

    #[test]
    fn self_closing_elements_close_themselves() {
        let root = parse(r#"<Screen name="Top"><Values x="1"/></Screen>"#);
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].children.len(), 1);
        assert_eq!(root.children[0].value("x"), Some("1"));
    }

    #[test]
    fn an_elements_own_attribute_beats_its_values_carrier() {
        let root = parse(r#"<Movie src="own"><Values src="carrier" sound="true"/></Movie>"#);
        let movie = &root.children[0];
        assert_eq!(movie.value("src"), Some("own"));
        assert_eq!(movie.flag("sound"), Some(true));
        assert_eq!(movie.flag("absent"), None);
    }

    /// The PS2 ship files really do ship this declaration. Its unquoted
    /// `encoding` leaves an odd number of quotes in the tag, so tracking them
    /// across a `>` puts the parser out of phase for the whole file: every
    /// element after it disappears.
    #[test]
    fn a_malformed_declaration_does_not_swallow_the_document() {
        let root = parse(concat!(
            r#" <?xml version="1.0" encoding=utf-81"?>"#,
            "\r\n<Handling>\r\n  <Stats team=\"Testers\"></Stats>\r\n</Handling>",
        ));
        assert_eq!(root.children.len(), 1, "{root:?}");
        assert_eq!(root.children[0].name, "Handling");
        assert_eq!(root.children[0].children[0].attr("team"), Some("Testers"));
    }

    #[test]
    fn a_comment_ends_at_its_own_terminator() {
        let root = parse(r#"<!-- a "quoted > thing --><Screen name="Top"></Screen>"#);
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].attr("name"), Some("Top"));
    }

    #[test]
    fn children_are_selected_by_name_case_insensitively() {
        let root = parse("<Stats><Class/><class/><Misc/></Stats>");
        assert_eq!(root.children[0].children_named("CLASS").count(), 2);
    }
}
