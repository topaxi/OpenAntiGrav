//! What the disc's `fury.envsettings` says, through the typed reader.
//!
//! The values menu-backdrop.md quotes from the file and from the
//! constructor, made executable: the eight static paths are all authored and
//! all mode 2, the morph modes run `9 9 9 9 10 10 11 11`, the dynamic modes
//! `5 5 5 9`, and the top-level colours are the file's.
//!
//! `#[ignore]`d because it needs `data/images/hdfury-ps3-eu-dec.iso`; run with
//! `just test-data`.

use oag_tables::envsettings::EnvSettings;
use oag_tables::fury_backdrop::{FuryBackdrop, STATIC_EFFECT_MODE};

const FILE: &str = "/data/fe/fury.envsettings";

fn read() -> Option<FuryBackdrop> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", image.display());
    let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
    let bytes = open.read_path(FILE).expect("the file reads");
    let text = String::from_utf8(bytes).expect("UTF-8");
    Some(FuryBackdrop::read(
        &EnvSettings::parse(&text).expect("every line parses"),
    ))
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso; run with `just test-data`"]
fn the_paths_are_all_authored_with_the_modes_the_page_quotes() {
    let Some(read) = read() else { return };
    assert_eq!(read.authored_static_paths().count(), 8);
    assert!(
        read.static_paths
            .iter()
            .all(|p| p.effect_mode == STATIC_EFFECT_MODE)
    );
    assert_eq!(
        read.morph_paths.map(|p| p.effect_mode),
        [9, 9, 9, 9, 10, 10, 11, 11]
    );
    assert!(read.morph_paths.iter().all(|p| p.is_authored()));
    assert_eq!(read.dynamic_paths.map(|p| p.effect_mode), [5, 5, 5, 9]);
    assert!(read.dynamic_paths.iter().all(|p| p.is_authored()));
    for path in &read.static_paths {
        assert!(
            (6.0..=10.0).contains(&path.duration),
            "static durations run 6..10 s, not {}",
            path.duration
        );
        assert!(path.point_size > 0.0 && path.fovy > 0.0);
        assert!(path.dof_strength > 0.0 && path.fog_length > 0.0);
    }
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso; run with `just test-data`"]
fn the_top_level_values_are_the_files() {
    let Some(read) = read() else { return };
    let close = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3);
    assert!(close(read.particle_colour, [0.541, 0.024, 0.0]));
    assert!(close(read.particle_ramp_colour, [11.765, 3.091, 0.231]));
    assert!(close(read.feedback, [0.647; 3]));
    assert_eq!(read.feedback_zoom, 0.0);
    assert!((read.music_pulse_base - 0.7).abs() < 1e-6);
    assert!((read.music_pulse_factor - 1.1).abs() < 1e-6);
    assert!((read.equaliser_damp_speed - 0.02).abs() < 1e-6);
    assert_eq!(read.equaliser_feedback_multiplier, 0.0);
    // The first path, as the file spells it.
    let first = read.static_paths[0];
    assert_eq!(first.start, [3.0, 2.0, 0.0]);
    assert_eq!(first.end, [5.0, 0.5, -5.0]);
    assert_eq!(first.focus_end, [0.0, 0.5, -3.0]);
    assert_eq!(first.fovy, 40.0);
    assert_eq!(first.duration, 8.0);
}
