//! Which ships author a per-ship `<Assist>` (2048's Normal level), and with which numbers.
//!
//! **`#[ignore]`d and never run in CI**: it needs the 2048 and Omega unpacked folders and
//! the HD, Pulse and Pure images.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test pilot_assist_levels_ground_truth --run-ignored all --no-capture
//! ```

use oag_core::buttons::{Button, Input};
use oag_gameplay::{InputSnapshot, PlayerInputs};
use oag_physics::pilot_assist::{Law, Laws, Level};
use oag_raceplay::{self as race, Race};
use oag_tables::handling::{self, Assist};

const HD: &str = "data/images/hdfury-ps3-eu-dec.iso";
const PULSE: &str = "data/images/pulse-psp-eu.chd";
const PURE: &str = "data/images/pure-psp-eu.chd";
const V2048: &str = "data/extracted/vita/PCSF00007";
const OMEGA: &str = "data/extracted/ps4";
/// The countdown and about 25 seconds of racing.
const TICKS: u64 = 1800;

/// Every craft file a title's roster ships, as `(name, entry)`.
fn craft_files(title: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let native = |dir: &str, teams: &[&str]| {
        let mut v = Vec::new();
        for team in teams {
            for n in 1..=4 {
                v.push((
                    format!("{team}\\{n}"),
                    format!(r"{dir}\{team}\{n}\handlingstats.xml"),
                ));
            }
        }
        v
    };
    match title {
        "2048" => {
            out.extend(native(
                oag_2048::race::HANDLING_DIR,
                &oag_2048::race::NATIVE_TEAMS,
            ));
            for team in oag_2048::race::GUEST_TEAMS {
                out.push((
                    team.to_string(),
                    format!(r"{}\{team}\handlingstats.xml", oag_2048::race::HD_SHIP_DIR),
                ));
            }
        }
        "Omega" => {
            out.extend(native(
                oag_omega::race::ERA_2048_HANDLING_DIR,
                &oag_2048::race::NATIVE_TEAMS,
            ));
        }
        _ => {}
    }
    out
}

fn open(title: &str) -> Option<oag_assets::Archives> {
    match title {
        "2048" => oag_testdata::exact("data/extracted/vita/PCSF00007")
            .map(|p| oag_2048::open(&p.display().to_string()).unwrap()),
        "Omega" => oag_testdata::exact("data/extracted/ps4")
            .map(|p| oag_omega::open(&p.display().to_string()).unwrap()),
        _ => None,
    }
}

/// Loads `source`'s default team and track at `class`, and the report that came with it.
fn load(source: &std::path::Path, class: &str, track: Option<&str>) -> (Race, Vec<String>) {
    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::TimeTrial,
        weapons_override: Some(false),
        track: track.map(str::to_string),
        ..race::Options::default()
    })
    .expect("loading the race");
    let report = loaded.report.clone();
    (Race::start(loaded.setup), report)
}

fn laws(path: &str, class: &str) -> Option<(Laws, Vec<String>)> {
    let source = oag_testdata::exact(path).or_else(|| oag_testdata::image(path))?;
    let (race, report) = load(&source, class, None);
    Some((race.pilot_assist_laws(), report))
}

const CLASSES: [&str; 4] = ["VENOM", "FLASH", "RAPIER", "PHANTOM"];

/// 2048 and Omega author `<Assist>` on every class of all 20 native craft, and the
/// same ten numbers on every one: the set Pulse, Pure and HD run as their Normal.
/// Their twelve HD-derived guest teams author none.
#[test]
#[ignore = "needs the 2048 and Omega unpacked folders"]
fn every_native_craft_authors_the_same_assist() {
    let mut seen: Vec<Assist> = Vec::new();
    for title in ["2048", "Omega"] {
        let Some(mut archives) = open(title) else {
            continue;
        };
        let files = craft_files(title);
        let mut native = 0;
        for (name, entry) in &files {
            let blob = archives.read_name(entry).expect("a roster file");
            let stats = handling::from_blob(&blob).unwrap();
            let guest = !name.contains('\\');
            for class in &stats.classes {
                match (guest, class.assist) {
                    (false, Some(a)) => {
                        if !seen.contains(&a) {
                            seen.push(a);
                        }
                        native += 1;
                    }
                    (true, None) => {}
                    (guest, assist) => panic!(
                        "{title} {name} {}: guest {guest}, {assist:?}",
                        class.raw_name
                    ),
                }
            }
        }
        println!(
            "{title}: {} files, {native} native class blocks",
            files.len()
        );
        assert_eq!(native, 100, "{title}: 20 craft by 5 classes");
    }
    assert_eq!(seen.len(), 1, "one block everywhere: {seen:?}");
}

