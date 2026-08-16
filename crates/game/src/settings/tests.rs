//! What the settings file in [`super`] is asserted to do: the defaults, the
//! `[graphics]`-to-`[display]` migration and the older files it has to keep
//! loading, and that every menu seed names a key the file has.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `settings.rs`: the tests are 207 lines, past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::*;

/// The default circuit has to be the one every capture was taken on, or a
/// fresh install races something a trace comparison cannot be run against.
///
/// Note what is *not* asserted: that the id composes into a path. It does
/// not - only the source can say which `.vex` an id loads, which is
/// `catalogue`'s job and is why `Race` carries no layout field.
#[test]
fn the_default_circuit_is_the_reference_scenarios_own() {
    let race = Race::default();
    assert_eq!(race.track, "16_Track");
    assert!(
        crate::race::DEFAULT_TRACK.contains(&race.track),
        "{} is not the circuit {} names",
        race.track,
        crate::race::DEFAULT_TRACK
    );
}

/// A settings file from a build that predates the race table must still
/// load, with the table filled in rather than the parse failing.
#[test]
fn an_older_settings_file_gains_the_race_table() {
    let settings: Settings = toml::from_str("[graphics]\nanisotropy = \"4x\"").expect("parse");
    assert_eq!(settings.race.track, "16_Track");
    assert_eq!(settings.race.class, "venom");
}

/// The pacing defaults, asserted from both directions: what a fresh
/// `Display` holds and what an empty file loads as.
#[test]
fn the_two_ways_of_getting_a_default_agree() {
    let fresh = Display::default();
    let loaded: Settings = toml::from_str("").expect("parse");
    assert_eq!(loaded.display.vsync, fresh.vsync);
    assert_eq!(loaded.display.frame_limit, fresh.frame_limit);
    assert_eq!(fresh.vsync, crate::perf::Vsync::Off);
    assert_eq!(
        fresh.frame_limit,
        crate::perf::FrameLimit::DEFAULT,
        "a fresh install should be limited, not unlimited"
    );
    // The three settings this split added, so a fresh install is the game
    // as its data authors framed it and as the renderer drew it.
    assert!(fresh.monitor.is_default());
    assert_eq!(fresh.brightness, crate::display::Brightness::NEUTRAL);
    assert_eq!(fresh.gamma, crate::display::Gamma::NEUTRAL);
    assert_eq!(
        Graphics::default().fov,
        crate::display::Fov::AUTHORED,
        "the default field of view has to be the disc's own"
    );
}

/// `vsync` was a boolean for one commit. A file written in that window has
/// to keep loading, and has to come back out as a name.
#[test]
fn a_boolean_vsync_still_loads_and_is_rewritten_as_a_name() {
    let off: Settings = toml::from_str("[display]\nvsync = false").expect("parse");
    assert_eq!(off.display.vsync, crate::perf::Vsync::Off);
    let on: Settings = toml::from_str("[display]\nvsync = true").expect("parse");
    assert_eq!(on.display.vsync, crate::perf::Vsync::On);

    let written = toml::to_string_pretty(&on).expect("serialise");
    assert!(written.contains("vsync = \"on\""), "{written}");
}

/// Parses `text` the way [`load`] does, migration included.
fn read(text: &str) -> Settings {
    let mut table: toml::Table = text.parse().expect("parse");
    migrate(&mut table);
    table.try_into().expect("deserialise")
}

