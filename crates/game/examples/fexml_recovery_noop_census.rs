//! Scratch census for `oag_tables::fexml`'s malformed-tag recovery
//! (`tag_end` now treats an unquoted `<` met mid-tag as a missing `>`, see
//! that function's own doc): parses **every** fexml-candidate entry across
//! every archive this project reads, on every disc this checkout has, once
//! with the OLD pipeline (`tag_end_old`, and - for a shortened blob -
//! `expand_old`/`dictionary_old`, all copied verbatim below) and once with
//! the fixed one (the crate's own `expand`/`parse`), and diffs the two
//! trees.
//!
//! **A shortened blob needs its own old `expand`, not just an old `parse`.**
//! The crate's own `expand` already calls the (now fixed) `tag_end`
//! internally, so feeding its output into `parse_old` would only exercise
//! `parse_old`'s own tree-building, never the part of the old pipeline that
//! actually had the bug for a shortened file (most of Pulse PSP's own
//! `Data.wad`) - that blind spot hid two real instances (Pulse PSP EU/USA's
//! own "stats holder" screen) on an earlier pass of this file that compared
//! `parse_old` against pre-expanded text either way.
//!
//! **Not a plain no-op**: of 2,966 fexml-candidate entries across eight
//! sources (six discs plus the two decrypted PS4/Vita trees), 23 differ -
//! not only the served copies of HD's own `EndRace_Definition.xml` this
//! recovery was written for, but the *same authoring bug* (a start tag
//! missing its own `>`) also shipped in `stats_definition.xml` (HD and
//! Omega), `grid_04.xml` (HD, Omega **and** 2048 - a campaign grid, where
//! the OLD tree loses ten real `PI_Cell` elements as descendants of the
//! wrongly-open tag rather than the grid's own children), 2048's own
//! `HUD_objectives.xml`, Omega's own `SP.xml` (2048's campaign schema, read
//! through `oag_tables::mjolnir` on top of this same tree) and Pulse's own
//! "stats holder" screen, on **all three** platforms (PSP EU, PSP USA, PS2
//! EU x2). One instance (`Data/environments2048/tower/track.pvsxml`) is not
//! fexml this project actually reads at all - `docs/formats/README.md`'s
//! own census marks `.pvsxml` "not opened or compared against" anything, so
//! this project never calls `fexml::parse` on it regardless of what the
//! recovery does there. In every other instance, the OLD tree loses real,
//! named elements as descendants of the wrongly-left-open tag; the NEW tree
//! recovers them as the tree's own authored siblings, never the other way
//! round. **This is a chosen recovery, not a reproduction of measured
//! original behaviour** - see `docs/formats/fexml.md`'s own recovery
//! section for the evidence and why that distinction matters.
//!
//! ```sh
//! cargo run -p oag-game --example fexml_recovery_noop_census
//! ```
//!
//! Not committed as a test: it needs every disc image and every decrypted
//! `data/extracted/` copy this project has - `oag_omega`/`oag_2048::open`
//! both read `Archives`, the same as the WAD titles, but only off a disc
//! image or an already-decrypted directory (their own `DATA_CANDIDATES` doc);
//! their `.pkg`s under `data/images/` are encrypted and not opened directly
//! anywhere in this codebase. Report a file count and any diff, do not
//! commit output.

use oag_assets::Container;
use oag_tables::fexml::Node;

/// Byte-for-byte the pre-recovery `fexml::tag_end`: quote-tracked scan for
/// the first unquoted `>`, comments/PIs ended at their own terminator. No
/// `<`-inside-a-tag recovery at all - this is the function being diffed
/// against.
fn tag_end_old(rest: &str) -> Option<usize> {
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

/// A byte-for-byte copy of `fexml::attributes` (private in that crate).
fn attributes_old(body: &str) -> Vec<(String, String)> {
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

        at += 1;
        if at >= bytes.len() || bytes[at] != b'"' {
            break;
        }
        at += 1;
        let value_at = at;
        while at < bytes.len() && bytes[at] != b'"' {
            at += 1;
        }
        out.push((key, body[value_at..at].to_string()));
        at += 1;
    }

    out
}

