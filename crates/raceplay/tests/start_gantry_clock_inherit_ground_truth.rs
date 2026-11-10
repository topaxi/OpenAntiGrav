//! The start gantry's clock on every title that stands one: Pulse's measured
//! clock, and HD's own race-manager window off its asset's `GO` edge.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run it with
//! `just test-data`. The tests skip with a message when an image is absent and
//! fail instead under `OAG_REQUIRE_GAME_DATA=1`.
//!
//! # What this pins
//!
//! Pulse's measured rule is that the board's `GO` edge lands on the tick
//! after the thrust gate opens (tick 273) and `GO` is held from there. HD
//! inherits only the start tick from that rule (its edge on the release);
//! from the release on it runs its own race manager's windows, read from the
//! EBOOT, not Pulse's hold.
//!
//! - **Pulse**: the rule, run on Pulse's own asset, finds frame 181 and so
//!   tick 92 - the number measured against the original. That is the check the
//!   rule is read the same way on every title.
//! - **HD**: the rule finds frame 203, so frame 0 is tick 70, and the loader
//!   says so. From the release on, HD runs its own race manager's window
//!   (`0x0005e948`, read from the EBOOT, confidence 85): the clock jumps to
//!   3.83 s and loops `[3.83, 5.25)`, 86 ticks. Pinned against the 2026-10-04
//!   RPCS3 capture: `GO` is lit on the release and its dark centres land
//!   within 2 ticks of the measured +23, +61 and +109. The start tick is
//!   bounded from below by the same capture (the green step on the release,
//!   confidence 75), not measured.
//! - **2048 and Omega**: the card pass is HD's, inherited, and plays the boards whose
//!   countdown is node motion (`321Go_2048.vex`, every native 2048 circuit, on both
//!   titles). Frame 0 lands `assembly_ticks` before the release, so `GO` slides in on it.
//!   The HD-lineage board (`321Go_StartFinish.vex`, on the HD-ported 2048 circuits and
//!   Omega's HD circuits) plays on HD's clock: its digits ride the UV keys its `.vex`
//!   `Mesh` nodes author (`docs/ghidra/functions/ps4-omega-eu/billboards.md`, "The UV
//!   keys"), whose `GO` edge is frame 202, so frame 0 is tick 71. Pinned on every such
//!   circuit: the clock line, and that `3`, `2`, `1` and `GO` light different vertices.

use oag_mesh::mesh;
use oag_raceplay as race;
use oag_raceplay::{LoadWorker, TextureSink, gantry};

