//! Where one tick's pointer goes: into the grid the stage on screen is
//! drawn in, and then to whichever model on that stage has something to
//! point at.
//!
//! **Ours** - see `oag_ui::pointer` for the vocabulary and for why a pointer
//! stops at the front end. Every function here is a free function or an
//! associated one taking the stage apart from `self`, for the reason
//! `Session::tick_prompt` gives: the caller in `session::frame` is already
//! inside a `match &mut self.stage`, so the stage arrives borrowed and
//! `self` cannot be.
//!
//! # The order is the pad's order
//!
//! `session::frame` routes a click through the same precedence it routes a
//! button: a selection screen takes the tick whole, then a binding capture
//! freezes the page, then a modal prompt eats the input, and only then do
//! the rows see it. A click reaching the rows behind a scrim is the bug that
//! ordering already prevents for cross, and a pointer that skipped it would
//! reintroduce it for a mouse.
//!
//! # A click on a screen with nothing to point at is a press
//!
//! The boot movies, `PRESS START`, the storage warning and the results table
//! all wait for start or cross and draw no target. There a click is the
//! press, synthesised through `Controls::tap` onto the keyboard's own latch,
//! so the screen reads it exactly as it reads a key - one tick later, which
//! is the latch's own delay and imperceptible. Never in a running race: a
//! click is not thrust, and `frame.rs` does not offer a race the pointer at
//! all until it is paused or over.

use oag_display::space::Space;
use oag_game::render::to_grid;
use oag_gameplay::input::Button;
use oag_input::Controls;
use oag_ui::pointer::Pointer;
use oag_ui::{font, menu};
use oag_ui_screens::picker;

use crate::menu_stage::MenuStage;

/// `pointer`, with its position moved from window pixels into `space`'s
/// grid through the aspect rectangle `rect` every stage draws into.
pub(super) fn in_grid(pointer: Pointer, space: Space, rect: (f32, f32, f32, f32)) -> Pointer {
    // A drag is a distance, so it is the difference of two mapped points
    // rather than a point: the letterbox offset cancels.
    let origin = to_grid(space, rect, (0.0, 0.0));
    let tip = to_grid(space, rect, pointer.drag);
    Pointer {
        at: pointer.at.map(|at| to_grid(space, rect, at)),
        press_at: pointer.press_at.map(|at| to_grid(space, rect, at)),
        drag: (tip.0 - origin.0, tip.1 - origin.1),
        ..pointer
    }
}

/// The one pointer the end screens of a multi-window race read this tick, in `space`'s grid:
/// window 1's (`first`, already in the grid) unless it did nothing, else the first further
/// window's that did something. The screens are one menu with one cursor, so one window's
/// pointer drives it per tick; a still mouse in one window cannot hold the others back.
/// **Chosen, not measured.**
pub(super) fn end_screen_pointer(
    first: Pointer,
    space: Space,
    seats: &[(Pointer, (f32, f32, f32, f32))],
) -> Pointer {
    if !first.is_idle() {
        return first;
    }
    seats
        .iter()
        .map(|&(pointer, rect)| in_grid(pointer, space, rect))
        .find(|pointer| !pointer.is_idle())
        .unwrap_or(first)
}

/// A click on a screen that only waits for a button: press start and cross
/// for it, which between them is what every such screen reads.
pub(super) fn press_for_click(controls: &mut Controls, pointer: &Pointer) {
    if pointer.clicked {
        controls.tap(Button::Start);
        controls.tap(Button::Cross);
    }
}

/// The rows of a menu page answering the pointer, plus the one thing the
/// rows cannot do for themselves: a click on a binding row opens the key
/// capture, the way `Session::maybe_begin_binding` does for cross - and
/// releases every key for the reason that function's doc gives.
///
/// Returns the menu's own events, for `Session::handle_menu`.
pub(super) fn menu_pointer(
    stage: &mut MenuStage,
    pointer: &Pointer,
    awaiting_binding: &mut Option<Button>,
    controls: &mut Controls,
) -> Vec<menu::MenuEvent> {
    if pointer.is_idle() {
        return Vec::new();
    }
    let regions = menu::pointer::regions(&stage.menu, &stage.skin, &stage.frame, &|text| {
        font::measure(&stage.text_atlas, text)
    });
    let hit = pointer
        .at
        .and_then(|at| menu::pointer::hit(&regions, at))
        .is_some();
    let events = stage.menu.pointer(pointer, &regions);
    if pointer.clicked
        && hit
        && awaiting_binding.is_none()
        && let Some(menu::Entry::Binding { button, .. }) =
            stage.menu.page().entries.get(stage.menu.selected())
    {
        *awaiting_binding = Some(*button);
        controls.release_all();
    }
    events
}

/// A selection screen answering the pointer, against the targets it just
/// drew. See `oag_ui_screens::picker::pointer`.
pub(super) fn picker_pointer(stage: &mut MenuStage, pointer: &Pointer) -> Vec<picker::Event> {
    let shared = stage.partner.is_some();
    let Some(stage_picker) = stage.picker.as_mut() else {
        return Vec::new();
    };
    drive_picker(stage_picker, &stage.skin, pointer, shared)
}

/// The second seat's half of a shared Ship Select answering the pointer, the
/// same way [`picker_pointer`] does the first's: each half's targets are its own
/// widgets, so a click or a hover lands on one.
pub(super) fn partner_pointer(stage: &mut MenuStage, pointer: &Pointer) -> Vec<picker::Event> {
    let Some(partner) = stage.partner.as_deref_mut() else {
        return Vec::new();
    };
    drive_picker(partner, &stage.skin, pointer, true)
}

