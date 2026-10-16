//! The online lobby's room list and room, as rows on the join screen
//! ([`oag_ui_screens::join::Join`]). Shared by the live screens (`session::online`) and their
//! stills (`--menu-page online-list`, `online-room`), so the two cannot differ.
//!
//! **Ours, chosen, not measured** (Omega's own `Game List` and `GameLobby` are the model for
//! what is on them, `docs/networking/online-lobby.md`, part 1).

use oag_net::wire::online::{Member, Phase, RoomInfo, RoomSettings, Snapshot};
use oag_ui::language::StringTable;
use oag_ui_screens::join::Join;

/// The lobby's port when an address names none: `oag-server lobby`'s own.
pub const DEFAULT_PORT: u16 = 60000;

/// `host:port` with [`DEFAULT_PORT`] added when none is given.
#[must_use]
pub fn with_port(typed: &str) -> String {
    let typed = typed.trim();
    if typed
        .rsplit_once(':')
        .is_some_and(|(_, port)| port.parse::<u16>().is_ok())
    {
        typed.to_string()
    } else {
        format!("{typed}:{DEFAULT_PORT}")
    }
}

fn say(strings: &StringTable, id: &str, english: &str) -> String {
    strings.get(id).unwrap_or(english).to_string()
}

/// Fills `screen` (a list) with the lobby's `rooms`: the owner on the left, the title and the
/// head count on the right, the circuit, class and mode as the line under the title while the
/// cursor is on it, and a room this machine cannot join dimmed with its reason. `track_name`
/// turns a circuit's entry name into the title's own name for it, else the folder is shown.
pub fn fill_list(
    screen: &mut Join,
    rooms: &[RoomInfo],
    strings: &StringTable,
    track_name: &dyn Fn(&str) -> Option<String>,
) {
    screen.set_seats(
        rooms
            .iter()
            .map(|room| {
                format!(
                    "{} ({}/{})",
                    room.settings.title.to_uppercase(),
                    room.members,
                    room.settings.max_players
                )
            })
            .collect(),
    );
    screen.set_row_labels(
        rooms
            .iter()
            .map(|room| room.owner.to_uppercase().chars().take(8).collect())
            .collect(),
    );
    let notes = rooms
        .iter()
        .map(|room| Some(room_line(room, strings, track_name)))
        .collect();
    screen.set_notes(notes, rooms.iter().map(|room| !room.joinable).collect());
}

/// The line under the title while the cursor is on a room: its circuit, class and mode, or
/// why it cannot be joined.
#[must_use]
pub fn room_line(
    room: &RoomInfo,
    strings: &StringTable,
    track_name: &dyn Fn(&str) -> Option<String>,
) -> String {
    if !room.joinable {
        return say(
            strings,
            "OAG_ONLINE_ROW_CLOSED",
            "NOT JOINABLE: FULL, STARTED, OR NOT YOUR TITLE, BUILD OR MODS",
        );
    }
    let track = if room.settings.track.is_empty() {
        say(strings, "OAG_ONLINE_DEFAULT_TRACK", "DEFAULT CIRCUIT")
    } else {
        track_name(&room.settings.track).unwrap_or_else(|| {
            room.settings
                .track
                .rsplit(['\\', '/'])
                .nth(1)
                .unwrap_or(&room.settings.track)
                .to_string()
        })
    };
    format!(
        "{} / {} / {}",
        track.to_uppercase(),
        room.settings.class.to_uppercase(),
        room.settings.mode.replace('_', " ").to_uppercase()
    )
}

/// A member's line in the room: the name, the host mark and whether the pick is made.
#[must_use]
pub fn member_line(member: &Member, strings: &StringTable) -> String {
    let mut line = member.name.to_uppercase();
    if member.owner {
        line.push_str(&format!(" ({})", say(strings, "OAG_ONLINE_HOST", "HOST")));
    }
    line.push_str(&format!(
        "  {}",
        if member.ready {
            say(strings, "OAG_ONLINE_READY", "READY")
        } else {
            say(strings, "OAG_ONLINE_PICKING", "PICKING")
        }
    ));
    line
}

/// The line under a room's title: the room's note, then where the room is.
#[must_use]
pub fn room_hint(room: &Snapshot, owner: bool, strings: &StringTable) -> String {
    let phase = match room.phase {
        Phase::Open if owner => say(
            strings,
            "OAG_ONLINE_ROOM_HINT_OWNER",
            "PRESS START WHEN EVERYONE IS READY",
        ),
        Phase::Open => say(strings, "OAG_ONLINE_ROOM_HINT", "THE HOST STARTS THE RACE"),
        Phase::Queued { position } => say(
            strings,
            "OAG_ONLINE_QUEUED",
            "ANOTHER GAME IS RACING: YOU ARE NUMBER %d IN LINE",
        )
        .replace("%d", &position.to_string()),
        Phase::Countdown { millis } => {
            say(strings, "OAG_ONLINE_COUNTDOWN", "THE RACE STARTS IN %d")
                .replace("%d", &millis.div_ceil(1000).to_string())
        }
        Phase::Racing => say(strings, "OAG_ONLINE_RACING", "THE RACE IS STARTING"),
    };
    if room.note.is_empty() {
        phase
    } else {
        format!("{} - {phase}", room.note.to_uppercase())
    }
}

/// Fills `screen` (the room form) from `room`: one seat per person up to the room's size, the
/// line under the title, and - for the owner, until START is pressed - ROOM SETTINGS and MAX
/// PLAYERS. A seat says the room's circuit, class and mode while the cursor is on it, so a guest
/// can see what the owner chose. `track_name` turns a circuit's entry name into the title's own.
pub fn fill_room(
    screen: &mut Join,
    room: &Snapshot,
    owner: bool,
    strings: &StringTable,
    track_name: &dyn Fn(&str) -> Option<String>,
) {
    screen.set_capacity(usize::from(room.settings.max_players));
    screen.set_owner((owner && room.phase == Phase::Open).then_some(room.settings.max_players));
    screen.set_seats(
        room.members
            .iter()
            .map(|m| member_line(m, strings))
            .collect(),
    );
    let info = room_info(&room.settings, strings, track_name);
    screen.set_notes(
        room.members.iter().map(|_| Some(info.clone())).collect(),
        Vec::new(),
    );
    screen.set_hint(room_hint(room, owner, strings));
}

/// A room's circuit, class and mode on one line.
#[must_use]
pub fn room_info(
    settings: &RoomSettings,
    strings: &StringTable,
    track_name: &dyn Fn(&str) -> Option<String>,
) -> String {
    let info = RoomInfo {
        id: 0,
        owner: String::new(),
        settings: settings.clone(),
        members: 0,
        joinable: true,
    };
    room_line(&info, strings, track_name)
}

/// The next room size: `current` moved by `by` seats, wrapping from `most` round to `fewest`
/// and back.
#[must_use]
pub fn step_max_players(current: i32, by: i32, fewest: i32, most: i32) -> i32 {
    let span = (most - fewest + 1).max(1);
    fewest + (current - fewest + by).rem_euclid(span)
}

#[cfg(test)]
mod tests;

/// The team a room pick names: `picked` when there is one, else the title's default team as the
/// RACE page's roster spells it (case aside), else the roster's first, else none.
pub fn room_team(picked: Option<&str>, default: &str, roster: &[oag_ui::menu::Choice]) -> String {
    if let Some(team) = picked {
        return team.to_string();
    }
    roster
        .iter()
        .find(|choice| choice.value.eq_ignore_ascii_case(default))
        .or_else(|| roster.first())
        .map(|choice| choice.value.clone())
        .unwrap_or_default()
}
