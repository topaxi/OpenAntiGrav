//! A split-screen race's HUD, per view, on a real circuit.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game split_hud_ground_truth:: --run-ignored all
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
        hud.split_hud.as_ref(),
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
    let all_draws = draws(&race, &hud);
    // This project's split-HUD file places each group of widgets on its own, so a
    // view is several draws and the reticle's: every view the same number of them.
    assert!(
        hud.split_hud.is_some(),
        "a four-view race reads the split-HUD file"
    );
    let per = all_draws.len() / 4;
    assert!(per > 1 && all_draws.len() == per * 4, "{}", all_draws.len());
    let views: Vec<&[oag_game::hud_overlay::ViewDraw<'_>]> = all_draws.chunks(per).collect();
    let clip_of = |i: usize| views[i][0].placement.clip;

    // The four craft are on different parts of the grid, so their readouts differ
    // in something the HUD prints.
    let speeds: Vec<u32> = views
        .iter()
        .map(|v| v[0].readout.speed_kmh as u32)
        .collect();
    assert!(
        speeds.iter().any(|&s| s != speeds[0]),
        "four slots, one speed: {speeds:?}"
    );

    let places: Vec<u32> = views.iter().map(|v| v[0].readout.place).collect();
    println!("places at tick 600: {places:?}");
    let mut distinct = places.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        places.len(),
        "two slots share a place: {places:?}"
    );

    let mut per_view = Vec::new();
    for (i, view) in views.iter().enumerate() {
        let frame = views_frame(view);
        let words = texts(&frame);
        assert!(!words.is_empty(), "a view with no text at all");
        let [cx0, cy0, cw, ch] = clip_of(i);
        for (x, y, text) in &words {
            assert!(
                *x >= cx0 && *x < cx0 + cw && *y >= cy0 && *y < cy0 + ch,
                "{text:?} at ({x}, {y}) is outside its view {:?}",
                clip_of(i)
            );
        }
        per_view.push(words);
    }
    assert!(
        per_view.windows(2).any(|pair| pair[0] != pair[1]),
        "every view drew the same text: one readout for all"
    );

    // Together, in the one list the renderer paints.
    let all = views_frame(&all_draws);
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
        hud.split_hud.as_ref(),
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

/// HD's concentric reticle is looked up by sprite name in the layout the view
/// draws, which in a two-view race is the authored split layout: if that names its
/// sight sprites differently from the title's table, HD draws no reticle in a split
/// and nothing else notices.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_draws_a_reticle_in_its_authored_split_view() {
    let Some(loaded) = hd_loaded(2) else {
        return;
    };
    let hud = loaded.hud;
    let mut race = race::Race::start(loaded.setup);
    race.set_sight_screen(hud.sight_screen());
    race.set_sight_dialect(hud.art.sights);
    oag_game::split_screen::apply(&mut race, 2);
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    for slot in 0..2 {
        race.sim.world.ships[slot].pickup.weapon = Some(oag_tables::weapons::Weapon::Missile);
    }

    let mut drawn = [false; 2];
    for _ in 0..200 {
        race.tick(&PlayerInputs::none());
        let views = draws(&race, &hud);
        for (i, view) in views.iter().enumerate() {
            let Some(sight) = view.readout.sight else {
                continue;
            };
            if !sight.visible() {
                continue;
            }
            let own = oag_hud::sight_draws(&view.cx, &sight);
            assert!(
                !own.is_empty(),
                "view {i}: the reticle is up and the authored split layout draws none of it"
            );
            // Where the reticle's centre lands: its view-relative position, moved
            // by the view's own placement, inside that view.
            let at = view
                .sight
                .expect("an authored view places its reticle on its own");
            let [cx, cy] = sight.centre();
            let (x, y) = (cx * at.scale + at.offset[0], cy * at.scale + at.offset[1]);
            let [x0, y0, w, h] = view.placement.clip;
            assert!(
                x >= x0 && x < x0 + w && y >= y0 && y < y0 + h,
                "view {i}: the reticle's centre ({x}, {y}) is outside its view {:?}",
                view.placement.clip
            );
            let frame = views_frame(&views[i..=i]);
            let near = |d: &Draw| match d {
                Draw::Sprite { rect, .. } | Draw::BlendedSprite { rect, .. } => {
                    let (mx, my) = (rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5);
                    (mx - x).abs() < 40.0 && (my - y).abs() < 40.0
                }
                _ => false,
            };
            assert!(
                frame.sprites.iter().any(near),
                "view {i}: no sprite of the frame is at the reticle's centre ({x}, {y})"
            );
            drawn[i] = true;
        }
    }
    assert!(
        drawn.iter().any(|&d| d),
        "a Missile in hand for 200 ticks and no view of an HD two-view race showed a reticle"
    );
}

