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

/// What `lan-lobby[:N]` or `lan-join[:N]` asks for: the screen and `N`, the people in the
/// lobby or the hosts found (two by default, none is allowed), or `None` for any other page.
#[must_use]
pub(super) fn lan(page: &str) -> Option<(bool, usize, usize)> {
    let (lobby, rest) = if let Some(rest) = page.strip_prefix("lan-lobby") {
        (true, rest)
    } else {
        (false, page.strip_prefix("lan-join")?)
    };
    if rest.is_empty() {
        return Some((lobby, 2, 0));
    }
    // `:N`, and for the list `:N:K`, the row the cursor is on (0, the first, by default).
    let mut parts = rest.strip_prefix(':')?.split(':');
    let people = parts.next()?.parse::<usize>().ok()?;
    let focus = parts.next().map_or(Some(0), |k| k.parse::<usize>().ok())?;
    Some((lobby, people.min(oag_gameplay::MAX_PLAYERS), focus))
}

/// What `online-room[:N]` or `online-list[:N[:K]]` asks for, as [`lan`] does for LAN: whether
/// it is the room, the people in it or the rooms listed, and the row the cursor is on.
#[must_use]
pub(super) fn online(page: &str) -> Option<(bool, usize, usize, bool)> {
    // `online-room-raced` is the room after a race: the last race's places on the seats.
    let (room, raced, rest) = if let Some(rest) = page.strip_prefix("online-room-raced") {
        (true, true, rest)
    } else if let Some(rest) = page.strip_prefix("online-room") {
        (true, false, rest)
    } else {
        (false, false, page.strip_prefix("online-list")?)
    };
    if rest.is_empty() {
        return Some((room, 2, 0, raced));
    }
    let mut parts = rest.strip_prefix(':')?.split(':');
    let people = parts.next()?.parse::<usize>().ok()?;
    let focus = parts.next().map_or(Some(0), |k| k.parse::<usize>().ok())?;
    Some((room, people.min(oag_gameplay::MAX_PLAYERS), focus, raced))
}

/// The online lobby's room (`people` in it, the first the host, the second still picking) or
/// its list of `people` rooms (the second one closed), drawn from the same fillers as the live
/// screens (`session::online`); the names are made up.
#[allow(
    clippy::too_many_arguments,
    reason = "the facts a still needs, each a separate thing the live screen has in hand"
)]
pub(super) fn capture_online(
    room: bool,
    raced: bool,
    people: usize,
    focus: usize,
    title: &str,
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
    use oag_net::wire::board::Board;
    use oag_net::wire::online::{Member, Phase, RoomInfo, RoomSettings, Snapshot};
    const NAMES: [&str; 8] = ["ADA", "BOB", "CYD", "DEE", "EVE", "FAY", "GUS", "HAL"];
    let settings = RoomSettings {
        title: title.into(),
        track: String::new(),
        class: "venom".into(),
        mode: "single_race".into(),
        max_players: 8,
        opponents: false,
        laps: 0,
        no_barrel_rolls: false,
        pilot_assist: true,
        weapons: true,
        zone_target: 0,
    };
    let screen = if room {
        let mut screen = Join::new(Labels::read_online_room(strings), oag_gameplay::MAX_PLAYERS);
        let snapshot = Snapshot {
            id: 1,
            settings,
            members: (0..people)
                .map(|n| Member {
                    name: NAMES[n].into(),
                    team: String::new(),
                    ready: n != 1,
                    owner: n == 0,
                })
                .collect(),
            phase: Phase::Open,
            note: if raced {
                "every person has finished".into()
            } else {
                String::new()
            },
            // Stated, like the rest of the page: the places the first people finished in,
            // best lap left out.
            board: Board {
                rows: (0..people.min(8))
                    .filter(|_| raced)
                    .map(|n| oag_net::wire::board::BoardRow {
                        place: [2, 1, 3, 4, 5, 6, 7, 8][n],
                        slot: u8::try_from(n).unwrap_or(0),
                        name: NAMES[n].into(),
                        laps: 3,
                        finish_ticks: Some(u32::try_from(3 * 60 * 61 + n * 97).unwrap_or(0)),
                        best_lap_ticks: None,
                        eliminations: 0,
                    })
                    .collect(),
            },
        };
        crate::online_screen::fill_room(&mut screen, &snapshot, true, strings, &|_| None);
        // No row is under the cursor unless the page asks for one.
        if focus > 0 {
            for _ in 0..=focus {
                screen.step(1);
            }
        }
        screen
    } else {
        let mut screen = Join::new(Labels::read_online_list(strings), 16).as_list();
        let rooms: Vec<RoomInfo> = (0..people)
            .map(|n| RoomInfo {
                id: u32::try_from(n).unwrap_or(0) + 1,
                owner: NAMES[n].into(),
                settings: settings.clone(),
                members: 1 + u8::try_from(n % 4).unwrap_or(0),
                joinable: n != 1,
            })
            .collect();
        crate::online_screen::fill_list(&mut screen, &rooms, strings, &|_| None);
        for _ in 0..=focus {
            screen.step(1);
        }
        screen
    };
    let mut list = oag_ui_screens::join::draw_list(&screen, skin, frame, backdrop, false).flatten();
    let (legend, ticker, font) = footer;
    let clip = menu_page::picker_footer(&mut list, legend, ticker, strings, font);
    (list, clip)
}