fn pulse_model() -> Option<mesh::Model> {
    let path = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let mut archives = oag_pulse::open(path.to_str().expect("utf-8")).expect("open as Pulse");
    let blob = archives
        .read_name(r"Data\Environments\321_Go\321Go_StartFinish.vex")
        .expect("the gantry resolves");
    Some(mesh::build("gantry", &blob).expect("the gantry builds"))
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn pulses_own_asset_gives_pulses_measured_start_through_the_rule() {
    let Some(model) = pulse_model() else { return };
    let edge = gantry::go_edge(&model).expect("Pulse's board samples its green");
    assert_eq!(edge.frame, 181, "the `u` step the capture was pinned on");
    let clock = gantry::Clock::inherited(edge).expect("before the release");
    assert_eq!(clock.start_tick, gantry::CLOCK_START_TICK);
    assert_eq!(clock.start_tick, 92);
    // The exit Pulse's own race manager keeps the clock short of before the
    // first crossing: Board, Text and Arrow teleport in at 350/351, and the
    // pre-lap window read from BOOT.BIN ends at frame 330.
    assert_eq!(edge.last_frame_before_exit, 350);
    assert!(gantry::PULSE_PRE_LAP_WINDOW.to * 60.0 < edge.last_frame_before_exit as f32);
}

/// How much lit white the board's animated vertices sample at `seconds`: the
/// sum of the alpha of every white texel a drawn, texture-animated vertex
/// lands on. `GO`'s bright and dark phases are this rising and falling.
/// `hidden` names the draws the gantry leaves out at `seconds`
/// (`gantry::PanelCull::hidden`): HD's model keeps its later states.
fn lit_white(model: &mesh::Model, hidden: &[u32], seconds: f32) -> u32 {
    let table = oag_mesh::mesh_render::TexAnims::sample(model, seconds);
    let mut total = 0;
    let lists: [&[mesh::DrawCall]; 3] = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ];
    for draw in lists.into_iter().flatten() {
        if hidden.contains(&oag_render::gantry::panel::key(draw)) {
            continue;
        }
        let Some(texture) = draw.texture.and_then(|t| model.textures.get(t)?.as_ref()) else {
            continue;
        };
        let Some(rgba) = texture.to_rgba() else {
            continue;
        };
        let (w, h) = (texture.width as usize, texture.height as usize);
        for index in &model.indices[draw.range.start as usize..draw.range.end as usize] {
            let vertex = &model.vertices[*index as usize];
            if vertex.anim == 0 {
                continue;
            }
            let [su, sv, ou, ov] = table.transform[vertex.anim as usize];
            let u = (vertex.texcoord[0] * su + ou).rem_euclid(1.0);
            let v = (vertex.texcoord[1] * sv + ov).rem_euclid(1.0);
            let x = ((u * w as f32) as usize).min(w - 1);
            let y = ((v * h as f32) as usize).min(h - 1);
            let p = &rgba[(y * w + x) * 4..(y * w + x) * 4 + 4];
            if p[0] >= 200 && p[1] >= 200 && p[2] >= 200 {
                total += u32::from(p[3]);
            }
        }
    }
    total
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hds_go_lights_on_the_release_and_pulses_in_the_originals_phase() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some("/data/environments/talons_junction/track.vex".to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let line = loaded
        .report
        .iter()
        .find(|l| l.starts_with("start gantry clock:"))
        .unwrap_or_else(|| panic!("no gantry clock line: {:#?}", loaded.report));
    assert!(line.contains("asset frame 203"), "{line}");
    assert!(line.contains("frame 0 is tick 70"), "{line}");
    assert!(line.contains("kept in [3.83, 5.25) s"), "{line}");
    assert!(line.contains("loops every 86 ticks"), "{line}");
    assert!(line.contains("0x0005e948"), "{line}");

    let card = loaded
        .billboards
        .adverts
        .iter()
        .find(|card| card.slot == 8)
        .expect("Talon's Junction shows a gantry card");
    let timeline = card.timeline.as_ref().expect("the gantry card is clocked");
    let (model, clock) = (&card.model, timeline.clock());
    assert_eq!(clock.window, Some(gantry::HD_PRE_LAP_WINDOW));
    let release = oag_race::COUNTDOWN_TICKS + 1;
    let cull = timeline.cull();
    let lit = |tick: u64| {
        let seconds = clock.seconds(tick);
        lit_white(model, cull.hidden(seconds), seconds)
    };
    let full = (release..release + 200).map(lit).max().expect("ticks");
    assert!(full > 0, "GO never lights");

    // `GO` is fully lit on the release itself, as on the original's step
    // frame; the tick before is still the red board. The inherited clock put
    // only the fading digits' ghost here, 3% of this.
    assert_eq!(lit(release), full, "GO is not lit on the release");
    assert!(lit(release - 1) < full / 10, "lit before the release");

    // The dark phases: runs where nothing lit white is sampled, as ticks
    // from the release. The 2026-10-04 capture's dark centres are +23, +61
    // and +109 on both boots (2 ticks a video frame); the clock held over
    // 221..359 put them at +50, +89 and +129.
    let mut centres = Vec::new();
    let mut run: Option<u64> = None;
    for k in 0..=130 {
        match (lit(release + k) == 0, run) {
            (true, None) => run = Some(k),
            (false, Some(start)) => {
                centres.push((start + k - 1) as f32 / 2.0);
                run = None;
            }
            _ => {}
        }
    }
    assert_eq!(centres.len(), 3, "{centres:?}");
    for (centre, measured) in centres.iter().zip([23.0, 61.0, 109.0]) {
        assert!(
            (centre - measured).abs() <= 2.0,
            "dark centre +{centre} against the capture's +{measured}: {centres:?}"
        );
    }
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn hds_clock_is_found_on_the_loader_path_a_race_really_takes() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: adapter.features() & wgpu::Features::TEXTURE_COMPRESSION_BC,
        ..Default::default()
    }))
    .expect("requesting the device");
    let loaded = LoadWorker::spawn(
        race::Options {
            source: image.display().to_string(),
            track: Some("/data/environments/talons_junction/track.vex".to_string()),
            ..race::Options::default()
        },
        None,
        Some(TextureSink { device, queue }),
        false,
    )
    .join()
    .expect("a worker that was joined once")
    .expect("the circuit loads");
    let line = loaded
        .report
        .iter()
        .find(|l| l.starts_with("start gantry clock:"))
        .unwrap_or_else(|| panic!("no gantry clock line: {:#?}", loaded.report));
    assert!(line.contains("frame 0 is tick 70"), "{line}");
}