/// Pulse, Pure and HD author no `<Assist>`: their Normal is the 2048 block, class by
/// class, and the ramp is 2048's. Extreme on Pulse and Pure is 2048's Extreme rungs;
/// on HD it is HD's own, which is not.
#[test]
#[ignore = "needs every title's source"]
fn titles_without_a_level_run_2048s_and_labelled_so() {
    let Some(v2048) = oag_testdata::exact(V2048) else {
        return;
    };
    for class in CLASSES {
        let (reference, report) = load(&v2048, class, None);
        let reference = reference.pilot_assist_laws();
        assert!(
            report
                .iter()
                .any(|l| l.contains("Normal: authored <Assist>")),
            "{report:?}"
        );
        let (normal, extreme) = (reference.normal.unwrap(), reference.extreme.unwrap());
        assert_eq!(normal.ramp.unwrap().full, 0.1);
        for (title, path) in [("Pulse", PULSE), ("Pure", PURE), ("HD", HD)] {
            let Some((got, report)) = laws(path, class) else {
                continue;
            };
            assert_eq!(got.normal, Some(normal), "{title} {class} Normal");
            assert!(
                report
                    .iter()
                    .any(|l| l.contains("Normal:") && l.contains("chosen, not measured")),
                "{title}: {report:?}"
            );
            let law: Law = got.extreme.unwrap();
            if title == "HD" {
                assert_ne!(law, extreme, "HD's Extreme is its own authored rung");
                assert!(
                    report.iter().any(|l| l.contains("Extreme: authored")),
                    "{report:?}"
                );
            } else {
                assert_eq!(law, extreme, "{title} {class} Extreme");
                assert!(
                    report
                        .iter()
                        .any(|l| l.contains("Extreme:") && l.contains("chosen, not measured")),
                    "{title}: {report:?}"
                );
            }
        }
        if let Some((omega, _)) = laws(OMEGA, class) {
            assert_eq!(omega.normal, Some(normal), "Omega {class}");
            assert_eq!(omega.extreme, Some(extreme), "Omega {class}");
        }
    }
}

fn thrust() -> PlayerInputs {
    let mut buttons = Input::new();
    buttons.begin_frame(Button::Cross.bit());
    PlayerInputs::single(InputSnapshot {
        buttons,
        ..InputSnapshot::EMPTY
    })
}

/// Wall-contact ticks, separate contacts, distance round the lap, and the peak speed.
fn fly(race: &mut Race) -> (u32, u32, f32, f32) {
    let inputs = thrust();
    let (mut ticks, mut contacts, mut peak) = (0, 0, 0.0f32);
    let mut touching = false;
    for _ in 0..TICKS {
        race.tick(&inputs);
        let ship = &race.sim.world.ships[0].physics;
        let now = ship.wall_contact_prev;
        ticks += u32::from(now);
        contacts += u32::from(now && !touching);
        touching = now;
        peak = peak.max(ship.body.linear_velocity.length());
    }
    let progress = race.sim.world.ships[0]
        .standing
        .distance(race.course().expect("a course"));
    (ticks, contacts, progress, peak)
}

fn demo(title: &str, path: &str, track: Option<&str>) {
    let Some(source) = oag_testdata::image(path) else {
        return;
    };
    let mut results = Vec::new();
    for level in Level::ALL {
        let (mut race, _) = load(&source, "VENOM", track);
        race.set_pilot_assist(level);
        let (ticks, contacts, progress, peak) = fly(&mut race);
        let ran = race.sim.world.ships[0].physics.pilot_assist != Default::default();
        println!(
            "{title} {:7}: {contacts:2} wall contacts, {ticks:4} ticks on a wall, \
             {progress:5.0} units round the lap, peak {peak:.0} u/s, assist state touched: {ran}",
            level.name()
        );
        results.push((level, ticks, contacts, progress, ran));
    }
    let [off, normal, extreme] = [&results[0], &results[1], &results[2]];
    assert!(!off.4, "Off never runs the law");
    assert!(normal.4 && extreme.4, "Normal and Extreme both ran it");
    assert!(
        extreme.2 < off.2 && extreme.1 < off.1 && extreme.3 > off.3,
        "Extreme must beat Off: {results:?}"
    );
}

/// The demo a player is: thrust held from the start, the stick never touched. The
/// numbers are printed for Normal, not asserted against an ordering.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_levels_off_normal_extreme() {
    demo("HD", HD, Some(oag_hd::race::DEFAULT_TRACK));
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_levels_off_normal_extreme() {
    let Some(source) = oag_testdata::image(PULSE) else {
        return;
    };
    let mut results = Vec::new();
    for level in Level::ALL {
        let (mut race, _) = load(&source, "VENOM", None);
        race.set_pilot_assist(level);
        let (ticks, contacts, progress, peak) = fly(&mut race);
        let ran = race.sim.world.ships[0].physics.pilot_assist != Default::default();
        println!(
            "Pulse {:7}: {contacts:2} wall contacts, {ticks:4} ticks on a wall, \
             {progress:5.0} units round the lap, peak {peak:.0} u/s, assist state touched: {ran}",
            level.name()
        );
        results.push(ran);
    }
    assert_eq!(results, [false, true, true]);
}

/// The AI flies the player's physics: no opponent gets the assist at any level.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn no_opponent_ever_gets_either_level() {
    let Some(source) = oag_testdata::image(HD) else {
        return;
    };
    for level in [Level::Normal, Level::Extreme] {
        let loaded = race::load(&race::Options {
            source: source.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            weapons_override: Some(false),
            track: Some(oag_hd::race::DEFAULT_TRACK.to_string()),
            ..race::Options::default()
        })
        .unwrap();
        let mut race = Race::start(loaded.setup);
        race.set_pilot_assist(level);
        let inputs = thrust();
        for _ in 0..900 {
            race.tick(&inputs);
        }
        assert_ne!(
            race.sim.world.ships[0].physics.pilot_assist,
            Default::default(),
            "{level:?}"
        );
        for slot in 1..race.sim.world.ship_count as usize {
            assert_eq!(
                race.sim.world.ships[slot].physics.pilot_assist,
                Default::default(),
                "{level:?} slot {slot}"
            );
        }
    }
}
