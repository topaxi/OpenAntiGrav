use super::*;

#[test]
fn the_two_atlas_triangles_map_corner_to_corner() {
    for (corners, atlas) in [(&CORNERS_UP, &ATLAS_UP), (&CORNERS_DOWN, &ATLAS_DOWN)] {
        for (corner, texel) in corners.iter().zip(atlas) {
            // Pull each corner a hair inward so it is inside its own cell.
            let p = [corner[0] * 0.999, corner[1] * 0.999];
            let row = if std::ptr::eq(corners, &CORNERS_UP) {
                0.0
            } else {
                1.0
            };
            let at = atlas_at([p[0], p[1] + row * CELL_DY]).expect("inside a cell");
            assert!((at[0] - texel[0]).abs() < 0.5 && (at[1] - texel[1]).abs() < 0.5);
        }
    }
}

#[test]
fn every_point_of_the_plane_is_in_some_cell() {
    for i in 0..200 {
        for j in 0..200 {
            let p = [i as f32 / 100.0 - 1.0, j as f32 / 100.0 - 1.0];
            assert!(atlas_at(p).is_some(), "{p:?}");
        }
    }
}

#[test]
fn a_flat_atlas_bakes_a_flat_picture() {
    let atlas = vec![200u8; 256 * 256 * 4];
    let picture = bake(&atlas, 256, 256).expect("big enough");
    assert_eq!(picture.len(), (VIEWPORT[0] * VIEWPORT[1] * 4) as usize);
    assert!(picture.chunks(4).all(|px| px == [200, 200, 200, 255]));
}

#[test]
fn the_tilt_gives_the_executed_scales() {
    // 59.7 px per unit along x, 55.1 along y, 2.6 px of shear per unit of y.
    let (ax, bx, by) = plane_matrix();
    let (sx, sy) = (ax * 480.0, -by * 272.0);
    assert!((sx - 59.7).abs() < 0.05, "{sx}");
    assert!((sy - 55.1).abs() < 0.05, "{sy}");
    assert!((bx * 480.0 + 2.61).abs() < 0.05, "{}", bx * 480.0);
    // Row period of the lattice on screen.
    assert!((CELL_DY * sy - 47.72).abs() < 0.05);
}
