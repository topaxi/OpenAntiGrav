//! The MODS page's switches and a mod's saved picks, as the settings file keeps them.

use super::tests::read;
use super::*;

#[test]
fn both_mod_switch_sets_round_trip() {
    let mods = read("[mods]\nenabled = [\"a.on\"]\ndisabled = [\"a.off\"]\n").mods;
    assert!(mods.enabled.contains("a.on") && mods.disabled.contains("a.off"));
    let settings = Settings { mods, ..fresh() };
    let written = toml::to_string_pretty(&settings).expect("serialise");
    assert_eq!(read(&written).mods, settings.mods, "{written}");
}

#[test]
fn the_mods_turned_off_round_trip_and_a_fresh_file_turns_none_off() {
    assert!(
        fresh().mods.disabled.is_empty(),
        "a mod installed later is on"
    );
    let settings = read("[mods]\ndisabled = [\"oag.hillside\", \"a.b\"]\n");
    let ids: Vec<&str> = settings.mods.disabled.iter().map(String::as_str).collect();
    assert_eq!(
        ids,
        ["a.b", "oag.hillside"],
        "kept sorted, for a stable rewrite"
    );
    let written = toml::to_string_pretty(&settings).expect("serialise");
    assert_eq!(read(&written).mods, settings.mods, "{written}");
    assert!(
        read("").mods.disabled.is_empty(),
        "an old file without [mods]"
    );
}

#[test]
fn a_mods_saved_picks_round_trip_and_survive_the_mod_being_turned_off() {
    use super::mod_settings;
    let mut settings = fresh();
    assert!(settings.mod_settings.is_empty());
    assert!(
        !toml::to_string_pretty(&settings)
            .expect("serialise")
            .contains("mod_settings")
    );

    assert!(mod_settings::set(
        &mut settings.mod_settings,
        "mod.oag.example.track",
        "Vostok"
    ));
    assert!(mod_settings::set(
        &mut settings.mod_settings,
        "mod.oag.example.team",
        "Feisar"
    ));
    assert!(!mod_settings::set(
        &mut settings.mod_settings,
        "race.track",
        "x"
    ));
    assert!(!mod_settings::set(
        &mut settings.mod_settings,
        "mod.nokey",
        "x"
    ));

    let written = toml::to_string_pretty(&settings).expect("serialise");
    assert!(
        written.contains("[mod_settings.\"oag.example\"]"),
        "{written}"
    );
    let back = read(&written);
    assert_eq!(back.mod_settings["oag.example"]["track"], "Vostok");
    assert_eq!(back.mod_settings, settings.mod_settings);

    let mut off = back;
    off.mods.disabled.insert("oag.example".into());
    let again = read(&toml::to_string_pretty(&off).expect("serialise"));
    assert!(again.mods.disabled.contains("oag.example"));
    assert_eq!(
        again.mod_settings["oag.example"]["team"], "Feisar",
        "kept while off"
    );
    let mut on = again;
    on.mods.disabled.clear();
    let seeds = mod_settings::seeds(&on.mod_settings);
    assert!(
        seeds.iter().any(|(k, v)| k == "mod.oag.example.track"
            && *v == oag_ui::menu::Value::Text("Vostok".into()))
    );
}