fn loaded_report(source: &str, track: Option<&str>) -> Vec<String> {
    race::load(&race::Options {
        source: source.to_string(),
        track: track.map(str::to_string),
        ..race::Options::default()
    })
    .expect("loading the race")
    .report
}

/// A circuit whose board is `321Go_2048.vex`: a card, clocked from the release.
fn native_board(source: &str, track: Option<&str>) {
    let report = loaded_report(source, track);
    assert!(
        report.iter().any(|l| l.starts_with(
            "start gantry /Data/Billboards/HD_Adverts/321Go/321Go_2048.vex: drawn through its own camera"
        )),
        "{report:#?}"
    );
    let clock = report
        .iter()
        .find(|l| l.starts_with("start gantry clock:"))
        .unwrap_or_else(|| panic!("no gantry clock line: {report:#?}"));
    assert!(
        clock.contains("frame 0 is tick 73") && clock.contains("(frame 200)"),
        "{clock}"
    );
}

/// A circuit whose track authors no `billboard8` surface in the base package: nothing to
/// show a card on. 2048's `park` is the one (Omega's copy ships `billboard8.gnf`).
fn no_surface(source: &str, track: Option<&str>) {
    let report = loaded_report(source, track);
    assert!(
        report.iter().any(|l| l.starts_with(
            "no start gantry: this circuit's track authors no billboard8 surface to show a card on"
        )),
        "{report:#?}"
    );
}

/// A circuit whose board is the HD-lineage `321Go_StartFinish.vex`: a card on
/// HD's clock, off the `GO` edge its `.vex` UV keys author, and its digits
/// change over the countdown.
fn curve_board_counts_down(source: &str, track: Option<&str>) {
    let loaded = race::load(&race::Options {
        source: source.to_string(),
        track: track.map(str::to_string),
        ..race::Options::default()
    })
    .expect("loading the race");
    let report = &loaded.report;
    assert!(
        report.iter().any(|l| l.starts_with(
            "start gantry /Data/Billboards/HD_Adverts/321Go/321Go_StartFinish.vex: drawn through its own camera"
        )),
        "{report:#?}"
    );
    let clock = report
        .iter()
        .find(|l| l.starts_with("start gantry clock:"))
        .unwrap_or_else(|| panic!("no gantry clock line: {report:#?}"));
    assert!(
        clock.contains("asset frame 202") && clock.contains("frame 0 is tick 71"),
        "{clock}"
    );

    let card = loaded
        .billboards
        .adverts
        .iter()
        .find(|card| card.slot == 8)
        .expect("the circuit shows a gantry card");
    let timeline = card.timeline.as_ref().expect("the gantry card is clocked");
    let (model, clock, cull) = (&card.model, timeline.clock(), timeline.cull());
    let lit = |tick: u64| {
        let seconds = clock.seconds(tick);
        lit_vertices(model, cull.hidden(seconds), seconds)
    };
    // The frames a player reads `3`, `2` and `1` on, and the release.
    let release = oag_race::COUNTDOWN_TICKS + 1;
    let (three, two, one, go) = (lit(140), lit(185), lit(230), lit(release));
    assert!(lit(80).is_empty(), "something is lit before the `3`");
    for (name, digit) in [("3", &three), ("2", &two), ("1", &one), ("GO", &go)] {
        assert!(!digit.is_empty(), "`{name}` lights nothing");
    }
    assert!(
        three != two && two != one && one != go && three != one,
        "the board shows the same glyph at two beats of the countdown"
    );
}

