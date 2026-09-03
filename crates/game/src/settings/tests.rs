//! What the settings file in [`super`] is asserted to do: the defaults, the
//! `[graphics]`-to-`[display]` migration and the older files it has to keep
//! loading, and that every menu seed names a key the file has.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `settings.rs`: the tests are 207 lines, past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::render_profile::{KNOWN_TITLES, MOVED_TO_RENDER_PROFILES, PROFILE_KEYS};
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
    migrate_render_profiles(&mut table);
    // The same order `load` runs them in, and this helper has to keep
    // mirroring it: a migration added to `load` and not to here is one every
    // test below silently stops covering.
    migrate_reconstruction_keys(&mut table);
    let mut settings: Settings = table.try_into().expect("deserialise");
    ensure_known_titles(&mut settings);
    settings
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
    // Stayed in `[graphics]`, and must not have been carried across with the
    // rest.
    assert_eq!(settings.graphics.anisotropy, Anisotropy::X4);
    assert_eq!(settings.graphics.perf_overlay, crate::perf::Overlay::Fps);
    // Moved a second time, out of `[graphics]` into every known title's own
    // render profile - see `a_file_written_before_the_render_profile_split_
    // seeds_every_known_title` for that migration on its own.
    for title in KNOWN_TITLES {
        assert_eq!(
            settings.render_profiles[*title].render_scale.percent(),
            75,
            "{title}"
        );
    }
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
///
/// The five relocated `graphics.*` rows are checked against every known
/// title's own `[render_profiles.<title>]` table rather than `[graphics]`
/// itself, since that is where `menu_seeds` actually reads and
/// `ensure_known_titles` actually writes them - see [`RenderProfile`].
#[test]
fn every_menu_seed_names_a_key_the_settings_file_has() {
    let mut settings = Settings::default();
    ensure_known_titles(&mut settings);
    let written = toml::to_string_pretty(&settings).expect("serialise");
    let table: toml::Table = written.parse().expect("parse");

    for (setting, _) in menu_seeds(&settings, Anisotropy::default(), oag_pulse::TITLE.name) {
        let Some((section, key)) = setting.split_once('.') else {
            // `language` is a bare key, and only present once picked.
            assert_eq!(setting, "language");
            continue;
        };
        if section == "graphics" && PROFILE_KEYS.contains(&key) {
            let profiles = table
                .get("render_profiles")
                .and_then(toml::Value::as_table)
                .unwrap_or_else(|| panic!("no [render_profiles] table for {setting}"));
            for title in KNOWN_TITLES {
                let profile = profiles
                    .get(*title)
                    .and_then(toml::Value::as_table)
                    .unwrap_or_else(|| panic!("no [render_profiles.{title}] table for {setting}"));
                assert!(
                    profile.contains_key(key),
                    "[render_profiles.{title}] has no {key}"
                );
            }
            continue;
        }
        let holder = table
            .get(section)
            .and_then(toml::Value::as_table)
            .unwrap_or_else(|| panic!("no [{section}] table for {setting}"));
        assert!(holder.contains_key(key), "[{section}] has no {key}");
    }
}

/// A file from before the render-profile split has one flat value per key
/// with no way to say which title it was tuned against, so migration has to
/// seed *every* known title's entry with it rather than guess one - see
/// [`migrate_render_profiles`]'s own doc comment for the rule.
#[test]
fn a_file_written_before_the_render_profile_split_seeds_every_known_title() {
    let settings = read(
        "\
[graphics]
anisotropy = \"4x\"
render_scale = 50
upscaler = \"fsr1\"
anti_aliasing = \"smaa\"
motion_blur = \"medium\"
",
    );

    // Stayed in `[graphics]`: this key was never part of the render-profile
    // split.
    assert_eq!(settings.graphics.anisotropy, Anisotropy::X4);

    // Moved, and identically, into every known title - not just one.
    for title in KNOWN_TITLES {
        let profile = &settings.render_profiles[*title];
        assert_eq!(profile.render_scale.percent(), 50, "{title}");
        // **Folded onto one axis by `migrate_reconstruction`**, which is the
        // lossy half of the ADR-0041 migration: the old file asked for `smaa`
        // *and* `fsr1`, a pairing the new row cannot express and one the menus
        // already warned about as `FIGHTS THE UPSCALER'S EDGE-ADAPTIVE
        // RESAMPLE`. The upscaler wins because it was what carried the frame.
        assert_eq!(
            profile.reconstruction,
            crate::display::Reconstruction::Fsr1,
            "{title}"
        );
        assert_eq!(profile.msaa, crate::display::Msaa::Off, "{title}");
        assert_eq!(
            profile.motion_blur,
            crate::display::MotionBlur::Medium,
            "{title}"
        );
    }

    // The file written back is in the new shape, with nothing left in
    // `[graphics]` to be migrated a second time.
    let written = toml::to_string_pretty(&settings).expect("serialise");
    let round_tripped = read(&written);
    for title in KNOWN_TITLES {
        assert_eq!(
            round_tripped.render_profiles[*title].render_scale.percent(),
            50,
            "{title}"
        );
    }
}