/// `(sprites, labels, widest, lowest)` of a composed layout's sprites.
type Extent = (usize, usize, f32, f32);

fn extent(layout: &oag_hud::Layout) -> Extent {
    let mut wide = 0.0_f32;
    let mut low = 0.0_f32;
    for sprite in &layout.sprites {
        wide = wide.max(sprite.rect[0] + sprite.rect[2]);
        low = low.max(sprite.rect[1] + sprite.rect[3]);
    }
    (layout.sprites.len(), layout.labels.len(), wide, low)
}

fn composed(archives: &mut oag_assets::Archives, root: &str) -> oag_hud::Composed {
    let composed = oag_hud::compose(root, |path| archives.read_name(path).ok())
        .unwrap_or_else(|| panic!("{root} composes"));
    assert!(
        composed.missing.is_empty() && composed.layout.skipped.is_empty(),
        "{root}: missing {:?}, skipped {:?}",
        composed.missing,
        composed.layout.skipped
    );
    composed
}

/// HD, 2048 and Omega ship the same four split layouts, byte for byte as far as a
/// composed layout can say, and all in HD's 1920x1080 grid with the second player
/// 540 lower. Omega's grid is HD's, so `oag_omega` could carry the same row; 2048's
/// HUD space is the Vita's, so its copy would need scaling first, and neither is
/// wired.
#[test]
#[ignore = "needs the HD image and the extracted Vita and PS4 packages"]
fn hd_2048_and_omega_ship_the_same_split_layouts_540_apart() {
    let mut found: Vec<(&str, [Extent; 4])> = Vec::new();
    let hd_roots: Vec<String> = oag_hd::hud::SPLIT
        .arcade
        .iter()
        .chain(&oag_hd::hud::SPLIT.elimination)
        .map(|root| (*root).to_string())
        .collect();
    let family = |directory: &str, names: [&str; 4]| -> Vec<String> {
        names
            .iter()
            .map(|name| format!("{directory}{name}.xml"))
            .collect()
    };
    let take = |archives: &mut oag_assets::Archives, roots: &[String]| {
        let mut out = [(0, 0, 0.0, 0.0); 4];
        for (slot, root) in roots.iter().enumerate() {
            out[slot] = extent(&composed(archives, root).layout);
        }
        out
    };
    if let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") {
        let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
        found.push(("HD", take(&mut archives, &hd_roots)));
    }
    if let Some(vita) = oag_testdata::exact("data/extracted/vita/PCSF00007/base") {
        let mut archives = oag_2048::open(&vita.display().to_string()).expect("2048 opens");
        let roots = family(
            r"Data\XML\SplitScreen_hud\",
            [
                "SplitScreen_HUD_0",
                "SplitScreen_HUD_1",
                "SplitScreen_Elimination_HUD_0",
                "SplitScreen_Elimination_HUD_1",
            ],
        );
        found.push(("2048", take(&mut archives, &roots)));
    }
    if let Some(ps4) = oag_testdata::exact("data/extracted/ps4") {
        let mut archives = oag_omega::open(&ps4.display().to_string()).expect("Omega opens");
        // The roots the title's own row names, so dropping the row fails here.
        let split = oag_omega::hud::LAYOUTS
            .split
            .expect("Omega wires the split row");
        let roots: Vec<String> = split
            .arcade
            .iter()
            .chain(&split.elimination)
            .map(|root| (*root).to_owned())
            .collect();
        assert_eq!(split.view_height, 540);
        found.push(("Omega", take(&mut archives, &roots)));
    }
    println!(
        "compared: {:?}",
        found.iter().map(|(t, _)| *t).collect::<Vec<_>>()
    );
    let Some((first, reference)) = found.first().copied() else {
        return;
    };
    for (title, measured) in &found {
        assert_eq!(
            measured, &reference,
            "{title}'s split layouts are not {first}'s"
        );
    }
    // The second player of each pair sits exactly one half-screen lower.
    for pair in [(0, 1), (2, 3)] {
        assert_eq!(reference[pair.1].3 - reference[pair.0].3, 540.0);
    }
}

