//! Pulse's billboard adverts: each slot's model is drawn through its own
//! camera and the circuit's placeholder quad shows the result.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The tests skip with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure.
//!
//! # What this reproduces
//!
//! A live PPSSPP frame of Talon's Junction (2026-10-06, see
//! `docs/ghidra/functions/psp-pulse-usa/billboards.md`) renders two adverts
//! into a 128 x 128 offscreen buffer and samples it on two track quads. The
//! matrices in that GE list are the model's own camera: an x scale of
//! `1 / tan(fov / 2)` with `fov` from the camera's one-key curve and a y scale
//! `aspect` times that. So the census here is that **every advert a Pulse
//! manifest names carries exactly one camera, with a one-key perspective
//! curve** - the shape `oag_vex::camera::Camera::fov_degrees` reads - and the
//! report tests are that a circuit's slots are drawn rather than suppressed.

use std::collections::BTreeSet;
use std::path::PathBuf;

use oag_raceplay as race;

const TRACK_IDS: &[&str] = &[
    "01_Track", "02_Track", "03_Track", "04_Track", "05_Track", "06_Track", "07_Track", "08_Track",
    "09_Track", "10_Track", "11_Track", "12_Track", "13_Track", "14_Track", "15_Track", "16_Track",
    "17_Track", "18_Track", "19_Track", "20_Track",
];

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_advert_a_manifest_names_has_one_camera_with_a_one_key_curve() {
    let Some(image) = image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");
    let mut named = BTreeSet::new();
    for id in TRACK_IDS {
        let Ok(blob) = archives.read_name(&format!(r"Data\Environments\{id}\trackstartup.xml"))
        else {
            continue;
        };
        let manifest =
            oag_tables::trackstartup::TrackStartup::parse(&String::from_utf8_lossy(&blob));
        for billboard in &manifest.billboards {
            if let Some(model) = billboard.location()
                && billboard.num != 8
            {
                named.insert(model.to_ascii_lowercase());
            }
        }
    }
    assert!(!named.is_empty(), "no circuit names an advert model");
    let mut fovs = BTreeSet::new();
    for name in &named {
        let blob = archives
            .read_name(name.trim_start_matches(['/', '\\']))
            .or_else(|_| archives.read_name(&name.replace('/', "\\")))
            .unwrap_or_else(|e| panic!("{name} is named by a manifest but not on the disc: {e}"));
        let cameras = oag_vex::camera::cameras(&blob);
        assert_eq!(cameras.len(), 1, "{name}: {} cameras", cameras.len());
        let camera = &cameras[0];
        let fov = camera
            .fov_degrees()
            .unwrap_or_else(|| panic!("{name}: no one-key perspective curve"));
        fovs.insert((format!("{fov:.2}"), format!("{:.1}", camera.aspect())));
        assert!(
            oag_raceplay::adverts::view_projection(camera).is_some(),
            "{name}: its camera gives no projection"
        );
    }
    println!("{} adverts; (fov, aspect) seen: {fovs:?}", named.len());
}

fn report(source: &str, track: Option<&str>) -> Option<String> {
    let image = image(source)?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        track: track.map(str::to_string),
        ..race::Options::default()
    })
    .expect("loading the race");
    let report = loaded.report.join("\n");
    println!("{report}");
    Some(report)
}

fn assert_talons_junction_draws_its_adverts(report: &str) {
    for slot in [1, 2, 6, 7] {
        assert!(
            report.contains(&format!("billboard slot {slot}: "))
                && report.contains("drawn through its own camera"),
            "slot {slot} is not drawn: {report}"
        );
    }
    assert!(
        report.contains("billboard8 placeholder draw(s) replaced by the start gantry"),
        "slot 8's placeholder is neither replaced nor reported: {report}"
    );
    assert!(
        !report.contains("placeholder draw(s) suppressed"),
        "a placeholder draw is still suppressed on a circuit whose slots all name a model: {report}"
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn pulse_psp_draws_talons_junctions_adverts() {
    let Some(report) = report("data/images/pulse-psp-usa.chd", None) else {
        return;
    };
    assert_talons_junction_draws_its_adverts(&report);
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_draws_talons_junctions_adverts() {
    let Some(report) = report("data/images/pulse-ps2-eu.chd", None) else {
        return;
    };
    assert_talons_junction_draws_its_adverts(&report);
}

/// A colour slot draws the advert the engine's pool hands it. De Konstruct
/// authors blue, blue and yellow portraits on slots 1, 2 and 4 and grey on 3,
/// which has no quad but still takes its entry out of the pool. The expected
/// models are `oag_tables::billboard_pool`'s shuffle and draw order, which a
/// live PPSSPP read of the pool matched entry for entry (2026-10-06).
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn colour_slots_draw_the_adverts_the_engines_pool_hands_them() {
    let Some(report) = report(
        "data/images/pulse-psp-usa.chd",
        Some(r"Data\Environments\05_Track\track.vex"),
    ) else {
        return;
    };
    for (slot, model) in [
        (1, "ASSEGAI_PORTRAIT_01.vex"),
        (2, "AURICOM_PORTRAIT_01.vex"),
        (4, "Egx_PORTRAIT_01.vex"),
    ] {
        assert!(
            report.contains(&format!("billboard slot {slot}: the colour"))
                && report.contains(&format!("{model} drawn through its own camera")),
            "slot {slot} should draw {model}: {report}"
        );
    }
    assert!(
        report.contains("billboard slot 5: ") && report.contains("drawn through its own camera"),
        "{report}"
    );
    assert!(
        !report.contains("placeholder draw(s) suppressed"),
        "a placeholder is still suppressed: {report}"
    );
}

/// The simulated pool equals the one read out of a running PSP: the 35 entries'
/// order after `FUN_08900e90`'s shuffle and Talon's Junction's one grey draw.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_simulated_pool_order_is_the_captured_one() {
    let Some(image) = image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");
    let catalogue = oag_tables::billboard_pool::parse(
        &archives
            .read_name(r"Data\Plugins\PI004\Definition.xml")
            .expect("the catalogue"),
    );
    assert_eq!(catalogue.len(), 35);
    let mut pool = oag_tables::billboard_pool::Pool::new(catalogue);
    let before: Vec<String> = pool.entries().iter().map(|e| e.name.clone()).collect();
    assert_eq!(&before[..3], ["Landscape22", "Landscape23", "Portrait2"]);
    assert_eq!(before[33], "Landscape20");
    let drawn = pool.draw("portrait", "grey").map(|e| e.name.clone());
    assert_eq!(drawn.as_deref(), Some("Portrait14"));
    let after: Vec<&str> = pool.entries().iter().map(|e| e.name.as_str()).collect();
    assert_eq!(after[34], "Portrait14");
    assert_eq!(after[12], "Landscape21");
}
