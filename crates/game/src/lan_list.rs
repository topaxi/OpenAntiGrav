//! The JOIN GAME list's rows: what each discovered host says, and whether this machine can join
//! it, worked out before connecting (`oag_net::discovery::compat`). Shared by the live screen
//! (`session::lan`) and its still (`--menu-page lan-join`), so the two cannot differ.
//!
//! A row reads the host's name and its state on the left (`ADA  JOINABLE`) and, on the right,
//! the title it runs and how many are in (`WIPEOUT PULSE (1/8)`). A host this machine cannot join stays in the list, dimmed and
//! unselectable, and its reason is the line under the title while the cursor is on it; a host it
//! can join says its circuit there. **Chosen, not measured** (maintainer's requirement,
//! 2026-10-14).

use oag_net::discovery::{Advert, Compat, Local, compat};
use oag_ui::language::StringTable;
use oag_ui_screens::join::Join;

/// The reason line for a state that cannot be joined, `None` for [`Compat::Joinable`].
#[must_use]
pub fn reason(state: Compat, strings: &StringTable) -> Option<String> {
    let say = |id: &str, english: &str| strings.get(id).unwrap_or(english).to_string();
    match state {
        Compat::Joinable => None,
        Compat::OtherTitle => Some(say(
            "OAG_LAN_ROW_TITLE",
            "A DIFFERENT TITLE OR PRESSING, OR YOU LACK THIS TITLE'S IMAGE",
        )),
        Compat::OtherVersion => Some(say("OAG_LAN_ROW_VERSION", "A DIFFERENT VERSION")),
        Compat::OtherContent => Some(say("OAG_LAN_ROW_CONTENT", "DIFFERENT MODS OR CONTENT")),
        Compat::Full => Some(say("OAG_LAN_ROW_FULL", "FULL")),
    }
}

/// The one or two words a row says for its state, `JOINABLE` for a host that can be joined.
#[must_use]
pub fn state_word(state: Compat, strings: &StringTable) -> String {
    let say = |id: &str, english: &str| strings.get(id).unwrap_or(english).to_string();
    match state {
        Compat::Joinable => say("OAG_LAN_STATE_JOINABLE", "JOINABLE"),
        Compat::OtherTitle => say("OAG_LAN_STATE_TITLE", "OTHER TITLE"),
        Compat::OtherVersion => say("OAG_LAN_STATE_VERSION", "OTHER VERSION"),
        Compat::OtherContent => say("OAG_LAN_STATE_CONTENT", "OTHER MODS"),
        Compat::Full => say("OAG_LAN_STATE_FULL", "FULL"),
    }
}

/// Fills `screen` (a list) with `hosts`: their rows, the dimmed ones noted. Returns each host's
/// state, in row order.
pub fn fill(
    screen: &mut Join,
    hosts: &[Advert],
    local: &Local<'_>,
    strings: &StringTable,
) -> Vec<Compat> {
    let states: Vec<Compat> = hosts.iter().map(|host| compat(local, host)).collect();
    screen.set_seats(
        hosts
            .iter()
            .map(|host| {
                format!(
                    "{} ({}/{})",
                    host.title.to_uppercase(),
                    host.people,
                    host.max
                )
            })
            .collect(),
    );
    // The name and the state on the left, the title and the seats on the right.
    screen.set_row_labels(
        hosts
            .iter()
            .zip(&states)
            .map(|(host, state)| {
                let name: String = host.name.to_uppercase().chars().take(6).collect();
                format!("{name}  {}", state_word(*state, strings))
            })
            .collect(),
    );
    screen.set_notes(
        hosts
            .iter()
            .zip(&states)
            .map(|(host, state)| {
                // A joinable host says its circuit; the others, why they cannot be joined.
                reason(*state, strings)
                    .or_else(|| Some(host.track.to_uppercase()).filter(|t| !t.is_empty()))
            })
            .collect(),
        states
            .iter()
            .map(|state| *state != Compat::Joinable)
            .collect(),
    );
    states
}
