//! The picture on the right-hand side of Wipeout Pure's SINGLE PLAYER screens,
//! read off the disc and drawn through the same path the live screen uses.
//!
//! **`#[ignore]`d and never run in CI.** It needs a disc image. `just test-data`.
//!
//! Each test drops a piece of the wiring if it is missing: no `<Watch>` reader,
//! no `Chain::art_key`, no entry `screen.xml` read, no placeholder fill - and
//! the draw list it checks goes empty or keeps the wrong text.

use oag_game::campaign::launch::plan_cell_for;
use oag_game::campaign::pure_art::{self, Art, Fills, Key, record_entry};
use oag_game::campaign::pure_chain::{Chain, Chosen, choose};
use oag_game::campaign::pure_list;
use oag_game::records::{Record, Store};
use oag_hud::sprite::Sheet;
use oag_tables::pure_ai_stats::PureClass;
use oag_tables::pure_flow::Step;
use oag_ui::frontend::Draw;
use oag_ui::language::StringTable;
use oag_ui_screens::pure_chain::{Choice, Row, Status};

const IMAGES: [&str; 2] = ["pure-psp-eu.chd", "pure-psp-usa.chd"];

struct Disc {
    source: String,
    chain: Chain,
    globals: Vec<(String, String)>,
}

fn disc(image: &str) -> Option<Disc> {
    let path = oag_testdata::image(image)?;
    let source = path.display().to_string();
    let mut archives = oag_pure::open(&source).expect("open as Pure");
    let campaign = pure_list::load(&mut archives, oag_pure::TITLE).expect("the campaign");
    let flow = pure_list::load_flow(&mut archives, oag_pure::TITLE).expect("the flow");
    let globals = style_globals(&mut archives);
    Some(Disc {
        source,
        chain: Chain::new(campaign, flow, PureClass::Vector),
        globals,
    })
}

/// The front end's own globals as a session builds them: the UI skin's and the
/// active style skin's.
fn style_globals(archives: &mut oag_assets::Archives) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in [
        r"Data\Plugins\PI001\GUI\Skin.xml",
        r"Data\Skins\Default\Skin.xml",
    ] {
        let blob = archives.read_name(entry).expect(entry);
        let text = oag_tables::fexml::text(&blob).expect(entry);
        out.extend(oag_ui::screen::Screens::from_xml(&text).globals);
    }
    out
}

fn answer(chain: &mut Chain, mode: &str, rest: &[usize]) {
    let store = Store::default();
    let tag = chain.modes().iter().position(|m| m == mode).expect("mode");
    let mut chosen = choose(chain, tag, oag_pure::TITLE, &store);
    for &tag in rest {
        assert_eq!(chosen, Chosen::Advanced, "{:?}", chain.at());
        chosen = choose(chain, tag, oag_pure::TITLE, &store);
    }
    assert_eq!(chosen, Chosen::Advanced, "{:?}", chain.at());
}

/// Reads `key`'s art and draws it, fully arrived, on a sheet of its textures.
fn drawn(disc: &Disc, key: &Key, fills: &Fills) -> Vec<Draw> {
    drawn_from(disc, key, fills).0
}

/// [`drawn`], and the textures the art names, in the order it draws them.
fn drawn_from(disc: &Disc, key: &Key, fills: &Fills) -> (Vec<Draw>, Vec<String>) {
    let mut archives = oag_pure::open(&disc.source).expect("open as Pure");
    let globals: Vec<(&str, &str)> = disc
        .globals
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let files = oag_pure::TITLE.campaign.flow.expect("a chain");
    let mut report = Vec::new();
    let Art { mut show, stills } = pure_art::load(
        &mut archives,
        key,
        (files.selection, Step::Class.screen()),
        &globals,
        &StringTable::default(),
        &mut report,
    )
    .unwrap_or_else(|error| panic!("{key:?}: {error:#}"));
    assert!(report.is_empty(), "{key:?}: {report:?}");
    if let Key::Entry {
        track: Some(track), ..
    } = key
    {
        fills.apply(track, &mut show);
    }
    let mut sheet_report = Vec::new();
    let sheet = Sheet::build(&stills, &mut sheet_report);
    assert_eq!(sheet.len(), stills.len(), "{sheet_report:?}");
    let sources = show.sources();
    (show.draws(10.0, &|src| sheet.get(src)), sources)
}

fn sprites(draws: &[Draw]) -> Vec<([f32; 4], [f32; 4])> {
    draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, uv, .. } => Some((*rect, *uv)),
            _ => None,
        })
        .collect()
}

