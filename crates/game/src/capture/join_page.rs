//! The split-screen join screen as a still: `--menu-page join[:N]`, with `N`
//! seats taken (none by default). Split out of `capture.rs` under the size rule.
//!
//! Nobody presses anything in a still, so the seats are made up: the first is the
//! keyboard and the rest are pads named as the screen names a pad with no name.
//! That is the only invented part; the rows, the words and the drawing are the
//! live screen's own (`oag_ui_screens::join`, labels off the same string table).

use super::menu_page::{self, TickerClip};
use oag_ui_screens::join::{Join, Labels};

/// The seat count `join[:N]` asks for, or `None` for any other page.
#[must_use]
pub(super) fn seats(page: &str) -> Option<usize> {
    let rest = page.strip_prefix("join")?;
    if rest.is_empty() {
        return Some(0);
    }
    rest.strip_prefix(':')?
        .parse::<usize>()
        .ok()
        .map(|n| n.min(oag_gameplay::MAX_PLAYERS))
}

/// Draws the screen with `seated` seats taken, on the frame the live one uses,
/// with the page's footer.
pub(super) fn capture(
    seated: usize,
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
    let labels = Labels::read(strings);
    let mut screen = Join::new(labels.clone(), oag_gameplay::MAX_PLAYERS);
    screen.set_seats(
        (0..seated)
            .map(|seat| {
                if seat == 0 {
                    labels.keyboard.clone()
                } else {
                    format!("{} {}", labels.pad, seat)
                }
            })
            .collect(),
    );
    let mut list = oag_ui_screens::join::draw_list(&screen, skin, frame, backdrop, false).flatten();
    let (legend, ticker, font) = footer;
    let clip = menu_page::picker_footer(&mut list, legend, ticker, strings, font);
    (list, clip)
}
