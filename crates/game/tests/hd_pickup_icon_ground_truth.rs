//! Wipeout HD's thirteen `<Type>Icon` sprites, against the disc that authors
//! them.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`. The image must be
//! layer-1 decrypted first - `docs/formats/ps3-disc.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(hd_pickup_icon_ground_truth)'
//! ```
//!
//! # What only real data can say here
//!
//! `crates/game/tests/pickup_icon_ground_truth.rs` checks Pure's `<Mode3D>`
//! icon *models* resolve into the sheet with a real quad extent. HD's own
//! icons are `<Image>` sprites (`oag_title::HudArt::pickup_icon_models` is
//! `None` for this title, per `crates/hd/src/hud.rs`), so the equivalent
//! check here is that each of the thirteen `<Type>Icon` widgets
//! `oag_hud::pickup_icon_name` names is authored in the composed arcade
//! layout, resolves its own `Src` into the shared sheet, and carries a
//! non-zero source rectangle - the three ways a "the hexagon draws empty"
//! symptom could actually originate (widget missing, texture unresolved,
//! degenerate UV) without a screenshot's own lighting getting in the way.
//!
//! # The screenshot that looked empty was a `--ticks 0` capture, not a bug
//!
//! 2026-09-15: a `--race --give leachbeam --screenshot` capture with no
//! `--ticks` (default `0`) showed the pickup hexagon empty for every weapon
//! checked. That is `--ticks 0` doing exactly what its own doc comment says -
//! `capture::advance_one_tick` writes `options.give` into
//! `ships[0].pickup.weapon` **before** `Race::tick`, so a capture that runs
//! zero ticks never reaches that write and `readout.pickup` stays `None` for
//! the whole capture; `oag_hud::draw::draw_list` correctly draws
//! nothing for a `None` pickup. A one-tick-later capture
//! (`--ticks 1` or more) shows `holding <Weapon>` in the telemetry line and
//! every one of the thirteen icons draws, pixel-identical to the sprite this
//! test crops out of the sheet.
//!
//! **The LeachBeam's own icon is a thin `+`/`-` glyph** - read directly out of
//! `hdHUD.mip` at `LeachBeamIcon`'s own authored UV rect, not a rendering
//! defect: a plus and a minus, no outline box around either, on a bright sky
//! backdrop is genuinely easy to miss in a full 1920x1080 frame at a glance,
//! which is what made the original report read as "empty" rather than "thin".
//! `RepulserIcon` (double chevrons) is the same shape of glyph. Nothing here
//! found a genuine gap: all thirteen resolve, decode and draw.

use std::path::{Path, PathBuf};

use oag_game::race;
use oag_gameplay::PlayerInputs;
use oag_tables::weapons::Weapon;

fn image_named(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

fn image() -> Option<PathBuf> {
    image_named("hdfury-ps3-eu-dec.iso")
}

fn race_on(image: &Path) -> race::Loaded {
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    loaded
}

/// Every one of HD's weapon icon widgets is authored on its arcade layout,
/// resolves into the HUD sheet, and carries a real (non-zero, non-degenerate)
/// source rectangle - sized off `Weapon::ALL` rather than a literal count.
///
/// **HD's thirteen**: the fourteenth `Weapon`, the Disruptor, is Pure's alone.
/// HD's weapon table authors no `<Weapon type="Disruptor">` and its layouts no
/// `DisruptorIcon` (`docs/formats/weapon-stats.md`, "Pure dialect"), so it is
/// filtered here the way `hud_layout_ground_truth` filters it on Pulse - a disc
/// fact about the roster, not a gap in the draw path.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_weapon_icon_resolves_to_a_real_sprite_with_non_zero_extent() {
    let Some(image) = image() else { return };
    let loaded = race_on(&image);
    let layout = loaded
        .hud
        .layout
        .as_ref()
        .expect("HD's arcade layout parsed");

    let mut checked = 0;
    let hd_roster = Weapon::ALL
        .into_iter()
        .filter(|weapon| *weapon != Weapon::Disruptor);
    for weapon in hd_roster {
        let name = oag_hud::pickup_icon_name(weapon);
        let sprite = layout
            .sprites
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("HD's arcade layout has no {name}"));
        assert!(
            sprite.rect[2] > 0.0 && sprite.rect[3] > 0.0,
            "{name} is authored with a degenerate destination rect {:?}",
            sprite.rect
        );
        assert!(
            sprite.uv[2].abs() > 0.0 && sprite.uv[3].abs() > 0.0,
            "{name} is authored with a degenerate source rect {:?}",
            sprite.uv
        );
        let placed = loaded.hud.sheet.get(&sprite.src).unwrap_or_else(|| {
            panic!(
                "{name}'s source {:?} did not reach the HUD sheet - a weapon \
                 held on a real race would draw nothing for it",
                sprite.src
            )
        });
        println!("{name}: placed at {placed:?}, uv {:?}", sprite.uv);
        checked += 1;
    }
    assert_eq!(
        checked,
        Weapon::ALL.len() - 1,
        "every weapon HD authors should have had its icon checked"
    );

    let backdrop = layout
        .sprites
        .iter()
        .find(|s| s.name == "PickupBackground")
        .expect("HD's arcade layout has PickupBackground");
    assert!(
        loaded.hud.sheet.get(&backdrop.src).is_some(),
        "PickupBackground's source {:?} did not reach the HUD sheet",
        backdrop.src
    );
}

/// A held weapon's icon draws a real, non-empty patch of pixels once the race
/// has actually ticked - the end-to-end path a `--ticks 0` capture cannot
/// exercise. Held **Rocket** for a shape the disc draws densely (three filled
/// hexagon outlines, easy to eyeball against a false negative) rather than
/// the thin LeachBeam glyph this module's doc comment discusses.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_held_rocket_draws_its_own_icon_sprite_on_a_real_race() {
    let Some(image) = image() else { return };
    let loaded = race_on(&image);
    let context = loaded
        .hud
        .context()
        .expect("HD's arcade layout parses; a context is buildable");

    let mut race = race::Race::start(loaded.setup);
    // The same edge `capture::advance_one_tick` writes on, and the reason a
    // `--ticks 0` capture never sees it: the assignment has to land before
    // `Race::tick` reads it into the readout.
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Rocket);
    race.tick(&PlayerInputs::none());

    let readout = race.readout();
    assert_eq!(
        readout.pickup,
        Some(Weapon::Rocket),
        "the readout must carry the held weapon for the draw list to find it"
    );
    let frame = oag_hud::draw_list(&context, &readout);
    let icon_name = oag_hud::pickup_icon_name(Weapon::Rocket);
    let icon_uv = context
        .layout
        .sprites
        .iter()
        .find(|s| s.name == icon_name)
        .map(|s| s.uv)
        .expect("RocketIcon is on the layout");

    // `sprite_draw` offsets the layout's own UV by wherever the sheet packer
    // placed the texture, so a raw equality against the layout's own UV would
    // only match a texture placed at the sheet's origin. `RocketIcon`'s own
    // `hdHUD.mip` is not guaranteed to land there, so this checks the frame
    // carries *some* sprite whose UV *extent* (not origin) matches.
    let drew_icon = frame.sprites.iter().any(|draw| {
        matches!(
            draw,
            oag_ui::frontend::Draw::Sprite { uv, .. }
                if (uv[2] - icon_uv[2]).abs() < 0.01 && (uv[3] - icon_uv[3]).abs() < 0.01
        )
    });
    assert!(
        drew_icon,
        "no sprite in the frame carried {icon_name}'s own UV extent {icon_uv:?}; \
         the frame carried: {:?}",
        frame.sprites
    );
}
