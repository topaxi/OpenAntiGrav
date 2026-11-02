use super::*;

/// The keys the official `oag.remix` page's START row reads, off the mod's own file when the
/// submodule is checked out, else the same keys written out.
fn remix_keys() -> menu::LaunchInputs {
    let official = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/mods/official");
    if official.join("remix/oag-mod.toml").is_file() {
        let mods = oag_game::mods::Mods::load_from(&[official], &Default::default()).unwrap();
        let built = mods.build(oag_ui::menu::BUILT_IN, None, None).unwrap();
        let page = built
            .definition
            .pages
            .iter()
            .find(|page| page.id == "oag.remix/remix")
            .expect("the RACE REMIX page");
        return page
            .entries
            .iter()
            .find_map(|entry| match entry {
                menu::Entry::Run {
                    action: menu::Action::LaunchRace,
                    inputs: Some(inputs),
                    ..
                } => Some(inputs.clone()),
                _ => None,
            })
            .expect("START launches through launch_race with its keys");
    }
    let key = |name: &str| Some(format!("mod.oag.remix.{name}"));
    menu::LaunchInputs {
        track_title: key("track_title"),
        track: key("track"),
        craft_title: key("craft_title"),
        team: key("team"),
        variant: key("variant"),
        mode: key("mode"),
        class: key("class"),
        difficulty: key("difficulty"),
    }
}

/// RACE REMIX's picks, saved under `[mod_settings."oag.remix"]` and read by its START row's
/// keys, build two-source options: the circuit off the track disc, the craft off the other.
/// The fixture is a candidate list surveyed off the discs `images` names; a missing disc skips.
fn remix_options(track_image: &str, craft_image: &str, class: &str) {
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

    let keys = remix_keys();
    let mut settings = Settings::default();
    for (key, value) in [
        (&keys.track_title, track.title()),
        (&keys.track, circuit.as_str()),
        (&keys.craft_title, craft.title()),
        (&keys.team, team.as_str()),
        (&keys.mode, "single_race"),
        (&keys.class, class),
        (&keys.difficulty, "skilled"),
    ] {
        let key = key.as_deref().expect("START names every input");
        assert!(settings::mod_settings::set(&mut settings.mod_settings, key, value));
    }
    let inputs = read_inputs(&settings, &keys).unwrap();
    let mut options = Options::default();
    assert!(apply_two_source(&titles, &mut options, &inputs, None));
    assert_eq!(options.source, track.source, "the circuit's disc");
    assert_eq!(
        options.craft_source.as_deref(),
        Some(craft.source.as_str()),
        "the craft's disc"
    );
    assert_eq!(
        options.track,
        track_catalogue.track(&circuit).map(catalogue::Track::entry_name),
        "the circuit named by key"
    );
    assert!(
        options.team.as_deref().is_some_and(|t| t.starts_with(team.as_str())),
        "the team named by key: {:?}",
        options.team
    );
    assert_eq!(options.class, class);
    assert_eq!(options.mode, oag_race::Mode::from_name("single_race").unwrap());
    assert_eq!(options.difficulty, oag_ai::Difficulty::from_name("skilled").unwrap());
}

#[test]
#[ignore = "needs the Pulse and Pure disc images in data/images/"]
fn a_pulse_craft_on_a_pure_track_launches_from_the_remix_mods_keys() {
    remix_options("pure-psp-eu.chd", "pulse-psp-eu.chd", "venom");
}

#[test]
#[ignore = "needs the Pulse and HD/Fury disc images in data/images/"]
fn an_hd_craft_on_a_pulse_track_launches_from_the_remix_mods_keys() {
    remix_options("pulse-psp-eu.chd", "hdfury-ps3-eu-dec.iso", "venom");
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
