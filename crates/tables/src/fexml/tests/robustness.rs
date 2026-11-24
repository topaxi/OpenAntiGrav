//! No byte input makes the front-end XML reader panic: it returns an error, or
//! a tree of whatever it could read. Disc and mod XML both reach it
//! (`oag_ui::language::load::overlay_plugins` forwards `<code`-prefixed bytes).

use super::*;
use crate::rumble;

/// xorshift64: this crate has no dependencies, and a test input must be the same
/// on every machine.
struct Xorshift(u64);

impl Xorshift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

/// Everything a document can be made of, including the characters that split a
/// slice: 2-, 3- and 4-byte characters, and the two (`U+0085`, `U+00A0`) that
/// `char::is_whitespace` accepts but whose bytes are ASCII-unsafe.
const TOKENS: &[&str] = &[
    "<",
    ">",
    "</",
    "/>",
    "\"",
    "=",
    "/",
    "!",
    "?",
    "-",
    "--",
    " ",
    "\n",
    "<!--",
    "-->",
    "<?",
    "?>",
    "<?xml version=\"1.0\"?>",
    "<code",
    "</code>",
    "<code as=\"Values\" bs=\"Screen\">",
    "as=\"Values\"",
    "a",
    "b",
    "x=\"1\"",
    "\u{e9}",
    "\u{85}",
    "\u{a0}",
    "\u{20ac}",
    "\u{1f600}",
    "\u{feff}",
    "<a",
    "<b d=\"",
    "</b>",
    "<a>",
    "</a>",
];

/// A shortened document, well formed, with non-ASCII text in a value.
const REAL_SHAPED: &str = concat!(
    r#"<?xml version="1.0"?>"#,
    "\n",
    r#"<code as="Values" bs="Screen" ds="name" es="Model" ls="Src"></code>"#,
    "\n<!-- a comment -->\n",
    "<b d=\"Caf\u{e9} Info\" x=\"FEGlobals->MenuXOffset\">",
    r#"<e d="Ship"><a l="Data\Ships\Feisar\ship_FE.vex" x="1.0"/></e>"#,
    r#"<a RotY="-0.5"</a>"#,
    "</b>\n",
);

/// Every entry point the claim names, plus the other caller of `tag_end`.
fn exercise(bytes: &[u8]) {
    let _ = is_fexml(bytes);
    let _ = expand(bytes);
    let _ = text(bytes);
    let lossy = String::from_utf8_lossy(bytes);
    let _ = dictionary(&lossy);
    let _ = parse(&lossy);
    let _ = asset_paths(&lossy);
    let _ = rumble::parse(bytes);
    if let Ok(expanded) = expand(bytes) {
        let _ = parse(&expanded);
        let _ = asset_paths(&expanded);
    }
}

#[test]
fn a_code_element_cut_short_by_an_angle_bracket_is_an_error() {
    // `tag_end` ends this tag at index 4, before the dictionary body would start.
    assert_eq!(expand(b"<code<"), Err(Error::NoDictionary));
    assert_eq!(dictionary("<code<"), Err(Error::NoDictionary));
    assert_eq!(dictionary("<code<a"), Err(Error::NoDictionary));
    assert_eq!(expand(b"<code<Foo as=\"x\">"), Err(Error::NoDictionary));
}

#[test]
fn a_multibyte_character_before_an_implied_tag_end_does_not_split_it() {
    let root = parse("<\u{e9}<");
    assert_eq!(root.children.len(), 1, "{root:?}");

    // The character is dropped whole, as the single byte is in
    // `an_unquoted_lt_ends_a_tag_one_character_short_of_it`.
    let root = parse("<Foo \u{20ac}</Foo><Bar/>");
    let names: Vec<_> = root.children.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, ["Foo", "Bar"], "{root:?}");

    let root = parse("<Foo x=\"\u{1f600}\"</Foo>");
    assert_eq!(root.children[0].attr("x"), Some("\u{1f600}"));

    let src = "<code as=\"Values\"></code><a x=\"1\" \u{e9}</a>";
    assert_eq!(
        expand(src.as_bytes()),
        Ok("<Values x=\"1\"></Values>".to_string()),
        "an unterminated tag keeps everything but its last character"
    );
    assert_eq!(asset_paths("<a x=\"d\\f.vex\" \u{e9}<b/>"), ["d\\f.vex"]);
    assert_eq!(
        dictionary("<code as=\"Values\" \u{e9}<"),
        Ok(HashMap::from([("a".to_string(), "Values".to_string())]))
    );
}

