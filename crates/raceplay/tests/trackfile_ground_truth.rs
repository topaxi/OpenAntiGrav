//! This project's track format against every disc circuit that ships: export,
//! serialise, read back, and compare to what the loader read. Then the strongest
//! check there is, a lone craft raced on the native circuit and on its exported
//! file, tick by tick. Disc-backed; the export is never written to the repository.

use oag_raceplay as race;
use oag_trackfile::{Track, from_parts, to_parts};

const IMAGES: [&str; 3] = [
    "data/images/pulse-psp-eu.chd",
    "data/images/pulse-psp-usa.chd",
    "data/images/pulse-ps2-eu.chd",
];

fn circuits(image: &str) -> Vec<String> {
    let mut archives = oag_pulse::open(image).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob)
        .or_else(|_| oag_tables::fexml::text(&blob))
        .expect("the definition expands or is plain text");
    race::catalogue::tracks(&definition)
        .iter()
        .map(race::catalogue::Track::entry_name)
        .collect()
}

fn options(image: &str, track: &str) -> race::Options {
    race::Options {
        source: image.to_string(),
        track: Some(track.to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    }
}

/// Everything the format holds. `node_index` (an index into a `.vex` tree a
/// file has none of) and `chunks` (byte ranges into the disc payload) do not
/// survive and are not compared; the ADR names both.
fn same(a: &Track, b: &Track) -> Result<(), String> {
    macro_rules! eq {
        ($what:literal, $x:expr, $y:expr) => {
            if $x != $y {
                return Err(format!("{} differs", $what));
            }
        };
    }
    eq!("id", a.id, b.id);
    eq!("title", a.title, b.title);
    eq!("ai", a.ai, b.ai);
    eq!("start", a.start, b.start);
    eq!("speedup pads", a.speedup_pads, b.speedup_pads);
    eq!("weapon pads", a.weapon_pads, b.weapon_pads);
    eq!("notes", a.notes, b.notes);
    eq!("collision node count", a.nodes.len(), b.nodes.len());
    for (i, (x, y)) in a.nodes.iter().zip(&b.nodes).enumerate() {
        eq!("node kind", x.kind, y.kind);
        eq!("node name", x.name, y.name);
        eq!("node version", x.geometry.version, y.geometry.version);
        eq!(
            "node mesh count",
            x.geometry.meshes.len(),
            y.geometry.meshes.len()
        );
        for (m, n) in x.geometry.meshes.iter().zip(&y.geometry.meshes) {
            if m.vertices != n.vertices
                || m.triangles != n.triangles
                || m.vertex_scalars != n.vertex_scalars
            {
                return Err(format!("collision node {i} geometry differs"));
            }
        }
    }
    Ok(())
}

#[test]
#[ignore = "needs the Pulse images"]
fn every_pulse_circuit_exports_and_reads_back_the_same() {
    let mut total = 0;
    let mut refused = Vec::new();
    let mut checked_images = 0;
    for image in IMAGES {
        let Some(path) = oag_testdata::image(image) else {
            continue;
        };
        checked_images += 1;
        let path = path.display().to_string();
        for track in circuits(&path) {
            total += 1;
            let exported = match race::export_track(&options(&path, &track)) {
                Ok(t) => t,
                Err(e) => {
                    refused.push(format!("{image} {track}: export refused: {e}"));
                    continue;
                }
            };
            let parts = match to_parts(&exported, "x.glb") {
                Ok(p) => p,
                Err(e) => {
                    refused.push(format!("{image} {track}: write refused: {e}"));
                    continue;
                }
            };
            let loaded = from_parts(&parts.manifest, &parts.glb)
                .unwrap_or_else(|e| panic!("{image} {track}: reads back: {e}"));
            same(&exported, &loaded.track)
                .unwrap_or_else(|e| panic!("{image} {track}: round trip: {e}"));
            // A second write of what was read is the same bytes.
            assert_eq!(
                parts,
                to_parts(&loaded.track, "x.glb").unwrap(),
                "{image} {track}: not byte-stable"
            );
        }
    }
    if checked_images == 0 {
        return;
    }
    println!("{total} circuit(s) across {checked_images} image(s); refused: {refused:#?}");
    assert!(refused.is_empty(), "{refused:#?}");
}

/// A lone craft on a disc circuit and on the same circuit through its track
/// file: the world hash on every tick must agree.
#[test]
#[ignore = "needs the Pulse PSP EU image"]
fn a_lone_craft_races_a_disc_circuit_and_its_exported_file_identically() {
    let Some(image) = oag_testdata::image(IMAGES[0]) else {
        return;
    };
    let image = image.display().to_string();
    let dir = std::env::temp_dir().join(format!("oag-trackfile-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for track in [
        r"Data\Environments\03_Track\track.vex",
        r"Data\Environments\16_Track\track.vex",
    ] {
        let exported = race::export_track(&options(&image, track)).expect("exports");
        let manifest = dir.join("t.track.toml");
        oag_trackfile::write(&exported, &manifest).unwrap();
        let mut native_options = options(&image, track);
        native_options.difficulty = oag_ai::Difficulty::Ace;
        let mut native = race::Race::start(race::load(&native_options).unwrap().setup);
        let mut from_file = {
            let mut o = native_options.clone();
            o.track = None;
            o.foreign_track = Some(manifest);
            race::Race::start(race::load(&o).unwrap().setup)
        };
        for race in [&mut native, &mut from_file] {
            for slot in 2..8 {
                race.sim.world.ships[slot].active = false;
            }
            race.sim.world.ships[0].active = false;
        }
        let inputs = oag_gameplay::PlayerInputs::none();
        let mut first_difference = None;
        for tick in 0..3_600u64 {
            native.tick(&inputs);
            from_file.tick(&inputs);
            if first_difference.is_none() && native.sim.state_hash() != from_file.sim.state_hash() {
                first_difference = Some(tick);
            }
        }
        assert_eq!(first_difference, None, "{track}: first differing tick");
        let moved = native.sim.world.ships[1].standing.lap;
        let travelled = native.sim.world.ships[1].physics.body.position.length();
        println!("{track}: lap {moved}, |position| {travelled:.0}");
        assert!(travelled > 100.0, "{track}: the AI craft did not drive");
    }
    std::fs::remove_dir_all(dir).ok();
}

fn workshop_items() -> Option<Vec<std::path::PathBuf>> {
    let root = oag_testdata::exact("data/unity/workshop-473770")?;
    let mut dirs: Vec<_> = std::fs::read_dir(root)
        .ok()?
        .map(|e| e.unwrap().path())
        .collect();
    dirs.sort();
    let mut out = Vec::new();
    for d in dirs {
        for e in std::fs::read_dir(d).ok()? {
            let p = e.ok()?.path();
            if p.extension().is_some_and(|x| x == "wtrk") {
                out.push(p);
            }
        }
    }
    Some(out)
}

/// Every Workshop track tagged `Original`: mapped, written in this format, read
/// back. Nothing is lost between the importer's output and the file.
#[test]
#[ignore = "needs data/unity"]
fn every_original_workshop_track_converts_and_reads_back_the_same() {
    let Some(items) = workshop_items() else {
        return;
    };
    let (mut converted, mut refused) = (0, Vec::new());
    for path in items {
        let Ok(file) = std::fs::read(&path) else {
            continue;
        };
        let Ok(w) = oag_ballisticng::wtrk::Wtrk::parse(&file) else {
            continue;
        };
        if !w.has_tag("Original") || w.has_tag("Port/Remake") {
            continue;
        }
        let mapped = match race::foreign::read(&path) {
            Ok(t) => t,
            Err(e) => {
                refused.push(format!("{}: {e}", w.title));
                continue;
            }
        };
        let parts = to_parts(&mapped, "x.glb").unwrap_or_else(|e| panic!("{}: {e}", w.title));
        let back =
            from_parts(&parts.manifest, &parts.glb).unwrap_or_else(|e| panic!("{}: {e}", w.title));
        same(&mapped, &back.track).unwrap_or_else(|e| panic!("{}: {e}", w.title));
        assert_eq!(
            parts,
            to_parts(&back.track, "x.glb").unwrap(),
            "{}: not byte-stable",
            w.title
        );
        converted += 1;
    }
    println!("converted {converted}, refused {refused:#?}");
    assert_eq!(converted + refused.len(), 27, "the census of Originals");
}

/// Lion's Den through `--unity-track` and through its converted file, a lone AI
/// craft on Pulse's physics: the same world hash on every tick.
#[test]
#[ignore = "needs a Pulse image and data/unity"]
fn lions_den_races_the_same_converted_as_loaded_directly() {
    let (Some(items), Some(image)) = (workshop_items(), oag_testdata::image(IMAGES[1])) else {
        return;
    };
    let wtrk = items
        .into_iter()
        .find(|p| p.parent().is_some_and(|d| d.ends_with("1392209921")))
        .expect("Lion's Den is in the census");
    let dir = std::env::temp_dir().join(format!("oag-trackfile-lion-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let manifest = dir.join("lions-den.track.toml");
    oag_trackfile::write(&race::foreign::read(&wtrk).unwrap(), &manifest).unwrap();

    let start = |track: std::path::PathBuf| {
        let o = race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            difficulty: oag_ai::Difficulty::Ace,
            foreign_track: Some(track),
            ..race::Options::default()
        };
        let loaded = race::load(&o).expect("loads");
        let mut race = race::Race::start(loaded.setup);
        for slot in 2..8 {
            race.sim.world.ships[slot].active = false;
        }
        race.sim.world.ships[0].active = false;
        race
    };
    let (mut direct, mut converted) = (start(wtrk), start(manifest));
    let inputs = oag_gameplay::PlayerInputs::none();
    let mut first_difference = None;
    for tick in 0..3_600u64 {
        direct.tick(&inputs);
        converted.tick(&inputs);
        if first_difference.is_none() && direct.sim.state_hash() != converted.sim.state_hash() {
            first_difference = Some(tick);
        }
    }
    assert_eq!(first_difference, None, "first differing tick");
    assert!(direct.sim.world.ships[1].physics.body.position.length() > 100.0);
    std::fs::remove_dir_all(dir).ok();
}

/// A committed track file on Pulse's physics: it loads, the report says what the
/// format carries nothing for, and a lone AI craft gets round it.
fn lone_craft_gets_round(file: &str, id: &str, title: &str) {
    let Some(image) = oag_testdata::image(IMAGES[1]) else {
        return;
    };
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../trackfile/tests/fixtures")
        .join(file);
    let o = race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        foreign_track: Some(manifest),
        ..race::Options::default()
    };
    let loaded = race::load(&o).expect("loads");
    let report = loaded.report.join("\n");
    for needle in [title, "schema v1", "not authored by schema v1", "sky, fog"] {
        assert!(
            report.contains(needle),
            "report lacks {needle:?}:\n{report}"
        );
    }
    assert_eq!(loaded.setup.foreign.as_deref(), Some(id));
    let mut race = race::Race::start(loaded.setup);
    for slot in 2..8 {
        race.sim.world.ships[slot].active = false;
    }
    race.sim.world.ships[0].active = false;
    let inputs = oag_gameplay::PlayerInputs::none();
    for _ in 0..7_200 {
        race.tick(&inputs);
    }
    let lap = race.sim.world.ships[1].standing.lap;
    println!(
        "{file}: lap {lap} after 7200 ticks, respawns {}",
        race.respawns_of(1)
    );
    assert!(lap >= 2, "got round at all: lap {lap}");
}

#[test]
#[ignore = "needs a Pulse image"]
fn a_lone_craft_gets_round_the_committed_fixture() {
    lone_craft_gets_round("oval.track.toml", "fixture-octagon", "Fixture Octagon");
}

/// The add-on's own output (`scripts/blender/oag_track`, `just blender-fixtures`):
/// a loop authored in Blender from eight control points, raced as the committed
/// file. Drop the add-on's spline, start or ribbon and this craft does not get round.
#[test]
#[ignore = "needs a Pulse image"]
fn a_lone_craft_gets_round_a_loop_authored_in_blender() {
    lone_craft_gets_round("blender_loop.track.toml", "blender-loop", "Blender Loop");
}

#[test]
#[ignore = "needs a Pulse image"]
fn a_lone_craft_gets_round_the_oval_re_exported_from_blender() {
    lone_craft_gets_round(
        "oval.reexport.track.toml",
        "fixture-octagon",
        "Fixture Octagon",
    );
}

/// The other titles' default circuits through the same exporter: Pure, HD,
/// Omega and 2048 (a DLC `.vpk`, the only plain-file 2048 source here).
#[test]
#[ignore = "needs the other titles' images"]
fn the_other_titles_default_circuits_round_trip_too() {
    let mut checked = Vec::new();
    for image in [
        "data/images/pure-psp-eu.chd",
        "data/images/hdfury-ps3-eu.iso",
        "data/images/omega-ps4-eu.pkg",
        "data/test-vpk/2048-PCSF00007-dlc1.vpk",
    ] {
        let Some(path) = oag_testdata::exact(image) else {
            continue;
        };
        let o = race::Options {
            source: path.display().to_string(),
            ..race::Options::default()
        };
        let exported = race::export_track(&o).unwrap_or_else(|e| panic!("{image}: {e}"));
        let parts = to_parts(&exported, "x.glb").unwrap_or_else(|e| panic!("{image}: {e}"));
        let back =
            from_parts(&parts.manifest, &parts.glb).unwrap_or_else(|e| panic!("{image}: {e}"));
        same(&exported, &back.track).unwrap_or_else(|e| panic!("{image}: {e}"));
        checked.push(image);
    }
    println!("round-tripped: {checked:?}");
}
