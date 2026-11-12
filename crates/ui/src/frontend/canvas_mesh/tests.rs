use super::*;

const CELL: (i32, i32) = (60, 30);

fn scene<'a>(
    camera: &'a Camera,
    height_of: &'a dyn Fn((i32, i32)) -> f32,
    skip: &'a dyn Fn((i32, i32)) -> bool,
    place: &'a dyn Fn((i32, i32), [[f32; 2]; 3]) -> Option<Placed>,
) -> Scene<'a> {
    Scene {
        camera,
        scale: (1.0, 1.0),
        height_of,
        skip,
        place,
    }
}

fn flat(_: (i32, i32)) -> f32 {
    0.0
}
fn nothing(_: (i32, i32)) -> bool {
    false
}
fn sheet(_: (i32, i32), at: [[f32; 2]; 3]) -> Option<Placed> {
    Some(Placed {
        at,
        uv: at.map(|corner| [corner[0] + 100.0, corner[1] + 200.0]),
    })
}

#[test]
fn a_raised_cell_draws_its_walls_and_a_flat_one_only_its_top() {
    let camera = Camera::scrolled((0.0, 0.0));
    let raised = |cell| if cell == CELL { 0.5 } else { 0.0 };
    let high = triangles(&scene(&camera, &raised, &nothing, &sheet), &[CELL]);
    let low = triangles(&scene(&camera, &flat, &nothing, &sheet), &[CELL]);
    // Every cell round it draws one top; the raised one adds six wall triangles.
    assert_eq!(high.len(), low.len() + 6);
}

#[test]
fn the_top_face_is_the_ground_picture_moved_by_the_lift() {
    let camera = Camera::scrolled((0.0, 0.0));
    let raised = |cell| if cell == CELL { 0.5 } else { 0.0 };
    let high = triangles(&scene(&camera, &raised, &nothing, &sheet), &[CELL]);
    let low = triangles(&scene(&camera, &flat, &nothing, &sheet), &[CELL]);
    // Some triangle of the raised list has the texture coordinates of one of the
    // flat list and all three positions moved by one screen vector, upwards.
    let moved = high.iter().any(|a| {
        low.iter().any(|b| {
            let lift = (a[0].at[0] - b[0].at[0], a[0].at[1] - b[0].at[1]);
            lift.1 < -1.0
                && lift.1 > -40.0
                && (0..3).all(|at| {
                    a[at].uv == b[at].uv
                        && (a[at].at[0] - b[at].at[0] - lift.0).abs() < 1e-3
                        && (a[at].at[1] - b[at].at[1] - lift.1).abs() < 1e-3
                })
        })
    });
    assert!(moved, "a raised cell's face rises on the screen");
}

#[test]
fn a_cell_beside_a_taller_one_is_shaded_and_an_isolated_one_is_not() {
    let camera = Camera::scrolled((0.0, 0.0));
    let tall = |cell| {
        if cell == (CELL.0 + 1, CELL.1) {
            0.8
        } else {
            0.0
        }
    };
    let beside = triangles(
        &scene(&camera, &tall, &nothing, &sheet),
        &[(CELL.0 + 1, CELL.1)],
    );
    let darkest = beside
        .iter()
        .flatten()
        .map(|corner| corner.color[0])
        .fold(1.0_f32, f32::min);
    assert!(
        darkest < 0.9,
        "the skirt of a raised cell is shaded: {darkest}"
    );
    let alone = triangles(&scene(&camera, &flat, &nothing, &sheet), &[CELL]);
    assert!(
        alone
            .iter()
            .flatten()
            .all(|corner| corner.color[0] >= 1.0 - 1e-6)
    );
}

#[test]
fn skipped_cells_and_cells_without_a_picture_draw_nothing() {
    let camera = Camera::scrolled((0.0, 0.0));
    let all = |_| true;
    assert!(triangles(&scene(&camera, &flat, &all, &sheet), &[CELL]).is_empty());
    let none = |_, _| None;
    assert!(triangles(&scene(&camera, &flat, &nothing, &none), &[CELL]).is_empty());
}
