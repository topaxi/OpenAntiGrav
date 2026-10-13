//! A split-screen race's HUD, per view, on a real circuit.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test split_hud_ground_truth --run-ignored all
//! ```
//!
//! Every failure this guards against still draws a HUD, and one that reads
//! plausibly:
//!
//! - one `Readout` for every view (every quadrant showing slot 0's lap and speed);
//! - a HUD that spills out of its view into a neighbour;
//! - a second view whose reticle is slot 0's, or none.
//!
//! The draw list is read rather than a frame: what each view's text says and
//! where it lands is the claim, and none of it needs a GPU.

use std::path::PathBuf;

use oag_game::hud_overlay::views_frame;
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input, InputSnapshot};
use oag_hud::Readout;
use oag_raceplay as race;
use oag_ui::frontend::Draw;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn split_race(players: usize) -> Option<(race::Race, oag_hud::Assets)> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        views: players as u8,
        ..race::Options::default()
    })
    .expect("loading the race");
    let hud = loaded.hud;
    let mut race = race::Race::start(loaded.setup);
    race.set_sight_screen(hud.space.size);
    oag_game::split_screen::apply(&mut race, players);
    Some((race, hud))
}

/// Every slot holding the throttle, which is what puts four craft at four speeds.
fn throttle(slots: usize) -> PlayerInputs {
    let mut buttons = Input::EMPTY;
    buttons.begin_frame(Button::Cross.bit());
    let snapshot = InputSnapshot {
        buttons,
        ..InputSnapshot::EMPTY
    };
    let mut inputs = PlayerInputs::none();
    for slot in 0..slots {
        inputs.set(slot, snapshot);
    }
    inputs
}

fn texts(frame: &oag_hud::Frame) -> Vec<(f32, f32, String)> {
    frame
        .hud_text
        .iter()
        .chain(&frame.small_text)
        .filter_map(|draw| match draw {
            Draw::Text { x, y, text, .. } => Some((*x, *y, text.clone())),
            _ => None,
        })
        .collect()
}

const VIEWPORT: (f32, f32, f32, f32) = (0.0, 0.0, 1440.0, 816.0);

fn draws<'a>(
    race: &race::Race,
    hud: &'a oag_hud::Assets,
) -> Vec<oag_game::hud_overlay::ViewDraw<'a>> {
    let readouts: Vec<Readout> = (0..race.viewport_count())
        .map(|slot| race.readout_for(slot))
        .collect();
    oag_game::hud_views::view_draws(
        hud.context().expect("the layout parsed"),
        hud.split.as_ref(),
        readouts,
        VIEWPORT,
        hud.space,
    )
}

/// Each view's HUD is its own slot's: four quadrants do not repeat one readout.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_view_draws_its_own_slots_numbers_inside_its_own_rectangle() {
    let Some((mut race, hud)) = split_race(4) else {
        return;
    };
    let inputs = throttle(4);
    for _ in 0..600 {
        race.tick(&inputs);
    }
    let views = draws(&race, &hud);
    assert_eq!(views.len(), 4);

    // The four craft are on different parts of the grid, so their readouts differ
    // in something the HUD prints.
    let speeds: Vec<u32> = views.iter().map(|v| v.readout.speed_kmh as u32).collect();
    assert!(
        speeds.iter().any(|&s| s != speeds[0]),
        "four slots, one speed: {speeds:?}"
    );

    let mut per_view = Vec::new();
    for (i, view) in views.iter().enumerate() {
        let frame = views_frame(&views[i..=i]);
        let words = texts(&frame);
        assert!(!words.is_empty(), "a view with no text at all");
        let [cx0, cy0, cw, ch] = view.placement.clip;
        for (x, y, text) in &words {
            assert!(
                *x >= cx0 && *x < cx0 + cw && *y >= cy0 && *y < cy0 + ch,
                "{text:?} at ({x}, {y}) is outside its view {:?}",
                view.placement.clip
            );
        }
        per_view.push(words);
    }
    assert!(
        per_view.windows(2).any(|pair| pair[0] != pair[1]),
        "every view drew the same text: one readout for all"
    );

    // Together, in the one list the renderer paints.
    let all = views_frame(&views);
    let one_by_one: usize = per_view.iter().map(Vec::len).sum();
    assert_eq!(texts(&all).len(), one_by_one);
}