/// One selection screen against `pointer`. On a page two people share
/// (`shared`) the wheel steps only the half whose arrows, panel or craft the
/// pointer is over, because a wheel has no position of its own to tell the two
/// lists apart.
fn drive_picker(
    picker: &mut crate::picker_stage::PickerStage,
    skin: &oag_ui::menu::Skin,
    pointer: &Pointer,
    shared: bool,
) -> Vec<picker::Event> {
    if pointer.is_idle() {
        return Vec::new();
    }
    let targets = picker::pointer::targets(&picker.model, &picker.layout, skin, &|src| {
        picker.placed(src)
    });
    let mut pointer = *pointer;
    if shared
        && pointer.scroll != 0
        && !pointer
            .at
            .is_some_and(|at| targets.iter().any(|target| oag_ui::pointer::contains(target.rect, at)))
    {
        pointer.scroll = 0;
    }
    picker.model.pointer(&pointer, &targets)
}

/// `Grid Selection`/`Cell Selection` answering the pointer, against the
/// targets they just drew. See `oag_ui_screens::campaign::pointer`.
///
/// The targets are built off a shared borrow of `campaign` first - its
/// layout and sprite sheet never change while a screen is open, only the
/// model's own selection does - so the mutable borrow `Model::pointer`
/// needs can start only after that borrow ends.
pub(super) fn campaign_pointer(
    stage: &mut MenuStage,
    pointer: &Pointer,
) -> Vec<oag_ui_screens::campaign::Event> {
    let Some(campaign) = stage.campaign.as_mut() else {
        return Vec::new();
    };
    if pointer.is_idle() {
        return Vec::new();
    }
    // Wipeout HD/Fury draws a completely different screen behind the same
    // two names - see `oag_ui_screens::campaign::hd`'s own module doc - so its
    // pointer targets are built and consumed by that module's own
    // functions, not Pulse's `oag_ui_screens::campaign::pointer`.
    if campaign.is_hd() {
        return match &campaign.screen {
            // `CampaignSelection::pointer` computes its own two click
            // targets internally (from the screen's own measured `Bracket`
            // rect - see `oag_ui_screens::campaign::selection`'s own module doc), so
            // no external target list is needed - only the same "re-borrow
            // mutably once matched" step `Grid`/`Cell` also take below.
            crate::campaign_stage::Screen::Selection(_) => {
                let crate::campaign_stage::Screen::Selection(model) = &mut campaign.screen else {
                    unreachable!("just matched Screen::Selection above")
                };
                model.pointer(pointer)
            }
            crate::campaign_stage::Screen::Grid(_) => {
                let card = campaign.flyer_card_rect();
                let targets = oag_ui_screens::campaign::hd::hd_grid_targets(
                    campaign.grid_layout(),
                    &|src| campaign.sprites.get(src),
                    card,
                );
                let crate::campaign_stage::Screen::Grid(model) = &mut campaign.screen else {
                    unreachable!("just matched Screen::Grid above")
                };
                model.hd_pointer(pointer, &targets)
            }
            crate::campaign_stage::Screen::Cell { model, .. } => {
                let targets = oag_game::campaign::hit::cell_targets(
                    model,
                    campaign.cell_layout(),
                    &campaign.sprites,
                );
                let difficulty_rect = oag_ui_screens::campaign::hd::difficulty_button_rect(
                    &campaign.cell_layout().screen,
                );
                let crate::campaign_stage::Screen::Cell { model, .. } = &mut campaign.screen else {
                    unreachable!("just matched Screen::Cell above")
                };
                model.hd_pointer(pointer, &targets, difficulty_rect)
            }
        };
    }
    let targets = match &campaign.screen {
        // Never built for a non-HD title - see
        // `crate::campaign_stage::Screen::Selection`'s own doc.
        crate::campaign_stage::Screen::Selection(_) => Vec::new(),
        crate::campaign_stage::Screen::Grid(model) => {
            oag_game::campaign::hit::grid_targets(model, campaign.grid_layout(), &campaign.sprites)
        }
        crate::campaign_stage::Screen::Cell { model, .. } => {
            oag_game::campaign::hit::cell_targets(model, campaign.cell_layout(), &campaign.sprites)
        }
    };
    match &mut campaign.screen {
        crate::campaign_stage::Screen::Selection(_) => Vec::new(),
        crate::campaign_stage::Screen::Grid(model) => model.pointer(pointer, &targets),
        crate::campaign_stage::Screen::Cell { model, .. } => model.pointer(pointer, &targets),
    }
}

#[cfg(test)]
mod end_screen_tests {
    use super::*;

    const RECT: (f32, f32, f32, f32) = (0.0, 0.0, 480.0, 272.0);

    fn click_at(x: f32, y: f32) -> Pointer {
        Pointer {
            at: Some((x, y)),
            moved: true,
            clicked: true,
            ..Pointer::default()
        }
    }

    /// A click in a further window drives the end screens, in that window's own layout (a window
    /// twice the grid's size halves the pixel), unless window 1 did something this tick. Fails
    /// if the further windows' pointers are dropped, or taken in window 1's pixels.
    #[test]
    fn a_click_in_a_further_window_is_read_in_that_windows_own_layout() {
        let big = (0.0, 0.0, 960.0, 544.0);
        let seats = [(Pointer::default(), RECT), (click_at(480.0, 272.0), big)];
        let read = end_screen_pointer(Pointer::default(), Space::PSP, &seats);
        assert!(read.clicked);
        assert_eq!(read.at, Some((240.0, 136.0)));

        let first = click_at(10.0, 20.0);
        let read = end_screen_pointer(first, Space::PSP, &seats);
        assert_eq!(read.at, Some((10.0, 20.0)), "window 1 did something: it wins");

        let read = end_screen_pointer(Pointer::default(), Space::PSP, &[]);
        assert!(read.is_idle());
    }
}
