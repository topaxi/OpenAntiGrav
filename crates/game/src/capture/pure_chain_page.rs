//! Wipeout Pure's SINGLE PLAYER chain as a still: `--menu-page
//! pure-chain[:MODE[:CLASS[:LEAGUE]]]`, the screen reached by answering each screen
//! in turn. Split out of `menu_page.rs` under the 1,000-line rule.

use anyhow::{Context, Result};

use super::menu_page::{self, TickerClip};
use super::open_for_previews;

/// What follows `pure-chain` in a `--menu-page` name: the answers that walk the
/// chain to the screen to draw, `:`-separated, `""` for the first screen. `None`
/// for any other page.
#[must_use]
pub(super) fn answers(page: &str) -> Option<&str> {
    let rest = page.strip_prefix("pure-chain")?;
    (rest.is_empty() || rest.starts_with(':')).then_some(rest)
}

/// Draws the screen `answers` reaches: MODE by name (`arcade`, `tournament`,
/// `time-trial`, `zone`, `free-play`), the rest by row number. Read against
/// whatever `records.toml` holds, through the same `pure_chain::present` the live
/// screen builds its rows with, and drawn on the same frame
/// (`oag_ui_screens::pure_chain::draw_list`), with the page's footer.
///
/// # Errors
///
/// No source is open, an answer names no mode or is not a row number, or one does
/// not lead on from the screen it answers.
#[expect(
    clippy::too_many_arguments,
    reason = "the facts `capture` already has in hand"
)]
pub(super) fn capture(
    answers: &str,
    race: Option<&oag_raceplay::Options>,
    title: &oag_title::Title,
    strings: &oag_ui::language::StringTable,
    backdrop: Option<oag_ui::menu::Picture>,
    skin: &oag_ui::menu::Skin,
    frame: &oag_ui::menu::Frame,
    footer: (
        Option<&oag_ui_screens::campaign::footer::NavigationLegend>,
        Option<&oag_ui_screens::campaign::footer::TickerLayout>,
        &oag_ui::font::Atlas,
    ),
) -> Result<(Vec<oag_ui::frontend::Draw>, TickerClip)> {
    use crate::campaign::pure_chain::{Chain, Chosen, choose, present};
    let race = race.context("--menu-page pure-chain needs a source open")?;
    let mut archives = open_for_previews(race)?;
    let campaign = crate::campaign::pure_list::load(&mut archives, title)?;
    let flow = crate::campaign::pure_list::load_flow(&mut archives, title)?;
    let records = crate::records::load();
    let mut chain = Chain::new(campaign, flow, oag_tables::pure_ai_stats::PureClass::Vector);
    let plain = |text: &str| {
        text.chars()
            .filter(char::is_ascii_alphanumeric)
            .collect::<String>()
            .to_ascii_lowercase()
    };
    for (index, answer) in answers.split(':').skip(1).enumerate() {
        let tag = if index == 0 {
            chain
                .modes()
                .iter()
                .position(|mode| plain(mode) == plain(answer))
                .with_context(|| format!("no mode named {answer:?}"))?
        } else {
            answer
                .parse::<usize>()
                .with_context(|| format!("{answer:?} is not a row number"))?
        };
        anyhow::ensure!(
            choose(&mut chain, tag, title, &records) == Chosen::Advanced,
            "{answer:?} does not lead on from {:?}",
            chain.at().screen()
        );
    }
    let view = present(&chain, strings, title, &records);
    let choice =
        oag_ui_screens::pure_chain::Choice::new(view.title, view.context, view.pitch, view.rows);
    let mut list =
        oag_ui_screens::pure_chain::draw_list(&choice, skin, frame, backdrop, false).flatten();
    let (legend, ticker, font) = footer;
    let clip = menu_page::picker_footer(&mut list, legend, ticker, strings, font);
    Ok((list, clip))
}
