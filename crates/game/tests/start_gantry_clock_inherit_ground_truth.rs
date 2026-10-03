//! The start gantry's clock on every title that stands one: Pulse's rule on
//! the title's own `GO` edge.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run it with
//! `just test-data`. The tests skip with a message when an image is absent and
//! fail instead under `OAG_REQUIRE_GAME_DATA=1`.
//!
//! # What this pins
//!
//! The maintainer's rule (2026-10-03): a title with no measured rule of its
//! own runs Pulse's, labelled **inherited from Pulse, unmeasured on <title>**.
//! Pulse's measured rule is that the board's `GO` edge lands on the tick after
//! the thrust gate opens (tick 273) and `GO` is held from there.
//!
//! - **Pulse**: the rule, run on Pulse's own asset, finds frame 181 and so
//!   tick 92 - the number measured against the original. That is the check the
//!   rule is read the same way on every title.
//! - **HD**: the rule finds frame 203, so frame 0 is tick 70, and the loader
//!   says so. **The start tick is measured on HD** (2026-10-04, confidence 75,
//!   two RPCS3 boots): the board's red-to-green step is on the video frame the
//!   race clock reads zero and the craft first moves, i.e. the release, within
//!   one 30 fps frame (2 ticks). The held span 221..359 is chosen, not measured.
//! - **2048 and Omega**: no circuit that was tried stands a gantry at all
//!   (`oag_render::gantry::mount` finds no `321backplate`) - 2048's ten native
//!   circuits, thirteen of its sixteen HD-ported ones (the base package's four
//!   and nine of the DLC's) and Omega's default, Tech De Ra (HD's circuit) and
//!   Altima (2048's) - so there is no clock to set. Pinned so the day a mount
//!   is found, this fails and the title gets its rule.

use oag_game::race::{self, LoadWorker, TextureSink, gantry};
use oag_render::mesh;

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
    // The exit the chosen loop end stops short of: Board, Text and Arrow
    // teleport in at 350/351, and the chosen end is one frame under the
    // derived one.
    assert_eq!(edge.last_frame_before_exit, 350);
    assert_eq!(
        (gantry::GO_LOOP.1 * 60.0).round() as u32 + 1,
        edge.last_frame_before_exit
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hds_go_edge_lands_on_the_release_and_is_held() {
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
    assert!(line.contains("landed on tick 273"), "{line}");
    assert!(line.contains("frame 0 is tick 70"), "{line}");
    assert!(
        line.contains("measured on Wipeout HD (2026-10-04, confidence 75)"),
        "{line}"
    );

    // The hold span, read back out of what the loader derived rather than
    // restated here: the digits are gone from `from` and the glyph's exit
    // keys start after `to`.
    let span = line
        .split("held over frames ")
        .nth(1)
        .and_then(|rest| rest.split(',').next())
        .and_then(|span| span.split_once(".."))
        .map(|(from, to)| (from.parse::<u32>().unwrap(), to.parse::<u32>().unwrap()))
        .unwrap_or_else(|| panic!("no hold span in: {line}"));
    assert_eq!(span, (221, 359), "{line}");

    let edge = gantry::GoEdge {
        frame: 203,
        settled_frame: span.0,
        last_frame_before_exit: span.1,
    };
    let clock = gantry::Clock::inherited(edge).expect("before the release");
    assert_eq!(clock.start_tick, 70);
    assert_eq!(clock.seconds(oag_race::COUNTDOWN_TICKS + 1) * 60.0, 203.0);
    for tick in (0..30_000).step_by(11) {
        let frame = clock.seconds(tick) * 60.0;
        assert!(
            frame < span.1 as f32 && (tick < 70 + 203 || frame >= 203.0),
            "tick {tick} is at frame {frame}"
        );
        if tick > 70 + span.1 as u64 {
            assert!(frame >= span.0 as f32, "tick {tick} replays frame {frame}");
        }
    }
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn hds_clock_is_found_on_the_loader_path_a_race_really_takes() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
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

fn no_gantry(source: &str, track: Option<&str>) {
    let loaded = race::load(&race::Options {
        source: source.to_string(),
        track: track.map(str::to_string),
        ..race::Options::default()
    })
    .expect("loading the race");
    assert!(
        loaded
            .report
            .iter()
            .any(|l| l.starts_with("no start gantry: this circuit's track authors no")),
        "a mount was found - this title now needs its clock rule: {:#?}",
        loaded.report
    );
    assert!(
        !loaded
            .report
            .iter()
            .any(|l| l.starts_with("start gantry clock:")),
        "{:#?}",
        loaded.report
    );
}

macro_rules! native_2048 {
    ($dir:literal; $($name:ident => $circuit:literal),* $(,)?) => {$(
        #[test]
        #[ignore = "needs data/extracted/vita/PCSF00007"]
        fn $name() {
            let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
                return;
            };
            no_gantry(
                &path.display().to_string(),
                Some(concat!($dir, $circuit, r"\track.vex")),
            );
        }
    )*};
}

native_2048! {
    r"Data\art\published\environments\";
    wipeout_2048_altima_stands_no_gantry => "altima",
    wipeout_2048_arena_stands_no_gantry => "arena",
    wipeout_2048_bridge_stands_no_gantry => "bridge",
    wipeout_2048_cathedral_stands_no_gantry => "cathedral",
    wipeout_2048_mall_stands_no_gantry => "mall",
    wipeout_2048_park_stands_no_gantry => "park",
    wipeout_2048_sol_stands_no_gantry => "sol",
    wipeout_2048_square_stands_no_gantry => "square",
    wipeout_2048_subway_stands_no_gantry => "subway",
    wipeout_2048_tower_stands_no_gantry => "tower",
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn omegas_default_circuit_stands_no_gantry() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    no_gantry(&path.display().to_string(), None);
}

native_2048! {
    r"Data\art\published\DLC1\environments\";
    hd_ported_anulpha_pass_stands_no_gantry => "Anulpha_Pass",
    hd_ported_chenghou_project_stands_no_gantry => "Chenghou_Project",
    hd_ported_moa_therma_stands_no_gantry => "Moa_Therma",
    hd_ported_vineta_k_stands_no_gantry => "Vineta_K",
    hd_ported_metropia_stands_no_gantry => "Metropia",
    hd_ported_sebenco_climb_stands_no_gantry => "Sebenco_Climb",
    hd_ported_sol_2_stands_no_gantry => "Sol_2",
    hd_ported_ubermall_stands_no_gantry => "Ubermall",
    hd_ported_amphiseum_stands_no_gantry => "amphiseum",
    hd_ported_modesto_heights_stands_no_gantry => "modesto_heights",
    hd_ported_talons_junction_stands_no_gantry => "talons_junction",
    hd_ported_tech_de_ra_stands_no_gantry => "tech_de_ra",
    hd_ported_zone_1_stands_no_gantry => "zone_1",
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn omegas_hd_and_2048_origin_circuits_stand_no_gantry() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    for track in [
        r"Data\environments\tech_de_ra\track.vex",
        r"Data\environments2048\altima\track.vex",
    ] {
        no_gantry(&path.display().to_string(), Some(track));
    }
}
