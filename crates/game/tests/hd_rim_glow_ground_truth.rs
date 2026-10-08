//! Wipeout HD's two rim-shaded weapon glows, routed off their own programs.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(hd_rim_glow_ground_truth)'
//! ```
//!
//! What `docs/rendering/hd-unlit-programs.md` claims of the disc, pinned:
//! the LeachBall's glow material earns `slots::RIM_GLOW`, the Plasma head's
//! earns `slots::RIM_EDGE`, the bloomring stays on the plain emissive path,
//! and the inline sphere these three share now reads a real, varying `Uv1`
//! rather than the colour bytes behind it.

use std::path::{Path, PathBuf};

use oag_mesh::mesh::{self, slots};

const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

/// Builds a weapon model with its materials and textures read through the
/// race's own `oag_assets::Archives` for Wipeout HD - the same precedence
/// `race::load` gets, which decides which of the LeachBall glow material's
/// two differing copies (`DATA00`'s, `DATA02`'s) is the one drawn.
fn build(image: &Path, path: &str) -> mesh::Model {
    let mut archives = oag_assets::Archives::open(&image.to_string_lossy(), oag_hd::TITLE)
        .expect("the archives open");
    let data = archives.read_name(path).expect("the .vex reads");
    let sibling = mesh::rcs::sibling_name(path).expect("a PS3 .vex names its .rcsmodel");
    let geometry = archives
        .read_name(&sibling)
        .expect("an .rcsmodel beside it");
    let (model, _) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        archives.read_name(name).ok()
    })
    .expect("the scene builds");
    model
}

/// The copy the race serves is `DATA00`'s - the one whose program bakes the
/// `0.9` `rim_glow` matches. Its length alone tells the two apart.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_race_serves_the_data00_copy_of_the_leachball_glow_material() {
    let Some(image) = image() else { return };
    let mut archives = oag_assets::Archives::open(&image.to_string_lossy(), oag_hd::TITLE)
        .expect("the archives open");
    let blob = archives
        .read_name("/data/weapons/materials/hd_leachbeam_ball_glow.rcsmaterial")
        .expect("the material reads");
    assert_eq!(
        blob.len(),
        28_368,
        "DATA00's copy is 28,368 bytes, DATA02's 29,984"
    );
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

/// The Plasma explosion's ring and halo each earn their own clock-scroll bit
/// (and none of the rim bits), off their own `UV_offset` programs; the
/// sphere's lit program matches neither and keeps the plain path.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_explosion_ring_and_halo_earn_their_clock_scroll_bits() {
    let Some(image) = image() else { return };
    for (path, bit, other) in [
        (
            "/data/weapons/hd_plasma_ring.vex",
            slots::CLOCK_SCROLL_RING,
            slots::CLOCK_SCROLL_HALO,
        ),
        (
            "/data/weapons/hd_plasma_halo.vex",
            slots::CLOCK_SCROLL_HALO,
            slots::CLOCK_SCROLL_RING,
        ),
    ] {
        let roles = roles(&build(&image, path));
        assert!(!roles.is_empty(), "{path}");
        for r in &roles {
            assert_eq!(r & bit, bit, "{path}: {r:#x}");
            assert_eq!(r & other, 0, "{path}: {r:#x}");
            assert_eq!(r & (slots::RIM_GLOW | slots::RIM_EDGE), 0, "{path}: {r:#x}");
        }
    }
    for r in roles(&build(&image, "/data/weapons/hd_plasma_sphere.vex")) {
        assert_eq!(
            r & (slots::CLOCK_SCROLL_RING | slots::CLOCK_SCROLL_HALO),
            0,
            "the sphere is a lit program: {r:#x}"
        );
    }
}