fn texts(draws: &[Draw]) -> Vec<String> {
    draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn no_fills(disc: &Disc) -> Fills {
    Fills::of(disc.chain.campaign(), oag_pure::TITLE.name, &[])
}

#[test]
#[ignore = "needs a disc image: just test-data"]
fn class_selection_draws_the_backdrop_and_the_focused_classs_graph_where_the_disc_puts_them() {
    for image in IMAGES {
        let Some(mut disc) = disc(image) else {
            continue;
        };
        answer_to_class(&mut disc.chain);
        let classes: Vec<String> = (0..5)
            .map(|tag| match disc.chain.art_key(tag) {
                Some(Key::Class(name)) => name,
                other => panic!("{image}: class row {tag} has {other:?}"),
            })
            .collect();
        assert_eq!(classes, ["Vector", "Venom", "Flash", "Rapier", "Phantom"]);
        let mut graphs = Vec::new();
        for tag in 0..5 {
            let key = disc.chain.art_key(tag).unwrap();
            let (draws, sources) = drawn_from(&disc, &key, &no_fills(&disc));
            let placed = sprites(&draws);
            assert_eq!(
                placed.len(),
                4,
                "{image} {key:?}: backdrop, frame, two curves"
            );
            let at: Vec<[f32; 2]> = placed.iter().map(|(rect, _)| [rect[0], rect[1]]).collect();
            assert_eq!(
                at,
                [[250.0, 54.0], [252.0, 47.0], [252.0, 47.0], [252.0, 47.0]],
                "{image} {key:?}: `Selection_Definition.xml`'s own positions"
            );
            graphs.push(sources[3].clone());
        }
        graphs.sort();
        graphs.dedup();
        assert_eq!(graphs.len(), 5, "{image}: each class has its own curve");
    }
}

fn answer_to_class(chain: &mut Chain) {
    answer(chain, "Arcade", &[]);
    assert_eq!(chain.at(), Step::Class);
}

#[test]
#[ignore = "needs a disc image: just test-data"]
fn league_and_tournament_selection_draw_the_entrys_own_emblem() {
    for image in IMAGES {
        let Some(mut disc) = disc(image) else {
            continue;
        };
        answer(&mut disc.chain, "Arcade", &[0]);
        assert_eq!(disc.chain.at(), Step::League);
        let mut seen = Vec::new();
        for tag in 0..3 {
            let key = disc
                .chain
                .art_key(tag)
                .unwrap_or_else(|| panic!("{image}: league {tag} has no art"));
            let (draws, sources) = drawn_from(&disc, &key, &no_fills(&disc));
            let placed = sprites(&draws);
            assert_eq!(placed.len(), 1, "{image} {key:?}");
            assert_eq!(
                [placed[0].0[0], placed[0].0[1]],
                [246.0, 45.0],
                "{image} {key:?}: `screen.xml`'s own position"
            );
            seen.push(sources[0].clone());
        }
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 3, "{image}: Alpha, Beta and Classic differ");

        let mut disc = self::disc(image).unwrap();
        answer(&mut disc.chain, "Tournament", &[0]);
        assert_eq!(disc.chain.at(), Step::Tournament);
        let key = disc.chain.art_key(0).expect("Alpha tournament has art");
        let league = Key::Entry {
            location: r"Data\Tournaments\Alpha".into(),
            zone: false,
            track: None,
        };
        assert_eq!(
            key, league,
            "{image}: a league and a tournament share an emblem"
        );
    }
}

#[test]
#[ignore = "needs a disc image: just test-data"]
fn every_circuit_draws_its_map_and_its_panel_with_a_fresh_profiles_values() {
    for image in IMAGES {
        let Some(mut disc) = disc(image) else {
            continue;
        };
        answer(&mut disc.chain, "Arcade", &[0, 0]);
        assert_eq!(disc.chain.at(), Step::Track);
        for tag in 0..4 {
            let key = disc.chain.art_key(tag).expect("a circuit");
            let fills = no_fills(&disc);
            let draws = drawn(&disc, &key, &fills);
            assert!(!sprites(&draws).is_empty(), "{image} {key:?}: the map");
            let said = texts(&draws);
            assert!(
                !said.iter().any(|t| t == "00' 00' 00"),
                "{image} {key:?}: the authored placeholder is on screen: {said:?}"
            );
            let records = said.iter().filter(|t| t.as_str() == "0.00.00").count();
            assert_eq!(records, 2, "{image} {key:?}: race and lap record: {said:?}");
        }
    }
}

