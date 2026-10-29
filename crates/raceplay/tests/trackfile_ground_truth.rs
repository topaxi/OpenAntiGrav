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

/// Pure's images, walked by the same export check: its circuits are the other
/// half of ADR-0076's art proof.
const PURE_IMAGES: [&str; 2] = [
    "data/images/pure-psp-eu.chd",
    "data/images/pure-psp-usa.chd",
];

fn is_pure(image: &str) -> bool {
    PURE_IMAGES.contains(&image)
}

fn open(image: &str, path: &str) -> oag_assets::Archives {
    if is_pure(image) {
        oag_pure::open(path).expect("mounting the disc")
    } else {
        oag_pulse::open(path).expect("mounting the disc")
    }
}

fn circuits(image: &str, path: &str) -> Vec<String> {
    let mut archives = open(image, path);
    let definition = if is_pure(image) {
        oag_pure::names::GAME_PLUGIN_DEFINITION
    } else {
        oag_pulse::names::GAME_PLUGIN_DEFINITION
    };
    let blob = archives
        .read_name(definition)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob)
        .or_else(|_| oag_tables::fexml::text(&blob))
        .expect("the definition expands or is plain text");
    race::catalogue::tracks(&definition)
        .iter()
        .map(race::catalogue::Track::entry_name)
        .collect()
}

#[path = "trackfile_ground_truth/art_proof.rs"]
mod art_proof;

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
    eq!("host", a.host, b.host);
    eq!("ai", a.ai, b.ai);
    eq!("start", a.start, b.start);
    eq!("speedup pads", a.speedup_pads, b.speedup_pads);
    eq!("weapon pads", a.weapon_pads, b.weapon_pads);
    eq!("notes", a.notes, b.notes);
    eq!("art", a.art, b.art);
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

/// Every circuit of one image: export, write, read back, compare, and the art
/// proof (`art_proof`). One test per image, because the walk is the slow part
/// and nextest runs tests, not loop iterations, in parallel.
fn every_circuit_exports_and_reads_back_the_same(image: &str) {
    let mut total = 0;
    let mut refused = Vec::new();
    let mut art = art_proof::Tally::default();
    {
        let Some(path) = oag_testdata::image(image) else {
            return;
        };
        let path = path.display().to_string();
        let mut archives = open(image, &path);
        for track in circuits(image, &path) {
            total += 1;
            let exported = match race::export_track(&options(&path, &track)) {
                Ok(t) => t,
                Err(e) => {
                    refused.push(format!("{image} {track}: export refused: {e}"));
                    continue;
                }
            };
            let host = if is_pure(image) {
                "wipeout-pure"
            } else {
                "wipeout-pulse"
            };
            assert_eq!(exported.host.as_deref(), Some(host), "{image} {track}");
            let parts = match to_parts(&exported, "x.glb") {
                Ok(p) => p,
                Err(e) => {
                    refused.push(format!("{image} {track}: write refused: {e}"));
                    continue;
                }
            };
            oag_schema::validate::accepts("track", 1, &parts.manifest)
                .unwrap_or_else(|e| panic!("{image} {track}: schema: {e:?}"));
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
            art_proof::art_draws_as_the_circuit(&mut archives, &track, &loaded.track, &mut art)
                .unwrap_or_else(|e| panic!("{image} {track}: art: {e}"));
        }
    }
    println!("{image}: {total} circuit(s); refused: {refused:#?}");
    println!("art: {art:#?}");
    assert!(refused.is_empty(), "{refused:#?}");
}

#[test]
#[ignore = "needs the Pulse PSP EU image"]
fn every_pulse_psp_eu_circuit_exports_and_reads_back_the_same() {
    every_circuit_exports_and_reads_back_the_same(IMAGES[0]);
}

#[test]
#[ignore = "needs the Pulse PSP USA image"]
fn every_pulse_psp_usa_circuit_exports_and_reads_back_the_same() {
    every_circuit_exports_and_reads_back_the_same(IMAGES[1]);
}

#[test]
#[ignore = "needs the Pulse PS2 image"]
fn every_pulse_ps2_circuit_exports_and_reads_back_the_same() {
    every_circuit_exports_and_reads_back_the_same(IMAGES[2]);
}

#[test]
#[ignore = "needs the Pure PSP EU image"]
fn every_pure_eu_circuit_exports_and_reads_back_the_same() {
    every_circuit_exports_and_reads_back_the_same(PURE_IMAGES[0]);
}

