//! What `oag_hud::runtime` finds to work on in **Wipeout HD / Fury's own
//! composed HUD layouts**: which `DamageBar` is the shield fill, and whether
//! the lap and place arcs it shows are authored where it looks for them.
//!
//! **`#[ignore]`d and never run in CI** - it needs the disc. See
//! `crates/raceplay/tests/hd_hud_ground_truth.rs`, whose composition this reuses.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-hud --run-ignored all \
//!     -E 'test(/^hd_hud_runtime_ground_truth::/)'
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
/// `arcade_hud.xml` composes two `DamageBar`s (the hexagon's own and a
/// pickup-bar piece), and `runtime::shield_fill` takes the one sharing its
/// background's texture, preferring the same source rectangle. Eliminator's,
/// Detonator's and Duel's one `DamageBar` is another rectangle of the same
/// sheet: Detonator's is the score box's white stripes. Measured 2026-09-23.
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

/// The stripes the layout authors are where the fill goes: one `DamageBar` on
/// its background's sheet, at another rectangle, in the three layouts whose
/// bar is not their background's own image.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_striped_fill_shares_its_background_s_sheet() {
    let Some(mut archives) = open() else {
        return;
    };
    let shield = oag_hd::hud::RUNTIME.shield;
    for (root, uv) in [
        ("/data/xml/elimination_hud.xml", [0.0, 135.0, 344.0, 115.0]),
        ("/data/xml/detonator_hud.xml", [0.0, 790.0, 553.0, 200.0]),
        (
            "/data/xml/duel_hud/duel_hud.xml",
            [0.0, 728.0, 480.0, 200.0],
        ),
    ] {
        let layout = composed(&mut archives, root).layout;
        let background = layout.sprite(shield.background).expect("a hexagon");
        let fills: Vec<&hud::Sprite> = layout
            .sprites
            .iter()
            .filter(|s| s.name == shield.fill && s.src == background.src)
            .collect();
        assert_eq!(fills.len(), 1, "{root}");
        assert_eq!(fills[0].uv, uv, "{root}");
    }
}

/// `Detonator_HUD.xml` scales its score and stage on two axes: the score is
/// narrower than it is tall, the stage 0.8 on both.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn detonator_s_text_scales_are_read_per_axis() {
    let Some(mut archives) = open() else {
        return;
    };
    let layout = composed(&mut archives, "/data/xml/detonator_hud.xml").layout;
    let scales = |name: &str| {
        let label = layout.label(name).expect("authored");
        (label.scale * label.stretch, label.scale)
    };
    for (name, expected) in [
        ("DetonatorScore", (0.9, 1.0)),
        ("DetonatorStage", (0.8, 0.8)),
        ("PulseBullets", (0.9, 0.9)),
    ] {
        let (x, y) = scales(name);
        assert!(
            (x - expected.0).abs() < 1e-6 && (y - expected.1).abs() < 1e-6,
            "{name}: {x} x {y}"
        );
    }
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

/// The proximity indicator's two widgets. In `arcade_hud.xml` they land where the running
/// original draws them (`docs/ghidra/functions/ps3-hdfury-eu/hud-readouts.md`, "The
/// proximity indicator"): the arc over x 627..1293, y 849..963, and the marker, 20 above
/// its authored place, over x 907..1014, y 952..1021. `elimination_hud.xml` authors a
/// shorter arc of its own, not seen on screen. No other default-skin root authors them.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_proximity_indicator_is_where_the_original_draws_it() {
    let Some(mut archives) = open() else {
        return;
    };
    let proximity = oag_hd::hud::RUNTIME.proximity.expect("HD's rule");
    let mut measured = Vec::new();
    for &root in DEFAULT_SKIN {
        let layout = composed(&mut archives, root).layout;
        let (Some(bar), Some(marker)) = (
            layout.sprite(proximity.bar),
            layout.sprite(proximity.background),
        ) else {
            continue;
        };
        let mut raised = marker.rect;
        raised[1] -= f32::from(proximity.lift);
        measured.push((root, bar.rect.map(f32::round), raised.map(f32::round)));
    }
    assert_eq!(
        measured,
        [
            (
                "/data/xml/arcade_hud.xml",
                [627.0, 849.0, 666.0, 114.0],
                [907.0, 952.0, 107.0, 69.0],
            ),
            (
                "/data/xml/elimination_hud.xml",
                [627.0, 831.0, 666.0, 51.0],
                [871.0, 902.0, 179.0, 69.0],
            ),
        ]
    );
}

/// Pilot Assist's indicator: which of the default skin's layouts author the six
/// widgets `oag_hd::hud::ART.assist` names. A layout without them draws no
/// indicator, which is the original's behaviour for that mode.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_assist_indicator_is_authored_where_a_race_can_use_it() {
    let Some(mut archives) = open() else {
        return;
    };
    let assist = oag_hd::hud::ART.assist.expect("HD authors an indicator");
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
