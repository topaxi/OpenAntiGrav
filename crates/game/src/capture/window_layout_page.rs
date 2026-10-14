//! The window layout screen as a still: `--menu-page window-layout[:PEOPLE[:WINDOWS[:MONITORS[:EDITED]]]]`
//! (4 people, 1 window, 1 monitor, the first window by default).
//!
//! Nobody presses anything in a still, so the choice is made up from the page's own numbers
//! and the monitors are called `MONITOR n`. That is the only invented part; the rows, the
//! words and the drawing are the live screen's own (`oag_ui_screens::window_layout`, labels
//! off the same string table, the preview cut by `oag_game::split_screen::seat_cells`).

use super::menu_page::{self, TickerClip};
use oag_ui_screens::window_layout::{Labels, Picker};

/// What the page name asks for, or `None` for any other page.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Request {
    people: usize,
    windows: usize,
    monitors: usize,
    edited: usize,
}

/// Parses `window-layout[:PEOPLE[:WINDOWS[:MONITORS[:EDITED]]]]`.
#[must_use]
pub(super) fn request(page: &str) -> Option<Request> {
    let rest = page.strip_prefix("window-layout")?;
    let mut numbers = rest
        .strip_prefix(':')
        .into_iter()
        .flat_map(|rest| rest.split(':'))
        .map(str::parse::<usize>);
    let mut next = |default: usize| numbers.next().and_then(Result::ok).unwrap_or(default);
    let request = Request {
        people: next(4),
        windows: next(1),
        monitors: next(1),
        edited: next(0),
    };
    (rest.is_empty() || rest.starts_with(':')).then_some(request)
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
    let monitors = (1..=request.monitors.max(1))
        .map(|n| format!("Monitor {n}"))
        .collect();
    let mut picker = Picker::new(
        Labels::read(strings),
        request.people,
        Vec::new(),
        monitors,
        false,
    );
    picker.set_window_count(request.windows);
    for _ in 0..request.edited {
        picker.pick_next_window();
    }
    let mut list = oag_ui_screens::window_layout::draw_list(
        &picker,
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
            request("window-layout"),
            Some(Request {
                people: 4,
                windows: 1,
                monitors: 1,
                edited: 0
            })
        );
        assert_eq!(
            request("window-layout:6:3:2:1"),
            Some(Request {
                people: 6,
                windows: 3,
                monitors: 2,
                edited: 1
            })
        );
        assert_eq!(request("window-layoutx"), None);
        assert_eq!(request("join:4"), None);
    }
}
