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
        begun: true,
        cache_file: None,
        held: Default::default(),
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
    scan.begun = true;
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

static NOT_HELD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../trackfile/tests/fixtures/oval.track.toml")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-custom-tracks-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A directory holding only the committed fixture track, so a scan of it finds
/// exactly one file.
fn lone_fixture(name: &str) -> PathBuf {
    let dir = scratch(name);
    let from = fixture();
    let from = from.parent().unwrap();
    for file in ["oval.track.toml", "oval.glb"] {
        std::fs::copy(from.join(file), dir.join(file)).unwrap();
    }
    dir
}

fn finish(scan: &mut Scan) {
    while !scan.poll() {
        std::thread::yield_now();
    }
}

#[test]
fn nothing_is_read_until_the_scan_is_begun() {
    let dir = lone_fixture("lazy");
    let mut scan = Scan::new(std::slice::from_ref(&dir), &[], None);
    assert!(!scan.is_empty(), "the file is found by its directory read");
    assert!(scan.task.is_none(), "no thread before begin");
    assert!(!scan.is_settled());
    assert!(!scan.poll());
    let rows = scan.choices(&StringTable::default());
    assert_eq!(rows.len(), 1);
    assert!(rows[0].unavailable.is_some(), "a row that cannot start yet");
    assert!(scan.raceable(&rows[0].value).is_err());

    scan.begin();
    assert!(scan.task.is_some(), "begin starts the thread");
    finish(&mut scan);
    assert!(scan.is_settled());
    assert_eq!(scan.listings[0].title, "Fixture Octagon");
    assert!(
        scan.raceable(&scan.choices(&StringTable::default())[0].value)
            .is_ok()
    );

    scan.begin();
    assert!(scan.task.is_none(), "a second begin starts nothing");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_held_scan_reads_nothing_until_released() {
    let dir = lone_fixture("held");
    let mut scan = Scan::new(std::slice::from_ref(&dir), &[], None);
    scan.hold(true);
    scan.begin();
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert!(!scan.poll(), "a held survey does not get through its file");
    assert!(!scan.is_settled());
    scan.hold(false);
    finish(&mut scan);
    assert_eq!(scan.listings[0].title, "Fixture Octagon");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_scan_with_no_files_is_settled_and_never_starts_a_thread() {
    let mut scan = Scan::new(&[], &[], None);
    assert!(scan.is_empty() && scan.is_settled());
    scan.begin();
    assert!(scan.task.is_none());
}

#[test]
fn the_cache_is_used_while_path_and_modification_time_match_and_dropped_when_either_moves() {
    let dir = scratch("cache");
    let file = cache::file_in(&dir);
    let files = [fixture()];

    let first = cache::survey(&files, Some(&file), &NOT_HELD);
    assert_eq!(first[0].title, "Fixture Octagon");
    let text = std::fs::read_to_string(&file).expect("the survey wrote its cache");
    assert!(text.contains("Fixture Octagon"));

    // A hit is the cached listing, not a re-read: prove it by changing what the
    // cache says and seeing the change come back.
    std::fs::write(&file, text.replace("Fixture Octagon", "From The Cache")).unwrap();
    assert_eq!(
        cache::survey(&files, Some(&file), &NOT_HELD)[0].title,
        "From The Cache"
    );

    // A moved modification time is a different file.
    let stale = std::fs::read_to_string(&file).unwrap();
    let moved: String = stale
        .lines()
        .map(|l| {
            if l.starts_with("modified_secs") {
                "modified_secs = 1".to_string()
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&file, moved).unwrap();
    assert_eq!(
        cache::survey(&files, Some(&file), &NOT_HELD)[0].title,
        "Fixture Octagon"
    );

    // A cache from another revision, or one that is not TOML, is not trusted.
    std::fs::write(&file, "revision = 0\n").unwrap();
    assert_eq!(
        cache::survey(&files, Some(&file), &NOT_HELD)[0].title,
        "Fixture Octagon"
    );
    std::fs::write(&file, "not toml [").unwrap();
    assert_eq!(
        cache::survey(&files, Some(&file), &NOT_HELD)[0].title,
        "Fixture Octagon"
    );

    // With no cache file every file is read.
    assert_eq!(
        cache::survey(&files, None, &NOT_HELD)[0].title,
        "Fixture Octagon"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_begun_scan_survives_through_the_cache_file_it_was_given() {
    let dir = lone_fixture("scan");
    let file = cache::file_in(&scratch("scan-cache"));
    let mut scan = Scan::new(std::slice::from_ref(&dir), &[], Some(file.clone()));
    scan.begin();
    finish(&mut scan);
    assert!(file.exists(), "the thread writes the cache");
    std::fs::remove_dir_all(dir).ok();
    std::fs::remove_dir_all(file.parent().unwrap()).ok();
}

/// The menu path for an own-format track, in a build without the `ballisticng`
/// feature: the scan lists the file, settles it raceable, and the options it
/// sets reach the same catalogue entry `--track-file` builds.
#[test]
fn an_own_format_track_is_listed_raceable_and_starts_a_race_whatever_the_features() {
    let dir = lone_fixture("menu-path");
    let mut scan = Scan::new(std::slice::from_ref(&dir), &[], None);
    scan.begin();
    finish(&mut scan);
    let strings = StringTable::default();
    let rows = scan.choices(&strings);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].label, "FIXTURE OCTAGON (TRACK FILE)");
    assert!(rows[0].unavailable.is_none());
    let path = scan
        .raceable(&rows[0].value)
        .expect("raceable")
        .to_path_buf();
    let mut options = oag_raceplay::Options::default();
    apply(&mut options, &path);
    let entry = oag_raceplay::foreign::entry(options.foreign_track.as_deref().unwrap())
        .expect("the race's catalogue entry");
    assert_eq!(entry.foreign.as_deref(), Some(path.as_path()));
    std::fs::remove_dir_all(dir).ok();
}

#[cfg(not(feature = "ballisticng"))]
#[test]
fn without_the_feature_discover_looks_for_no_workshop_directory() {
    assert!(super::workshop_search_path().is_empty());
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

#[cfg(feature = "ballisticng")]
mod disc {
    use super::*;

    const ID: &str = "1392209921";

    fn workshop() -> Option<PathBuf> {
        oag_testdata::exact("data/ballisticng/workshop-473770")
    }

    fn scan_all(root: PathBuf) -> Scan {
        let mut scan = Scan::new(&[], &[root], None);
        scan.begin();
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