/// The property the whole split exists for: two titles asking for the same
/// row get back each their *own* stored value, not one shared or averaged
/// answer - proven at the boundary `menu_seeds` resolves through, without
/// needing a live `Session`.
#[test]
fn menu_seeds_resolves_the_render_profile_by_title() {
    let mut settings = Settings::default();
    ensure_known_titles(&mut settings);
    settings
        .render_profiles
        .get_mut(oag_pulse::TITLE.name)
        .expect("seeded above")
        .render_scale = "50".parse().expect("valid scale");
    settings
        .render_profiles
        .get_mut(oag_hd::TITLE.name)
        .expect("seeded above")
        .render_scale = "100".parse().expect("valid scale");

    let value_for = |title: &str| -> String {
        menu_seeds(&settings, Anisotropy::default(), title)
            .into_iter()
            .find(|(key, _)| *key == "graphics.render_scale")
            .map(|(_, value)| value.to_string())
            .expect("graphics.render_scale is always seeded")
    };

    assert_eq!(value_for(oag_pulse::TITLE.name), "50");
    assert_eq!(value_for(oag_hd::TITLE.name), "100");
}

/// Every migratable key either is a profile key or is consumed on the way in.
///
/// The two lists mean different things - "lives in the profile" against "used
/// to live flat in `[graphics]` and has to be moved" - and a key added to
/// `RenderProfile` after the split has no flat past, so putting it in the
/// migration list would have `migrate_render_profiles` looking in `[graphics]`
/// for something that was never written there.
///
/// **The third case arrived with ADR-0041**: `upscaler` and `anti_aliasing`
/// still have to be *moved* out of a pre-split `[graphics]`, and are then
/// folded into `reconstruction` and `msaa` by `migrate_reconstruction`, so
/// they are migratable without being profile keys. Listing them here is what
/// keeps that from being a way to sneak a key past the invariant: a migratable
/// key that is neither a field nor consumed is a value read out of a file and
/// dropped on the floor.
#[test]
fn every_migratable_key_is_a_profile_key() {
    // Consumed by `migrate_reconstruction` rather than deserialised.
    const FOLDED: [&str; 2] = ["upscaler", "anti_aliasing"];
    for key in MOVED_TO_RENDER_PROFILES {
        assert!(
            PROFILE_KEYS.contains(&key) || FOLDED.contains(&key),
            "{key} migrates into a profile field that does not exist"
        );
    }
    assert!(PROFILE_KEYS.len() >= MOVED_TO_RENDER_PROFILES.len());
}

/// ADR-0041's fold, on the shape a file written *after* the render-profile
/// split actually has: the two old keys already inside a profile table.
///
/// The existing migration test covers the other route - a pre-split flat
/// `[graphics]`, moved into every profile and then folded - and the two are
/// different code paths through `load`, which is why both are pinned. This is
/// the one a player upgrading today takes.
#[test]
fn the_reconstruction_fold_reads_a_profile_that_was_already_split() {
    let settings = read(
        "\
[render_profiles.\"Wipeout HD\"]
render_scale = 150
target_fps = \"120\"
minimum_resolution = 50
upscaler = \"fsr3\"
upscale_sharpness = \"0.2\"
anti_aliasing = \"msaa4x\"
motion_blur = \"high\"
",
    );
    let profile = &settings.render_profiles[oag_hd::TITLE.name];
    // The lossless case: the two really were orthogonal, so both survive.
    assert_eq!(profile.reconstruction, crate::display::Reconstruction::Fsr3);
    assert_eq!(profile.msaa, crate::display::Msaa::X4);
    // Untouched keys are still untouched - the fold removes two and inserts
    // two, and a migration that also reset a neighbour would be invisible here
    // without this.
    assert_eq!(profile.render_scale.percent(), 150);
    assert_eq!(profile.target_fps.hz(), Some(120));
    assert_eq!(profile.motion_blur, crate::display::MotionBlur::High);

    // The lossy case, argued in `migrate_reconstruction`: a pairing the new
    // axis cannot express, and one the menus already warned about. The
    // upscaler wins because it was what carried the frame onto the surface.
    let settings = read(
        "\
[render_profiles.\"Wipeout HD\"]
upscaler = \"fsr1\"
anti_aliasing = \"fxaa\"
",
    );
    let profile = &settings.render_profiles[oag_hd::TITLE.name];
    assert_eq!(profile.reconstruction, crate::display::Reconstruction::Fsr1);
    assert_eq!(profile.msaa, crate::display::Msaa::Off);

    // No upscaler, so the spatial pass was what resolved the frame and it is
    // what carries over.
    let settings = read(
        "\
[render_profiles.\"Wipeout HD\"]
anti_aliasing = \"smaa\"
",
    );
    let profile = &settings.render_profiles[oag_hd::TITLE.name];
    assert_eq!(profile.reconstruction, crate::display::Reconstruction::Smaa);
    assert_eq!(profile.msaa, crate::display::Msaa::Off);

    // A file already on this side of the split is left entirely alone, which
    // is what stops the fold running twice and resetting `msaa` to off on the
    // second launch.
    let settings = read(
        "\
[render_profiles.\"Wipeout HD\"]
reconstruction = \"fsr3\"
msaa = \"4x\"
",
    );
    let profile = &settings.render_profiles[oag_hd::TITLE.name];
    assert_eq!(profile.reconstruction, crate::display::Reconstruction::Fsr3);
    assert_eq!(profile.msaa, crate::display::Msaa::X4);
}
