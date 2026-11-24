//! The hex grid as the game drives it: `Picker::tick` first, then the presses of the frame.

use super::*;

const DT: f32 = 1.0 / 60.0;

fn press(picker: &mut Picker, button: Button) {
    let mut input = Input::default();
    input.begin_frame(0);
    input.begin_frame(1 << button as u32);
    picker.update(&mut input);
}

fn frame(picker: &mut Picker, button: Option<Button>) {
    picker.tick(DT);
    if let Some(button) = button {
        press(picker, button);
    }
}

fn grid() -> hex::HexGrid {
    layout().hd.as_deref().unwrap().hex.unwrap()
}

fn centre_entry(grid: &hex::HexGrid, picker: &Picker) -> usize {
    grid.cells_at(
        picker.index(),
        picker.entries().len(),
        0.0,
        Some(picker.slide()),
    )
    .iter()
    .find(|c| c.offset == 0)
    .unwrap()
    .entry
}

fn ring_y(picker: &Picker) -> f32 {
    let layout = layout();
    let extra = layout.hd.as_deref().unwrap();
    body(picker, &layout, extra, &Frame::default(), &|n| sheet(n))
        .into_iter()
        .rev()
        .find_map(|draw| match draw {
            Draw::Sprite { rect, .. } if rect[2] == 92.0 => Some(rect[1]),
            _ => None,
        })
        .unwrap()
}

/// The grid draws what the slide shows, not the selection: right after a press, before the
/// tick that starts the slide, the old team is still the centre column.
#[test]
fn a_press_does_not_flash_the_new_team_before_its_slide() {
    let grid = grid();
    let mut picker = picker_of(&["A", "B", "C", "D", "E", "F"], "C");
    frame(&mut picker, None);
    let before = grid.cells_at(2, 6, 0.0, Some(picker.slide()));
    press(&mut picker, Button::Right);
    assert_eq!(picker.index(), 3);
    assert_eq!(
        grid.cells_at(picker.index(), 6, 0.0, Some(picker.slide())),
        before
    );
}

/// Presses during a slide queue: one column per slide, the lattice never skips a team.
#[test]
fn presses_during_a_slide_step_one_column_each() {
    let grid = grid();
    let mut picker = picker_of(&["A", "B", "C", "D", "E", "F"], "C");
    frame(&mut picker, None);
    press(&mut picker, Button::Right);
    let mut seen = Vec::new();
    for f in 0..40 {
        frame(&mut picker, (f == 2).then_some(Button::Right));
        let centre = centre_entry(&grid, &picker);
        if seen.last() != Some(&centre) {
            seen.push(centre);
        }
        assert!(
            !picker.slide().moving() || picker.slide().shown() <= 4,
            "never past the selection"
        );
    }
    assert_eq!(picker.index(), 4);
    assert_eq!(
        seen,
        vec![2, 3, 4],
        "each team is the centre column in turn"
    );
}

/// The ring follows the shown team's parity, not the selection's, when the selection runs ahead.
#[test]
fn the_ring_eases_toward_the_shown_teams_row_while_a_second_press_waits() {
    let mut picker = picker_of(&["A", "B", "C", "D", "E", "F"], "C");
    frame(&mut picker, None);
    let rest = ring_y(&picker);
    press(&mut picker, Button::Right);
    frame(&mut picker, None);
    frame(&mut picker, Some(Button::Right));
    assert_eq!(picker.index(), 4);
    // Shown team 3 is odd (42 low); the ring has eased one tick from 42 above it.
    assert!((ring_y(&picker) - (rest + 42.0 - 21.0)).abs() < 1e-3);
}

/// On a roster of odd length the wrap still flips the half-row phase: nothing jumps at the snap.
#[test]
fn a_wrap_on_an_odd_roster_keeps_the_columns_continuous() {
    let grid = grid();
    let mut picker = picker_of(&["A", "B", "C"], "C");
    frame(&mut picker, None);
    press(&mut picker, Button::Right);
    let y_of = |picker: &Picker, entry: usize| {
        grid.cells_at(picker.index(), 3, 0.0, Some(picker.slide()))
            .iter()
            .find(|c| c.entry == entry && c.row == 0)
            .unwrap()
            .rect[1]
    };
    let mut last = 0.0;
    for _ in 0..6 {
        frame(&mut picker, None);
        assert!(picker.slide().moving() || picker.slide().shown() == 0);
        last = y_of(&picker, 1);
    }
    frame(&mut picker, None);
    assert!(!picker.slide().moving());
    assert_eq!(
        y_of(&picker, 1),
        last,
        "team B keeps its height through the snap"
    );
}

/// A drag already showed the columns moving, so folding it into a step slides nothing further and
/// draws only the columns the pan draws, all of them at the pan's own fade.
#[test]
fn a_drag_that_folds_into_a_step_does_not_slide_on_top_of_the_pan() {
    let grid = grid();
    let mut picker = picker_of(&["A", "B", "C", "D", "E", "F"], "C");
    frame(&mut picker, None);
    let found = pan_targets(&picker);
    picker.pointer(&drag_by(PITCH_X * 0.65), &found);
    assert_eq!(picker.index(), 1, "the drag folded a column");
    frame(&mut picker, None);
    assert!(!picker.slide().moving());
    assert_eq!(picker.slide().shown(), picker.index());
    let cells = grid.cells_at(picker.index(), 6, picker.pan(), Some(picker.slide()));
    assert!(
        cells.len() <= 6 * 7,
        "no extra slide column on top of the pan"
    );
}