/// The Bomb's detonation models: the fireball and its white core earn
/// `BOMB_FIRE` (the two rim bits *together*), the shockwaves `BOMB_SHOCK`
/// (the two clock-scroll bits together), and the bloomring stays on the plain
/// emissive path. Pairs, so no single-bit test in the shader or the loader
/// reads them as the LeachBall, the Plasma head or the explosion ring.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_bomb_blast_models_earn_their_paired_bits() {
    let Some(image) = image() else { return };
    for path in [
        "/data/weapons/hd_bomb_sphere.vex",
        "/data/weapons/hd_bomb_sphere_white.vex",
    ] {
        let roles = roles(&build(&image, path));
        assert!(!roles.is_empty(), "{path}");
        for r in &roles {
            assert_eq!(r & slots::BOMB_FIRE, slots::BOMB_FIRE, "{path}: {r:#x}");
            assert_eq!(r & slots::BOMB_SHOCK, 0, "{path}: {r:#x}");
        }
    }
    let shock = build(&image, "/data/weapons/hd_bomb_shockwaves.vex");
    let shock_roles = roles(&shock);
    assert!(!shock_roles.is_empty());
    for r in &shock_roles {
        assert_eq!(r & slots::BOMB_SHOCK, slots::BOMB_SHOCK, "{r:#x}");
        assert_eq!(r & slots::BOMB_FIRE, 0, "{r:#x}");
    }
    // `ff 9f 00 4c` twice at the end of each vertex: the coordinate is read
    // at `+0x0a` (it was NaN, zeroed), and the colours ride the vertex.
    let (lo, hi) = shock
        .vertices
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), v| {
            (lo.min(v.texcoord[1]), hi.max(v.texcoord[1]))
        });
    assert!(hi - lo > 0.2, "v spans {lo}..{hi}");
    assert!(
        shock
            .vertices
            .iter()
            .any(|v| v.colour[0] > 0.9 && v.sun_mask > 0.0 && v.sun_mask < 1.0),
        "the first colour's red and the second colour's alpha are carried"
    );
    for r in roles(&build(&image, "/data/weapons/hd_bomb_sphere_bloomring.vex")) {
        assert_eq!(r & slots::EMISSIVE, slots::EMISSIVE, "the ring: {r:#x}");
        assert_eq!(r & (slots::BOMB_FIRE | slots::BOMB_SHOCK), 0, "{r:#x}");
    }
}

/// The laid Bomb's halo earns `BOMB_HALO`, its vertex
/// program's `Speed` (`6.428`, authored on the material) is folded into `v`,
/// and `hd_bomb.vex` carries the same material on one chunk of its own. No
/// other Bomb model and no Plasma model earns the bit.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_laid_bombs_halo_earns_its_bit_and_its_v_offset() {
    let Some(image) = image() else { return };
    let halo = build(&image, "/data/weapons/hd_bomb_halo.vex");
    assert!(!halo.vertices.is_empty());
    for v in &halo.vertices {
        let r = v.slots & slots::ROLE_MASK;
        assert_eq!(r & slots::BOMB_HALO, slots::BOMB_HALO, "{r:#x}");
        assert_eq!(r & (slots::BOMB_FIRE | slots::BOMB_SHOCK), 0, "{r:#x}");
        assert!(
            (6.0..9.0).contains(&v.texcoord[1]),
            "v carries Speed 6.428: {}",
            v.texcoord[1]
        );
    }
    let body = roles(&build(&image, "/data/weapons/hd_bomb.vex"));
    assert!(
        body.iter().any(|r| r & slots::BOMB_HALO != 0),
        "hd_bomb.vex's sixth chunk is the same material"
    );
    for path in [
        "/data/weapons/hd_bomb_sphere.vex",
        "/data/weapons/hd_bomb_sphere_white.vex",
        "/data/weapons/hd_bomb_shockwaves.vex",
        "/data/weapons/hd_bomb_sphere_bloomring.vex",
        "/data/weapons/hd_plasma_sphere.vex",
        "/data/weapons/hd_plasma_halo.vex",
    ] {
        for r in roles(&build(&image, path)) {
            assert_eq!(r & slots::BOMB_HALO, 0, "{path}: {r:#x}");
        }
    }
}

/// The Missile explosion's three programs each earn their own pair of bits,
/// and no Bomb or Plasma model earns any of the three.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_missile_explosion_programs_earn_their_pairs() {
    let Some(image) = image() else { return };
    let missile = roles(&build(&image, "/data/weapons/hd_missile_explosion.vex"));
    assert!(!missile.is_empty());
    let pairs = [
        slots::MISSILE_CORE,
        slots::MISSILE_RAYS,
        slots::MISSILE_SHOCK,
    ];
    for pair in pairs {
        assert!(
            missile.iter().any(|r| r & pair == pair),
            "no surface earned {pair:#x}: {missile:x?}"
        );
    }
    for r in &missile {
        assert!(
            pairs.iter().any(|&pair| r & pair == pair),
            "every surface is one of the three: {r:#x}"
        );
        // Each pair shares one bit with a Bomb pair, so the test is the whole
        // pair, never the bit.
        assert_ne!(r & slots::BOMB_FIRE, slots::BOMB_FIRE, "{r:#x}");
        assert_ne!(r & slots::BOMB_SHOCK, slots::BOMB_SHOCK, "{r:#x}");
        assert_eq!(r & slots::BOMB_HALO, 0, "{r:#x}");
    }
    for path in [
        "/data/weapons/hd_bomb_sphere.vex",
        "/data/weapons/hd_bomb_shockwaves.vex",
        "/data/weapons/hd_bomb_halo.vex",
        "/data/weapons/hd_plasma_ring.vex",
        "/data/weapons/hd_plasma_halo.vex",
    ] {
        for r in roles(&build(&image, path)) {
            for pair in pairs {
                assert_ne!(r & pair, pair, "{path}: {r:#x}");
            }
        }
    }
}
