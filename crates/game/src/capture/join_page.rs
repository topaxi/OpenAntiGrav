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
pub(super) fn lan(page: &str) -> Option<(bool, usize)> {
    let (lobby, rest) = if let Some(rest) = page.strip_prefix("lan-lobby") {
        (true, rest)
    } else {
        (false, page.strip_prefix("lan-join")?)
    };
    if rest.is_empty() {
        return Some((lobby, 2));
    }
    rest.strip_prefix(':')?
        .parse::<usize>()
        .ok()
        .map(|n| (lobby, n.min(oag_gameplay::MAX_PLAYERS)))
}

/// LAN's lobby with `people` waiting (the host first) or its host list with `people` hosts,
/// drawn like the live screens (`session::lan`); the names are made up, the rest is the
/// live screen's own.
pub(super) fn capture_lan(
    lobby: bool,
    people: usize,
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
        let local = oag_net::discovery::Local {
            protocol: 1,
            build: "build",
            title: "THIS TITLE",
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
                    title: "THIS TITLE".into(),
                    platform: "PSP".into(),
                    build: "build".into(),
                    content: 7,
                    mode: "single_race".into(),
                    max: 8,
                    track: format!("Track {}", n + 1),
                    people: 1 + u8::try_from(n % 3).unwrap_or(0),
                };
                match n % 5 {
                    1 => host.title = "OTHER TITLE".into(),
                    2 => host.protocol = 2,
                    3 => host.content = 8,
                    4 => host.people = host.max,
                    _ => {}
                }
                host
            })
            .collect();
        crate::lan_list::fill(&mut screen, &hosts, &local, strings);
        // The cursor on the first incompatible host, so its reason shows as the hint.
        screen.step(1);
        if people > 1 {
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