/// The reticle follows the view's own slot: a Missile in slot 3's hand puts a
/// reticle up in slot 3's view and in nobody else's.
///
/// Slot 3 because the grid is staggered with slot 0 at the back and slot 1 at the
/// front: slot 3 has two craft ahead of it, inside the Missile's window, while
/// the countdown still holds the field on the grid.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_later_views_reticle_follows_its_own_slots_lock() {
    let Some((mut race, _hud)) = split_race(4) else {
        return;
    };
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    race.sim.world.ships[3].pickup.weapon = Some(oag_tables::weapons::Weapon::Missile);
    let mut seen = false;
    for _ in 0..200 {
        race.tick(&PlayerInputs::none());
        for slot in [0, 1, 2] {
            let up = race.readout_for(slot).sight.is_some_and(|s| s.visible());
            assert!(!up, "slot {slot} holds nothing and has a reticle up");
        }
        let v = race.readout_for(3).sight.is_some_and(|s| s.visible());
        seen |= v;
    }
    assert!(
        seen,
        "slot 3 held a Missile for 200 ticks with two craft ahead and its view never showed a reticle"
    );
}

fn hd_loaded(views: u8) -> Option<race::Loaded> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    Some(
        race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            views,
            ..race::Options::default()
        })
        .expect("loading the race"),
    )
}

/// HD authors a two-player split HUD and a two-view race draws it: player `i`'s
/// own layout in view `i`, and nothing is read for any other count.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_draws_its_own_authored_split_layouts_in_a_two_view_race() {
    let (Some(one), Some(two)) = (hd_loaded(1), hd_loaded(2)) else {
        return;
    };
    assert!(
        one.hud.split.is_none(),
        "a one-view race reads no split layout"
    );
    let hud = two.hud;
    let split = hud.split.as_ref().expect("HD ships the split layouts");

    // Both are the disc's, whole, and the second is the first moved down half the
    // screen: the claim `oag_hd::hud::SPLIT` rests on.
    let extent = |layout: &oag_hud::Layout| {
        layout
            .sprites
            .iter()
            .map(|s| s.rect[1] + s.rect[3])
            .fold(0.0_f32, f32::max)
    };
    let [top, bottom] = &split.layouts;
    assert_eq!((top.sprites.len(), bottom.sprites.len()), (109, 109));
    assert!(top.skipped.is_empty() && bottom.skipped.is_empty());
    assert_eq!(
        extent(bottom) - extent(top),
        split.view.1,
        "player 1's layout is player 0's moved down one view"
    );
    assert_eq!(split.view, (1920.0, 540.0));
    assert_eq!(hud.sight_screen(), (1920.0, 540.0));

    // Every texture either layout names is in the one sheet the overlay binds.
    for layout in &split.layouts {
        for reference in layout.textures() {
            assert!(
                hud.sheet.get(reference).is_some(),
                "{reference} is named by a split layout and missing from the sheet"
            );
        }
    }

    // Drawn: each view's text is its own layout's and sits in its own half.
    let readouts = vec![Readout::blank(), Readout::blank()];
    let views = oag_game::hud_views::view_draws(
        hud.context().expect("the layout parsed"),
        hud.split.as_ref(),
        readouts,
        VIEWPORT,
        hud.space,
    );
    assert!(std::ptr::eq(views[0].cx.layout, &split.layouts[0]));
    assert!(std::ptr::eq(views[1].cx.layout, &split.layouts[1]));
    assert!(views.iter().all(|v| v.sight.is_some()));
    // The reticle is placed apart because it is view-relative: the second view's
    // is a half screen lower than the first's.
    let (a, b) = (views[0].sight.unwrap(), views[1].sight.unwrap());
    assert_eq!(a.scale, b.scale);
    assert!(b.clip[1] > a.clip[1], "the second view is below the first");
    for (i, view) in views.iter().enumerate() {
        let frame = views_frame(&views[i..=i]);
        let words = texts(&frame);
        assert!(!words.is_empty(), "view {i} drew no text");
        let [x0, y0, w, h] = view.placement.clip;
        for (x, y, text) in &words {
            assert!(
                *x >= x0 && *x < x0 + w && *y >= y0 && *y < y0 + h,
                "view {i}: {text:?} at ({x}, {y}) is outside {:?}",
                view.placement.clip
            );
        }
    }
}
