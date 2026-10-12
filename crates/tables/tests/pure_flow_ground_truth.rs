//! [`oag_tables::pure_flow`] against Wipeout Pure's own screens, on both
//! pressings.
//!
//! **`#[ignore]`d and never run in CI**: it needs a disc image. `just test-data`.
//!
//! What it asserts is what the screens' redirects say, evaluated: the menus a
//! player sees and the order of the screens each mode walks. The routes are the
//! ones `docs/formats/race-setup.md` records off the same files, so a reader that
//! stopped following a `Redirect` would fail here rather than quietly walk a
//! different chain.

use oag_tables::pure_flow::{self, Flow, Step};

const IMAGES: [&str; 2] = [
    "data/images/pure-psp-usa.chd",
    "data/images/pure-psp-eu.chd",
];

fn read(image: &str) -> Option<Flow> {
    let path = oag_testdata::image(image)?;
    let mut archives = oag_pure::open(path.to_str().expect("utf-8 path")).expect("open as Pure");
    let skin = archives
        .read_name(oag_pure::names::FRONTEND_ROOT)
        .expect("Skin.xml");
    let selection = archives
        .read_name(oag_pure::names::RACE_BOX_DEFINITION)
        .expect("Selection_Definition.xml");
    Some(
        pure_flow::parse(
            &oag_tables::fexml::text(&skin).expect("text"),
            &oag_tables::fexml::text(&selection).expect("text"),
        )
        .unwrap_or_else(|e| panic!("{image}: {e}")),
    )
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_menus_are_the_ones_the_original_shows() {
    for image in IMAGES {
        let Some(flow) = read(image) else { continue };
        assert_eq!(
            flow.main_modes.first().map(String::as_str),
            Some("Single Player"),
            "{image}"
        );
        assert_eq!(
            flow.modes,
            ["Arcade", "Tournament", "Time Trial", "Zone", "Free Play"],
            "{image}"
        );
        assert_eq!(
            flow.classes,
            ["Vector", "Venom", "Flash", "Rapier", "Phantom"],
            "{image}: five classes authored, whatever a profile offers"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn each_mode_walks_the_screens_the_redirects_name() {
    let league = [
        Step::Class,
        Step::League,
        Step::Track,
        Step::Team,
        Step::Launch,
    ];
    for image in IMAGES {
        let Some(flow) = read(image) else { continue };
        for mode in ["Arcade", "Time Trial", "Free Play"] {
            assert_eq!(flow.route(mode).unwrap(), league, "{image}: {mode}");
        }
        assert_eq!(
            flow.route("Tournament").unwrap(),
            [Step::Class, Step::Tournament, Step::Team, Step::Launch],
            "{image}"
        );
        assert_eq!(
            flow.route("Zone").unwrap(),
            [Step::ZoneTrack, Step::Launch],
            "{image}: Zone has no class, league or team"
        );
    }
}
