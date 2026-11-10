use super::*;

fn sprite(width: u16, height: u16, fill: u8) -> Sprite {
    Sprite {
        width,
        height,
        rgba: vec![fill; usize::from(width) * usize::from(height) * 4].into(),
    }
}

#[test]
fn a_placed_sprite_lands_on_the_sheet_where_its_rect_says() {
    let mut sheet = Sheet::default();
    assert!(sheet.pixels().is_none());
    let first = sheet.place(&sprite(64, 64, 7)).expect("fits");
    let second = sheet.place(&sprite(32, 32, 9)).expect("fits");
    assert_eq!(
        first,
        [0.0, 0.0, 64.0 / SHEET_SIZE as f32, 64.0 / SHEET_SIZE as f32]
    );
    // Same shelf, `PAD` texels to the right.
    assert_eq!(second[0], (64 + PAD) as f32 / SHEET_SIZE as f32);
    let pixels = sheet.pixels().expect("allocated");
    assert_eq!(pixels[0], 7);
    assert_eq!(pixels[(64 + PAD) as usize * 4], 9);
    assert_eq!(sheet.len(), 2);
}

#[test]
fn the_sheet_holds_only_the_rows_its_sprites_reach() {
    let mut sheet = Sheet::default();
    assert_eq!(sheet.rows(), 0);
    sheet.place(&sprite(64, 40, 7)).expect("fits");
    assert_eq!(sheet.rows(), 40);
    let row_bytes = SHEET_SIZE as usize * 4;
    assert_eq!(sheet.pixels().expect("placed").len(), 40 * row_bytes);

    // A wider sprite than is left on the shelf opens the next one, below the
    // first shelf's tallest sprite and the pad.
    let wide = u16::try_from(SHEET_SIZE).expect("fits a u16");
    sheet.place(&sprite(wide, 10, 9)).expect("fits");
    assert_eq!(sheet.rows(), 40 + PAD + 10);

    // What the grown list holds is what a full-size list would, zero below.
    let pixels = sheet.pixels().expect("placed");
    let mut full = vec![0_u8; SHEET_SIZE as usize * row_bytes];
    full[..pixels.len()].copy_from_slice(pixels);
    assert_eq!(full[39 * row_bytes], 7);
    assert_eq!(full[(40 + PAD as usize) * row_bytes], 9);
    assert!(full[(52 * row_bytes)..].iter().all(|&byte| byte == 0));
    assert_eq!(pixels[..64 * 4], full[..64 * 4]);
}

#[test]
fn an_identical_sprite_is_placed_once() {
    let mut sheet = Sheet::default();
    let a = sheet.place(&sprite(32, 32, 1)).expect("fits");
    let generation = sheet.generation();
    let b = sheet.place(&sprite(32, 32, 1)).expect("fits");
    assert_eq!(a, b);
    assert_eq!(sheet.len(), 1);
    assert_eq!(sheet.generation(), generation, "no change, no re-upload");
}

#[test]
fn a_full_sheet_refuses_rather_than_overlapping() {
    let mut sheet = Sheet::default();
    let mut placed = 0u8;
    while sheet.place(&sprite(256, 256, placed)).is_some() {
        placed += 1;
    }
    // As many 256s fit across as `PAD` leaves room for, and as many down.
    let per_side = (SHEET_SIZE + PAD) / (256 + PAD);
    assert_eq!(u32::from(placed), per_side * per_side);
}

#[test]
fn an_atlas_cell_is_its_grid_square_pulled_in_half_a_texel() {
    let atlas = Atlas {
        columns: 4,
        rows: 4,
        random_frame: true,
    };
    assert_eq!(atlas.frames(), 16);
    let texel = 0.5 / SHEET_SIZE as f32;
    // Frame 5 is column 1, row 1.
    let cell = atlas.cell([0.0, 0.0, 0.5, 0.25], 5);
    assert!((cell[0] - (0.125 + texel)).abs() < 1e-7);
    assert!((cell[1] - (0.0625 + texel)).abs() < 1e-7);
    assert!((cell[2] - (0.25 - texel)).abs() < 1e-7);
    assert!((cell[3] - (0.125 - texel)).abs() < 1e-7);
}

#[test]
fn mapping_a_quad_marks_it_sampled_and_moves_its_uvs() {
    let mut corners = [oag_mesh::mesh::GpuVertex {
        texcoord: [1.0, 1.0],
        normal: [0.0, 0.0, 1.0],
        ..bytemuck::Zeroable::zeroed()
    }];
    map_to_cell(&mut corners, [0.5, 0.5, 0.75, 1.0]);
    assert_eq!(corners[0].texcoord, [0.75, 1.0]);
    assert_eq!(corners[0].normal[0], 1.0);
}
