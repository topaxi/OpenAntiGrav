//! The navigation sounds each prompt logs, on the pad and on the pointer.
//! `TagEntry` is Pulse's own widget (its cues are `TagInput_Update`'s); the
//! grid and the yes/no dialog are ours, so their cues are chosen.

use oag_gameplay::input::Button;
use oag_ui::pointer::Pointer;

use super::*;

fn tick(buttons: &[Button]) -> Input {
    let mut input = Input::new();
    let mask = buttons.iter().fold(0u32, |mask, &b| mask | 1 << b.index());
    input.begin_frame(mask);
    input
}

fn skin() -> Skin {
    Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    )
}

fn confirm() -> Confirm {
    Confirm::new(ConfirmLabels::default())
}

fn keyboard(initial: &str, limit: usize) -> Keyboard {
    Keyboard::new(Labels::default(), initial, limit)
}

fn click(at: (f32, f32)) -> Pointer {
    Pointer {
        at: Some(at),
        moved: true,
        clicked: true,
        ..Pointer::default()
    }
}

fn centre(rect: [f32; 4]) -> (f32, f32) {
    (rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5)
}

#[test]
fn a_confirms_yes_accepts_and_its_no_and_back_decline() {
    let mut dialog = confirm();
    dialog.update(&mut tick(&[Button::Left]));
    assert_eq!(dialog.take_nav(), [Nav::Stepped(Some(Dir::Left))]);
    assert_eq!(
        dialog.update(&mut tick(&[Button::Cross])),
        Outcome::Accepted
    );
    assert_eq!(dialog.take_nav(), [Nav::Accept]);

    let mut dialog = confirm();
    assert_eq!(
        dialog.update(&mut tick(&[Button::Cross])),
        Outcome::Cancelled
    );
    assert_eq!(dialog.take_nav(), [Nav::Decline]);
    assert_eq!(
        dialog.update(&mut tick(&[Button::Circle])),
        Outcome::Cancelled
    );
    assert_eq!(dialog.take_nav(), [Nav::Decline]);
    assert!(dialog.take_nav().is_empty(), "taking drains the log");
}

#[test]
fn a_confirm_answers_the_pointer_with_the_pads_cues() {
    let skin = skin();
    let [no, yes] = Confirm::answer_rects(&skin);
    let mut dialog = confirm();
    let hover = Pointer {
        at: Some(centre(yes)),
        moved: true,
        ..Pointer::default()
    };
    dialog.pointer(&hover, &skin);
    dialog.pointer(&hover, &skin);
    assert_eq!(
        dialog.take_nav(),
        [Nav::Stepped(None)],
        "once, not per event"
    );
    assert_eq!(
        dialog.pointer(&click(centre(yes)), &skin),
        Outcome::Accepted
    );
    assert_eq!(dialog.take_nav(), [Nav::Accept]);

    let mut dialog = confirm();
    assert_eq!(
        dialog.pointer(&click(centre(no)), &skin),
        Outcome::Cancelled
    );
    assert_eq!(dialog.take_nav(), [Nav::Decline]);
    let back = Pointer {
        back: true,
        ..Pointer::default()
    };
    assert_eq!(confirm().pointer(&back, &skin), Outcome::Cancelled);
}

#[test]
fn the_grid_plays_a_step_a_typed_key_a_refusal_and_the_answer() {
    let mut grid = keyboard("", 1);
    grid.update(&mut tick(&[Button::Right]));
    assert_eq!(grid.take_nav(), [Nav::Moved(Some(Dir::Right))]);
    grid.edit(Edit::Type('a'));
    grid.edit(Edit::Type('b'));
    assert_eq!(
        grid.take_nav(),
        [Nav::Stepped(Some(Dir::Right)), Nav::Decline],
        "the second character does not fit"
    );
    grid.edit(Edit::Delete);
    grid.edit(Edit::Delete);
    assert_eq!(
        grid.take_nav(),
        [Nav::Stepped(Some(Dir::Left)), Nav::Decline]
    );
    assert_eq!(grid.update(&mut tick(&[Button::Start])), Outcome::Accepted);
    assert_eq!(grid.take_nav(), [Nav::Accept]);
    assert_eq!(
        grid.update(&mut tick(&[Button::Circle])),
        Outcome::Cancelled
    );
    assert_eq!(grid.take_nav(), [Nav::Decline]);
}

#[test]
fn the_grid_plays_the_same_cues_for_the_pointer() {
    let skin = skin();
    let on_cell = |cell: usize| {
        let grid = Keyboard::grid(&skin);
        (
            grid.left + (cell % COLUMNS) as f32 * grid.cell_w + grid.cell_w * 0.5,
            grid.top + (cell / COLUMNS) as f32 * grid.cell_h + grid.cell_h * 0.5,
        )
    };
    let mut grid = keyboard("", 4);
    let hover = Pointer {
        at: Some(on_cell(3)),
        moved: true,
        ..Pointer::default()
    };
    grid.pointer(&hover, &skin);
    grid.pointer(&hover, &skin);
    assert_eq!(grid.take_nav(), [Nav::Moved(None)], "once, not per event");
    grid.pointer(&click(on_cell(3)), &skin);
    assert_eq!(grid.take_nav(), [Nav::Stepped(Some(Dir::Right))]);
    let back = Pointer {
        back: true,
        ..Pointer::default()
    };
    assert_eq!(grid.pointer(&back, &skin), Outcome::Cancelled);
    assert_eq!(grid.take_nav(), [Nav::Decline]);
}
