use super::*;
use oag_raceplay::foreign::survey::Kind;

fn listing(title: &str, problem: Option<&str>) -> Listing {
    Listing {
        path: PathBuf::from(format!("/tracks/{title}.wtrk")),
        id: format!("foreign:{title}"),
        title: title.to_string(),
        author: None,
        kind: Kind::Workshop,
        tags: Vec::new(),
        problem: problem.map(str::to_string),
    }
}

fn scanned(listings: Vec<Listing>) -> Scan {
    Scan {
        files: listings.iter().map(|l| l.path.clone()).collect(),
        listings,
        task: None,
    }
}

#[test]
fn a_file_that_will_not_load_is_listed_unusable_with_its_reason_and_never_raced() {
    let scan = scanned(vec![
        listing("Good", None),
        listing("Bad", Some("not a Workshop Track (Version 1)")),
    ]);
    let strings = StringTable::default();
    let choices = scan.choices(&strings);
    assert_eq!(choices.len(), 2, "a broken file is not hidden");
    assert_eq!(choices[0].unavailable, None);
    assert_eq!(
        choices[1].unavailable.as_deref(),
        Some("not a Workshop Track (Version 1)")
    );
    assert_eq!(choices[0].label, "GOOD (WORKSHOP)");
    assert!(scan.raceable(&choices[0].value).is_ok());
    let refused = scan.raceable(&choices[1].value).unwrap_err();
    assert!(
        refused.contains("Bad") && refused.contains("not a Workshop Track"),
        "{refused}"
    );
}

#[test]
fn a_long_title_is_cut_to_fit_and_a_known_author_is_named() {
    let mut long = listing("Gare d'Europa (WipEout2097)", None);
    let text = label(&long);
    assert_eq!(text, "GARE D'EUROPA.. (WORKSHOP)");
    assert!(text.chars().count() <= ROW_WIDTH);
    long.title = "Oval".to_string();
    long.author = Some("Someone".to_string());
    assert_eq!(label(&long), "OVAL BY SOMEONE (WORKSHOP)");
}

#[test]
fn a_value_the_scan_never_found_is_not_a_track() {
    let scan = scanned(vec![listing("Good", None)]);
    assert!(scan.raceable("/elsewhere/other.wtrk").is_err());
}

#[test]
fn files_still_being_read_are_rows_that_cannot_start() {
    let mut scan = scanned(Vec::new());
    scan.files = vec![PathBuf::from("/tracks/lion.wtrk")];
    scan.task = Some(Task::spawn("test", Vec::new));
    let choices = scan.choices(&StringTable::default());
    assert_eq!(choices.len(), 1);
    assert_eq!(choices[0].label, "LION.WTRK");
    assert!(choices[0].unavailable.is_some());
    // Finished with nothing listed: still no track to race.
    while !scan.poll() {
        std::thread::yield_now();
    }
    assert!(scan.raceable("/tracks/lion.wtrk").is_err());
}

#[test]
fn applying_a_custom_track_clears_what_an_earlier_launch_left() {
    let mut options = oag_raceplay::Options {
        track: Some("16_Track".to_string()),
        craft_source: Some("other.chd".to_string()),
        ..oag_raceplay::Options::default()
    };
    apply(&mut options, Path::new("/tracks/lion.wtrk"));
    assert_eq!(
        options.foreign_track.as_deref(),
        Some(Path::new("/tracks/lion.wtrk"))
    );
    assert_eq!(options.track, None);
    assert_eq!(options.craft_source, None);
}

mod disc {
    use super::*;

    const ID: &str = "1392209921";

    fn workshop() -> Option<PathBuf> {
        oag_testdata::exact("data/ballisticng/workshop-473770")
    }

    fn scan_all(root: PathBuf) -> Scan {
        let mut scan = Scan::start(&[root]);
        while !scan.poll() && scan.task.is_some() {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        scan
    }

    /// The scan finds all 32 Workshop items on this host, every one of the 27
    /// tagged `Original` loads (the race reads them the same way), and the
    /// `Port/Remake` ones are listed too: they are a user's files.
    #[test]
    #[ignore = "needs data/ballisticng"]
    fn the_scan_lists_every_workshop_item_and_the_originals_all_load() {
        let Some(root) = workshop() else { return };
        let scan = scan_all(root);
        assert_eq!(scan.listings.len(), 32);
        let originals: Vec<&Listing> = scan
            .listings
            .iter()
            .filter(|l| {
                l.tags.iter().any(|t| t == "Original") && !l.tags.iter().any(|t| t == "Port/Remake")
            })
            .collect();
        assert_eq!(originals.len(), 27, "the census of Originals");
        for listing in &originals {
            assert_eq!(listing.problem, None, "{}", listing.title);
        }
        let strings = StringTable::default();
        let choices = scan.choices(&strings);
        assert_eq!(choices.len(), 32);
        assert!(choices.iter().any(|c| c.label == "LION'S DEN (WORKSHOP)"));
        assert!(
            scan.listings
                .iter()
                .any(|l| l.tags.iter().any(|t| t == "Port/Remake"))
        );
    }

    /// A launch from the CUSTOM row and `--ballisticng-track lion` build the same
    /// race: the same world hash on every tick, a lone AI craft on Pulse.
    #[test]
    #[ignore = "needs a Pulse image and data/ballisticng"]
    fn the_menu_route_and_the_command_line_route_race_lions_den_identically() {
        let (Some(root), Some(image)) = (
            workshop(),
            oag_testdata::image("data/images/pulse-psp-usa.chd"),
        ) else {
            return;
        };
        let scan = scan_all(root.clone());
        let listing = scan
            .listings
            .iter()
            .find(|l| l.id == format!("foreign:{ID}"))
            .expect("Lion's Den is listed");
        let from_menu = {
            // A remix that had raced something else first: stale, then applied.
            let mut options = oag_raceplay::Options {
                source: image.display().to_string(),
                class: "VENOM".to_string(),
                mode: oag_race::Mode::SingleRace,
                difficulty: oag_ai::Difficulty::Ace,
                track: Some("16_Track".to_string()),
                ..oag_raceplay::Options::default()
            };
            let path = scan.raceable(&value(&listing.path)).expect("raceable");
            apply(&mut options, path);
            options
        };
        let from_cli = oag_raceplay::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            difficulty: oag_ai::Difficulty::Ace,
            foreign_track: oag_source::ballisticng::resolve_in(&[root], "lion's den"),
            ..oag_raceplay::Options::default()
        };
        assert_eq!(from_menu.foreign_track, from_cli.foreign_track);
        let start = |options: &oag_raceplay::Options| {
            let loaded = oag_raceplay::load(options).expect("loads");
            let mut race = oag_raceplay::Race::start(loaded.setup);
            for slot in 2..8 {
                race.sim.world.ships[slot].active = false;
            }
            race.sim.world.ships[0].active = false;
            race
        };
        let (mut menu, mut cli) = (start(&from_menu), start(&from_cli));
        let inputs = oag_gameplay::PlayerInputs::none();
        for tick in 0..1_800u64 {
            menu.tick(&inputs);
            cli.tick(&inputs);
            assert_eq!(menu.sim.state_hash(), cli.sim.state_hash(), "tick {tick}");
        }
        assert!(menu.sim.world.ships[1].physics.body.position.length() > 100.0);
    }
}
