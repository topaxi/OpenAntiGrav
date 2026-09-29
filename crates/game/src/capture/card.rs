//! `--until card[:N[:EVENT NAME]]`: stop with Wipeout 2048's campaign-map event
//! card open, rather than on a state-machine state (the card is a sub-state
//! of `newFEshell`, so `--until` has no name for it).
//!
//! The pulsed `--press` buttons open the card, and once it is open the pulse
//! turns to the right button so `N` pages are stepped through. `EVENT NAME`
//! puts the map's cursor on that event first; a locked event's card never
//! opens, so name an unlocked one.

use oag_gameplay::input::Button;
use oag_ui::frontend::Frontend;

/// What `--until card...` asked for.
pub(super) struct CardTarget {
    page: usize,
    /// The event to select, taken (`None`) once the cursor is on it.
    event: Option<String>,
}

impl CardTarget {
    /// `None` when `until` is not a `card` request at all.
    pub(super) fn parse(until: Option<&str>) -> Option<Self> {
        let rest = until?.strip_prefix("card")?;
        if rest.is_empty() {
            return Some(Self {
                page: 0,
                event: None,
            });
        }
        let rest = rest.strip_prefix(':')?;
        let (page, event) = match rest.split_once(':') {
            Some((page, event)) => (page, Some(event.to_string())),
            None => (rest, None),
        };
        Some(Self {
            page: page.parse().ok()?,
            event,
        })
    }

    /// The card is open on the page asked for.
    pub(super) fn reached(&self, frontend: &Frontend) -> bool {
        frontend.event_card_page() == Some(self.page)
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
        } else {
            pressed
        })
    }
}
