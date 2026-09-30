use super::*;
use bytemuck::Zeroable;

fn at(x: f32, y: f32) -> GpuVertex {
    GpuVertex {
        position: [x, y, 0.0],
        texcoord: [x, y],
        slots: 0xc4,
        ..GpuVertex::zeroed()
    }
}

/// A triangle wholly inside the rectangle comes back as itself.
#[test]
fn a_triangle_inside_is_untouched() {
    let out = clip_polygon(
        vec![at(0.0, 0.0), at(1.0, 0.0), at(0.0, 1.0)],
        [-10.0, -10.0, 10.0, 10.0],
    );
    assert_eq!(out.len(), 3);
}

/// A triangle wholly outside is gone.
#[test]
fn a_triangle_outside_is_dropped() {
    let out = clip_polygon(
        vec![at(20.0, 0.0), at(21.0, 0.0), at(20.0, 1.0)],
        [-10.0, -10.0, 10.0, 10.0],
    );
    assert!(out.is_empty());
}

/// A triangle straddling one edge is cut on it: every corner is inside, the
/// cut corners sit exactly on the bound, and the texture coordinate was carried
/// to the cut rather than left at its source.
#[test]
fn a_straddling_triangle_is_cut_on_the_edge_with_its_uvs() {
    let out = clip_polygon(
        vec![at(0.0, 0.0), at(20.0, 0.0), at(0.0, 10.0)],
        [-10.0, -10.0, 10.0, 10.0],
    );
    assert!(out.len() >= 3);
    for v in &out {
        assert!(v.position[0] <= 10.0 + 1e-5, "{:?}", v.position);
        assert_eq!(v.texcoord, [v.position[0], v.position[1]]);
        assert_eq!(v.slots, 0xc4, "per-draw constants ride through");
    }
    assert!(out.iter().any(|v| v.position[0] == 10.0));
}
