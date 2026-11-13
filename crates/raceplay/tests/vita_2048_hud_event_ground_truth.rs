//! Wipeout 2048's `XP` and `OBJECTIVE` lines on the extracted package: a campaign event draws the
//! first and neither draws the second; a DLC circuit raced without one draws neither.
//!
//! **`#[ignore]`d and never run in CI.** It needs the extracted Vita packages, base, patch and DLC1,
//! under `data/extracted/vita/PCSF00007`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/vita_2048_hud_event_ground_truth::/)'
//! ```
//!
//! The law is `Hud_BindWidgets` (`0x81192cd0`), which binds the current `SP.xml` event at `hud+4`:
//! `docs/ghidra/functions/vita-2048-eu-v104/race-hud-selection.md`. The originals this is checked
//! against are `data/reference/compare-page/2048-vita/empire-climb` (an event: `XP 0`, no
//! `OBJECTIVE`) and `dlc-vineta-k` (the HD Campaign's Uplift Race: neither).
//!
//! What this fails on: dropping `Setup::campaign_event` from `load_event`, dropping either of
//! `text_for`'s two gates, or a layout that stops authoring the widgets (the vacuity guards).
//!
//! The DLC load also checks the composite's exposure keys
//! ([`the_pack_copy_sets_the_exposure_held_for_a_comparison`]): one load serves both claims.

use std::path::{Path, PathBuf};

use oag_ui::frontend::Draw;

const VINETA_K: &str = r"Data\art\published\DLC1\environments\Vineta_K\track.vex";

fn source() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007");
    if path.join("base/PSP2/data.psarc").exists() && path.join("dlc1/PSP2/dlc1.psarc").exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not complete", path.display());
    None
}

fn options(source: &Path) -> oag_raceplay::Options {
    oag_raceplay::Options {
        source: source.display().to_string(),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    }
}

/// Every `HUD`-font string the loaded race's first frame draws, and whether the layout authors
/// the two widgets at all.
fn first_frame(loaded: oag_raceplay::Loaded) -> (Vec<String>, bool, bool) {
    let race = oag_raceplay::Race::start(loaded.setup);
    let readout = race.readout();
    let context = loaded.hud.context().expect("the layout parses");
    let authored = (
        context.layout.label("RaceXPTxt").is_some(),
        context.layout.label("ObjectiveTxt").is_some(),
    );
    let text = oag_hud::draw_list(&context, &readout)
        .hud_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    (text, authored.0, authored.1)
}

/// `2048 - Event 1` authors a pass objective and no `m_options`: the original draws `XP` and no
/// `OBJECTIVE`, and so do we, because `load_event` says an event exists.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn a_campaign_event_draws_xp_and_no_objective() {
    let Some(source) = source() else { return };
    let loaded = oag_raceplay::load_event(&options(&source), "2048 - Event 1").expect("it loads");
    assert!(loaded.campaign_2048_event.is_some());
    assert!(loaded.setup.campaign_event);
    let (text, xp_authored, objective_authored) = first_frame(loaded);
    assert!(xp_authored && objective_authored, "the layout authors both");
    assert!(text.iter().any(|text| text == "XP"), "{text:?}");
    assert!(
        !text.iter().any(|text| text.contains("OBJECTIVE")),
        "{text:?}"
    );
}

/// The DLC's Vineta K raced as the HD Campaign's Race is: no event object. Neither line draws,
/// as in the original's frame, and the rest of the HUD does.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn a_dlc_circuit_raced_with_no_event_draws_neither() {
    let Some(source) = source() else { return };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        track: Some(VINETA_K.to_string()),
        team: Some("Feisar".to_string()),
        mode: oag_race::Mode::SingleRace,
        exposure_hold: Some(1.15),
        ..options(&source)
    })
    .expect("the DLC circuit loads");
    the_pack_copy_sets_the_exposure_held_for_a_comparison(&loaded);
    assert!(loaded.campaign_2048_event.is_none());
    assert!(!loaded.setup.campaign_event);
    let (text, xp_authored, objective_authored) = first_frame(loaded);
    assert!(xp_authored && objective_authored, "the layout authors both");
    assert!(!text.iter().any(|text| text == "XP"), "{text:?}");
    assert!(
        !text.iter().any(|text| text.contains("OBJECTIVE")),
        "{text:?}"
    );
    assert!(
        text.iter().any(|text| text == "POS"),
        "the place still draws: {text:?}"
    );
}