/// Every file written before the split has all of these in `[graphics]`,
/// and serde would drop each one silently. A player who had set borderless
/// at 120 Hz has to still be set that way after upgrading.
#[test]
fn a_file_written_before_the_split_keeps_its_display_settings() {
    let settings = read(
        "\
[graphics]
anisotropy = \"4x\"
aspect = \"free\"
window_mode = \"borderless\"
window_size = \"1920x1080\"
render_scale = 75
vsync = \"smooth\"
frame_limit = \"120\"
perf_overlay = \"fps\"
",
    );

    // Moved.
    assert_eq!(settings.display.aspect, crate::display::Aspect::Free);
    assert_eq!(
        settings.display.window_mode,
        crate::display::WindowMode::Borderless
    );
    assert_eq!(
        settings.display.window_size,
        crate::display::Size::new(1920, 1080)
    );
    assert_eq!(settings.display.vsync, crate::perf::Vsync::Smooth);
    assert_eq!(settings.display.frame_limit.hz(), Some(120));
    // Stayed, and must not have been carried across with the rest.
    assert_eq!(settings.graphics.anisotropy, Anisotropy::X4);
    assert_eq!(settings.graphics.render_scale.percent(), 75);
    assert_eq!(settings.graphics.perf_overlay, crate::perf::Overlay::Fps);
    // Added, so they come out as their defaults rather than as an error.
    assert!(settings.display.monitor.is_default());
    assert_eq!(settings.graphics.fov, crate::display::Fov::AUTHORED);

    // And the file written back is in the new shape, with nothing left in
    // the old table to be migrated a second time.
    let written = toml::to_string_pretty(&settings).expect("serialise");
    let round_tripped = read(&written);
    assert_eq!(round_tripped.display.aspect, crate::display::Aspect::Free);
    assert_eq!(round_tripped.display.frame_limit.hz(), Some(120));
}

/// The already-migrated case, where something has put an old key back: the
/// new spelling is the one the player last saw on the menu, so it wins.
#[test]
fn a_display_table_wins_over_a_leftover_graphics_key() {
    let settings = read(
        "\
[display]
aspect = \"ps2\"

[graphics]
aspect = \"free\"
window_mode = \"borderless\"
",
    );
    assert_eq!(settings.display.aspect, crate::display::Aspect::Ps2);
    // The key that only the old table had still moves across.
    assert_eq!(
        settings.display.window_mode,
        crate::display::WindowMode::Borderless
    );
}

/// A file that never had a `[graphics]` table at all, and a file that is
/// already entirely in the new shape: both have to come through untouched.
#[test]
fn migration_leaves_a_file_that_needs_none_alone() {
    for text in [
        "",
        "[display]\naspect = \"ps2\"\n",
        "[race]\nclass = \"flash\"\n",
    ] {
        let mut table: toml::Table = text.parse().expect("parse");
        let before = table.clone();
        migrate(&mut table);
        assert_eq!(table, before, "{text:?}");
    }
}

/// A bad value in the moved key has to be reported against the key, not
/// swallowed by the migration - a typo should be visible.
#[test]
fn a_malformed_moved_key_is_still_an_error() {
    let mut table: toml::Table = "[graphics]\naspect = \"16:9\"\n".parse().expect("parse");
    migrate(&mut table);
    let error = table
        .try_into::<Settings>()
        .expect_err("16:9 is not an aspect");
    assert!(error.to_string().contains("aspect"), "{error}");
}

/// A `display` that is not a table means the parse below is about to fail
/// with the key name. Migration must leave the file alone rather than
/// emptying `[graphics]` into nowhere on the way to that message.
#[test]
fn a_display_key_that_is_not_a_table_leaves_the_file_untouched() {
    let text = "display = 3\n\n[graphics]\naspect = \"free\"\n";
    let mut table: toml::Table = text.parse().expect("parse");
    let before = table.clone();
    migrate(&mut table);
    assert_eq!(table, before, "nothing may be moved out of [graphics]");
    assert!(table.try_into::<Settings>().is_err(), "and it still fails");
}

/// Every seed names a setting the file can actually hold, which is what
/// stops a renamed field leaving a menu row showing its list's first
/// option instead of the player's own value.
#[test]
fn every_menu_seed_names_a_key_the_settings_file_has() {
    let settings = Settings::default();
    let written = toml::to_string_pretty(&settings).expect("serialise");
    let table: toml::Table = written.parse().expect("parse");

    for (setting, _) in menu_seeds(&settings, Anisotropy::default()) {
        let Some((section, key)) = setting.split_once('.') else {
            // `language` is a bare key, and only present once picked.
            assert_eq!(setting, "language");
            continue;
        };
        let holder = table
            .get(section)
            .and_then(toml::Value::as_table)
            .unwrap_or_else(|| panic!("no [{section}] table for {setting}"));
        assert!(holder.contains_key(key), "[{section}] has no {key}");
    }
}