#[test]
fn a_rumble_pattern_with_a_multibyte_character_before_an_implied_end_is_an_error() {
    let src = "<RumbleApp><RumbleData><Type \u{e9}<FPS>60</FPS></RumbleData></RumbleApp>";
    assert!(rumble::parse(src.as_bytes()).is_err());
}

#[test]
fn every_truncation_of_real_shaped_xml_is_survived() {
    for sample in [REAL_SHAPED, SAMPLE] {
        let bytes = sample.as_bytes();
        for cut in 0..=bytes.len() {
            exercise(&bytes[..cut]);
            exercise(&bytes[cut..]);
        }
    }
}

#[test]
fn every_single_byte_edit_of_real_shaped_xml_is_survived() {
    let bytes = REAL_SHAPED.as_bytes();
    for at in 0..bytes.len() {
        for replacement in [b'<', b'>', b'"', b'=', b'/', 0xc3, 0x85, 0x00] {
            let mut edited = bytes.to_vec();
            edited[at] = replacement;
            exercise(&edited);
        }
        let mut cut = bytes.to_vec();
        cut.remove(at);
        exercise(&cut);
    }
}

#[test]
fn random_token_soup_is_survived() {
    let mut rng = Xorshift(0x9e37_79b9_7f4a_7c15);
    for _ in 0..4000 {
        let mut doc = String::new();
        if rng.below(2) == 0 {
            doc.push_str("<code as=\"Values\" bs=\"b\"></code>");
        }
        for _ in 0..rng.below(40) {
            doc.push_str(TOKENS[rng.below(TOKENS.len())]);
        }
        exercise(doc.as_bytes());
    }
}

#[test]
fn random_bytes_are_survived() {
    let mut rng = Xorshift(0x2545_f491_4f6c_dd1d);
    for _ in 0..3000 {
        let mut bytes: Vec<u8> = if rng.below(2) == 0 {
            b"<code".to_vec()
        } else {
            Vec::new()
        };
        for _ in 0..rng.below(64) {
            bytes.push(rng.next().to_le_bytes()[0]);
        }
        exercise(&bytes);
    }
}

#[test]
fn mutated_real_shaped_xml_is_survived() {
    let mut rng = Xorshift(0xd1b5_4a32_d192_ed03);
    for _ in 0..3000 {
        let mut doc = REAL_SHAPED.as_bytes().to_vec();
        for _ in 0..1 + rng.below(4) {
            let at = rng.below(doc.len() + 1);
            match rng.below(4) {
                0 => {
                    let token = TOKENS[rng.below(TOKENS.len())];
                    doc.splice(at..at, token.bytes());
                }
                1 => {
                    let end = (at + rng.below(12)).min(doc.len());
                    doc.drain(at..end);
                }
                2 => doc.truncate(at),
                _ => {
                    if let Some(byte) = doc.get_mut(at) {
                        *byte = rng.next().to_le_bytes()[0];
                    }
                }
            }
        }
        exercise(&doc);
    }
}

#[test]
fn nesting_deeper_than_any_file_is_cut_off_rather_than_overflowing_the_stack() {
    let doc = "<a>".repeat(200_000);
    let root = parse(&doc);

    let mut depth = 0;
    let mut node = &root;
    while let Some(child) = node.children.first() {
        depth += 1;
        node = child;
    }
    assert_eq!(depth, MAX_DEPTH, "the tree stops at the nesting limit");
    // Cloning and comparing walk the tree recursively too.
    assert_eq!(root.clone(), root);
}

#[test]
fn nesting_within_the_limit_parses_whole() {
    let doc = format!("{}{}", "<a>".repeat(200), "</a>".repeat(200));
    let mut node = &parse(&doc);
    let mut depth = 0;
    while let Some(child) = node.children.first() {
        depth += 1;
        node = child;
    }
    assert_eq!(depth, 200);
}