/// Vineta K's exposure keys come from `dlc1.psarc`'s copy of its `track.EnvSettings` (scale 0.5,
/// max 8, gate 0.3), not the base `data.psarc`'s (0.6, 12, 0.2): the original's setters read the
/// pack's on that circuit (`docs/ghidra/functions/vita-2048-eu-v104/exposure.md`). The comparison
/// switch `Options::exposure_hold` lands as a held exposure.
fn the_pack_copy_sets_the_exposure_held_for_a_comparison(loaded: &oag_raceplay::Loaded) {
    use oag_post::wo_composite::{Exposure, Params};
    assert_eq!(
        loaded.wo_composite,
        Some(Params {
            exposure_scale: 0.5,
            exposure_max: 8.0,
            exposure_speed: 0.05,
            bloom_factor: 0.2,
            bloom_gate: 0.3,
            exposure: Exposure::Held(1.15),
        }),
        "{:#?}",
        loaded.report
    );
}

/// The HD Campaign's Zone event reads `Zone_HUD_HD.xml`, which authors the `SCORE` readout top
/// centre and `SpeedClass`/`Zone Txt`/`ZonePlus0` bottom left (the original's frames
/// `67-zone-grid`/`68-zone-5`); the native `Zone_HUD.xml` authors none of them. Fails if
/// `Options::add_on_campaign` stops selecting the layout.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_hd_campaigns_zone_reads_its_hd_layout() {
    let Some(source) = source() else { return };
    let load = |campaign: Option<&str>| {
        oag_raceplay::load(&oag_raceplay::Options {
            track: Some(VINETA_K.to_string()),
            team: Some("Feisar".to_string()),
            mode: oag_race::Mode::Zone,
            add_on_campaign: campaign.map(str::to_string),
            ..options(&source)
        })
        .expect("the DLC circuit loads")
    };
    let authored = |loaded: &oag_raceplay::Loaded| {
        let layout = loaded.hud.context().expect("the layout parses").layout;
        ["Score", "SpeedClass", "Zone Txt", "ZonePlus0"].map(|name| layout.label(name).is_some())
    };
    let native = load(None);
    assert_eq!(authored(&native), [false; 4], "{:#?}", native.report);
    let hd = load(Some("HD"));
    assert!(
        hd.report
            .iter()
            .any(|line| line.contains("Zone_HUD_HD.xml")),
        "{:#?}",
        hd.report
    );
    assert_eq!(authored(&hd), [true; 4], "{:#?}", hd.report);
}

/// The HD Campaign's grid with Pilot Assist Extreme on, as the original's frame
/// (`data/reference/compare-page/2048-vita/dlc-vineta-k`): the `PilotAssist` icon is its 256x128
/// texture drawn whole into the layout's 64x32 box (`HUD_assist_indicator.xml` authors no
/// `TxtrWidth`), and `TOTAL` reads hundredths (`0.00.00`) over `CURRENT` in tenths (`0.00.0`).
///
/// Fails if a sprite with no authored texture size goes back to cropping a 64x32 patch off the
/// corner (a solid white bar), if the icon stops following the setting, or if
/// `HudArt::total_time_hundredths` is dropped from 2048. Omega plays HD's layouts and HD authors
/// no widget of this kind (checked over HD's 191 HUD files), so it is **checked, not wired**.
///
/// It ends on [`a_grant_flies_the_missile_icon_to_the_slot_and_widens_the_frame`]: the same
/// 17 s load serves both claims.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn pilot_assist_draws_its_icon_whole_and_the_total_reads_hundredths() {
    let Some(source) = source() else { return };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        track: Some(VINETA_K.to_string()),
        team: Some("Feisar".to_string()),
        mode: oag_race::Mode::SingleRace,
        ..options(&source)
    })
    .expect("the DLC circuit loads");
    let context = loaded.hud.context().expect("the layout parses");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    let icon_uv = |race: &oag_raceplay::Race| {
        oag_hud::draw_list(&context, &race.readout())
            .sprites
            .iter()
            .find_map(|draw| match draw {
                Draw::Sprite { rect, uv, .. } if rect[2] == 64.0 && rect[3] == 32.0 => Some(*uv),
                _ => None,
            })
    };
    assert_eq!(icon_uv(&race), None, "no icon with the assist off");
    race.set_pilot_assist(oag_physics::pilot_assist::Level::Extreme);
    let uv = icon_uv(&race).expect("the icon is up with Extreme on");
    assert_eq!(&uv[2..], &[256.0, 128.0], "the whole texture: {uv:?}");

    let frame = oag_hud::draw_list(&context, &race.readout());
    let text: Vec<&str> = frame
        .hud_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(text.contains(&"0.00.00"), "TOTAL in hundredths: {text:?}");
    assert!(text.contains(&"0.00.0"), "CURRENT in tenths: {text:?}");

    a_grant_flies_the_missile_icon_to_the_slot_and_widens_the_frame(&context, &mut race);
}

