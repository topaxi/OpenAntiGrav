//! Which layout file each circuit's White and Black entry loads, off the disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run with `just test-data`, or alone:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game \
//!     --test pulse_variant_titles_ground_truth --run-ignored all
//! ```
//!
//! The maintainer's ruling of 2026-10-03 is to align with the original, and the
//! original has no fixed "Black is the reversed layout" rule: each `PI_Track`'s
//! display title is its id looked up in the language's `entries.xml`, and which
//! of a circuit's two entries is titled White is per circuit. This test reads
//! all 24 and pins the colour word of each, as a table of ids and colours only
//! (the titles themselves are shipped content and stay on the disc).
//!
//! The table is the disc's, and PPSSPP agreed with it live on 2026-10-03 for
//! Basilico (both), de Konstruct (both) and Metropia White: the craft's start
//! position sat on the layout this table names, not the other one. See
//! `docs/formats/track.md`, "White and Black".

use oag_game::catalogue;

/// `(id, colour the title ends in, whether the entry is the reversed layout)`.
const EXPECTED: &[(&str, &str, bool)] = &[
    ("16_Track", "White", false),
    ("32_Track", "Black", true),
    ("03_Track", "White", false),
    ("19_Track", "Black", true),
    ("18_Track", "White", true),
    ("02_Track", "Black", false),
    ("10_Track", "White", false),
    ("26_Track", "Black", true),
    ("21_Track", "White", true),
    ("05_Track", "Black", false),
    ("04_Track", "White", false),
    ("20_Track", "Black", true),
    ("25_Track", "White", true),
    ("09_Track", "Black", false),
    ("30_Track", "White", true),
    ("14_Track", "Black", false),
    ("17_Track", "White", true),
    ("01_Track", "Black", false),
    ("13_Track", "White", false),
    ("29_Track", "Black", true),
    ("06_Track", "White", false),
    ("22_Track", "Black", true),
    ("07_Track", "White", false),
    ("23_Track", "Black", true),
];

fn titles(image: &str) -> Option<Vec<(catalogue::Track, String)>> {
    let path = oag_testdata::image(image)?;
    let mut archives = oag_pulse::open(&path.display().to_string()).expect("opening Pulse");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let xml = oag_tables::fexml::text(&blob).expect("the definition is not shortened");
    let tracks = catalogue::tracks(&xml);
    let mut english = None;
    for plugin in oag_pulse::LANGUAGE_PLUGINS {
        let Ok(entries) = archives.read_name(&oag_pulse::names::language_entries(plugin)) else {
            continue;
        };
        let table = oag_ui::language::StringTable::from_xml(
            &oag_tables::fexml::expand(&entries).expect("entries.xml expands"),
        );
        english = Some(table);
        break;
    }
    let strings = english.expect("a language table");
    let names = oag_ui::language::CircuitNames::choose(
        &[("language".to_string(), strings.clone())],
        &tracks.iter().map(|t| t.id.clone()).collect::<Vec<_>>(),
    )
    .expect("every circuit is named");
    Some(
        tracks
            .into_iter()
            .map(|t| {
                let title = catalogue::label(&t, &names, &strings, &[]);
                (t, title)
            })
            .collect(),
    )
}

fn check(image: &str) {
    let Some(rows) = titles(image) else { return };
    assert_eq!(
        rows.len(),
        EXPECTED.len(),
        "{image}: the disc declares 24 circuits"
    );
    for ((track, title), (id, colour, reversed)) in rows.iter().zip(EXPECTED) {
        assert_eq!(&track.id, id, "{image}: entry order");
        assert!(
            title.ends_with(colour),
            "{image}: {id} is titled {title:?}, expected a title ending in {colour}"
        );
        assert_eq!(track.reversed, *reversed, "{image}: {id} Reversed flag");
    }
    for pair in rows.chunks(2) {
        let stem = |t: &str| t.rsplit_once(' ').map(|(s, _)| s.to_string());
        assert_eq!(
            stem(&pair[0].1),
            stem(&pair[1].1),
            "{image}: adjacent entries are one circuit's White and Black"
        );
        assert_eq!(
            pair[0].0.location, pair[1].0.location,
            "{image}: a pair shares one environment directory"
        );
        assert_ne!(
            pair[0].0.reversed, pair[1].0.reversed,
            "{image}: one each way"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_usa_disc_titles_every_entry_white_or_black_per_circuit() {
    check("data/images/pulse-psp-usa.chd");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_eu_disc_agrees() {
    check("data/images/pulse-psp-eu.chd");
}
