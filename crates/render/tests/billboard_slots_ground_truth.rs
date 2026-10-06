//! Which track quads the original samples an advert on, checked against a live
//! frame.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this test:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all billboard_slots
//! ```
//!
//! # What it reproduces
//!
//! A PPSSPP GE dump of Talon's Junction at the start line (2026-10-06,
//! `docs/ghidra/functions/psp-pulse-usa/billboards.md`) holds two track draws
//! that sample the 128 x 128 advert buffer. Their vertex-buffer bounding boxes,
//! in the track's own world space, were
//! `x -254.5..-243.4, y -15.3..-0.3, z -270.6..-233.7` and
//! `x 34.24, y -41.2..-30.6, z -208.2..-162.8`. This test finds the draws bound
//! to `billboard7.tga` and `billboard8.tga` in the same circuit's `track.vex`
//! and asserts they are those two boxes: an advert is shown on the quad
//! textured with its own slot's number, which is the rule
//! `oag_raceplay::adverts` applies.

use oag_core::math::Vec3;
use oag_mesh::mesh;
use oag_render::gantry;

/// `(slot, lo, hi)` as the GE vertex buffers measured them.
const SAMPLED: [(u32, [f32; 3], [f32; 3]); 2] = [
    (7, [-254.46, -15.27, -270.60], [-243.45, -0.27, -233.74]),
    (8, [34.24, -41.17, -208.23], [34.24, -30.59, -162.75]),
];

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_quads_the_live_frame_sampled_are_slot_7_and_slot_8_placeholders() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");
    let name = r"Data\Environments\16_Track\track.vex";
    let blob = archives.read_name(name).expect("the track");
    let model = mesh::build_with_textures(name, &blob, None).expect("decoding it");
    let slots = gantry::placeholder_texture_slots(&model);
    for (number, want_lo, want_hi) in SAMPLED {
        let found = [
            &model.draws,
            &model.alpha_tested_draws,
            &model.transparent_draws,
        ]
        .into_iter()
        .flatten()
        .filter(|d| {
            d.texture
                .is_some_and(|t| slots.iter().any(|&(s, n)| s == t && n == number))
        })
        .any(|d| {
            let mut lo = Vec3::splat(f32::MAX);
            let mut hi = Vec3::splat(f32::MIN);
            for i in d.range.clone() {
                let p = Vec3::from(model.vertices[model.indices[i as usize] as usize].position);
                lo = lo.min(p);
                hi = hi.max(p);
            }
            (lo - Vec3::from(want_lo)).abs().max_element() < 1.0
                && (hi - Vec3::from(want_hi)).abs().max_element() < 1.0
        });
        assert!(
            found,
            "no draw bound to billboard{number}.tga matches the box the original sampled"
        );
    }
}
