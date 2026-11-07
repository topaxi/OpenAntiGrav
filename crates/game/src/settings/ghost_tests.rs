//! The ghost rows of the Game Options page: seeded where the title lists them, default on.

use super::*;

/// The ghost rows are seeded only where the title's options list them, default to on, survive the
/// settings file, and a title without the row ignores a saved `off`.
#[test]
fn the_ghost_rows_are_seeded_where_listed_default_on_and_round_trip() {
    let seeded = |title, key: &str| {
        menu_seeds(
            &Settings::default(),
            Anisotropy::default(),
            title,
            oag_disc::Platform::Psp,
        )
        .into_iter()
        .any(|(seed, _)| seed == key)
    };
    for key in [
        "display.ghost_ship_visible",
        "display.auto_load_ghost",
        "display.multiplayer_tags",
    ] {
        assert!(seeded(oag_pulse::TITLE, key), "{key}");
        assert!(!seeded(oag_pure::TITLE, key), "{key}");
        // HD's own GAME OPTIONS lists the ghost ship's visibility and nothing else of the three.
        assert_eq!(
            seeded(oag_hd::TITLE, key),
            key == "display.ghost_ship_visible",
            "{key}"
        );
    }

    let mut settings = Settings::default();
    assert!(settings.display.ghost_ship_shown(oag_pulse::TITLE));
    assert!(settings.display.ghost_auto_loads(oag_pulse::TITLE));
    settings.display.ghost_ship_visible = false;
    settings.display.auto_load_ghost = false;
    assert!(settings.display.multiplayer_tags_shown(oag_pulse::TITLE));
    settings.display.multiplayer_tags = false;
    assert!(!settings.display.multiplayer_tags_shown(oag_pulse::TITLE));
    settings.display.multiplayer_tags = true;
    assert!(!settings.display.multiplayer_tags_shown(oag_pure::TITLE));
    assert!(!settings.display.ghost_ship_shown(oag_pulse::TITLE));
    assert!(!settings.display.ghost_auto_loads(oag_pulse::TITLE));
    assert!(settings.display.ghost_ship_shown(oag_pure::TITLE));
    assert!(settings.display.ghost_auto_loads(oag_pure::TITLE));
    assert!(
        !settings.display.ghost_ship_shown(oag_hd::TITLE),
        "HD reads the same setting"
    );
    assert!(settings.display.ghost_auto_loads(oag_hd::TITLE));
    let written = toml::to_string_pretty(&settings).expect("serialise");
    let back: Settings = toml::from_str(&written).expect("parse");
    assert!(!back.display.ghost_ship_visible && !back.display.auto_load_ghost);
    assert!(back.display.multiplayer_tags);
    let missing: Settings = toml::from_str("[display]\n").expect("parse");
    assert!(missing.display.ghost_ship_visible && missing.display.auto_load_ghost);
}
