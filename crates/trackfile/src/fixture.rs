//! A track authored by this project, for tests and for trying the format: a
//! rounded octagon, 40 units wide, one speed pad and one weapon pad.
//!
//! **Chosen, not measured**: every dimension below is this project's own (the
//! width and the pad box follow the foreign mapping's choices so the host's
//! physics is comfortable on it). The generator uses arithmetic and `sqrt` only,
//! so the committed fixture is reproducible on every platform
//! (`cargo run -p oag-trackfile --example make_fixture`).

use oag_vex::collision::{
    CollisionGeometry, CollisionMesh, CollisionNode, HEADER_WORD, SurfaceKind,
};
use oag_vex::pads::PadVolume;
use oag_vex::track::{AiTrack, Junction, Path, SHORT_POINT_VERSION, SplinePoint, StartPosition};

use crate::Track;

const CORNERS: [[f32; 2]; 8] = [
    [-900.0, -300.0],
    [-300.0, -500.0],
    [300.0, -500.0],
    [900.0, -300.0],
    [900.0, 300.0],
    [300.0, 500.0],
    [-300.0, 500.0],
    [-900.0, 300.0],
];
const HALF_WIDTH: f32 = 20.0;
const WALL_HEIGHT: f32 = 15.0;
/// How far behind the lap line the start slot sits: the host engine's own
/// start-line advance (`oag_race::Course::START_LINE_ADVANCE`), repeated here
/// because this crate must not depend on the simulation.
const START_BEHIND: f32 = 154.0;
const SMOOTHINGS: usize = 3;

fn chaikin(ring: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let n = ring.len();
    (0..n)
        .flat_map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            [
                [a[0] * 0.75 + b[0] * 0.25, a[1] * 0.75 + b[1] * 0.25],
                [a[0] * 0.25 + b[0] * 0.75, a[1] * 0.25 + b[1] * 0.75],
            ]
        })
        .collect()
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add_scaled(a: [f32; 3], d: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] + d[0] * s, a[1] + d[1] * s, a[2] + d[2] * s]
}