/// LAN's lobby with `people` waiting (the host first) or its host list with `people` hosts,
/// drawn like the live screens (`session::lan`); the names are made up, the rest is the
/// live screen's own.
#[allow(
    clippy::too_many_arguments,
    reason = "the facts a still needs, each a separate thing the live screen has in hand"
)]
pub(super) fn capture_lan(
    lobby: bool,
    people: usize,
    focus: usize,
    title: &str,
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
    const NAMES: [&str; 8] = ["ADA", "BOB", "CYD", "DEE", "EVE", "FAY", "GUS", "HAL"];
    let screen = if lobby {
        let labels = Labels::read_lan_lobby(strings);
        let you = strings.get("OAG_LAN_YOU").unwrap_or("YOU").to_string();
        let mut screen = Join::new(labels, oag_gameplay::MAX_PLAYERS);
        screen.set_seats(
            (0..people)
                .map(|n| {
                    if n == 0 {
                        format!("{} ({you})", NAMES[0])
                    } else {
                        NAMES[n].to_string()
                    }
                })
                .collect(),
        );
        screen
    } else {
        let mut screen = Join::new(Labels::read_lan_list(strings), 16).as_list();
        // The first host can be joined; the next ones each show one reason in turn (another
        // title, version, content, a full race).
        // The still's hosts run this title, or (for the "other title" row) another one.
        let elsewhere = ["Wipeout Pulse", "Wipeout HD"]
            .into_iter()
            .find(|name| *name != title)
            .unwrap_or("Another title");
        let local = oag_net::discovery::Local {
            protocol: 1,
            build: "build",
            title,
            platform: "PSP",
            content: 7,
        };
        let hosts: Vec<oag_net::discovery::Advert> = (0..people)
            .map(|n| {
                let mut host = oag_net::discovery::Advert {
                    protocol: 1,
                    port: 4435,
                    hash: [0; 32],
                    name: NAMES[n].to_string(),
                    title: title.into(),
                    platform: "PSP".into(),
                    build: "build".into(),
                    content: 7,
                    mode: "single_race".into(),
                    max: 8,
                    track: format!("Track {}", n + 1),
                    people: 1 + u8::try_from(n % 3).unwrap_or(0),
                };
                match n % 5 {
                    1 => host.title = elsewhere.into(),
                    2 => host.protocol = 2,
                    3 => host.content = 8,
                    4 => host.people = host.max,
                    _ => {}
                }
                host
            })
            .collect();
        crate::lan_list::fill(&mut screen, &hosts, &local, strings);
        // The cursor on row `focus`, so its line shows as the help line.
        for _ in 0..=focus {
            screen.step(1);
        }
        screen
    };
    let mut list = oag_ui_screens::join::draw_list(&screen, skin, frame, backdrop, false).flatten();
    let (legend, ticker, font) = footer;
    let clip = menu_page::picker_footer(&mut list, legend, ticker, strings, font);
    (list, clip)
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
