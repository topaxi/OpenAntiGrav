//! Renders a synthetic collision soup through the real capture path.
//!
//! `#[ignore]`d because it needs a GPU adapter, which CI does not have - the
//! same reason no other test in this crate touches wgpu. Run it by hand to get a
//! picture of the collision view without a disc image:
//!
//! ```sh
//! cargo nextest run -p oag-render --run-ignored all -E 'test(collision)' --no-capture
//! ```
//!
//! It writes to `$OAG_COLLISION_PNG` or a temporary file. What it asserts is
//! only that the whole path runs and produces a PNG; the value is the picture,
//! and the geometry below is shaped so a wrong answer is obvious in it - a
//! straight corridor with a floor between two walls, which must look like a
//! corridor.

use oag_formats::collision::{CollisionGeometry, CollisionMesh, CollisionNode, SurfaceKind};
use oag_render::collision::{self, Style};

/// A quad as two triangles, from four corners in order.
fn quad(a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3]) -> CollisionMesh {
    CollisionMesh {
        vertices: vec![a, b, c, d],
        triangles: vec![[0, 1, 2], [0, 2, 3]],
        vertex_scalars: vec![1.0; 4],
        chunks: Vec::new(),
    }
}

fn node(kind: SurfaceKind, meshes: Vec<CollisionMesh>) -> CollisionNode {
    CollisionNode {
        kind,
        node_index: 0,
        name: None,
        geometry: CollisionGeometry { version: 0, meshes },
    }
}

/// A straight corridor: floor strip, two walls, segmented so there is enough
/// geometry for the outline pass to be worth looking at.
fn corridor() -> Vec<CollisionNode> {
    const SEGMENTS: usize = 40;
    const LENGTH: f32 = 400.0;
    const HALF_WIDTH: f32 = 12.0;
    const WALL_HEIGHT: f32 = 8.0;

    let mut floor = Vec::new();
    let mut walls = Vec::new();
    for i in 0..SEGMENTS {
        let z0 = LENGTH * (i as f32 / SEGMENTS as f32) - LENGTH * 0.5;
        let z1 = LENGTH * ((i + 1) as f32 / SEGMENTS as f32) - LENGTH * 0.5;
        floor.push(quad(
            [-HALF_WIDTH, 0.0, z0],
            [HALF_WIDTH, 0.0, z0],
            [HALF_WIDTH, 0.0, z1],
            [-HALF_WIDTH, 0.0, z1],
        ));
        for side in [-HALF_WIDTH, HALF_WIDTH] {
            walls.push(quad(
                [side, 0.0, z0],
                [side, WALL_HEIGHT, z0],
                [side, WALL_HEIGHT, z1],
                [side, 0.0, z1],
            ));
        }
    }
    vec![
        node(SurfaceKind::Floor, floor),
        node(SurfaceKind::Wall, walls),
    ]
}

#[test]
#[ignore = "needs a GPU adapter"]
fn a_synthetic_corridor_renders_through_the_capture_path() {
    let nodes = corridor();

    // The numbers the CLI prints, asserted here so the report is not the only
    // thing checking them.
    let walls = collision::bounds_of(&nodes, |k| k == SurfaceKind::Wall).expect("wall bounds");
    let floor = collision::bounds_of(&nodes, |k| k == SurfaceKind::Floor).expect("floor bounds");
    assert!(
        collision::contains(walls, floor, 0.0),
        "the walls must enclose the floor, {walls:?} vs {floor:?}"
    );

    let stem = std::env::var("OAG_COLLISION_PNG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("oag-collision-corridor.png"));

    for (style, suffix) in [(Style::Wireframe, "wire"), (Style::Solid, "solid")] {
        let model = collision::build_model("corridor", &nodes, style, false);
        assert!(!model.vertices.is_empty(), "{style:?} produced nothing");

        let path = stem.with_file_name(format!(
            "{}-{suffix}.png",
            stem.file_stem().unwrap_or_default().to_string_lossy()
        ));
        oag_render::mesh_render::capture_from(
            &model,
            &path,
            1280,
            960,
            0.9,
            0.85,
            oag_render::mesh_render::Anisotropy::default(),
        )
        .expect("capturing the collision view");

        let bytes = std::fs::read(&path).expect("reading the capture back");
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "not a PNG");
        assert!(bytes.len() > 4096, "suspiciously small capture");
        eprintln!("wrote {}", path.display());
    }
}
