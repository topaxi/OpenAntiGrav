use super::*;

fn ring_set(cell: (i32, i32)) -> Vec<(i32, i32)> {
    RING.iter()
        .map(|(dc, dr)| (cell.0 + dc, cell.1 + dr))
        .collect()
}

#[test]
fn the_ring_is_eighteen_cells_around_the_footprint_and_not_in_it() {
    let mut cells = RING.to_vec();
    cells.sort_unstable();
    cells.dedup();
    assert_eq!(cells.len(), 18);
    for (dc, dr) in RING {
        assert!(
            !((0..FOOTPRINT.0).contains(&dc) && (0..FOOTPRINT.1).contains(&dr)),
            "({dc}, {dr}) is a cell of the tile"
        );
    }
    // Two columns each side on the footprint's two rows, five on the rows
    // above and below (`0x8106fbd8`).
    for row in [0, 1] {
        assert_eq!(RING.iter().filter(|(_, dr)| *dr == row).count(), 4);
    }
    for row in [-1, 2] {
        assert_eq!(RING.iter().filter(|(_, dr)| *dr == row).count(), 5);
    }
}

#[test]
fn the_higher_fill_wins_and_the_order_of_buttons_does_not_matter() {
    let near = CanvasButton {
        cell: (10, 10),
        tile: Tile::Raised,
        ring: 4,
    };
    let far = CanvasButton {
        cell: (14, 10),
        tile: Tile::Raised,
        ring: 2,
    };
    let one = resolve(&[near, far]);
    let mut other = resolve(&[far, near]);
    let mut sorted = one.clone();
    sorted.sort_by_key(|p| p.cell);
    other.sort_by_key(|p| p.cell);
    assert_eq!(sorted, other, "painting order does not change the result");
    // (13, 10) is in `near`'s ring (fill 4) and in `far`'s ring (fill 2).
    let shared = one.iter().find(|p| p.cell == (13, 10)).expect("painted");
    assert_eq!(shared.fill, 4);
}

#[test]
fn a_tiles_own_cells_beat_every_ring_and_a_raised_tile_is_not_a_ground_cell() {
    let raised = CanvasButton {
        cell: (10, 10),
        tile: Tile::Raised,
        ring: 0,
    };
    // `raised`'s footprint reaches (12, 11); this button's ring covers (12, 11)
    // at fill 4.
    let neighbour = CanvasButton {
        cell: (9, 11),
        tile: Tile::Flat([0.5; 3]),
        ring: 4,
    };
    assert!(ring_set((9, 11)).contains(&(12, 11)));
    let painted = resolve(&[raised, neighbour]);
    assert!(
        !painted.iter().any(|p| p.cell == (12, 11)),
        "a raised tile's cell is the prism's, not a ring cell"
    );
    let flat = painted
        .iter()
        .find(|p| p.cell == (9, 11))
        .expect("a flat tile's cell");
    assert_eq!((flat.fill, flat.colour), (0, [0.5; 3]));
    let flat_over_ring = resolve(&[
        neighbour,
        CanvasButton {
            cell: (6, 10),
            tile: Tile::Raised,
            ring: 4,
        },
    ]);
    assert!(
        flat_over_ring
            .iter()
            .filter(|p| p.fill == 4)
            .all(|p| !(9..12).contains(&p.cell.0) || !(11..13).contains(&p.cell.1)),
        "a ring never paints over a flat tile's cells (priority 6)"
    );
}

#[test]
fn a_button_with_no_ring_paints_no_ring_cell() {
    let painted = resolve(&[CanvasButton {
        cell: (20, 20),
        tile: Tile::Flat([1.0; 3]),
        ring: 0,
    }]);
    assert_eq!(painted.len(), 6, "only its six cells");
    assert!(painted.iter().all(|p| p.fill == 0));
}

/// An atlas whose red channel is its `x`, scaled: a shifted read shows as a
/// different red.
fn gradient() -> (Vec<u8>, u32, u32) {
    let (w, h) = (400_u32, 256_u32);
    let mut rgba = Vec::new();
    for _y in 0..h {
        for x in 0..w {
            rgba.extend([(x * 255 / (w - 1)) as u8, 0, 0, 255]);
        }
    }
    (rgba, w, h)
}

fn mean_red(picture: &Picture) -> f32 {
    let texels = picture.rgba.chunks_exact(4).filter(|t| t[3] == 255);
    let (sum, count) = texels.fold((0.0, 0.0), |(s, n), t| (s + f32::from(t[0]), n + 1.0));
    sum / count
}

#[test]
fn each_fill_reads_the_atlas_where_its_shift_points() {
    let (atlas, w, h) = gradient();
    let pictures = bake(&atlas, w, h).expect("the atlas holds every shifted triangle");
    assert_eq!(pictures.len(), 10);
    let red = |fill: u8, up: bool| {
        let picture = pictures
            .iter()
            .find(|p| p.name == name(fill, up))
            .expect("picture");
        mean_red(picture)
    };
    for up in [true, false] {
        // The red grows with `x`, and the fills' x shifts are 0, 188, 141, 94, 47.
        let order: Vec<u8> = {
            let mut fills = vec![0, 1, 2, 3, 4];
            fills.sort_by(|a, b| red(*a, up).total_cmp(&red(*b, up)));
            fills
        };
        assert_eq!(order, vec![0, 4, 3, 2, 1], "up = {up}");
    }
}

#[test]
fn a_picture_is_clear_outside_its_cell_and_the_anchor_is_inside_it() {
    let (atlas, w, h) = gradient();
    let pictures = bake(&atlas, w, h).expect("baked");
    for up in [true, false] {
        let picture = pictures.iter().find(|p| p.name == name(0, up)).unwrap();
        let layout = layout(up);
        assert_eq!(picture.size, layout.size);
        let (w, h) = picture.size;
        let alpha = |x: u32, y: u32| picture.rgba[((y * w + x) * 4 + 3) as usize];
        assert_eq!(alpha(0, 0), 0, "the corner is outside the cell");
        assert_eq!(alpha(w - 1, h - 1), 0);
        assert_eq!(
            alpha(layout.anchor.0 as u32, layout.anchor.1 as u32),
            255,
            "the centre of a cell is in it"
        );
        let covered = picture.rgba.chunks_exact(4).filter(|t| t[3] == 255).count();
        // A cell is about 60 x 48 px, half of that as a triangle.
        assert!((1200..2000).contains(&covered), "{covered} px");
    }
}

#[test]
fn a_cell_lands_where_the_camera_projects_it() {
    let camera = Camera::scrolled((100.0, -40.0));
    for cell in [(67, 31), (68, 31)] {
        let at = camera.project([cell.0 as f32 * CELL_DX, cell.1 as f32 * CELL_DY, 0.0]);
        let origin = origin(&camera, cell);
        let anchor = layout(is_up(cell)).anchor;
        assert!((origin.0 + anchor.0 - at.0).abs() < 1e-3);
        assert!((origin.1 + anchor.1 - at.1).abs() < 1e-3);
    }
}