#[test]
#[ignore = "needs a disc image: just test-data"]
fn a_stored_result_replaces_the_fresh_record_and_zone_shows_its_medal_counts() {
    for image in IMAGES {
        let Some(mut disc) = disc(image) else {
            continue;
        };
        let location = disc
            .chain
            .campaign()
            .definition
            .tracks
            .iter()
            .find(|t| t.name == "Vineta K")
            .expect("Vineta K")
            .location
            .clone();
        let row = Record {
            title: "wipeout pure".into(),
            track: record_entry(&location),
            mode: "single_race".into(),
            class: "vector".into(),
            best_total_ticks: Some(60 * (60 + 23) + 45),
            best_lap_ticks: Some(60 * 31),
            ..Record::default()
        };
        let fills = Fills::of(disc.chain.campaign(), oag_pure::TITLE.name, &[row]);
        answer(&mut disc.chain, "Arcade", &[0, 0]);
        let key = disc.chain.art_key(0).expect("Vineta K");
        let said = texts(&drawn(&disc, &key, &fills));
        assert!(said.contains(&"1.23.75".to_string()), "{image}: {said:?}");
        assert!(said.contains(&"0.31.00".to_string()), "{image}: {said:?}");

        let mut disc = self::disc(image).unwrap();
        answer(&mut disc.chain, "Zone", &[]);
        assert_eq!(disc.chain.at(), Step::ZoneTrack);
        let key = disc.chain.art_key(0).expect("the first zone");
        assert!(
            matches!(&key, Key::Entry { zone: true, .. }),
            "{image}: {key:?}"
        );
        let said = texts(&drawn(&disc, &key, &no_fills(&disc)));
        for want in ["25", "20", "15", "0"] {
            assert!(
                said.contains(&want.to_string()),
                "{image}: {want} in {said:?}"
            );
        }
        assert!(
            !said.iter().any(|t| t == "00" || t == "00' 00' 00"),
            "{image}: placeholders on screen: {said:?}"
        );

        let zone = disc
            .chain
            .campaign()
            .definition
            .tracks
            .iter()
            .find(|track| format!("{:?}", track.kind) == "Zone")
            .expect("a Zone circuit");
        let held = Record {
            title: "wipeout pure".into(),
            track: record_entry(&zone.location),
            mode: "zone".into(),
            class: "venom".into(),
            best_total_ticks: Some(600),
            ..Record::default()
        };
        let fills = Fills::of(disc.chain.campaign(), oag_pure::TITLE.name, &[held]);
        let said = texts(&drawn(&disc, &key, &fills));
        assert!(
            !said.contains(&"0".to_string()),
            "{image}: a stored Zone result is not shown as the fresh 0: {said:?}"
        );
    }
}

#[test]
#[ignore = "needs a disc image: just test-data"]
fn art_follows_the_focused_row_and_single_player_has_none() {
    for image in IMAGES {
        let Some(mut disc) = disc(image) else {
            continue;
        };
        for tag in 0..5 {
            assert_eq!(
                disc.chain.art_key(tag),
                None,
                "{image}: Single Player row {tag}"
            );
        }
        answer_to_class(&mut disc.chain);
        let mut choice = Choice::new(
            "CLASS".into(),
            String::new(),
            18.0,
            (0..2)
                .map(|tag| Row::event(format!("C{tag}"), tag, Status::Open, String::new()))
                .collect(),
        );
        let focus = |choice: &Choice| {
            disc.chain
                .art_key(choice.selected_row().expect("a row").tag)
        };
        assert_eq!(focus(&choice), Some(Key::Class("Vector".into())));
        choice.select_tag(1);
        assert_eq!(focus(&choice), Some(Key::Class("Venom".into())));
    }
}

#[test]
#[ignore = "needs a disc image: just test-data"]
fn the_record_key_is_the_entry_the_launch_races_for_race_classic_and_zone_events() {
    use oag_tables::pure_campaign::EventKind;
    let ladder: &[&str] = oag_pure::TITLE
        .race
        .speed_classes
        .map_or(&[][..], |classes| classes.names);
    for image in IMAGES {
        let Some(disc) = disc(image) else {
            continue;
        };
        let campaign = disc.chain.campaign();
        let mut archives = oag_pure::open(&disc.source).expect("open as Pure");
        let def = archives
            .read_name(oag_pure::campaign::DEFINITION_ENTRY)
            .expect("Definition.xml");
        let def = oag_tables::fexml::text(&def).expect("text");
        let zones = oag_raceplay::catalogue::tracks_of_kind(&def, "Zone");
        let ordinary = oag_raceplay::catalogue::tracks(&def);
        let mut kinds = std::collections::BTreeSet::new();
        for event in campaign.events(PureClass::Vector) {
            if event.kind == EventKind::Tournament {
                continue;
            }
            let plan = plan_cell_for(&event.cell, "Venom", ladder, |mode, id| {
                let list = if mode == oag_race::Mode::Zone {
                    &zones
                } else {
                    &ordinary
                };
                list.iter()
                    .find(|t| t.id == id)
                    .map(oag_raceplay::catalogue::Track::entry_name)
                    .or_else(|| campaign.track_entry(id))
            })
            .unwrap_or_else(|e| panic!("{image}: {} does not plan: {e}", event.name));
            let track = campaign
                .definition
                .tracks
                .iter()
                .find(|track| track.name == event.subject)
                .unwrap_or_else(|| panic!("{image}: no circuit {}", event.subject));
            assert_eq!(
                plan.leg_entries[0].to_ascii_lowercase(),
                record_entry(&track.location),
                "{image}: {} records under the entry it races",
                event.name
            );
            kinds.insert(format!("{:?}", track.kind));
        }
        assert_eq!(
            kinds.into_iter().collect::<Vec<_>>(),
            ["Classic", "Race", "Zone"],
            "{image}: every kind of circuit was checked"
        );
    }
}
