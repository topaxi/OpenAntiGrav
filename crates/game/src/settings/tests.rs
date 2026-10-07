//! What the settings file in [`super`] is asserted to do: the defaults, the
//! `[graphics]`-to-`[display]` migration and the older files it has to keep
//! loading, and that every menu seed names a key the file has.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `settings.rs`: the tests are 207 lines, past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::render_profile::{MOVED_TO_RENDER_PROFILES, PROFILE_KEYS, known_profiles};
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
        oag_raceplay::DEFAULT_TRACK.contains(&race.track),
        "{} is not the circuit {} names",
        race.track,
        oag_raceplay::DEFAULT_TRACK
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

/// `[graphics] lod` was removed on 2026-09-23 (`docs/formats/vex.md`'s
/// `LodGroup` section says why), and every file written before then carries it - either
/// spelling must still load, and the next save must drop the key.
#[test]
fn a_leftover_lod_key_loads_and_is_not_written_back() {
    for value in ["both", "single"] {
        let settings: Settings = toml::from_str(&format!("[graphics]\nlod = \"{value}\""))
            .expect("a file with the removed key still parses");
        let written = toml::to_string(&settings).expect("serialise");
        assert!(
            !written.contains("lod ="),
            "the removed key came back on save:\n{written}"
        );
    }
}

/// The pacing defaults, asserted from both directions: what a fresh
/// `Display` holds and what an empty file loads as.
#[test]
fn the_two_ways_of_getting_a_default_agree() {
    let fresh = Display::default();
    let loaded: Settings = toml::from_str("").expect("parse");
    assert_eq!(loaded.display.vsync, fresh.vsync);
    assert_eq!(loaded.display.frame_limit, fresh.frame_limit);
    assert_eq!(fresh.vsync, oag_present::perf::Vsync::Off);
    assert_eq!(
        fresh.frame_limit,
        oag_present::perf::FrameLimit::DEFAULT,
        "a fresh install should be limited, not unlimited"
    );
    // The three settings this split added, so a fresh install is the game
    // as its data authors framed it and as the renderer drew it.
    assert!(fresh.monitor.is_default());
    assert_eq!(fresh.brightness, oag_display::display::Brightness::NEUTRAL);
    assert_eq!(fresh.gamma, oag_display::display::Gamma::NEUTRAL);
    assert_eq!(
        Graphics::default().fov,
        oag_display::display::Fov::AUTHORED,
        "the default field of view has to be the disc's own"
    );
}

