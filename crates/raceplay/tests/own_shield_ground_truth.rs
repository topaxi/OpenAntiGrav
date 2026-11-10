//! `graphics.own_shield` on a real disc: Pulse and Pure draw our own hull-shaped shield shell
//! when the setting is on and the disc's own when it is off; HD does not offer it.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^own_shield_ground_truth::/)'
//! ```
//!
//! The picture is `docs/rendering/own-shield.md`.

use oag_raceplay as race;

fn load(path: &std::path::Path, own_shield: bool) -> race::Loaded {
    race::load(&race::Options {
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        own_shield,
        ..race::Options::default()
    })
    .expect("loading the race")
}

#[test]
fn the_setting_is_off_by_default() {
    assert!(!race::Options::default().own_shield);
}

/// Off, every craft's shell is the disc's (no own-shield shader, the disc's palette); on, it is the
/// hull's own geometry under the own-shield shader with our palette, and so is the cockpit sphere.
#[test]
#[ignore = "needs Pulse's and Pure's images"]
fn pulse_and_pure_draw_their_own_shell_only_with_the_setting_on() {
    let mut checked = Vec::new();
    for image in [
        "data/images/pulse-psp-eu.chd",
        "data/images/pure-psp-eu.chd",
    ] {
        let Some(path) = oag_testdata::exact(image) else {
            continue;
        };
        let off = load(&path, false);
        for livery in &off.liveries {
            let shell = livery.shield.as_ref().expect("the disc authors a shell");
            assert!(shell.own_shield.is_none(), "{image}: off keeps the disc's");
        }
        assert_eq!(
            off.setup.shield_palette,
            oag_render::shield::PULSE_PALETTE,
            "{image}: off keeps the disc's palette"
        );
        assert!(
            off.shield_cockpit
                .as_ref()
                .is_some_and(|m| m.own_shield.is_none()),
            "{image}: off keeps the disc's sphere"
        );

        let on = load(&path, true);
        for (own, hull) in on.liveries.iter().map(|l| (l.shield.as_ref(), &l.hull)) {
            let own = own.expect("a shell");
            assert!(
                own.own_shield.is_some_and(|inflate| inflate > 0.0),
                "{image}"
            );
            assert_eq!(
                own.vertices.len(),
                hull.vertices.len(),
                "{image}: the hull's shape"
            );
            assert!(own.textures.is_empty(), "{image}: no texture is read");
        }
        assert_eq!(on.setup.shield_palette, oag_render::shield::OWN_PALETTE);
        assert_eq!(
            on.shield_cockpit.as_ref().and_then(|m| m.own_shield),
            Some(0.0)
        );
        assert!(
            on.report
                .iter()
                .any(|l| l.contains("as our own shield shell")),
            "{image}: the report names it"
        );
        checked.push(image);
    }
    println!("PSP titles checked: {checked:?}");
}

/// HD draws the HD-style shield itself, so the setting changes nothing there.
#[test]
#[ignore = "needs HD's image"]
fn hd_ignores_the_setting() {
    let Some(path) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let on = load(&path, true);
    assert!(
        on.liveries
            .iter()
            .all(|l| l.shield.as_ref().is_none_or(|m| m.own_shield.is_none()))
    );
    assert_eq!(on.setup.shield_palette, oag_render::shield::HD_PALETTE);
}
