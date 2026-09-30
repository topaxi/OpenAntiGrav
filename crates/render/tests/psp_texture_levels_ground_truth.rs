//! Every PSP `.vex` texture reaches the GPU with the one level the running
//! original samples, whatever chain the file declares.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(psp_texture_levels_ground_truth)'
//! ```
//!
//! The measurement behind `PSP_SAMPLED_LEVELS` is in
//! `docs/rendering/frame-audit.md`. This pins the wiring: a loader that went
//! back to the authored depth would leave Talon's Junction's scenery with the
//! 4- and 5-level chains it declares, and a scenery texture would blur again.

use oag_render::mesh::{self, PSP_SAMPLED_LEVELS};

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn talons_junction_scenery_is_capped_at_the_base_level() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("the disc opens");
    let name = r"Data\Environments\16_Track\track.vex";
    let blob = archives.read_name(name).expect("the track is on the disc");
    let model = mesh::build_with_textures(name, &blob, None).expect("the track builds");

    let mut distinct = std::collections::HashSet::new();
    let mut checked = 0;
    for texture in model.textures.iter().flatten() {
        if !distinct.insert(std::sync::Arc::as_ptr(texture)) {
            continue;
        }
        assert_eq!(
            texture.mip_count,
            Some(PSP_SAMPLED_LEVELS),
            "{}: uploaded with more than the level the original samples",
            texture.label
        );
        checked += 1;
    }
    assert!(checked > 100, "{checked} textures checked");
}
