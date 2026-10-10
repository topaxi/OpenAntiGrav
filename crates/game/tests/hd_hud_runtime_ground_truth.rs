//! What `oag_hud::runtime` finds to work on in **Wipeout HD / Fury's own
//! composed HUD layouts**: which `DamageBar` is the shield fill, and whether
//! the lap and place arcs it shows are authored where it looks for them.
//!
//! **`#[ignore]`d and never run in CI** - it needs the disc. See
//! `crates/game/tests/hd_hud_ground_truth.rs`, whose composition this reuses.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(hd_hud_runtime_ground_truth)'
//! ```

use oag_hud as hud;

fn open() -> Option<oag_assets::Archives> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    oag_hd::open(&image.display().to_string()).ok()
}

fn composed(archives: &mut oag_assets::Archives, root: &str) -> hud::Composed {
    let mut read = |path: &str| archives.read_name(path).ok();
    hud::compose(root, &mut read).unwrap_or_else(|| panic!("composing {root}"))
}

/// The default skin's roots - the ones a race loads.
const DEFAULT_SKIN: &[&str] = &[
    "/data/xml/arcade_hud.xml",
    "/data/xml/elimination_hud.xml",
    "/data/xml/timetrial_hud.xml",
    "/data/xml/speedlap_hud.xml",
    "/data/xml/zone_hud.xml",
    "/data/xml/detonator_hud.xml",
    "/data/xml/duel_hud/duel_hud.xml",
];

/// `(root, DamageBar widgets, of which share DamageBarBg's source rectangle)`.
///
/// **One fill per layout that has a hexagon, whichever way the name is
/// repeated.** `arcade_hud.xml` composes two `DamageBar`s (the hexagon's own
/// and a pickup-bar piece), and `runtime::shield_fill` takes the one sharing
/// its background's texture and source rectangle. That rule has to pick
/// exactly one wherever a hexagon is drawn, and it picks none on the three
/// layouts whose `DamageBar` is a different image from its background -
/// Eliminator's, Detonator's and Duel's, where the fill is left undrawn
/// rather than guessed at. Measured 2026-09-23.
const FILLS: &[(&str, usize, usize)] = &[
    ("/data/xml/arcade_hud.xml", 2, 1),
    ("/data/xml/elimination_hud.xml", 1, 0),
    ("/data/xml/timetrial_hud.xml", 1, 1),
    ("/data/xml/speedlap_hud.xml", 1, 1),
    ("/data/xml/zone_hud.xml", 1, 1),
    ("/data/xml/detonator_hud.xml", 1, 0),
    ("/data/xml/duel_hud/duel_hud.xml", 1, 0),
];

#[test]
#[ignore = "needs a disc image in data/images/"]
fn one_damage_bar_shares_its_background_s_source_rectangle() {
    let Some(mut archives) = open() else {
        return;
    };
    let shield = oag_hd::hud::RUNTIME.shield;
    let mut measured = Vec::new();
    for &root in DEFAULT_SKIN {
        let layout = composed(&mut archives, root).layout;
        let fills: Vec<&hud::Sprite> = layout
            .sprites
            .iter()
            .filter(|s| s.name == shield.fill)
            .collect();
        let matching = layout.sprite(shield.background).map_or(0, |bg| {
            fills
                .iter()
                .filter(|s| s.src == bg.src && s.uv == bg.uv)
                .count()
        });
        measured.push((root, fills.len(), matching));
    }
    assert_eq!(measured, FILLS);
}

/// Every segment either arc names is authored, in every default-skin layout
/// that authors the arc's first segment - so a lit segment never silently
/// fails to find its widget.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_arc_segment_is_authored_where_its_arc_is() {
    let Some(mut archives) = open() else {
        return;
    };
    let runtime = oag_hd::hud::RUNTIME;
    let mut with_arcs = Vec::new();
    for &root in DEFAULT_SKIN {
        let layout = composed(&mut archives, root).layout;
        for arc in [runtime.lap_arc, runtime.place_arc] {
            if layout.sprite(&format!("{}0", arc.prefix)).is_none() {
                continue;
            }
            for k in 0..arc.segments {
                let name = format!("{}{k}", arc.prefix);
                assert!(layout.sprite(&name).is_some(), "{root}: {name}");
            }
            with_arcs.push((root, arc.prefix));
        }
    }
    assert!(
        with_arcs.contains(&("/data/xml/arcade_hud.xml", "LapBar"))
            && with_arcs.contains(&("/data/xml/arcade_hud.xml", "PosBar")),
        "the arcade layout carries both arcs: {with_arcs:?}"
    );
}

/// Pilot Assist's indicator: which of the default skin's layouts author the six
/// widgets `oag_hd::hud::RUNTIME.assist` names. A layout without them draws no
/// indicator, which is the original's behaviour for that mode.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_assist_indicator_is_authored_where_a_race_can_use_it() {
    let Some(mut archives) = open() else {
        return;
    };
    let assist = oag_hd::hud::RUNTIME
        .assist
        .expect("HD authors an indicator");
    let names: Vec<&str> = [assist.background, assist.main]
        .into_iter()
        .chain(assist.left.iter().copied())
        .chain(assist.right.iter().copied())
        .collect();
    let mut carrying = Vec::new();
    for &root in DEFAULT_SKIN {
        let layout = composed(&mut archives, root).layout;
        let found = names.iter().filter(|n| layout.sprite(n).is_some()).count();
        println!("{root}: {found} of {}", names.len());
        for name in &names {
            if let Some(sprite) = layout.sprite(name) {
                println!(
                    "    {name}: rect {:?} src {} color {:?}",
                    sprite.rect, sprite.src, sprite.color
                );
            }
        }
        if found == names.len() {
            carrying.push(root);
        }
    }
    assert!(
        carrying.contains(&"/data/xml/arcade_hud.xml"),
        "the arcade layout embeds HUD_assist_indicator.xml: {carrying:?}"
    );
}
