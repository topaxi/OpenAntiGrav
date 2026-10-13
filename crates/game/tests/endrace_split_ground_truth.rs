//! Pulse's `EndRace` tables and its two-person screens against the discs they are read off.
//!
//! **`#[ignore]`d and never run in CI** - it needs game content. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test endrace_split_ground_truth --run-ignored all
//! ```
//!
//! What it pins that a miniature cannot:
//!
//! - the PS2's `EndRace Results` rows sit where its own file puts them, not at the PSP's
//!   constants (the geometry fix: the Eliminator's eighth place was highlighted on row 4);
//! - the PS2 authors `EndRace Results Split` and `EndRace Rewards Split`, and the PSP's
//!   borrowed copies, built from the numbers in `oag_ui_screens::endrace::borrow`, are the
//!   same screens: expressed in the PS2's grid they draw what the file draws, and in the
//!   PSP's they sit where the file's widgets do scaled;
//! - the redirect chain the file authors.

use std::path::PathBuf;

use oag_display::space::Space;
use oag_ui::frontend::{Draw, Placed};
use oag_ui::language::StringTable;
use oag_ui::menu::{Frame, Layers, Skin};
use oag_ui_screens::endrace::{
    EliminationResults, EliminationRow, LapSplit, Loyalty, Results, Rewards, borrow,
    elimination_results_draw_list, split_results_draw_list, split_rewards_draw_list,
};

const PS2: &str = "data/images/pulse-ps2-eu.chd";
const PSP: &str = "data/images/pulse-psp-eu.chd";

fn load(image: &str, space: Space) -> Option<oag_game::endrace::EndRaceScreens> {
    let path: PathBuf = oag_testdata::image(image)?;
    let mut archives = oag_source::title::open_source(&path.display().to_string(), vec![], vec![])
        .expect("the disc opens")
        .archives;
    Some(
        oag_game::endrace::load(
            &mut archives,
            &StringTable::default(),
            oag_ui_screens::picker::FaceScales::default(),
            [space.size.0, space.size.1],
            &oag_hud::sprite::Sheet::default(),
            &[],
            oag_pulse::TITLE,
        )
        .expect("Pulse's EndRace screens read"),
    )
}

fn skin(space: Space) -> Skin {
    Skin::new(oag_pulse::FRONT_END.menu.unwrap(), space, 22.0)
}

fn placed(_: &str) -> Option<Placed> {
    Some(Placed {
        x: 0,
        y: 0,
        width: 32,
        height: 32,
        quad_extent: None,
        blend: None,
    })
}

fn row(team: &str, kills: u32, deaths: u32, player: bool) -> EliminationRow {
    EliminationRow {
        team_name: Some(team.to_string()),
        kills,
        deaths,
        player,
    }
}

/// The wrapper fill's top as the file places it: the reader leaves a colour-only
/// `Image`'s own `OffsetY` out of `y`.
fn wrapper_top(layout: &oag_ui_screens::endrace::Layout, name: &str) -> f32 {
    layout
        .screen
        .fills
        .iter()
        .find(|fill| fill.name.as_deref() == Some(name))
        .map(|fill| fill.y + fill.own_offset.1)
        .unwrap_or_else(|| panic!("{name} is a fill"))
}

