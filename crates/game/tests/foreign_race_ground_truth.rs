//! A foreign (Unity) track raced on Pulse's physics, rules and ship, through the
//! real loader. Needs a Pulse image and the Workshop items, so `#[ignore]`d.
//!
//! What is asserted is what was measured the day it was written (Lion's Den);
//! the all-tracks table is a scratch sweep behind `OAG_SWEEP=1` that prints.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use std::path::PathBuf;

fn item(id: &str) -> Option<PathBuf> {
    let root = oag_testdata::exact("data/ballisticng/workshop-473770")?;
    Some(std::fs::read_dir(root.join(id)).ok()?.next()?.ok()?.path())
}

fn options(track: PathBuf) -> Option<race::Options> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    Some(race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        foreign_track: Some(track),
        ..race::Options::default()
    })
}

#[test]
#[ignore = "needs a disc image and data/ballisticng"]
fn lions_den_loads_and_the_report_names_what_it_did_not_map() {
    let (Some(track), Some(_)) = (
        item("1392209921"),
        oag_testdata::image("data/images/pulse-psp-usa.chd"),
    ) else {
        return;
    };
    let loaded = race::load(&options(track).unwrap()).expect("loads");
    let report = loaded.report.join("\n");
    for needle in [
        "foreign track \"Lion's Den\"",
        "chosen",
        "jump(s)",
        "scenery colliders ignored",
        "not drawn: sky, fog, lights",
        "not mapped: 74 recharge strip",
        "2 collider(s), 2676 triangle(s)",
    ] {
        assert!(
            report.contains(needle),
            "report lacks {needle:?}:\n{report}"
        );
    }
    assert!(loaded.sky_model.is_none() && loaded.fog_volumes.is_empty());
    assert_eq!(loaded.setup.speedup_pads.len(), 48);
    assert_eq!(
        loaded.setup.foreign.as_deref(),
        Some("1392209921"),
        "records key"
    );
    assert!(loaded.setup.course.is_some(), "the ring closes: laps count");
    // Nothing of a disc circuit is borrowed for it.
    assert!(
        !report.contains("Skycube") && !report.contains("fogCube"),
        "{report}"
    );
}

struct Solo {
    laps: u32,
    best: Option<u64>,
    respawns: u32,
}

fn solo(track: PathBuf) -> Option<Solo> {
    let mut o = options(track)?;
    o.difficulty = oag_ai::Difficulty::Ace;
    let loaded = race::load(&o).ok()?;
    let mut race = race::Race::start(loaded.setup);
    for slot in 2..8 {
        race.sim.world.ships[slot].active = false;
    }
    race.sim.world.ships[0].active = false;
    let (mut best, mut lap, mut started) = (None::<u64>, race.sim.world.ships[1].standing.lap, 0);
    let mut recovered = false;
    for tick in 0..18_000u64 {
        let before = race.respawns_of(1);
        race.tick(&PlayerInputs::none());
        recovered |= race.respawns_of(1) != before;
        let now = race.sim.world.ships[1].standing.lap;
        if now != lap {
            if lap > 1 && !recovered {
                best = Some(best.map_or(tick - started, |b| b.min(tick - started)));
            }
            started = tick;
            lap = now;
            recovered = false;
        }
    }
    Some(Solo {
        laps: lap,
        best,
        respawns: race.respawns_of(1),
    })
}

#[test]
#[ignore = "needs a disc image and data/ballisticng"]
fn an_ai_craft_laps_lions_den_on_pulse_physics() {
    let Some(track) = item("1392209921") else {
        return;
    };
    let Some(s) = solo(track) else { return };
    println!(
        "Lion's Den: laps {}, best clean lap {:?} ticks, respawns {}",
        s.laps, s.best, s.respawns
    );
    assert!(s.laps >= 2, "got round at all: {} lap(s)", s.laps);
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1 and read the table"]
fn sweep_every_original_track() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(root) = oag_testdata::exact("data/ballisticng/workshop-473770") else {
        return;
    };
    let mut dirs: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    dirs.sort();
    for d in dirs {
        let Some(path) = std::fs::read_dir(&d)
            .ok()
            .and_then(|mut r| r.next())
            .and_then(Result::ok)
            .map(|e| e.path())
        else {
            continue;
        };
        let Ok(file) = std::fs::read(&path) else {
            continue;
        };
        let Ok(w) = oag_ballisticng::wtrk::Wtrk::parse(&file) else {
            continue;
        };
        if !w.has_tag("Original") || w.has_tag("Port/Remake") {
            continue;
        }
        match solo(path) {
            Some(s) => println!(
                "{:<22} laps {:<3} best clean {:>8} respawns {}",
                w.title,
                s.laps,
                s.best
                    .map_or("none".into(), |b| format!("{:.1}s", b as f32 / 60.0)),
                s.respawns
            ),
            None => println!("{:<22} did not load", w.title),
        }
    }
}