fn unit(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / l, v[1] / l, v[2] / l]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = sub(a, b);
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// The fixture track.
#[must_use]
pub fn oval() -> Track {
    let mut ring = CORNERS.to_vec();
    for _ in 0..SMOOTHINGS {
        ring = chaikin(&ring);
    }
    let n = ring.len();
    let pos: Vec<[f32; 3]> = ring.iter().map(|p| [p[0], 0.0, p[1]]).collect();
    let up = [0.0, 1.0, 0.0];
    let forward: Vec<[f32; 3]> = (0..n)
        .map(|i| unit(sub(pos[(i + 1) % n], pos[(i + n - 1) % n])))
        .collect();
    let right: Vec<[f32; 3]> = forward.iter().map(|f| cross(*f, up)).collect();

    let gaps: Vec<f32> = (0..n).map(|i| dist(pos[i], pos[(i + 1) % n])).collect();
    let total: f32 = gaps.iter().sum();
    let mut run = 0.0;
    let points: Vec<SplinePoint> = (0..n)
        .map(|i| {
            let progress = run / total;
            run += gaps[i];
            SplinePoint {
                pos: pos[i],
                tangent: forward[i],
                down: [0.0, -1.0, 0.0],
                lateral: right[i],
                progress,
                half_width_left: HALF_WIDTH,
                half_width_right: HALF_WIDTH,
                ai_bound_left: -HALF_WIDTH * 0.6,
                ai_bound_right: HALF_WIDTH * 0.6,
                racing_line: 0.0,
                section_id: 0xff,
                flags: 0,
                light_scale: [255; 4],
                unk_0x5c: None,
                unk_0x5d: None,
                unk_0x66: None,
            }
        })
        .collect();
    let max_spacing = gaps.iter().copied().fold(0.0f32, f32::max);

    let edge = |i: usize, side: f32, y: f32| {
        let mut p = add_scaled(pos[i], right[i], side * HALF_WIDTH);
        p[1] = y;
        p
    };
    let mut floor = Vec::new();
    let mut inner = Vec::new();
    let mut outer = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        floor.push([
            edge(i, -1.0, 0.0),
            edge(i, 1.0, 0.0),
            edge(j, -1.0, 0.0),
            edge(j, 1.0, 0.0),
        ]);
        inner.push([
            edge(i, -1.0, 0.0),
            edge(j, -1.0, 0.0),
            edge(i, -1.0, WALL_HEIGHT),
            edge(j, -1.0, WALL_HEIGHT),
        ]);
        outer.push([
            edge(j, 1.0, 0.0),
            edge(i, 1.0, 0.0),
            edge(j, 1.0, WALL_HEIGHT),
            edge(i, 1.0, WALL_HEIGHT),
        ]);
    }
    let mesh = |quads: Vec<[[f32; 3]; 4]>| {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        for q in quads {
            let k = vertices.len() as u16;
            vertices.extend(q);
            triangles.push([k, k + 1, k + 2]);
            triangles.push([k + 1, k + 3, k + 2]);
        }
        CollisionMesh {
            vertex_scalars: vec![1.0; vertices.len()],
            vertices,
            triangles,
            chunks: Vec::new(),
        }
    };
    let node = |kind, name: &str, index, quads| CollisionNode {
        kind,
        node_index: index,
        name: Some(name.to_string()),
        geometry: CollisionGeometry {
            version: HEADER_WORD,
            meshes: vec![mesh(quads)],
        },
    };
    let mut walls = inner;
    walls.extend(outer);
    let nodes = vec![
        node(SurfaceKind::Floor, "fixture floor", 0, floor),
        node(SurfaceKind::Wall, "fixture wall", 1, walls),
    ];

    let pad_at = |i: usize| {
        let rows = [
            [right[i][0], right[i][1], right[i][2], 0.0],
            [up[0], up[1], up[2], 0.0],
            [forward[i][0], forward[i][1], forward[i][2], 0.0],
            [pos[i][0], pos[i][1], pos[i][2], 1.0],
        ];
        let mut to_world = [0.0; 16];
        for (r, row) in rows.iter().enumerate() {
            to_world[r * 4..r * 4 + 4].copy_from_slice(row);
        }
        PadVolume {
            to_world,
            min: [-HALF_WIDTH * 0.5, -2.0, -10.0],
            max: [HALF_WIDTH * 0.5, 8.0, 10.0],
            disabled: 0.0,
        }
    };

    let mut remaining = START_BEHIND;
    let mut at = 0;
    let mut start = pos[0];
    for _ in 0..n {
        let prev = (at + n - 1) % n;
        let step = dist(pos[prev], pos[at]);
        if step >= remaining {
            let t = remaining / step;
            start = add_scaled(pos[at], sub(pos[prev], pos[at]), t);
            break;
        }
        remaining -= step;
        at = prev;
        start = pos[at];
    }
    let f = unit([forward[at][0], 0.0, forward[at][2]]);
    Track {
        id: "fixture-octagon".to_string(),
        title: "Fixture Octagon".to_string(),
        ai: AiTrack {
            version: SHORT_POINT_VERSION,
            paths: vec![Path {
                points,
                max_spacing,
                entry: Some(0),
                exit: Some(0),
            }],
            junctions: vec![Junction {
                prev: [Some(0), None],
                next: [Some(0), None],
            }],
        },
        start: StartPosition {
            position: start,
            left: cross(up, f),
            up,
            forward: f,
        },
        nodes,
        speedup_pads: vec![pad_at(n / 4)],
        weapon_pads: vec![pad_at(n * 3 / 4)],
        notes: vec![
            "authored by this project: a rounded octagon, 40 units wide (chosen, not measured)"
                .to_string(),
        ],
    }
}