/// An Eliminator race the player finished last in: the highlight sits on row 8 and the
/// eight row backings sit at eight distinct tops, the file's own. **Fails on a build that
/// still carries the PSP's row geometry**: its highlight is at `93 + 20 * 7 = 233` against
/// the PS2 file's row 8 at about 375, and its eight backings pile up near the top.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd and pulse-psp-eu.chd"]
fn a_tables_rows_and_highlight_are_where_its_own_file_puts_them() {
    for (image, space) in [(PS2, Space::PS2), (PSP, Space::PSP)] {
        let Some(screens) = load(image, space) else {
            continue;
        };
        let model = EliminationResults::new(vec![
            row("Assegai", 0, 1, true),
            row("Feisar", 2, 4, false),
            row("EGX", 5, 5, false),
            row("Qirex", 3, 3, false),
            row("Goteki", 3, 1, false),
            row("AG_Systems", 2, 4, false),
            row("Triakis", 1, 2, false),
            row("Piranha", 4, 1, false),
        ]);
        assert_eq!(model.place(), Some(8));
        let layers = elimination_results_draw_list(
            &model,
            &screens.results,
            &skin(space),
            &Frame::default(),
            &StringTable::default(),
            None,
            false,
            &placed,
        );
        let wide = space.size.0 * 0.9;
        let highlight: Vec<f32> = layers
            .body
            .iter()
            .filter_map(|draw| match draw {
                Draw::Fill { rect, .. } if rect[2] >= wide && rect[3] < space.size.1 * 0.5 => {
                    Some(rect[1])
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            highlight.len(),
            1,
            "{image}: one highlight, got {highlight:?}"
        );
        let top8 = wrapper_top(&screens.results, "tablebg8");
        assert!(
            (highlight[0] - top8).abs() < 0.01,
            "{image}: the highlight is at {}, row 8's top at {top8}",
            highlight[0]
        );
        let mut backings: Vec<f32> = layers
            .body
            .iter()
            .filter_map(|draw| match draw {
                Draw::GradientFill { rect, .. } if rect[2] >= wide && rect[3] > 5.0 => {
                    Some(rect[1])
                }
                _ => None,
            })
            .collect();
        backings.sort_by(f32::total_cmp);
        let tops: Vec<f32> = (1..=8)
            .map(|row| wrapper_top(&screens.results, &format!("tablebg{row}")))
            .collect();
        assert_eq!(backings.len(), 8, "{image}: {backings:?}");
        for (row, (backing, top)) in backings.iter().zip(&tops).enumerate() {
            assert!(
                (backing - top).abs() < 0.01,
                "{image}: row {}'s backing is at {backing}, its top at {top}",
                row + 1
            );
        }
        if image == PSP {
            // Untouched: the PSP's own 20 px pitch from 92, the highlight the literal.
            assert!((highlight[0] - 233.0).abs() < 0.01, "{highlight:?}");
            assert!(
                (backings[1] - backings[0] - 21.0).abs() < 0.01,
                "{backings:?}"
            );
        }
    }
}

fn person(place: u8, ticks: &[u32], boosts: bool) -> Results {
    Results {
        headline: oag_ui_screens::endrace::Headline::Position(place),
        laps: ticks
            .iter()
            .enumerate()
            .map(|(index, ticks)| LapSplit {
                lap: index as u32 + 1,
                ticks: *ticks,
                boosts: boosts.then_some(index as u32 + 3),
            })
            .collect(),
        total_ticks: ticks.iter().map(|ticks| u64::from(*ticks)).sum(),
    }
}

/// A draw list as comparable text: every draw but the confirm prompt (its position is the
/// one thing the screens keep to themselves), sorted, and with the file's one authored
/// duplicate dropped (`topbarcenter_0` is written twice).
fn comparable(layers: &Layers) -> Vec<String> {
    let mut lines: Vec<String> = layers
        .body
        .iter()
        .filter(|draw| !matches!(draw, Draw::Text { text, .. } if text.starts_with("FE_CONFIRM")))
        .map(|draw| format!("{draw:?}"))
        .collect();
    lines.sort();
    lines.dedup();
    lines
}

fn people() -> [Results; 2] {
    [
        person(5, &[5549, 2960, 2996], true),
        person(7, &[5701, 3007, 3052], false),
    ]
}

/// `EndRace Results Split` is the screen the PS2 authors: the one built from the numbers the
/// PSP borrows, expressed in the PS2's own grid, draws exactly what the file draws.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn the_borrowed_results_split_draws_what_the_ps2_file_draws() {
    let Some(screens) = load(PS2, Space::PS2) else {
        return;
    };
    let authored = screens
        .results_split
        .as_ref()
        .expect("the PS2 authors EndRace Results Split");
    let borrowed = borrow::results_split(&screens.results, borrow::PS2_GRID).unwrap();
    let draw = |layout| {
        split_results_draw_list(
            &people(),
            layout,
            &skin(Space::PS2),
            &Frame::default(),
            &StringTable::default(),
            None,
            false,
            &placed,
        )
    };
    let (authored, borrowed) = (comparable(&draw(authored)), comparable(&draw(&borrowed)));
    assert!(
        authored.len() > 40,
        "a real table draws: {}",
        authored.len()
    );
    assert_eq!(authored, borrowed);
}

/// `EndRace Rewards Split` is the PS2's single Rewards panel twice: stamping the PS2's own
/// single screen at the PS2's offsets draws what the file's split screen draws.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn the_stamped_rewards_split_draws_what_the_ps2_file_draws() {
    let Some(screens) = load(PS2, Space::PS2) else {
        return;
    };
    let authored = screens
        .rewards_split
        .as_ref()
        .expect("the PS2 authors EndRace Rewards Split");
    let single = screens.rewards.as_ref().expect("Pulse authors Rewards");
    let stamped = borrow::rewards_split(single, borrow::PS2_GRID).unwrap();
    let panels = [panel("Assegai", 90, 90), panel("Feisar", 60, 41_300)];
    let draw = |layout| {
        split_rewards_draw_list(
            &panels,
            layout,
            &skin(Space::PS2),
            &Frame::default(),
            &StringTable::default(),
            None,
            false,
            &placed,
        )
    };
    let (authored, stamped) = (comparable(&draw(authored)), comparable(&draw(&stamped)));
    assert!(
        authored.len() > 20,
        "a real panel draws: {}",
        authored.len()
    );
    assert_eq!(authored, stamped);
}

