//! `<Text DisplayType="TELETYPE">`, against the discs that author it.
//!
//! `#[ignore]`d: needs a real image under `data/images/`. `just test-data`.
//!
//! The census walks every entry of the title's front-end archives that reads as
//! text, parses it as front-end XML, and counts the `<Text>` widgets by their
//! authored `DisplayType`. It exists so the claim "Pulse authors a `TELETYPE`
//! text and the original draws it whole" rests on a count off the disc, and so
//! dropping `oag_ui::screen::Text::display_type` fails a test.

use oag_assets::{Archives, Container};
use oag_tables::fexml;
use oag_ui::screen::{DisplayType, Screens};

/// `(entry index, widget idstring or String, authored value)` for every `Text`
/// carrying a `DisplayType`, read straight off the parsed tree.
fn authored(node: &fexml::Node, out: &mut Vec<(String, DisplayType)>) {
    if node.name.eq_ignore_ascii_case("Text")
        && let Some(value) = node.value("DisplayType")
    {
        let label = node
            .value("idstring")
            .or_else(|| node.value("String"))
            .unwrap_or("")
            .to_string();
        out.push((label, DisplayType::from_authored(value)));
    }
    for child in &node.children {
        authored(child, out);
    }
}

/// Every text entry of every front-end archive, as `(archive, entry, xml)`.
fn front_end_xml(image: &str, title: &'static oag_title::Title) -> Vec<(String, usize, String)> {
    let mut archives = Archives::open(image, title).expect("the archives open");
    let mut out = Vec::new();
    let mut containers: Vec<&mut Container> = vec![&mut archives.data];
    if let Some(fe) = archives.fe.as_mut() {
        containers.push(fe);
    }
    containers.extend(archives.extra.iter_mut());
    for container in containers {
        let wad = match container {
            Container::Wad(wad) => wad,
            Container::Psarc(psarc) => {
                for index in 0..psarc.paths().len() {
                    if !psarc.paths()[index].to_ascii_lowercase().ends_with(".xml") {
                        continue;
                    }
                    let label = psarc.label().to_string();
                    let Ok(raw) = psarc.read(index) else { continue };
                    if let Ok(xml) = fexml::text(&raw) {
                        out.push((label, index, xml));
                    }
                }
                continue;
            }
        };
        let label = wad.label().to_string();
        for index in 0..wad.directory().entries.len() {
            // `fexml::text` wants the whole blob to be UTF-8, so a head that is
            // not (a cut character at its end aside) rules the entry out
            // before it is read and decompressed whole.
            let Ok(head) = wad.peek(index, 64) else { continue };
            if std::str::from_utf8(&head).is_err_and(|e| e.error_len().is_some()) {
                continue;
            }
            let Ok(raw) = wad.read(index) else { continue };
            // A shortened file spells the attribute as one letter, so only a
            // file with neither the name nor a `<code>` dictionary is skipped.
            if !raw.windows(6).any(|w| w == b"<code>")
                && !raw.windows(11).any(|w| w == b"DisplayType")
            {
                continue;
            }
            if let Ok(xml) = fexml::text(&raw) {
                out.push((label.clone(), index, xml));
            }
        }
    }
    out
}

fn census(
    image_name: &str,
    title: &'static oag_title::Title,
) -> Option<Vec<(String, DisplayType)>> {
    let path = oag_testdata::image(image_name)?;
    let mut all = Vec::new();
    for (_, _, xml) in front_end_xml(&path.to_string_lossy(), title) {
        authored(&fexml::parse(&xml), &mut all);
    }
    Some(all)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_authors_teletype_texts_and_nothing_scrolls() {
    let Some(all) = census("pulse-psp-usa.chd", oag_pulse::TITLE) else {
        return;
    };
    let teletype: Vec<&str> = all
        .iter()
        .filter(|(_, d)| *d == DisplayType::Teletype)
        .map(|(l, _)| l.as_str())
        .collect();
    let mut distinct = teletype.clone();
    distinct.sort_unstable();
    distinct.dedup();
    eprintln!("{} distinct labels: {distinct:?}", distinct.len());
    assert!(
        teletype.contains(&"BOOT_PRESS_ANY2"),
        "BOOT_PRESS_ANY2 is authored TELETYPE; found {teletype:?}"
    );
    assert!(
        teletype.contains(&"FE_CONFIRM"),
        "prompts are authored TELETYPE: {teletype:?}"
    );
    assert!(
        all.iter().all(|(_, d)| *d != DisplayType::Scrolling),
        "no Text on the PSP disc authors SCROLLING"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_screens_reader_keeps_what_the_tree_walk_finds() {
    let Some(path) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let mut by_tree = 0;
    let mut by_screens = 0;
    for (_, _, xml) in front_end_xml(&path.to_string_lossy(), oag_pulse::TITLE) {
        let mut found = Vec::new();
        authored(&fexml::parse(&xml), &mut found);
        by_tree += found
            .iter()
            .filter(|(_, d)| *d == DisplayType::Teletype)
            .count();
        by_screens += Screens::from_xml(&xml)
            .screens
            .iter()
            .flat_map(|s| &s.texts)
            .filter(|t| t.display_type == DisplayType::Teletype)
            .count();
    }
    assert!(by_tree > 0, "the disc authors TELETYPE texts");
    assert!(
        by_screens > 0,
        "the Screens reader carries display_type through"
    );
    assert!(by_screens <= by_tree, "the reader never invents one");
    eprintln!("TELETYPE texts: {by_tree} in the tree, {by_screens} reached through Screens");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_other_pressings_census() {
    for (image, title) in [
        ("pulse-psp-eu.chd", oag_pulse::TITLE),
        ("pulse-ps2-eu.chd", oag_pulse::TITLE),
        ("pure-psp-usa.chd", oag_pure::TITLE),
        ("hdfury-ps3-eu.iso", oag_hd::TITLE),
    ] {
        let Some(all) = census(image, title) else {
            continue;
        };
        let count = |wanted| all.iter().filter(|(_, d)| *d == wanted).count();
        assert!(
            count(DisplayType::Teletype) > 0,
            "{image} authors TELETYPE texts"
        );
        assert_eq!(
            count(DisplayType::Scrolling),
            0,
            "{image} authors no SCROLLING text"
        );
        eprintln!(
            "{image}: TELETYPE {} NORMAL {}",
            count(DisplayType::Teletype),
            count(DisplayType::Normal)
        );
    }
}
