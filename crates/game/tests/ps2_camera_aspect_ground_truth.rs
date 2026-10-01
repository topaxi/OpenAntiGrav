//! The PS2 engine builds the race projection at `640/448`, and the PSP's
//! projection did not move when that was taught here.
//!
//! **`#[ignore]`d and never run in CI.** It needs the PS2 and PSP Pulse discs.
//! See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this pins
//!
//! A running PCSX2, 2026-10-01 (`docs/ghidra/functions/ps2-pulse-eu/camera.md`,
//! "The projection the race renders with"): the race camera at rest on the
//! grid, flying `OPT_CLOSE`, carries a fov of `60.000004` degrees and a matrix
//! of `m00 = 1.21244`, `m11 = 1.73205`, `m11 / m00 = 1.42857`, read on two
//! loads of the same savestate and again 30 frames later. That is the authored
//! `<ExternalCameraClose fov="60">` at the aspect `Camera_SubmitScene` loads from
//! the literal at `0x0013e604`, `10/7`, with the `Aspect Ratio` option at `4:3`.
//!
//! - **PS2**: at a `640x448` window ours builds that matrix, to the digits the
//!   live read has.
//! - **PSP**: every projection, at six window shapes including the ones narrower
//!   than the PSP's own, is bit-identical to the chain the PSP source always
//!   went through (`fit_vertical_fov` at `AUTHORED_ASPECT`).

use oag_display::display::Fov;
use oag_game::race;
use oag_render::camera::{fit_vertical_fov, projection};

fn start(image: &str) -> Option<race::Race> {
    let image = oag_testdata::image(image)?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(race::Race::start(loaded.setup))
}

/// The shapes a window can have: the PSP's own, 16:9, the PS2's frame, 4:3, a
/// square and a portrait. The last four are narrower than the PSP's, which is
/// where `fit_vertical_fov` is active and a wrong authored aspect would show.
const SHAPES: [f32; 6] = [
    480.0 / 272.0,
    16.0 / 9.0,
    640.0 / 448.0,
    4.0 / 3.0,
    1.0,
    9.0 / 16.0,
];

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn the_ps2_race_projects_at_the_aspect_the_original_measured() {
    let Some(race) = start("data/images/pulse-ps2-eu.chd") else {
        return;
    };
    let aspect = 640.0 / 448.0;
    let m = race
        .projection(aspect, 1000.0, Fov::AUTHORED)
        .to_cols_array();
    // The live PCSX2 read, to the five digits it printed.
    assert!((m[5] - 1.73205).abs() < 1e-4, "m11 {}", m[5]);
    assert!((m[0] - 1.21244).abs() < 1e-4, "m00 {}", m[0]);
    assert!((m[5] / m[0] - 1.42857).abs() < 1e-4);

    // At the PSP's own shape and wider the vertical field is the authored one
    // whichever constant is fitted to, so the default presentation is unchanged.
    for wide in [480.0 / 272.0, 16.0 / 9.0] {
        let m = race.projection(wide, 1000.0, Fov::AUTHORED).to_cols_array();
        assert!((m[5] - 1.73205).abs() < 1e-4, "m11 at {wide}: {}", m[5]);
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn the_psp_projection_is_bit_identical_to_the_chain_it_always_had() {
    let Some(race) = start("data/images/pulse-psp-eu.chd") else {
        return;
    };
    // Tick 0: no forward speed, so the speed widen is exactly zero and the
    // authored field is the close block's own. Read at a window wider than
    // either authored shape, where the fit is the identity.
    let fov = race.vertical_fov(2.0, Fov::AUTHORED);
    for aspect in SHAPES {
        let expected = projection(
            fit_vertical_fov(fov, race::AUTHORED_ASPECT, aspect),
            aspect,
            1.0,
            1000.0,
        );
        assert_eq!(
            race.projection(aspect, 1000.0, Fov::AUTHORED)
                .to_cols_array()
                .map(f32::to_bits),
            expected.to_cols_array().map(f32::to_bits),
            "the PSP projection moved at aspect {aspect}"
        );
    }
}
