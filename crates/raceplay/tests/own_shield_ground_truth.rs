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
//!
//! `graphics.own_rockets` rides the same loads (a race load is the whole cost of this file):
//! [`rockets_are_the_discs_or_ours`], `docs/rendering/own-rockets.md`.

use oag_raceplay as race;

/// A race with both of our opt-in looks off, or both on.
fn load(path: &std::path::Path, own_shield: bool) -> race::Loaded {
    race::load(&race::Options {
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        own_shield,
        own_rockets: own_shield,
        ..race::Options::default()
    })
    .expect("loading the race")
}

/// The shell is around the hull and clear of it: its box holds the hull's with room on every
/// side, and none of its vertices is near any of the hull's. The bounds are fractions of the
/// stand-off because the hull's vertices here include the levels of detail and blended-only
/// meshes the shell is not measured from.
fn stands_off(shell: &oag_mesh::mesh::Model, hull: &oag_mesh::mesh::Model, image: &str) {
    let gap = oag_livery::own_shield::STAND_OFF / 2.0;
    let fixed = || hull.vertices.iter().filter(|v| v.xform == 0);
    for axis in 0..3 {
        let of = |vertices: &mut dyn Iterator<Item = f32>| {
            vertices.fold((f32::MAX, f32::MIN), |(low, high), v| {
                (low.min(v), high.max(v))
            })
        };
        let (hull_low, hull_high) = of(&mut fixed().map(|v| v.position[axis]));
        let (low, high) = of(&mut shell.vertices.iter().map(|v| v.position[axis]));
        assert!(
            low < hull_low - gap && high > hull_high + gap,
            "{image}: axis {axis} shell {low}..{high} hull {hull_low}..{hull_high}"
        );
    }
    let near = shell
        .vertices
        .iter()
        .filter(|vertex| {
            fixed().any(|v| {
                (0..3)
                    .map(|axis| (v.position[axis] - vertex.position[axis]).powi(2))
                    .sum::<f32>()
                    < gap * gap
            })
        })
        .count();
    eprintln!("{image}: {near} of {} near", shell.vertices.len());
    assert!(
        near * 50 < shell.vertices.len(),
        "{image}: {near} of {} shell vertices are on the hull",
        shell.vertices.len()
    );
}

#[test]
fn the_setting_is_off_by_default() {
    assert!(!race::Options::default().own_shield);
    assert!(!race::Options::default().own_rockets);
}

/// The Rocket's three triggers and the fallback-layer effect of ours each plays with
/// `graphics.own_rockets` on.
const OWN_ROCKET: [(oag_title::Trigger, &str); 3] = [
    (
        oag_title::Trigger::RocketFlare,
        r"Data\Psys\OAGRocketFlight.toml",
    ),
    (
        oag_title::Trigger::TrackBlast,
        r"Data\Psys\OAGRocketImpactTrack.toml",
    ),
    (
        oag_title::Trigger::CraftBlast,
        r"Data\Psys\OAGRocketImpactCraft.toml",
    ),
];

/// Off, the race holds none of our rocket effects and says nothing of them, so every rocket
/// trigger plays what the disc authors and the picture is the one it was. On, each of the
/// three triggers plays ours, whatever the disc authors there (Pure authors no track blast).
fn rockets_are_the_discs_or_ours(off: &race::Loaded, on: &race::Loaded, image: &str) {
    for (trigger, name) in OWN_ROCKET {
        assert!(
            off.setup.effects.get(name).is_none(),
            "{image}: off loaded {name}"
        );
        let ours = on
            .setup
            .effects
            .get(name)
            .unwrap_or_else(|| panic!("{image}: on did not load {name}"));
        let playing = on.setup.handles.get(trigger);
        assert!(
            playing.is_some_and(|effect| std::sync::Arc::ptr_eq(effect, ours)),
            "{image}: {trigger:?} does not play {name}"
        );
        assert!(
            off.setup
                .handles
                .get(trigger)
                .is_none_or(|effect| !std::sync::Arc::ptr_eq(effect, ours)),
            "{image}: off plays ours on {trigger:?}"
        );
        assert!(
            ours.unplaced_sprites().next().is_none(),
            "{image}: a puff sprite of {name} did not fit the sheet"
        );
    }
    let named = |loaded: &race::Loaded| loaded.report.iter().any(|l| l.contains("own rockets"));
    assert!(!named(off), "{image}: off names our rockets in the report");
    assert!(named(on), "{image}: the report names our rockets");
    assert!(
        on.report.iter().all(|l| !l.contains("left out")),
        "{image}: {:?}",
        on.report
    );
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
        "data/images/pulse-ps2-eu.chd",
    ] {
        let Some(path) = oag_testdata::exact(image) else {
            continue;
        };
        let off = load(&path, false);
        for livery in &off.liveries {
            let shell = livery.shield.as_ref().expect("the disc authors a shell");
            assert!(!shell.own_shield, "{image}: off keeps the disc's");
        }
        assert_ne!(
            off.setup.shield_palette,
            oag_render::shield::OWN_PALETTE,
            "{image}: off keeps the disc's palette"
        );
        assert!(
            off.shield_cockpit.as_ref().is_some_and(|m| !m.own_shield),
            "{image}: off keeps the disc's sphere"
        );

        let on = load(&path, true);
        for (own, hull) in on.liveries.iter().map(|l| (l.shield.as_ref(), &l.hull)) {
            let own = own.expect("a shell");
            assert!(own.own_shield, "{image}");
            stands_off(own, hull, image);
            assert!(own.textures.is_empty(), "{image}: no texture is read");
        }
        assert_eq!(on.setup.shield_palette, oag_render::shield::OWN_PALETTE);
        assert_eq!(on.shield_cockpit.as_ref().map(|m| m.own_shield), Some(true));
        assert!(
            on.report.iter().any(|l| l.contains("our own shield shell")),
            "{image}: the report names it"
        );
        rockets_are_the_discs_or_ours(&off, &on, image);
        checked.push(image);
    }
    assert!(!checked.is_empty(), "no Pulse or Pure image to check");
    println!("titles checked: {checked:?}");
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
            .all(|l| l.shield.as_ref().is_none_or(|m| !m.own_shield))
    );
    assert_eq!(on.setup.shield_palette, oag_render::shield::HD_PALETTE);
    for (_, name) in OWN_ROCKET {
        assert!(on.setup.effects.get(name).is_none(), "HD loaded {name}");
    }
    assert!(on.report.iter().all(|l| !l.contains("own rockets")));
}