/// 2048's pickup grant on the disc's own layout and Missile icon: the icon sits at the authored
/// top-centre rect while the caption is up, then flies down and left at `dt * 1800.0` a frame
/// with `PickupBgFrame` widening, and settles at the slot (`Hud_UpdatePickupIcon`,
/// `Hud_BindWidgets`). Runs on the race
/// [`pilot_assist_draws_its_icon_whole_and_the_total_reads_hundredths`] already loaded.
///
/// Fails if `Readout::pickup_fly` is dropped from the readout or the draw (the icon snaps), if
/// the slot loses its `2.0` x nudge, or if the frame stops following the flight.
fn a_grant_flies_the_missile_icon_to_the_slot_and_widens_the_frame(
    context: &oag_hud::Context<'_>,
    race: &mut oag_raceplay::Race,
) {
    let slot = race.player_slot();
    // Whatever the race held before, a few ticks of nothing re-arm the pop-in (the height runs out
    // at 10/s, then the next frame resets it).
    race.sim.world.ships[slot].pickup.weapon = None;
    for _ in 0..20 {
        race.tick(&oag_gameplay::PlayerInputs::none());
    }
    race.sim.world.ships[slot].pickup.weapon = Some(oag_tables::weapons::Weapon::Missile);

    // The icon is the one 85x86 sprite (`PickupIcon`'s size) and the frame the one 95 high
    // (`PickupBgFrame`'s height, which the fly leaves alone).
    let frame_of = |race: &oag_raceplay::Race| {
        let mut icon = None;
        let mut frame = None;
        for draw in &oag_hud::draw_list(context, &race.readout()).sprites {
            if let Draw::Sprite { rect, .. } = draw {
                if rect[2..] == [85.0, 86.0] {
                    icon = Some(*rect);
                } else if rect[3] == 95.0 {
                    frame = Some(*rect);
                }
            }
        }
        (icon, frame)
    };
    // The icon pops in over its first ticks (`hud+0x220` falls `10 -> 1` at 70/s, `hud+0x224`
    // rises `0 -> 1` at 7/s): a wide bar about the authored centre, not yet its 85x86 box.
    race.tick(&oag_gameplay::PlayerInputs::none());
    let popping = oag_hud::draw_list(context, &race.readout())
        .sprites
        .iter()
        .find_map(|draw| match draw {
            Draw::Sprite { rect, .. }
                if (rect[0] + rect[2] * 0.5 - 480.0).abs() < 0.01
                    && (rect[1] + rect[3] * 0.5 - 60.0).abs() < 0.01 =>
            {
                Some(*rect)
            }
            _ => None,
        })
        .expect("the popping icon draws about the authored centre");
    assert!(
        popping[2] > 85.0 * 5.0 && popping[3] < 86.0 * 0.3,
        "{popping:?}"
    );
    // Nine more ticks finish the pop (`9 / 70` s wide, `1 / 7` s high); the caption has run ten.
    for _ in 0..9 {
        race.tick(&oag_gameplay::PlayerInputs::none());
    }
    let mut seen = Vec::new();
    for _ in 0..80 {
        race.tick(&oag_gameplay::PlayerInputs::none());
        let (icon, frame) = frame_of(race);
        seen.push((
            race.readout().pickup_announcing,
            (icon.expect("the Missile icon draws once popped"), frame),
        ));
    }
    let announcing = seen.iter().take_while(|(up, _)| *up).count();
    assert!(
        (49..=50).contains(&announcing),
        "{announcing} ticks of caption left after the pop"
    );
    for (_, (icon, frame)) in &seen[..announcing] {
        assert_eq!(
            icon[..2],
            [437.5, 17.0],
            "the authored rect while announcing"
        );
        assert_eq!(*frame, None, "no frame while announcing");
    }
    let flight = &seen[announcing..];
    let moved = flight
        .iter()
        .take_while(|(_, (icon, _))| icon[..2] != [101.5, 404.5])
        .count();
    assert!((12..=14).contains(&moved), "{moved} ticks in flight");
    let mut last = (f32::MIN, f32::MAX, 0.0);
    for (_, (icon, frame)) in &flight[..moved] {
        let width = frame.expect("the frame draws in flight")[2];
        assert!(
            icon[1] > last.0 && icon[0] < last.1 && width > last.2,
            "{icon:?}"
        );
        last = (icon[1], icon[0], width);
    }
    let (_, (icon, frame)) = flight.last().expect("it settled");
    assert_eq!(icon[..2], [101.5, 404.5], "the slot");
    assert_eq!(frame.expect("the frame stays")[2..], [110.0, 95.0]);
}