#[test]
#[ignore = "needs the Pure PSP USA image"]
fn every_pure_usa_circuit_exports_and_reads_back_the_same() {
    every_circuit_exports_and_reads_back_the_same(PURE_IMAGES[1]);
}

/// A lone craft on a disc circuit and on the same circuit through its track
/// file: the world hash on every tick must agree. The file's pad plates are the
/// circuit's (as many vertices and triangles in each pad model), a weapon plate
/// sits on every weapon pad that triggers, and in Time Trial the weapon plates
/// are decoded but not drawn, as the circuit's are.
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
        let native_loaded = race::load(&native_options).unwrap();
        let native_sky = native_loaded
            .sky_model
            .as_ref()
            .map(|m| (m.vertices.len(), m.indices.len(), m.draws.len()));
        let native_hull = native_loaded.light.hull;
        let size =
            |m: Option<&oag_mesh::mesh::Model>| m.map(|m| (m.vertices.len(), m.indices.len()));
        let native_pads = (
            size(native_loaded.pad_model.as_ref()),
            size(native_loaded.weapon_pad_model.as_ref()),
        );
        assert!(
            native_pads.0.is_some() && native_pads.1.is_some(),
            "{track}: pads"
        );
        assert!(
            native_hull.enabled(),
            "{track}: the circuit lights its hulls"
        );
        let mut native = race::Race::start(native_loaded.setup);
        let mut from_file = {
            let mut o = native_options.clone();
            o.track = None;
            o.foreign_track = Some(manifest);
            let loaded = race::load(&o).unwrap();
            // The exported art draws through the foreign path: the circuit's
            // own meshes over its own textures, and its own sky.
            let report = loaded.report.join("\n");
            assert!(!report.contains("not drawn, their texture"), "{report}");
            assert!(report.contains("art sky:"), "{report}");
            assert_eq!(
                loaded.sky_model.as_ref().map(|m| (
                    m.vertices.len(),
                    m.indices.len(),
                    m.draws.len()
                )),
                native_sky,
                "{track}: the file's sky is not the circuit's"
            );
            assert!(!report.contains("draws nothing"), "{report}");
            // The file's ambient and sun reach the hulls through Pulse's GE rig.
            assert_eq!(
                loaded.light.hull, native_hull,
                "{track}: the file's hull rig is not the circuit's"
            );
            assert!(
                loaded.track_model.draws.len() > 1_000,
                "{track}: the art model is not the circuit's"
            );
            assert_eq!(
                (
                    size(loaded.pad_model.as_ref()),
                    size(loaded.weapon_pad_model.as_ref())
                ),
                native_pads,
                "{track}: the file's pad plates are not the circuit's"
            );
            assert!(
                report.contains("drawing the track file's weapon pad plates"),
                "{report}"
            );
            let plates = loaded.weapon_pad_model.as_ref().unwrap();
            for (i, pad) in loaded.setup.weapon_pads.iter().enumerate() {
                let centre = pad.centre();
                let nearest = plates
                    .vertices
                    .iter()
                    .map(|v| {
                        (0..3)
                            .map(|k| (v.position[k] - centre[k]).powi(2))
                            .sum::<f32>()
                            .sqrt()
                    })
                    .fold(f32::INFINITY, f32::min);
                assert!(
                    nearest < 10.0,
                    "{track}: weapon pad {i} has no plate ({nearest})"
                );
            }
            let time_trial = race::load(&race::Options {
                mode: oag_race::Mode::TimeTrial,
                ..o.clone()
            })
            .unwrap();
            assert!(
                time_trial
                    .report
                    .iter()
                    .any(|l| l.starts_with("decoded but not drawing (weapons off in this mode) the track file's weapon pad plates")),
                "{:#?}",
                time_trial.report
            );
            race::Race::start(loaded.setup)
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

#[cfg(feature = "ballisticng")]
fn workshop_items() -> Option<Vec<std::path::PathBuf>> {
    let root = oag_testdata::exact("data/ballisticng/workshop-473770")?;
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

#[cfg(feature = "ballisticng")]
/// Every Workshop track tagged `Original`: mapped, written in this format, read
/// back. Nothing is lost between the importer's output and the file.
#[test]
#[ignore = "needs data/ballisticng"]
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
        // A Workshop track borrows nothing from a disc, so it names no host.
        assert_eq!(mapped.host, None, "{}", w.title);
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

#[cfg(feature = "ballisticng")]
/// Lion's Den through `--ballisticng-track` and through its converted file, a lone AI
/// craft on Pulse's physics: the same world hash on every tick.
#[test]
#[ignore = "needs a Pulse image and data/ballisticng"]
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
/// format carries nothing for, and a lone AI craft gets round it. `art` names the
/// report lines a file with an art section must print (none of them a miss);
/// empty, the file must have none.
fn lone_craft_gets_round(file: &str, id: &str, title: &str, art: &[&str]) {
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
    let no_art = ["no art section"];
    let art_lines = if art.is_empty() { &no_art[..] } else { art };
    for needle in [
        title,
        "schema v1",
        "not authored by schema v1",
        "fog, point lights",
    ]
    .iter()
    .chain(art_lines)
    {
        assert!(
            report.contains(needle),
            "report lacks {needle:?}:\n{report}"
        );
    }
    for miss in [
        "draws nothing",
        "does not read",
        "did not resolve",
        "none drawn",
    ] {
        assert!(!report.contains(miss), "report has {miss:?}:\n{report}");
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
    lone_craft_gets_round("oval.track.toml", "fixture-octagon", "Fixture Octagon", &[]);
}

/// The add-on's own output (`scripts/blender/oag_track`, `just blender-fixtures`):
/// a loop authored in Blender from eight control points, raced as the committed
/// file. Drop the add-on's spline, start or ribbon and this craft does not get round;
/// drop its art and the report lines below go missing: the generated checker
/// draws, its own sky (gradient, checker overhead) is built, the Sun lamp lights the hulls and
/// the shadows, and the flare takes the checker as its atlas.
#[test]
#[ignore = "needs a Pulse image"]
fn a_lone_craft_gets_round_a_loop_authored_in_blender() {
    lone_craft_gets_round(
        "blender_loop.track.toml",
        "blender-loop",
        "Blender Loop",
        &[
            "art: 1 mesh(es), 2 triangle(s), 1 texture(s)\n",
            "art: draw_collision: the flat-shaded floor and walls are drawn under the art",
            "art skybox: the track's own sky, 1 face picture(s), 5 face(s) of zenith-to-horizon gradient",
            "art: the sun (0.000, 1.000, -0.000) sets the mapped shadows' direction",
            "art hull lights: the PSP rig from the ambient, the sun and 1 further directional(s)",
            "art lens flare: along (0.000, 1.000, -0.000), the track's sun, atlas 64x64",
        ],
    );
}

/// The loop with keyframed motion (`roundtrip.py motion`, ADR-0076 decision 10)
/// races as the loop does, and its motion reaches the renderer's node table: the
/// airship's two material slots on one node, the sun turning. Drop the mapping
/// and the report lines go missing.
#[test]
#[ignore = "needs a Pulse image"]
fn a_lone_craft_gets_round_a_loop_whose_art_moves() {
    lone_craft_gets_round(
        "blender_motion.track.toml",
        "blender-motion",
        "Blender Motion",
        &[
            "art motion: 2 animation(s), 2 mesh(es) on 1 node(s)",
            "art: the sun turns with its animation",
        ],
    );
}

#[test]
#[ignore = "needs a Pulse image"]
fn a_lone_craft_gets_round_the_oval_re_exported_from_blender() {
    lone_craft_gets_round(
        "oval.reexport.track.toml",
        "fixture-octagon",
        "Fixture Octagon",
        &[],
    );
}

/// Which title a track file loads on (ADR-0076 decisions 13 and 14). The Blender
/// loop draws only its own art (a checker and its own sky), so it loads with its
/// art on HD's own disc. Re-pointed at Pulse 03_Track's sky, it pins the Pulse
/// disc: with HD's as the track source it is refused by name rather than drawn
/// without it, and as a remix (Pulse track source, HD craft, what the game's
/// launches arrange) it loads with nothing missed. The oval, offered on Pulse
/// alone (`titles`), is refused on HD. Drop the host check, the titles filter,
/// the own sky, or route the track side through the craft's disc, and one part
/// fails.
#[test]
#[ignore = "needs a Pulse and an HD image"]
fn a_track_file_loads_on_the_disc_it_borrows_from_and_where_its_author_offers_it() {
    let (Some(pulse), Some(hd)) = (
        oag_testdata::image(IMAGES[1]),
        oag_testdata::image("data/images/hdfury-ps3-eu.iso"),
    ) else {
        return;
    };
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../trackfile/tests/fixtures");
    let options = |source: &std::path::Path,
                   craft: Option<&std::path::Path>,
                   track: &std::path::Path| race::Options {
        source: source.display().to_string(),
        craft_source: craft.map(|c| c.display().to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        foreign_track: Some(track.to_path_buf()),
        ..race::Options::default()
    };
    let no_art_miss = |report: &str| {
        // The craft side (HD's engine flares) reports on its own lines; the
        // track's art is what the track source must answer.
        for line in report.lines().filter(|l| l.starts_with("art")) {
            for miss in [
                "draws nothing",
                "does not read",
                "did not resolve",
                "none drawn",
                "stays black",
            ] {
                assert!(!line.contains(miss), "art line has {miss:?}: {line}");
            }
        }
    };

    // Own art only: natively on HD, art and own sky drawn.
    let own = fixtures.join("blender_loop.track.toml");
    let loaded = race::load(&options(&hd, None, &own))
        .unwrap_or_else(|e| panic!("own art loads on HD natively: {e:#}"));
    let report = loaded.report.join("\n");
    for needle in [
        "racing on Wipeout HD",
        "art: 1 mesh(es), 2 triangle(s), 1 texture(s)\n",
        "art skybox: the track's own sky, 1 face picture(s), 5 face(s) of zenith-to-horizon gradient",
    ] {
        assert!(
            report.contains(needle),
            "report lacks {needle:?}:\n{report}"
        );
    }
    no_art_miss(&report);

    // The same loop pinned to Pulse by a host sky.
    let dir = std::env::temp_dir().join(format!("oag-trackfile-host-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut pinned = oag_trackfile::read(&own).unwrap().track;
    let art = pinned.art.as_mut().unwrap();
    art.skybox = None;
    art.sky = Some(r"Data\Environments\03_Track\track.vex".to_string());
    pinned.host = Some("wipeout-pulse".to_string());
    let pinned_path = dir.join("pinned.track.toml");
    oag_trackfile::write(&pinned, &pinned_path).unwrap();
    std::fs::copy(
        fixtures.join("blender_checker.png"),
        dir.join("blender_checker.png"),
    )
    .unwrap();
    let Err(refused) = race::load(&options(&hd, None, &pinned_path)) else {
        panic!("a Pulse-pinned track with HD's disc as the track source is refused");
    };
    let why = format!("{refused:#}");
    for needle in [
        "borrows its art from the Wipeout Pulse disc",
        "track source is Wipeout HD",
        "Race Remix",
    ] {
        assert!(why.contains(needle), "refusal lacks {needle:?}: {why}");
    }
    let loaded = race::load(&options(&pulse, Some(&hd), &pinned_path)).expect("the remix loads");
    let report = loaded.report.join("\n");
    for needle in [
        "track file borrows from wipeout-pulse's disc; the track source is Wipeout Pulse",
        r"art sky: Data\Environments\03_Track\track.vex's Skycube",
    ] {
        assert!(
            report.contains(needle),
            "report lacks {needle:?}:\n{report}"
        );
    }
    no_art_miss(&report);
    assert_eq!(loaded.setup.foreign.as_deref(), Some("blender-loop"));

    // A filter: the oval offered on Pulse alone is refused on HD.
    let text = std::fs::read_to_string(fixtures.join("oval.track.toml")).unwrap();
    let offered = text.replacen(
        "glb = \"oval.glb\"",
        "titles = [\"wipeout-pulse\"]\nglb = \"oval.glb\"",
        1,
    );
    assert_ne!(offered, text);
    std::fs::write(dir.join("oval.track.toml"), offered).unwrap();
    std::fs::copy(fixtures.join("oval.glb"), dir.join("oval.glb")).unwrap();
    let refused = race::load(&options(&hd, None, &dir.join("oval.track.toml")));
    std::fs::remove_dir_all(&dir).ok();
    let Err(refused) = refused else {
        panic!("a track offered on Pulse alone is refused on HD");
    };
    let why = format!("{refused:#}");
    assert!(
        why.contains("offered on Wipeout Pulse") && why.contains("not on Wipeout HD"),
        "{why}"
    );
}

/// The other titles' default circuits through the same exporter: Pure, HD,
/// Omega and 2048 (a DLC `.vpk`, and the base game's package decrypted and
/// extracted offline: the `.pkg` itself is PFS-encrypted and not read in place).
#[test]
#[ignore = "needs the other titles' images"]
fn the_other_titles_default_circuits_round_trip_too() {
    let mut checked = Vec::new();
    for image in [
        "data/images/pure-psp-eu.chd",
        "data/images/hdfury-ps3-eu.iso",
        "data/images/omega-ps4-eu.pkg",
        "data/test-vpk/2048-PCSF00007-dlc1.vpk",
        "data/extracted/vita/PCSF00007",
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