/// `vsync` was a boolean for one commit. A file written in that window has
/// to keep loading, and has to come back out as a name.
#[test]
fn a_boolean_vsync_still_loads_and_is_rewritten_as_a_name() {
    let off: Settings = toml::from_str("[display]\nvsync = false").expect("parse");
    assert_eq!(off.display.vsync, oag_present::perf::Vsync::Off);
    let on: Settings = toml::from_str("[display]\nvsync = true").expect("parse");
    assert_eq!(on.display.vsync, oag_present::perf::Vsync::On);

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
    migrate_platform_split(&mut table);
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
    assert_eq!(settings.display.aspect, oag_display::display::Aspect::Free);
    assert_eq!(
        settings.display.window_mode,
        oag_display::display::WindowMode::Borderless
    );
    assert_eq!(
        settings.display.window_size,
        oag_display::display::Size::new(1920, 1080)
    );
    assert_eq!(settings.display.vsync, oag_present::perf::Vsync::Smooth);
    assert_eq!(settings.display.frame_limit.hz(), Some(120));
    // Stayed in `[graphics]`, and must not have been carried across with the
    // rest.
    assert_eq!(settings.graphics.anisotropy, Anisotropy::X4);
    assert_eq!(
        settings.graphics.perf_overlay,
        oag_present::perf::Overlay::Fps
    );
    // Moved a second time, out of `[graphics]` into every known
    // (title, platform) pair's own render profile - see
    // `a_file_written_before_the_render_profile_split_seeds_every_known_title`
    // for that migration on its own.
    for (title, platform) in known_profiles() {
        let key = profile_key(title, platform);
        assert_eq!(
            settings.render_profiles[&key].render_scale.percent(),
            75,
            "{key}"
        );
    }
    // Added, so they come out as their defaults rather than as an error.
    assert!(settings.display.monitor.is_default());
    assert_eq!(settings.graphics.fov, oag_display::display::Fov::AUTHORED);

    // And the file written back is in the new shape, with nothing left in
    // the old table to be migrated a second time.
    let written = toml::to_string_pretty(&settings).expect("serialise");
    let round_tripped = read(&written);
    assert_eq!(
        round_tripped.display.aspect,
        oag_display::display::Aspect::Free
    );
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
    assert_eq!(settings.display.aspect, oag_display::display::Aspect::Ps2);
    // The key that only the old table had still moves across.
    assert_eq!(
        settings.display.window_mode,
        oag_display::display::WindowMode::Borderless
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
///
/// The example used to be `16:9`, which stopped being malformed when `Aspect`
/// grew a free-form `w:h` shape. `21:nine` is the same test with a value that
/// is still wrong: it has the separator and fails on the number, so it proves
/// the error comes from parsing the aspect rather than from the key's shape.
#[test]
fn a_malformed_moved_key_is_still_an_error() {
    let mut table: toml::Table = "[graphics]\naspect = \"21:nine\"\n".parse().expect("parse");
    migrate(&mut table);
    let error = table
        .try_into::<Settings>()
        .expect_err("21:nine is not an aspect");
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

    for (setting, _) in menu_seeds(
        &settings,
        Anisotropy::default(),
        oag_pulse::TITLE,
        oag_disc::Platform::Psp,
    ) {
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
            for (title, platform) in known_profiles() {
                let profile_key = profile_key(title, platform);
                let profile = profiles
                    .get(&profile_key)
                    .and_then(toml::Value::as_table)
                    .unwrap_or_else(|| {
                        panic!("no [render_profiles.{profile_key}] table for {setting}")
                    });
                assert!(
                    profile.contains_key(key),
                    "[render_profiles.{profile_key}] has no {key}"
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

    // Moved, and identically, into every known (title, platform) row - not
    // just one, and not just one platform of a title that ships two.
    for (title, platform) in known_profiles() {
        let key = profile_key(title, platform);
        let profile = &settings.render_profiles[&key];
        assert_eq!(profile.render_scale.percent(), 50, "{key}");
        // **Folded onto one axis by `migrate_reconstruction`**, which is the
        // lossy half of the ADR-0041 migration: the old file asked for `smaa`
        // *and* `fsr1`, a pairing the new row cannot express and one the menus
        // already warned about as `FIGHTS THE UPSCALER'S EDGE-ADAPTIVE
        // RESAMPLE`. The upscaler wins because it was what carried the frame.
        assert_eq!(
            profile.reconstruction,
            oag_display::display::Reconstruction::Fsr1,
            "{key}"
        );
        assert_eq!(profile.msaa, oag_display::display::Msaa::Off, "{key}");
        assert_eq!(
            profile.motion_blur,
            oag_display::display::MotionBlur::Medium,
            "{key}"
        );
    }

    // The file written back is in the new shape, with nothing left in
    // `[graphics]` to be migrated a second time.
    let written = toml::to_string_pretty(&settings).expect("serialise");
    let round_tripped = read(&written);
    for (title, platform) in known_profiles() {
        let key = profile_key(title, platform);
        assert_eq!(
            round_tripped.render_profiles[&key].render_scale.percent(),
            50,
            "{key}"
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
        .get_mut(&profile_key(oag_pulse::TITLE, oag_disc::Platform::Psp))
        .expect("seeded above")
        .render_scale = "50".parse().expect("valid scale");
    settings
        .render_profiles
        .get_mut(&profile_key(oag_hd::TITLE, oag_disc::Platform::Ps3))
        .expect("seeded above")
        .render_scale = "100".parse().expect("valid scale");

    let value_for = |title: &oag_title::Title, platform: oag_disc::Platform| -> String {
        menu_seeds(&settings, Anisotropy::default(), title, platform)
            .into_iter()
            .find(|(key, _)| *key == "graphics.render_scale")
            .map(|(_, value)| value.to_string())
            .expect("graphics.render_scale is always seeded")
    };

    assert_eq!(value_for(oag_pulse::TITLE, oag_disc::Platform::Psp), "50");
    assert_eq!(value_for(oag_hd::TITLE, oag_disc::Platform::Ps3), "100");
}

/// The other half of the same property: the **same title**, opened off its
/// two different platforms, also reads back two different rows rather than
/// one shared one - the split this whole thread exists for.
#[test]
fn menu_seeds_resolves_pulse_psp_and_pulse_ps2_separately() {
    let mut settings = Settings::default();
    ensure_known_titles(&mut settings);
    settings
        .render_profiles
        .get_mut(&profile_key(oag_pulse::TITLE, oag_disc::Platform::Psp))
        .expect("seeded above")
        .render_scale = "50".parse().expect("valid scale");
    settings
        .render_profiles
        .get_mut(&profile_key(oag_pulse::TITLE, oag_disc::Platform::Ps2))
        .expect("seeded above")
        .render_scale = "100".parse().expect("valid scale");

    let value_for = |platform: oag_disc::Platform| -> String {
        menu_seeds(&settings, Anisotropy::default(), oag_pulse::TITLE, platform)
            .into_iter()
            .find(|(key, _)| *key == "graphics.render_scale")
            .map(|(_, value)| value.to_string())
            .expect("graphics.render_scale is always seeded")
    };

    assert_eq!(value_for(oag_disc::Platform::Psp), "50");
    assert_eq!(value_for(oag_disc::Platform::Ps2), "100");
}

/// The migration this whole split exists for: a file that still has a bare
/// `[render_profiles."Wipeout Pulse"]` table, tuned before Pulse's PSP and
/// PS2 rows were split apart, has no way to say which platform the value was
/// tuned against - so both of Pulse's rows come back seeded with it, on the
/// "absence is not evidence of wrong" rule `migrate_platform_split`'s own doc
/// names, and the bare key itself is gone once split.
#[test]
fn a_bare_title_key_seeds_both_platform_rows_and_is_removed() {
    let settings = read(
        "\
[render_profiles.\"Wipeout Pulse\"]
render_scale = 60
",
    );
    let psp_key = profile_key(oag_pulse::TITLE, oag_disc::Platform::Psp);
    let ps2_key = profile_key(oag_pulse::TITLE, oag_disc::Platform::Ps2);
    assert_eq!(
        settings.render_profiles[&psp_key].render_scale.percent(),
        60
    );
    assert_eq!(
        settings.render_profiles[&ps2_key].render_scale.percent(),
        60
    );

    // Gone, not left beside the two rows it seeded - see
    // `migrate_platform_split`'s own doc for why a leftover would round-trip
    // forever as a row nothing reads.
    let written = toml::to_string_pretty(&settings).expect("serialise");
    assert!(
        !written.contains("[render_profiles.\"Wipeout Pulse\"]\n"),
        "the bare key survived the split:\n{written}"
    );
}

/// A platform row a player already tuned must not be clobbered by the bare
/// key's value sitting beside it - the same "new spelling wins" rule
/// `a_display_table_wins_over_a_leftover_graphics_key` proves for `[display]`.
#[test]
fn a_platform_row_already_present_wins_over_the_bare_key() {
    let settings = read(
        "\
[render_profiles.\"Wipeout Pulse\"]
render_scale = 60

[render_profiles.\"Wipeout Pulse (PS2)\"]
render_scale = 100
",
    );
    assert_eq!(
        settings.render_profiles[&profile_key(oag_pulse::TITLE, oag_disc::Platform::Ps2)]
            .render_scale
            .percent(),
        100,
        "the tuned PS2 row must survive, not the bare key's value"
    );
    assert_eq!(
        settings.render_profiles[&profile_key(oag_pulse::TITLE, oag_disc::Platform::Psp)]
            .render_scale
            .percent(),
        60,
        "the PSP row had nothing of its own, so it takes the bare key's value"
    );
}

/// [`load`]'s own contract: a file already in the canonical shape reads and
/// writes back byte-identical, so a player who never changes a setting never
/// sees a diff on disk. Proven across the platform-split migration
/// specifically, since that is the one [`read`] (this file's helper) has to
/// keep in step with `load` for - see its own doc comment.
#[test]
fn a_settings_file_is_stable_across_a_second_load() {
    // `read("")` is what a fresh install's first load produces - every table
    // filled in, `render_profiles` included via `ensure_known_titles` - which
    // is the canonical shape a second load has to reproduce exactly. A bare
    // `Settings::default()` is not that: it skips `ensure_known_titles`
    // entirely, the way `load` never does once a file exists.
    let once = toml::to_string_pretty(&read("")).expect("serialise");
    let twice = toml::to_string_pretty(&read(&once)).expect("serialise");
    assert_eq!(
        once, twice,
        "a canonical file must not move on a second load"
    );
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
    let profile = &settings.render_profiles[&profile_key(oag_hd::TITLE, oag_disc::Platform::Ps3)];
    // The lossless case: the two really were orthogonal, so both survive.
    assert_eq!(
        profile.reconstruction,
        oag_display::display::Reconstruction::Fsr3
    );
    assert_eq!(profile.msaa, oag_display::display::Msaa::X4);
    // Untouched keys are still untouched - the fold removes two and inserts
    // two, and a migration that also reset a neighbour would be invisible here
    // without this.
    assert_eq!(profile.render_scale.percent(), 150);
    assert_eq!(profile.target_fps.hz(), Some(120));
    assert_eq!(profile.motion_blur, oag_display::display::MotionBlur::High);

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
    let profile = &settings.render_profiles[&profile_key(oag_hd::TITLE, oag_disc::Platform::Ps3)];
    assert_eq!(
        profile.reconstruction,
        oag_display::display::Reconstruction::Fsr1
    );
    assert_eq!(profile.msaa, oag_display::display::Msaa::Off);

    // No upscaler, so the spatial pass was what resolved the frame and it is
    // what carries over.
    let settings = read(
        "\
[render_profiles.\"Wipeout HD\"]
anti_aliasing = \"smaa\"
",
    );
    let profile = &settings.render_profiles[&profile_key(oag_hd::TITLE, oag_disc::Platform::Ps3)];
    assert_eq!(
        profile.reconstruction,
        oag_display::display::Reconstruction::Smaa
    );
    assert_eq!(profile.msaa, oag_display::display::Msaa::Off);

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
    let profile = &settings.render_profiles[&profile_key(oag_hd::TITLE, oag_disc::Platform::Ps3)];
    assert_eq!(
        profile.reconstruction,
        oag_display::display::Reconstruction::Fsr3
    );
    assert_eq!(profile.msaa, oag_display::display::Msaa::X4);
}

/// `Bindings`'s own tests prove it round-trips through a `BTreeMap`; this is
/// the other half, that the map itself survives a real TOML file - `settings`'s
/// actual persistence format, which nothing above exercises. `bindings` has
/// to stay `Controls`'s last field for this to parse at all: `toml` refuses
/// a scalar after a table inside the same table.
#[test]
fn controls_bindings_round_trips_through_a_real_toml_file() {
    let settings = Settings::default();
    let written = toml::to_string_pretty(&settings).expect("serialise");
    let round_tripped: Settings = toml::from_str(&written).expect("parse");
    assert_eq!(
        round_tripped.controls.bindings.len(),
        oag_input::bindings::Bindings::default().to_pairs().len(),
        "every candidate key is written, complete, every time"
    );
    assert_eq!(round_tripped.controls.bindings, settings.controls.bindings);
}

/// A Wipeout HD/Fury profile that has never picked a model opens Ship Select
/// on `concept1` (measured on RPCS3 with an empty `savedata`, 2026-09-29); a
/// stored variant, or a deliberate classic-hull pick, wins over that.
#[test]
fn hd_opens_ship_select_on_concept1_until_a_model_is_picked() {
    let mut race = Race::default();
    assert_eq!(race.opening_variant(oag_hd::TITLE), "_c1");
    assert_eq!(race.opening_variant(oag_pulse::TITLE), "");

    race.variant_chosen = true;
    assert_eq!(race.opening_variant(oag_hd::TITLE), "");

    let stored = Race {
        variant: "_n1".to_string(),
        ..Race::default()
    };
    assert_eq!(stored.opening_variant(oag_hd::TITLE), "_n1");
}

/// The KILLS row reaches a launch only in an Eliminator, and only once touched.
#[test]
fn the_kills_pick_is_an_eliminator_launch_option_only() {
    use oag_race::Mode;
    let untouched = Race::default();
    assert_eq!(untouched.eliminator_kill_target(Mode::Eliminator), None);
    let picked = Race {
        kill_target: "15".to_string(),
        ..Race::default()
    };
    assert_eq!(picked.eliminator_kill_target(Mode::Eliminator), Some(15));
    assert_eq!(picked.eliminator_kill_target(Mode::TimeTrial), None);
}

/// The WEAPONS row reaches a launch only in a single race, and only once
/// touched: every other mode's answer is the mode's own.
#[test]
fn the_weapons_pick_is_a_single_race_launch_option_only() {
    use oag_race::Mode;
    let untouched = Race::default();
    assert_eq!(untouched.weapons_override(Mode::SingleRace), None);
    let off = Race {
        weapons: "Off".to_string(),
        ..Race::default()
    };
    assert_eq!(off.weapons_override(Mode::SingleRace), Some(false));
    for mode in [
        Mode::TimeTrial,
        Mode::SpeedLap,
        Mode::Zone,
        Mode::Eliminator,
    ] {
        assert_eq!(off.weapons_override(mode), None, "{mode:?}");
    }
    let on = Race {
        weapons: "On".to_string(),
        ..Race::default()
    };
    assert_eq!(on.weapons_override(Mode::SingleRace), Some(true));
}

#[test]
fn the_hud_scale_defaults_to_sharp_bilinear_and_reads_every_spelling() {
    use oag_display::display::HudScale;
    let empty: Settings = toml::from_str("").expect("parse");
    assert_eq!(empty.graphics.hud_scale, HudScale::SharpBilinear);
    for mode in HudScale::ALL {
        let settings: Settings =
            toml::from_str(&format!("[graphics]\nhud_scale = \"{mode}\"")).expect("parse");
        assert_eq!(settings.graphics.hud_scale, mode);
    }
    assert!(toml::from_str::<Settings>("[graphics]\nhud_scale = \"crisp\"").is_err());
}

/// `file = ""` is the disable switch, so a load -> save -> load must not
/// normalise it away; an absent key must stay absent, or the resolved default
/// would be frozen into the file.
#[test]
fn the_log_section_survives_a_rewrite_as_written() {
    for (text, file, filter) in [
        ("", None, None),
        ("[log]\nfile = \"\"\n", Some(""), None),
        (
            "[log]\nfile = \"/x/y.log\"\nfilter = \"warn\"\n",
            Some("/x/y.log"),
            Some("warn"),
        ),
    ] {
        let once = read(text);
        assert_eq!(once.log.file.as_deref(), file, "{text}");
        assert_eq!(once.log.filter.as_deref(), filter, "{text}");
        let written = toml::to_string_pretty(&once).expect("serialise");
        let twice = read(&written);
        assert_eq!(twice.log.file, once.log.file, "{written}");
        assert_eq!(twice.log.filter, once.log.filter, "{written}");
        assert_eq!(written.contains("file ="), file.is_some(), "{written}");
    }
}

/// An unreadable settings file (an Android reinstall restoring the app's
/// files under another owner) must not stop the boot: defaults, no write.
#[cfg(unix)]
#[test]
fn an_unreadable_settings_file_boots_on_defaults() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("oag-settings-unreadable-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let file = dir.join("settings.toml");
    std::fs::write(&file, "[graphics]\nanisotropy = \"4x\"\n").expect("write");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    let unreadable = std::fs::read_to_string(&file).is_err();
    let loaded = load_from(&file);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).expect("chmod back");
    let after = std::fs::read_to_string(&file).expect("read back");
    std::fs::remove_dir_all(&dir).ok();
    if !unreadable {
        return; // running as root: the mode bits do not bind
    }
    assert!(
        loaded
            .expect("a permission error is not fatal")
            .graphics
            .anisotropy
            == Settings::default().graphics.anisotropy
    );
    assert!(
        after.contains("4x"),
        "the unreadable file must not be overwritten"
    );
}