/// The drawn, texture-animated vertices that sample lit white at `seconds`:
/// which glyph the board shows. See [`lit_white`].
fn lit_vertices(model: &mesh::Model, hidden: &[u32], seconds: f32) -> Vec<u32> {
    let table = oag_mesh::mesh_render::TexAnims::sample(model, seconds);
    let mut lit = Vec::new();
    let lists: [&[mesh::DrawCall]; 3] = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ];
    for draw in lists.into_iter().flatten() {
        if hidden.contains(&oag_render::gantry::panel::key(draw)) {
            continue;
        }
        let Some(texture) = draw.texture.and_then(|t| model.textures.get(t)?.as_ref()) else {
            continue;
        };
        let Some(rgba) = texture.to_rgba() else {
            continue;
        };
        let (w, h) = (texture.width as usize, texture.height as usize);
        for index in &model.indices[draw.range.start as usize..draw.range.end as usize] {
            let vertex = &model.vertices[*index as usize];
            if vertex.anim == 0 {
                continue;
            }
            let [su, sv, ou, ov] = table.transform[vertex.anim as usize];
            let u = (vertex.texcoord[0] * su + ou).rem_euclid(1.0);
            let v = (vertex.texcoord[1] * sv + ov).rem_euclid(1.0);
            let x = ((u * w as f32) as usize).min(w - 1);
            let y = ((v * h as f32) as usize).min(h - 1);
            let p = &rgba[(y * w + x) * 4..(y * w + x) * 4 + 4];
            if p[0] >= 200 && p[1] >= 200 && p[2] >= 200 && p[3] >= 200 {
                lit.push(*index);
            }
        }
    }
    lit.sort_unstable();
    lit.dedup();
    lit
}

macro_rules! circuits_2048 {
    ($check:ident, $dir:literal; $($name:ident => $circuit:literal),* $(,)?) => {$(
        #[test]
        #[ignore = "needs data/extracted/vita/PCSF00007"]
        fn $name() {
            let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
                return;
            };
            $check(
                &path.display().to_string(),
                Some(concat!($dir, $circuit, r"\track.vex")),
            );
        }
    )*};
}

circuits_2048! {
    native_board, r"Data\art\published\environments\";
    wipeout_2048_altima_plays_its_own_board => "altima",
    wipeout_2048_arena_plays_its_own_board => "arena",
    wipeout_2048_bridge_plays_its_own_board => "bridge",
    wipeout_2048_cathedral_plays_its_own_board => "cathedral",
    wipeout_2048_mall_plays_its_own_board => "mall",
    wipeout_2048_sol_plays_its_own_board => "sol",
    wipeout_2048_square_plays_its_own_board => "square",
    wipeout_2048_subway_plays_its_own_board => "subway",
    wipeout_2048_tower_plays_its_own_board => "tower",
}

circuits_2048! {
    no_surface, r"Data\art\published\environments\";
    wipeout_2048_park_has_no_billboard8_surface => "park",
}

circuits_2048! {
    curve_board_counts_down, r"Data\art\published\DLC1\environments\";
    hd_ported_anulpha_pass_counts_down_on_the_curve_board => "Anulpha_Pass",
    hd_ported_chenghou_project_counts_down_on_the_curve_board => "Chenghou_Project",
    hd_ported_moa_therma_counts_down_on_the_curve_board => "Moa_Therma",
    hd_ported_vineta_k_counts_down_on_the_curve_board => "Vineta_K",
    hd_ported_metropia_counts_down_on_the_curve_board => "Metropia",
    hd_ported_sebenco_climb_counts_down_on_the_curve_board => "Sebenco_Climb",
    hd_ported_sol_2_counts_down_on_the_curve_board => "Sol_2",
    hd_ported_ubermall_counts_down_on_the_curve_board => "Ubermall",
    hd_ported_amphiseum_counts_down_on_the_curve_board => "amphiseum",
    hd_ported_modesto_heights_counts_down_on_the_curve_board => "modesto_heights",
    hd_ported_talons_junction_counts_down_on_the_curve_board => "talons_junction",
    hd_ported_tech_de_ra_counts_down_on_the_curve_board => "tech_de_ra",
    hd_ported_zone_1_counts_down_on_the_curve_board => "zone_1",
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn omegas_hd_circuits_count_down_on_the_curve_board() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    for track in [None, Some(r"Data\environments\tech_de_ra\track.vex")] {
        curve_board_counts_down(&path.display().to_string(), track);
    }
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn omegas_2048_circuit_plays_its_own_board() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    native_board(
        &path.display().to_string(),
        Some(r"Data\environments2048\altima\track.vex"),
    );
}