fn panel(team: &str, award: u32, total: u32) -> Rewards {
    Rewards {
        medal: None,
        campaign: false,
        loyalty: Some(Loyalty {
            team_name: team.to_string(),
            award,
            total,
        }),
    }
}

/// The PSP's borrowed Rewards panels sit where the PS2's widgets do, scaled to its grid:
/// within a pixel down the screen and within 8 across it. The PSP's single panel is
/// hand-authored at about the PS2's ratio, not exactly (`loyaltynum` is at 340 against the
/// PS2's 445 * 0.75 = 334), and a panel stamped from it keeps the PSP's own numbers.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd and pulse-psp-eu.chd"]
fn the_psps_borrowed_rewards_panels_sit_where_the_ps2s_do_scaled() {
    let (Some(ps2), Some(psp)) = (load(PS2, Space::PS2), load(PSP, Space::PSP)) else {
        return;
    };
    let authored = ps2.rewards_split.as_ref().unwrap();
    let borrowed = psp.rewards_split.as_ref().expect("the PSP borrows it");
    let [kx, ky] = [
        borrow::PS2_GRID[0].recip() * 480.0,
        borrow::PS2_GRID[1].recip() * 272.0,
    ];
    let text = |layout: &oag_ui_screens::endrace::Layout, name: &str| {
        layout
            .screen
            .texts
            .iter()
            .find(|text| text.name.as_deref() == Some(name))
            .map(|text| (text.x, text.y))
            .unwrap_or_else(|| panic!("{name} is a text"))
    };
    for name in [
        "RewardLine1_0",
        "RewardLine2_0",
        "RewardLoyaltyActive_0",
        "loyaltynum_0",
        "RewardLine1_1",
        "RewardLine2_1",
        "loyaltynum_1",
    ] {
        let ((ax, ay), (bx, by)) = (text(authored, name), text(borrowed, name));
        assert!(
            (ax * kx - bx).abs() <= 8.0 && (ay * ky - by).abs() <= 1.0,
            "{name}: the PS2 has ({ax}, {ay}), scaled ({}, {}), the PSP's borrowed is ({bx}, {by})",
            ax * kx,
            ay * ky
        );
    }
    for name in ["BGgradient_0", "BGgradient_1"] {
        let (a, b) = (wrapper_top(authored, name), wrapper_top(borrowed, name));
        assert!(
            (a * ky - b).abs() <= 1.0,
            "{name}: {a} scaled is {}, borrowed {b}",
            a * ky
        );
    }
}

/// The chain the PS2 authors - `Results Split` -> `Rewards Split` -> `Race End Save` - and
/// the PSP's borrowed screens carry the same.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd and pulse-psp-eu.chd"]
fn the_authored_redirect_chain_is_results_then_rewards_then_save() {
    let goto = |layout: &oag_ui_screens::endrace::Layout| -> Vec<String> {
        layout
            .screen
            .redirects
            .iter()
            .filter_map(|redirect| redirect.goto.clone())
            .collect()
    };
    for (image, space) in [(PS2, Space::PS2), (PSP, Space::PSP)] {
        let Some(screens) = load(image, space) else {
            continue;
        };
        let results = screens.results_split.as_ref().expect(image);
        let rewards = screens.rewards_split.as_ref().expect(image);
        assert_eq!(
            goto(results),
            ["EndRace Rewards Split", "Race End Save"],
            "{image}: Results Split"
        );
        assert_eq!(goto(rewards), ["Race End Save"], "{image}: Rewards Split");
    }
}
