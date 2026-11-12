use super::*;

const CENTRE: (f32, f32) = (20.0, 10.0);

fn ground(_: (i32, i32)) -> u8 {
    0
}

/// The cell at column offset `dc` from the centre column on the centre row.
fn cell_at(dc: i32) -> (i32, i32) {
    (
        (CENTRE.0 / CELL_DX) as i32 + dc,
        (CENTRE.1 / CELL_DY).round() as i32,
    )
}

fn run(levels: &[f32], priority: &dyn Fn((i32, i32)) -> u8) -> Wave {
    let mut wave = Wave::default();
    for &level in levels {
        wave.step(level, Some(CENTRE), priority, &[]);
    }
    wave
}

#[test]
fn the_centre_cell_follows_a_step_in_the_bass_at_full_gain() {
    // d is about 0 at the cell on the centre: f = 1, k = 0, so the first
    // step's lift is 600 * (h0 - h1) = 600 * 0.1 = 60.
    let mut wave = Wave::default();
    let centre = cell_at(0);
    wave.step(0.1, Some(CENTRE), &ground, &[]);
    let dx = centre.0 as f32 * CELL_DX - CENTRE.0;
    let dy = centre.1 as f32 * CELL_DY - CENTRE.1;
    let f = (RADIUS * RADIUS - (dx * dx + dy * dy)) / (RADIUS * RADIUS);
    let expected = (600.0 * f * 0.1) as i32;
    // The first step's decay is int(230 / 60) = 3.
    let alpha = expected.min(255) - 3;
    assert_eq!(wave.cells[&centre].alpha, alpha);
    // The texture eased 15 % of the way.
    assert!((wave.cells[&centre].shown - 0.15 * alpha as f32).abs() < 1e-3);
}

fn falloff(cell: (i32, i32)) -> f32 {
    let dx = cell.0 as f32 * CELL_DX - CENTRE.0;
    let dy = cell.1 as f32 * CELL_DY - CENTRE.1;
    (RADIUS * RADIUS - (dx * dx + dy * dy)) / (RADIUS * RADIUS)
}

#[test]
fn a_far_cell_follows_the_bass_late() {
    // The cell 3 units out has f = 40/49 or so and k = int(30 (1 - f)) = 5. A
    // bass that was 0.3 five steps ago and 0 six steps ago lifts it by
    // int(600 f * 0.3) and leaves the centre cell, whose k is 0, untouched.
    let far = cell_at(6);
    let f = falloff(far);
    let k = ((1.0 - f) * 30.0) as usize;
    assert_eq!(k, 5, "f = {f}");
    let mut wave = Wave::default();
    wave.history[k] = 0.3;
    wave.lift(CENTRE, &ground);
    assert_eq!(wave.cells[&far].alpha, (600.0 * f * 0.3) as i32);
    assert!(!wave.cells.contains_key(&cell_at(0)));
}

#[test]
fn a_cell_outside_the_radius_never_moves() {
    let wave = run(&[0.0, 1.0, 0.0, 1.0, 0.0], &ground);
    let outside = cell_at(((RADIUS + 1.0) / CELL_DX) as i32);
    assert_eq!(wave.height(outside), 0.0);
    assert!(wave.height(cell_at(0)) > 0.0);
}

#[test]
fn a_painted_tile_never_moves_and_a_ring_cell_does() {
    let tile = cell_at(0);
    let ring = cell_at(2);
    let priority = move |cell: (i32, i32)| {
        if cell == tile {
            7
        } else if cell == ring {
            3
        } else {
            0
        }
    };
    let wave = run(&[0.0, 1.0, 0.0, 1.0], &priority);
    assert_eq!(wave.height(tile), 0.0);
    assert!(wave.height(ring) > 0.0);
}

#[test]
fn the_alpha_is_clamped_and_decays_at_230_a_second() {
    let mut wave = Wave::default();
    let centre = cell_at(0);
    // A huge step saturates at 255.
    wave.step(1.0, Some(CENTRE), &ground, &[]);
    assert_eq!(wave.cells[&centre].alpha, 255 - 3);
    // With no centre nothing lifts; a second of steps drains 230.
    for _ in 0..59 {
        wave.step(1.0, None, &ground, &[]);
    }
    let left = wave.cells[&centre].alpha;
    assert!((255 - 230 - left).abs() <= 4, "left {left}");
}

#[test]
fn a_flat_group_moves_as_one_height() {
    let mut wave = run(&[0.0, 1.0], &ground);
    let group = [
        cell_at(0),
        cell_at(1),
        cell_at(2),
        (cell_at(0).0, cell_at(0).1 + 1),
        (cell_at(1).0, cell_at(1).1 + 1),
        (cell_at(2).0, cell_at(2).1 + 1),
    ];
    wave.flatten(&group);
    let first = wave.height(group[0]);
    assert!(
        group
            .iter()
            .all(|&cell| (wave.height(cell) - first).abs() < 1e-6)
    );
}