/// A byte-for-byte copy of `fexml::parse`, calling `tag_end_old` instead of
/// the (now fixed) crate function.
fn parse_old(xml: &str) -> Node {
    let mut stack = vec![Node {
        name: "#document".to_string(),
        ..Node::default()
    }];
    let mut at = 0usize;

    while at < xml.len() {
        let Some(open) = xml[at..].find('<').map(|i| i + at) else {
            break;
        };
        let Some(close) = tag_end_old(&xml[open..]).map(|i| i + open) else {
            break;
        };
        at = close + 1;

        let inner = &xml[open + 1..close];
        if inner.starts_with('!') || inner.starts_with('?') {
            continue;
        }

        if let Some(name) = inner.strip_prefix('/') {
            let name = name.trim();
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
            attrs: attributes_old(&inner[name_end..]),
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

    while stack.len() > 1 {
        let node = stack.pop().expect("checked above");
        if let Some(parent) = stack.last_mut() {
            parent.children.push(node);
        }
    }
    stack.pop().unwrap_or_default()
}

/// A byte-for-byte copy of `fexml::dictionary`, calling `tag_end_old`.
fn dictionary_old(xml: &str) -> Option<std::collections::HashMap<String, String>> {
    let start = xml.find("<code")?;
    let end = tag_end_old(&xml[start..])? + start;
    let body = &xml[start + 5..end];

    let mut map = std::collections::HashMap::new();
    for (key, value) in attributes_old(body) {
        if let Some(short) = key.strip_suffix('s') {
            map.insert(short.to_string(), value);
        }
    }
    if map.is_empty() { None } else { Some(map) }
}

/// A byte-for-byte copy of `fexml::expand_tag`, calling `attributes_old`.
fn expand_tag_old(tag: &str, map: &std::collections::HashMap<String, String>) -> String {
    let inner = &tag[1..tag.len() - 1];
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
    for (key, value) in attributes_old(&inner[name_end..]) {
        let key = map.get(&key).map_or(key.as_str(), String::as_str);
        out.push_str(&format!(" {key}=\"{value}\""));
    }
    if self_closing {
        out.push('/');
    }
    out.push('>');
    out
}

/// A byte-for-byte copy of `fexml::expand`, running entirely through the
/// pre-recovery `tag_end_old` - **this**, not a call into the crate's own
/// (now fixed) `expand`, is what makes a shortened blob's own comparison
/// meaningful: the crate's `expand` already runs the fixed `tag_end`
/// internally, so diffing its output against itself would hide the
/// recovery on every shortened file (most of Pulse PSP's own `Data.wad`).
fn expand_old(data: &[u8]) -> Option<String> {
    let xml = std::str::from_utf8(data).ok()?;
    let map = dictionary_old(xml)?;

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

        let Some(close) = tag_end_old(rest) else {
            out.push_str(rest);
            return Some(out);
        };

        out.push_str(&expand_tag_old(&rest[..=close], &map));
        rest = &rest[close + 1..];
    }
    out.push_str(rest);

    Some(out)
}

struct Report {
    checked: usize,
    diffs: Vec<String>,
}

/// Reads every entry out of `container` and, for anything that looks like
/// fexml text (starts with `<` after UTF-8 decoding), parses it both ways
/// and compares.
fn census_container(label: &str, container: &mut Container, report: &mut Report) {
    match container {
        Container::Wad(archive) => {
            let count = archive.directory().entries.len();
            for index in 0..count {
                let Ok(data) = archive.read(index) else {
                    continue;
                };
                check_entry(&format!("{label}#{index}"), &data, report);
            }
        }
        Container::Psarc(archive) => {
            let paths: Vec<String> = archive.paths().to_vec();
            for path in paths {
                let Ok(data) = archive.read_path(&path) else {
                    continue;
                };
                check_entry(&format!("{label}:{path}"), &data, report);
            }
        }
    }
}

fn check_entry(name: &str, data: &[u8], report: &mut Report) {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    // A leading UTF-8 BOM (`EF BB BF`) is not Unicode whitespace, so
    // `str::trim_start` leaves it in place - HD's own `EndRace_Definition.xml`
    // ships one, and skipping the BOM here (not in `fexml::parse` itself,
    // which never needed to care: it finds `<` by scanning the whole string,
    // not just the prefix) is what stops this pre-filter from silently
    // dropping the very file this recovery was written for.
    let trimmed = text.trim_start_matches('\u{feff}').trim_start();
    if !trimmed.starts_with('<') {
        return;
    }

    // Both forms this format ships: shortened (`<code`, expand first) and
    // plain (`<?xml`/a bare element). The two sides run **different**
    // expansions when shortened - `expand_old` through `tag_end_old` for
    // the old tree, the crate's own (fixed) `expand` for the new one - so a
    // shortened blob's own recovery is not hidden behind the shared
    // `expand` call both trees would otherwise go through identically.
    let (old_body, new_body) = if oag_tables::fexml::is_fexml(data) {
        let Some(old) = expand_old(data) else { return };
        let Ok(new) = oag_tables::fexml::expand(data) else {
            return;
        };
        (old, new)
    } else {
        (text.to_string(), text.to_string())
    };

    report.checked += 1;
    let old = parse_old(&old_body);
    let new = oag_tables::fexml::parse(&new_body);
    if old != new {
        report.diffs.push(name.to_string());
    }
}

fn census(label: &str, archives: &mut oag_assets::Archives, report: &mut Report) {
    census_container(&format!("{label}/data"), &mut archives.data, report);
    if let Some(fe) = &mut archives.fe {
        census_container(&format!("{label}/fe"), fe, report);
    }
    for (i, extra) in archives.extra.iter_mut().enumerate() {
        census_container(&format!("{label}/extra{i}"), extra, report);
    }
    for (i, pack) in archives.packs.iter_mut().enumerate() {
        census_container(&format!("{label}/pack{i}"), pack, report);
    }
}

fn main() {
    let mut report = Report {
        checked: 0,
        diffs: Vec::new(),
    };

    type Opener = fn(&str) -> anyhow::Result<oag_assets::Archives>;
    let discs: &[(&str, &str, Opener)] = &[
        ("pulse-psp-eu", "data/images/pulse-psp-eu.chd", |s| {
            oag_pulse::open(s).map_err(anyhow::Error::from)
        }),
        ("pulse-psp-usa", "data/images/pulse-psp-usa.chd", |s| {
            oag_pulse::open(s).map_err(anyhow::Error::from)
        }),
        ("pulse-ps2-eu", "data/images/pulse-ps2-eu.chd", |s| {
            oag_pulse::open(s).map_err(anyhow::Error::from)
        }),
        ("pure-psp-eu", "data/images/pure-psp-eu.chd", |s| {
            oag_pure::open(s).map_err(anyhow::Error::from)
        }),
        ("pure-psp-usa", "data/images/pure-psp-usa.chd", |s| {
            oag_pure::open(s).map_err(anyhow::Error::from)
        }),
        ("hdfury-ps3-eu", "data/images/hdfury-ps3-eu-dec.iso", |s| {
            oag_hd::open(s).map_err(anyhow::Error::from)
        }),
        // Both PS4/Vita packages are encrypted `.pkg`s `oag_assets::source::Layout`
        // cannot open directly - `oag_omega::open`/`oag_2048::open` read a disc
        // image *or an already-decrypted extracted directory*
        // (`crates/omega/src/lib.rs`'s own `DATA_CANDIDATES` doc), so these two
        // point at the checkout's own `data/extracted/` copies rather than the
        // raw `.pkg`s under `data/images/`.
        ("omega-ps4-eu", "data/extracted/ps4", |s| {
            oag_omega::open(s).map_err(anyhow::Error::from)
        }),
        ("2048-vita-eu", "data/extracted/vita/PCSF00007", |s| {
            oag_2048::open(s).map_err(anyhow::Error::from)
        }),
    ];

    for (name, path, open) in discs {
        if !std::path::Path::new(path).exists() {
            println!("{name}: {path} not present, skipping");
            continue;
        }
        match open(path) {
            Ok(mut archives) => {
                let before = report.checked;
                census(name, &mut archives, &mut report);
                println!("{name}: {} fexml entries checked", report.checked - before);
            }
            Err(e) => println!("{name}: failed to open: {e}"),
        }
    }

    println!(
        "\nTotal fexml-candidate entries checked: {}",
        report.checked
    );
    if report.diffs.is_empty() {
        println!("No diffs: the recovery is a no-op on every entry checked.");
    } else {
        println!("{} diff(s):", report.diffs.len());
        for d in &report.diffs {
            println!("  {d}");
        }
    }
}
