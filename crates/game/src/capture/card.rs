//! `--until card[:N[:EVENT NAME]]` and `--until map:EVENT NAME`: stop with
//! Wipeout 2048's campaign map's event card open, or with the map showing the
//! named event, rather than on a state-machine state (the card is a sub-state
//! of `newFEshell`, so `--until` has no name for it).
//!
//! The pulsed `--press` buttons open the card, and once it is open the pulse
//! turns to the right button so `N` pages are stepped through. `EVENT NAME`
//! puts the map's cursor on that event first - a downloadable campaign's cell
//! (`grid0_3_1`) brings its own map up - and a locked event's card never opens,
//! so name an unlocked one. `map:` stops there, with no card.

use oag_gameplay::input::Button;
use oag_ui::frontend::Frontend;

/// What `--until card...` or `--until map:...` asked for.
pub(super) struct CardTarget {
    /// The card's page, `None` for a map with no card.
    page: Option<usize>,
    /// The event to select, taken (`None`) once the cursor is on it.
    event: Option<String>,
}

impl CardTarget {
    /// `None` when `until` is not a `card` or `map` request at all.
    pub(super) fn parse(until: Option<&str>) -> Option<Self> {
        let until = until?;
        if let Some(event) = until.strip_prefix("map:") {
            return Some(Self {
                page: None,
                event: Some(event.to_string()),
            });
        }
        let rest = until.strip_prefix("card")?;
        if rest.is_empty() {
            return Some(Self {
                page: Some(0),
                event: None,
            });
        }
        let rest = rest.strip_prefix(':')?;
        let (page, event) = match rest.split_once(':') {
            Some((page, event)) => (page, Some(event.to_string())),
            None => (rest, None),
        };
        Some(Self {
            page: Some(page.parse().ok()?),
            event,
        })
    }

    /// The card is open on the page asked for.
    pub(super) fn reached(&self, frontend: &Frontend) -> bool {
        match self.page {
            Some(page) => frontend.event_card_page() == Some(page),
            None => {
                self.event.is_none()
                    && frontend
                        .machine()
                        .is(oag_2048::frontend::states::NEW_FE_SHELL)
            }
        }
    }

    /// Puts the cursor on the named event once the map is up, and says which
    /// buttons to pulse this tick: `pressed` until the card is open, then
    /// right.
    pub(super) fn step(&mut self, frontend: &mut Frontend, pressed: u32) -> anyhow::Result<u32> {
        if let Some(name) = &self.event
            && frontend
                .machine()
                .is(oag_2048::frontend::states::NEW_FE_SHELL)
            && !frontend.event_card_open()
        {
            anyhow::ensure!(
                frontend.select_campaign_event(name),
                "--until card: no campaign event named {name:?}"
            );
            self.event = None;
        }
        Ok(if frontend.event_card_open() {
            Button::Right.bit()
        } else if self.page.is_none() && self.event.is_none() {
            0
        } else {
            pressed
        })
    }
}