/// Every name a split-HUD file writes is a widget its title authors, in some mode, or
/// one of the assist indicator's, which this project splices in for the titles whose
/// discs author none. A typo would otherwise draw nothing and say nothing, and a rule
/// whose names stopped matching would leave a view with no HUD.
#[test]
#[ignore = "needs data/images/{pulse-psp-eu,pulse-ps2-eu,pure-psp-eu}.chd and hdfury-ps3-eu-dec.iso"]
fn the_shipped_split_files_name_only_widgets_their_titles_author() {
    use oag_hud::split_hud::{LineHeights, SplitHud, arrange};
    let assist = oag_hud::Layout::from_xml(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/fallback/Data/XML/HUD_assist_indicator.xml"
        ))
        .unwrap(),
    );
    let heights = LineHeights {
        hud: 25.0,
        small: 10.0,
        default: 13.0,
    };
    let mut checked = Vec::new();
    for (image, file) in [
        ("data/images/pulse-psp-eu.chd", "Pulse"),
        ("data/images/pulse-ps2-eu.chd", "Pulse"),
        ("data/images/pure-psp-eu.chd", "Pure"),
        ("data/images/hdfury-ps3-eu-dec.iso", "HD"),
    ] {
        let Some(path) = oag_testdata::image(image) else {
            continue;
        };
        let opened =
            oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
                .expect("the title opens");
        let (mut archives, title) = (opened.archives, opened.title);
        let layouts = [
            title.hud.arcade,
            title.hud.time_trial,
            title.hud.speed_lap,
            title.hud.zone,
            title.hud.elimination,
        ];
        let mut whole = oag_hud::Layout::default();
        let mut arcade = oag_hud::Layout::default();
        for entry in layouts {
            let composed = composed(&mut archives, entry);
            let layout = &composed.layout;
            if entry == title.hud.arcade {
                arcade = layout.clone();
            }
            whole.sprites.extend(layout.sprites.iter().cloned());
            whole.fills.extend(layout.fills.iter().cloned());
            whole.labels.extend(layout.labels.iter().cloned());
            whole.models.extend(layout.models.iter().cloned());
        }
        for layout in [&mut whole, &mut arcade] {
            layout.sprites.extend(assist.sprites.iter().cloned());
        }
        let text = std::fs::read_to_string(format!(
            "{}/../../assets/fallback/Data/XML/SplitHud_{file}.toml",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let hud = SplitHud::parse(&text).expect("the shipped file parses");
        let names: Vec<&str> = whole
            .sprites
            .iter()
            .map(|w| w.name.as_str())
            .chain(whole.fills.iter().map(|w| w.name.as_str()))
            .chain(whole.labels.iter().map(|w| w.name.as_str()))
            .chain(whole.models.iter().map(|w| w.name.as_str()))
            .collect();
        for rule in &hud.rules {
            for group in &rule.groups {
                for pattern in &group.widgets {
                    let one = oag_hud::split_hud::Group {
                        widgets: vec![pattern.clone()],
                        ..group.clone()
                    };
                    assert!(
                        names.iter().any(|name| one.matches(name)),
                        "{file} {:?}: `{pattern}` names no widget of {image}",
                        rule.name
                    );
                }
            }
            // And the rule is not an empty HUD: most of its groups find widgets in
            // the single-race layout.
            let pieces = arrange(rule, &arcade, heights);
            assert!(
                pieces.len() * 2 >= rule.groups.len() && pieces.len() >= 3,
                "{file} {:?}: only {} of {} groups place anything in the arcade layout",
                rule.name,
                pieces.len(),
                rule.groups.len()
            );
        }
        checked.push(image);
    }
    println!("checked {checked:?}");
}

/// The load report says which layout answers a split race, and says nothing about the
/// file for one view, which reads, reports and arranges none of it.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and hdfury-ps3-eu-dec.iso"]
fn the_load_report_names_the_split_layout_that_answers() {
    let load = |image: &str, views: u8| {
        let path = oag_testdata::image(image)?;
        Some(
            race::load(&race::Options {
                source: path.display().to_string(),
                class: "VENOM".to_string(),
                mode: oag_race::Mode::SingleRace,
                views,
                ..race::Options::default()
            })
            .expect("loading the race")
            .report,
        )
    };
    if let (Some(four), Some(one)) = (
        load("data/images/pulse-psp-usa.chd", 4),
        load("data/images/pulse-psp-usa.chd", 1),
    ) {
        let lines = |report: &[String]| -> Vec<String> {
            report
                .iter()
                .filter(|line| line.contains("SplitHud") || line.contains("split layout"))
                .cloned()
                .collect()
        };
        let four = lines(&four);
        assert!(
            four.iter()
                .any(|l| l.contains("SplitHud_Pulse.toml answered by this project's fallback")),
            "{four:?}"
        );
        for rule in ["strip", "landscape", "square", "portrait"] {
            assert!(
                four.iter()
                    .any(|l| l.contains(&format!("layout \"{rule}\", "))),
                "{rule}: {four:?}"
            );
        }
        assert!(
            four.iter()
                .any(|l| l.contains("answered by the file, the whole-screen layout elsewhere")),
            "{four:?}"
        );
        assert!(
            lines(&one).is_empty(),
            "one view reads no split file: {:?}",
            lines(&one)
        );
    }
    if let Some(two) = load("data/images/hdfury-ps3-eu-dec.iso", 2) {
        assert!(
            two.iter().any(|l| l.contains(
                "the title's authored layout answers, and this project's file where it has none"
            )),
            "{two:?}"
        );
    }
}
