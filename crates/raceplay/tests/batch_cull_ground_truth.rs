//! Pulse's circuit draws each batch single-sided when the batch's own `pass_mask` says so.
//!
//! **`#[ignore]`d and never run in CI.** It needs the discs in `data/images/`.
//!
//! A trackside board on Moa Therma is a front copy and a back copy in one plane: the front
//! copy faces down the track, the back copy (reversed winding, the picture mirrored) faces
//! the other way, and the original's `GU_CULL_FACE` shows each side its own. Drawn
//! two-sided the copies z-fight, and the board stipples frame to frame while a craft flies
//! past it (`docs/rendering/batch-cull.md`).

use oag_raceplay as race;
use std::path::{Path, PathBuf};

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// The ornament board of Moa Therma's second arch, the one a player sees flicker.
const BOARD_TEXTURE: &str = "concentration shuuchuu2.tga";

/// `track.vex` scene nodes that carry the board: the front copy in two sections' groups and
/// the back copy in one of them.
const FRONT_COPIES: [u32; 2] = [868, 929];
const BACK_COPY: u32 = 928;

/// The original's chase camera at a placed craft a few units from the player's reported pose
/// (PPSSPP, Moa Therma White, `psp-trace.py --camera`, tick 0 of one static placement).
const EYE: [f32; 3] = [344.3, 6.91, 303.4];

fn load(path: &Path, track: Option<&str>) -> race::Loaded {
    race::load(&race::Options {
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        track: track.map(str::to_string),
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("loading {track:?} from {}: {e:#}", path.display()))
}

/// The geometric normal of a draw's first triangle and a point on it.
fn first_triangle(model: &oag_mesh::mesh::Model, node: u32) -> ([f32; 3], [f32; 3], usize) {
    let draw = model
        .draws
        .iter()
        .find(|d| {
            d.node == Some(node)
                && d.texture
                    .and_then(|t| model.textures.get(t))
                    .and_then(Option::as_ref)
                    .is_some_and(|t| t.label == BOARD_TEXTURE)
        })
        .unwrap_or_else(|| panic!("node {node} draws no {BOARD_TEXTURE}"));
    assert!(
        draw.culled,
        "node {node}: the batch authors a single-sided draw"
    );
    let at =
        |k: usize| model.vertices[model.indices[draw.range.start as usize + k] as usize].position;
    let (a, b, c) = (at(0), at(1), at(2));
    let (u, v) = (
        [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
        [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
    );
    (
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ],
        a,
        (draw.range.end - draw.range.start) as usize,
    )
}

fn facing(normal: [f32; 3], on: [f32; 3]) -> f32 {
    (0..3).map(|i| normal[i] * (EYE[i] - on[i])).sum()
}

/// Moa Therma's board is two copies back to back, and only one faces the camera: with the
/// cull honoured the other is not drawn, without it they share every pixel.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_moa_therma_board_is_a_front_and_a_back_copy_and_one_faces_the_camera() {
    let Some(path) = image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let loaded = load(&path, Some(r"Data\Environments\03_Track\track.vex"));
    let model = &loaded.track_model;
    let (front_a, on_a, count_a) = first_triangle(model, FRONT_COPIES[0]);
    let (front_b, on_b, _) = first_triangle(model, FRONT_COPIES[1]);
    let (back, on_back, count_back) = first_triangle(model, BACK_COPY);
    println!(
        "front {:?} on {:?} ({count_a} idx), twin {:?}, back {:?} on {:?} ({count_back} idx)",
        front_a, on_a, front_b, back, on_back
    );
    let (toward_a, toward_b, toward_back) = (
        facing(front_a, on_a),
        facing(front_b, on_b),
        facing(back, on_back),
    );
    assert!(
        toward_a * toward_b > 0.0,
        "the two front copies face the same way ({toward_a}, {toward_b})"
    );
    assert!(
        toward_a * toward_back < 0.0,
        "the back copy faces the other way ({toward_a} against {toward_back})"
    );
    assert!(
        toward_back > 0.0,
        "from the original's camera it is the back copy that faces the eye ({toward_back})"
    );
    assert!(loaded.track_model.per_batch_cull);
}

/// The title's rule decides: Pulse's circuits cull per batch on both of its discs (the PSP
/// measured, the PS2 carried), and Pure's, which nothing here measured, stay two-sided.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_circuit_culls_per_batch_where_its_titles_rule_says_so() {
    for (name, expected) in [
        ("data/images/pulse-psp-usa.chd", true),
        ("data/images/pulse-ps2-eu.chd", true),
        ("data/images/pure-psp-usa.chd", false),
    ] {
        let Some(path) = image(name) else { continue };
        let loaded = load(&path, None);
        assert_eq!(loaded.track_model.per_batch_cull, expected, "{name}");
        let culled = loaded.track_model.draws.iter().filter(|d| d.culled).count();
        assert!(
            culled * 2 > loaded.track_model.draws.len(),
            "{name}: most of a circuit's batches author a single-sided draw ({culled} of {})",
            loaded.track_model.draws.len()
        );
    }
}
