//! The seat assignment screen as a still: `--menu-page seat-assign[:SEATS[:ORDER[:UNCONFIRMED]]]`
//! (`SEATS` is the seats per window, `2,1` by default; `ORDER` is who sits in each pane, the
//! join order by default; `UNCONFIRMED` is a person, from 1, who has not confirmed).
//!
//! Nobody presses anything in a still, so the assignment is made up from the page's own
//! numbers. That is the only invented part; the rows, the words and the drawing are the live
//! screen's own (`oag_ui_screens::seat_assign`, labels off the same string table, the preview
//! cut by `oag_game::split_screen::seat_cells`).

use super::menu_page::{self, TickerClip};
use oag_gameplay::input::Button;
use oag_ui_screens::seat_assign::{Labels, Pane, SeatAssign};

/// What the page name asks for, or `None` for any other page.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Request {
    seats: Vec<usize>,
    order: Vec<usize>,
    unconfirmed: Option<usize>,
}

/// Parses `seat-assign[:SEATS[:ORDER[:UNCONFIRMED]]]`; a list that does not parse is the
/// default.
#[must_use]
pub(super) fn request(page: &str) -> Option<Request> {
    let rest = page.strip_prefix("seat-assign")?;
    if !(rest.is_empty() || rest.starts_with(':')) {
        return None;
    }
    let mut parts = rest
        .strip_prefix(':')
        .into_iter()
        .flat_map(|rest| rest.split(':'));
    let list = |text: Option<&str>| -> Vec<usize> {
        text.map(|text| text.split(',').map_while(|n| n.parse().ok()).collect())
            .unwrap_or_default()
    };
    let seats = Some(list(parts.next()))
        .filter(|seats| !seats.is_empty())
        .unwrap_or(vec![2, 1]);
    let order = list(parts.next());
    let unconfirmed = parts.next().and_then(|n| n.parse::<usize>().ok());
    Some(Request {
        seats,
        order,
        unconfirmed,
    })
}

/// Draws the screen on the frame the live one uses, with the page's footer.
pub(super) fn capture(
    request: &Request,
    strings: &oag_ui::language::StringTable,
    backdrop: Option<oag_ui::menu::Picture>,
    skin: &oag_ui::menu::Skin,
    frame: &oag_ui::menu::Frame,
    footer: (
        Option<&oag_ui_screens::campaign::footer::NavigationLegend>,
        Option<&oag_ui_screens::campaign::footer::TickerLayout>,
        &oag_ui::font::Atlas,
    ),
) -> (Vec<oag_ui::frontend::Draw>, TickerClip) {
    let windows = request
        .seats
        .iter()
        .map(|&seats| Pane {
            seats: seats.clamp(1, 8),
            vertical: false,
        })
        .collect();
    let mut screen = SeatAssign::new(Labels::read(strings), windows, Some(&request.order));
    if let Some(person) = request.unconfirmed.and_then(|n| n.checked_sub(1)) {
        // Moves the person away and back: the only way in is the live screen's own press.
        screen.press(person, Button::Right.bit());
        screen.press(person, Button::Left.bit());
    }
    let mut list = oag_ui_screens::seat_assign::draw_list(
        &screen,
        skin,
        frame,
        backdrop,
        false,
        &crate::split_screen::seat_cells,
    )
    .flatten();
    let (legend, ticker, font) = footer;
    let clip = menu_page::picker_footer(&mut list, legend, ticker, strings, font);
    (list, clip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_name_carries_its_numbers() {
        assert_eq!(
            request("seat-assign"),
            Some(Request {
                seats: vec![2, 1],
                order: Vec::new(),
                unconfirmed: None
            })
        );
        assert_eq!(
            request("seat-assign:1,1,1:2,0,1:3"),
            Some(Request {
                seats: vec![1, 1, 1],
                order: vec![2, 0, 1],
                unconfirmed: Some(3)
            })
        );
        assert_eq!(request("seat-assignx"), None);
        assert_eq!(request("window-layout"), None);
    }
}
