use super::*;

/// The remix launch and a mod page's `launch_race` with `mod.<id>.<key>` keys build the
/// same options. The fixture is a candidate list surveyed off the discs `images` names;
/// a missing disc skips the pair.
fn same_options_either_way(track_image: &str, craft_image: &str, class: &str) {
    let (Some(track_image), Some(craft_image)) = (
        oag_testdata::image(track_image),
        oag_testdata::image(craft_image),
    ) else {
        return;
    };
    let titles = oag_game::launcher::survey(&[track_image, craft_image]);
    let playable: Vec<&Candidate> = titles.iter().filter(|c| c.is_playable()).collect();
    let [track, craft] = playable[..] else {
        panic!("the survey found {} playable titles, not two", playable.len());
    };
    let track_catalogue = remix::catalogue(&track.source).expect("the track disc's catalogue");
    let craft_catalogue = remix::catalogue(&craft.source).expect("the craft disc's catalogue");
    let circuit = track_catalogue.tracks.last().map(|(t, _)| t.id.clone()).unwrap();
    let team = craft_catalogue.teams.last().unwrap().value.clone();

    let mut remix_settings = Settings::default();
    remix_settings.remix.track_title = track.title().to_string();
    remix_settings.remix.track = circuit.clone();
    remix_settings.remix.craft_title = craft.title().to_string();
    remix_settings.remix.team = team.clone();
    remix_settings.race.mode = "single".into();
    remix_settings.race.class = class.into();
    remix_settings.ai.difficulty = "skilled".into();

    let mut mod_settings = Settings::default();
    for (key, value) in [
        ("track_title", track.title()),
        ("track", circuit.as_str()),
        ("craft_title", craft.title()),
        ("team", team.as_str()),
        ("mode", "single"),
        ("class", class),
        ("difficulty", "skilled"),
    ] {
        assert!(settings::mod_settings::set(
            &mut mod_settings.mod_settings,
            &format!("mod.oag.test.{key}"),
            value
        ));
    }
    let mod_keys = menu::LaunchInputs {
        track_title: Some("mod.oag.test.track_title".into()),
        track: Some("mod.oag.test.track".into()),
        craft_title: Some("mod.oag.test.craft_title".into()),
        team: Some("mod.oag.test.team".into()),
        variant: Some("mod.oag.test.variant".into()),
        mode: Some("mod.oag.test.mode".into()),
        class: Some("mod.oag.test.class".into()),
        difficulty: Some("mod.oag.test.difficulty".into()),
    };

    let from_remix = read_inputs(&remix_settings, &menu::LaunchInputs::remix()).unwrap();
    let from_mod = read_inputs(&mod_settings, &mod_keys).unwrap();
    assert_eq!(from_remix, from_mod, "the same values, read by key");

    let mut remix_options = Options::default();
    assert!(apply_two_source(&titles, &mut remix_options, &from_remix, None));
    let mut mod_options = Options::default();
    assert!(apply_two_source(&titles, &mut mod_options, &from_mod, None));
    assert_eq!(format!("{remix_options:?}"), format!("{mod_options:?}"));
    assert_eq!(remix_options.source, track.source, "the circuit's disc");
    assert_eq!(
        remix_options.craft_source.as_deref(),
        Some(craft.source.as_str()),
        "the craft's disc"
    );
    assert_eq!(
        remix_options.track,
        track_catalogue.track(&circuit).map(catalogue::Track::entry_name),
        "the circuit named by key"
    );
}

#[test]
#[ignore = "needs the Pulse and Pure disc images in data/images/"]
fn a_pulse_craft_on_a_pure_track_launches_the_same_by_remix_or_mod_keys() {
    same_options_either_way("pure-psp-eu.chd", "pulse-psp-eu.chd", "venom");
}

#[test]
#[ignore = "needs the Pulse and HD/Fury disc images in data/images/"]
fn an_hd_craft_on_a_pulse_track_launches_the_same_by_remix_or_mod_keys() {
    same_options_either_way("pulse-psp-eu.chd", "hdfury-ps3-eu-dec.iso", "venom");
}

#[test]
fn a_key_no_setting_stores_is_refused_by_name() {
    let keys = menu::LaunchInputs {
        track: Some("graphics.vsync".into()),
        ..menu::LaunchInputs::default()
    };
    let why = read_inputs(&Settings::default(), &keys).unwrap_err();
    assert!(why.contains("graphics.vsync"), "{why}");
}

#[test]
fn a_mod_key_nothing_stored_yet_reads_empty() {
    let keys = menu::LaunchInputs {
        track: Some("mod.oag.test.track".into()),
        ..menu::LaunchInputs::default()
    };
    let inputs = read_inputs(&Settings::default(), &keys).unwrap();
    assert_eq!(inputs.track, "");
}
