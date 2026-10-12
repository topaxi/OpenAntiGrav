//! [`oag_tables::pure_campaign`] against Wipeout Pure's own files, on both
//! pressings.
//!
//! **`#[ignore]`d and never run in CI**: it needs a disc image. `just test-data`.
//!
//! The reader refuses an `<Unlock>` attribute it has no branch for, so this is
//! also the proof that every row the disc ships is modelled. What it asserts is
//! shape and agreement between the pressings, never an authored figure
//! (ADR-0006).

use std::collections::BTreeMap;

use oag_tables::pure_ai_stats::PureClass;
use oag_tables::pure_campaign::{self, Definition, EventKind, Stats, TrackKind};

const IMAGES: [&str; 2] = [
    "data/images/pure-psp-usa.chd",
    "data/images/pure-psp-eu.chd",
];

const DEFINITION: &str = r"Data\Plugins\PI001\Definition.xml";

fn read(image: &str) -> Option<(Definition, BTreeMap<String, Stats>)> {
    let path = oag_testdata::image(image)?;
    let mut archives = oag_pure::open(path.to_str().expect("utf-8 path")).expect("open as Pure");
    let blob = archives.read_name(DEFINITION).expect("Definition.xml");
    let definition =
        pure_campaign::parse(&oag_tables::fexml::text(&blob).expect("text")).expect("a definition");
    let stats = definition
        .tracks
        .iter()
        .map(|track| {
            let entry = format!(r"{}\stats.xml", track.location);
            let blob = archives
                .read_name(&entry)
                .unwrap_or_else(|e| panic!("{image}: {entry}: {e}"));
            let text = oag_tables::fexml::text(&blob).expect("text");
            let stats = pure_campaign::parse_stats(&text)
                .unwrap_or_else(|e| panic!("{image}: {entry}: {e}"));
            (track.name.clone(), stats)
        })
        .collect();
    Some((definition, stats))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_definition_reads_whole_and_both_pressings_agree() {
    let mut seen = Vec::new();
    for image in IMAGES {
        let Some(read) = read(image) else { continue };
        seen.push((image, read));
    }
    for (image, (definition, _)) in &seen {
        let count = |kind| definition.tracks.iter().filter(|t| t.kind == kind).count();
        assert_eq!(
            (
                count(TrackKind::Race),
                count(TrackKind::Classic),
                count(TrackKind::Zone)
            ),
            (8, 4, 4),
            "{image}: circuits by kind"
        );
        assert_eq!(definition.leagues.len(), 3, "{image}: leagues");
        assert_eq!(definition.tournaments.len(), 4, "{image}: tournaments");
        let rows = definition
            .tracks
            .iter()
            .map(|t| t.unlocks.len())
            .sum::<usize>()
            + definition
                .leagues
                .iter()
                .map(|s| s.unlocks.len())
                .sum::<usize>()
            + definition
                .tournaments
                .iter()
                .map(|s| s.unlocks.len())
                .sum::<usize>()
            + definition
                .teams
                .iter()
                .map(|t| t.unlocks.len())
                .sum::<usize>();
        assert_eq!(rows, 17, "{image}: the census of <Unlock> rows");
        for series in definition.leagues.iter().chain(&definition.tournaments) {
            for track in &series.tracks {
                assert!(
                    definition.tracks.iter().any(|t| &t.name == track),
                    "{image}: {} lists {track}, which is not a PI_Track",
                    series.name
                );
            }
        }
    }
    println!("{} pressing(s) read", seen.len());
    for pair in seen.windows(2) {
        assert_eq!(
            pair[0].1.0, pair[1].1.0,
            "the pressings author one campaign"
        );
        assert_eq!(pair[0].1.1, pair[1].1.1, "and one set of medal figures");
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_class_gets_an_event_for_every_circuit_it_can_race_and_zone_has_none() {
    for image in IMAGES {
        let Some((definition, stats)) = read(image) else {
            continue;
        };
        for class in PureClass::ALL {
            let list = pure_campaign::events(&definition, &stats, class);
            let count = |kind| list.iter().filter(|e| e.kind == kind).count();
            assert_eq!(count(EventKind::Tournament), 4, "{image} {class:?}");
            assert_eq!(count(EventKind::Race), 12, "{image} {class:?}");
            assert_eq!(count(EventKind::TimeTrial), 12, "{image} {class:?}");
            assert_eq!(count(EventKind::Zone), 4, "{image} {class:?}");
            for event in &list {
                let (g, s, b) = (event.cell.gold, event.cell.silver, event.cell.bronze);
                match event.kind {
                    EventKind::Zone => assert!(g > s && s > b && b > 0, "{image} {}", event.name),
                    _ => assert!(g < s && s < b, "{image} {}", event.name),
                }
            }
        }
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_fresh_profile_opens_the_first_league_and_the_first_zone_and_nothing_gated() {
    for image in IMAGES {
        let Some((definition, stats)) = read(image) else {
            continue;
        };
        let none = |_: &str| None;
        let progress = pure_campaign::Progress {
            class: "Venom",
            medal_count: 0,
            medal_of: &none,
        };
        let open: Vec<String> = pure_campaign::events(&definition, &stats, PureClass::Venom)
            .into_iter()
            .filter(|e| pure_campaign::all_met(&e.unlocks, &progress))
            .map(|e| e.name)
            .collect();
        assert!(
            open.contains(&"tournament/Alpha/Venom".to_string()),
            "{image}"
        );
        assert!(open.contains(&"zone/Zone 1".to_string()), "{image}");
        for locked in [
            "tournament/Beta/Venom",
            "tournament/Classic/Venom",
            "tournament/Championship/Venom",
            "race/Sinucit/Venom",
            "race/Karbonis/Venom",
            "zone/Zone 2",
            "zone/Zone 4",
        ] {
            assert!(
                !open.contains(&locked.to_string()),
                "{image}: {locked} opens unearned"
            );
        }
    }
}
