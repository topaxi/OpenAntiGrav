use super::*;

fn pixel(picture: &[u8], at: (f32, f32)) -> [u8; 4] {
    let (width, _) = layout().size;
    let (x, y) = (at.0 as usize, at.1 as usize);
    let i = (y * width as usize + x) * 4;
    [picture[i], picture[i + 1], picture[i + 2], picture[i + 3]]
}

#[test]
fn the_top_face_is_the_cell_colour_and_opaque() {
    let picture = bake();
    let anchor = layout().anchor;
    assert_eq!(pixel(&picture, anchor), [255, 255, 255, 255]);
}

#[test]
fn the_wall_below_the_face_is_opaque_and_darkens_towards_its_foot() {
    let picture = bake();
    let anchor = layout().anchor;
    // The top face's lower edge is a row and a half of cells below its centre;
    // the front wall hangs below it by the prism's height on screen.
    let (_, face_height) = super::super::canvas_camera::tile_face();
    let edge = anchor.1 + face_height * 0.5;
    let wall = |below: f32| pixel(&picture, (anchor.0, edge + below));
    let upper = wall(3.0);
    let lower = wall(22.0);
    assert_eq!(upper[3], 255, "the wall is opaque: {upper:?}");
    assert_eq!(lower[3], 255, "{lower:?}");
    assert!(upper[0] < 255, "a wall is shaded: {upper:?}");
    assert!(
        lower[0] < upper[0],
        "darker at its foot: {upper:?} {lower:?}"
    );
}

#[test]
fn the_ground_beside_the_tile_is_shade_only_and_far_ground_is_clear() {
    let picture = bake();
    let (width, height) = layout().size;
    // Black with alpha: it darkens the lattice under it.
    let corner = pixel(&picture, (1.0, 1.0));
    assert_eq!(
        corner,
        [0, 0, 0, 0],
        "the picture's corner is untouched ground"
    );
    let shaded = picture
        .chunks(4)
        .filter(|px| px[3] > 0 && px[3] < 255 && px[0] == 0)
        .count();
    assert!(
        shaded > (width * height) as usize / 100,
        "the ground at the tile's foot is shaded: {shaded} of {} pixels",
        width * height
    );
}

#[test]
fn a_tint_is_the_colours_square_root() {
    assert_eq!(tint([0.25, 1.0, 0.0]), [0.5, 1.0, 0.0, 1.0]);
}

#[test]
fn the_cell_colours_are_the_canvases_own() {
    assert_eq!(
        colours::PASSED,
        [0x20 as f32 / 255.0, 0xd0 as f32 / 255.0, 0.0]
    );
}
