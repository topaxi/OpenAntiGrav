//! Wipeout HD's two rim-shaded weapon glows, routed off their own programs.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_rim_glow_ground_truth)'
//! ```
//!
//! What `docs/rendering/hd-unlit-programs.md` claims of the disc, pinned:
//! the LeachBall's glow material earns `slots::RIM_GLOW`, the Plasma head's
//! earns `slots::RIM_EDGE`, the bloomring stays on the plain emissive path,
//! and the inline sphere these three share now reads a real, varying `Uv1`
//! rather than the colour bytes behind it.

use std::path::{Path, PathBuf};

use oag_render::mesh::{self, slots};

const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

/// Builds a `DATA02` weapon model with its materials and textures read the
/// way the race's `Archives` serves them: `DATA00` first, where a copy
/// exists in both (the LeachBall's glow material does, and the two copies
/// differ).
fn build(image: &Path, path: &str) -> mesh::Model {
    let archive = |name: &str| format!("{}:PS3_GAME/USRDIR/{name}", image.display());
    let spec = archive("DATA02.PSARC");
    let data = mesh::read_blob(&spec, path).expect("the .vex reads");
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data).expect("an .rcsmodel beside it");
    let (model, _) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        ["DATA00.PSARC", "DATA02.PSARC"]
            .iter()
            .find_map(|a| mesh::read_blob(&archive(a), name).ok())
    })
    .expect("the scene builds");
    model
}

fn roles(model: &mesh::Model) -> Vec<u32> {
    model
        .transparent_draws
        .iter()
        .chain(&model.draws)
        .map(|draw| model.vertices[model.indices[draw.range.start as usize] as usize].slots)
        .collect()
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_leachball_glows_through_its_own_program_and_its_ring_stays_emissive() {
    let Some(image) = image() else { return };
    let model = build(&image, "/data/weapons/hd_leachbeam_ball_bloomring.vex");
    let roles = roles(&model);
    assert_eq!(roles.len(), 2, "the sphere and the ring");
    let glowing = roles.iter().filter(|r| *r & slots::RIM_GLOW != 0).count();
    assert_eq!(glowing, 1, "exactly the sphere: {roles:x?}");
    for r in &roles {
        assert_eq!(r & slots::RIM_EDGE, 0, "{r:#x}");
        if r & slots::RIM_GLOW == 0 {
            assert_eq!(r & slots::EMISSIVE, slots::EMISSIVE, "the ring: {r:#x}");
        }
    }
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_plasma_head_earns_rim_edge_and_its_inline_sphere_reads_a_real_uv() {
    let Some(image) = image() else { return };
    let model = build(&image, "/data/weapons/hd_plasma_ball.vex");
    let roles = roles(&model);
    assert!(!roles.is_empty());
    for r in &roles {
        assert_eq!(r & slots::RIM_EDGE, slots::RIM_EDGE, "{r:#x}");
        assert_eq!(r & slots::RIM_GLOW, 0, "{r:#x}");
    }
    // The stride-18 inline chunk's tail is `ff ff ff cc`; before the fix
    // every coordinate was that read as two NaN halves, zeroed on emit.
    let (lo, hi) = model
        .vertices
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), v| {
            (lo.min(v.texcoord[0]), hi.max(v.texcoord[0]))
        });
    assert!(hi - lo > 0.9, "u spans {lo}..{hi}");
}
