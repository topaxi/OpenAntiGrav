use super::*;
use oag_raceplay::photo::effects::{Effects, Row};

fn meta() -> Meta {
    let race = Race {
        track: "Data\\Tracks\\Foo.vex".into(),
        mode: "time_trial".into(),
        class: "venom".into(),
        team: "feisar".into(),
        seed: hex(0xDEAD_BEEF),
    };
    let camera = Camera {
        lens: "free".into(),
        eye: [1.0, 2.0, 3.0],
        forward: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        fov_deg: Some(70.0),
    };
    let craft = Craft {
        position: [4.0, 5.0, 6.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
    };
    Meta {
        format: FORMAT,
        shot: Shot {
            tick: 1237,
            state_hash: hex(0x1234),
            width: 2,
            height: 2,
            source: "photo mode".into(),
            stream: true,
            ..Shot::default()
        },
        title: Title {
            name: "Wipeout Pulse".into(),
            serial: Some("ULES-00000".into()),
            platform: "Psp".into(),
        },
        reproduce: Reproduce {
            exact: "oag-game --from-png <this file>".into(),
            flags: flags(&race, Some(&camera), Some(&craft), None, (2, 2)),
        },
        race: Some(race),
        camera: Some(camera),
        craft: Some(craft),
        mods: vec![Mod {
            id: "oag.extras".into(),
            version: "1.0.0".into(),
            sha256: "ab".repeat(32),
        }],
        build: Build::this(),
        ..Meta::default()
    }
}

#[test]
fn the_text_round_trips() {
    let meta = meta();
    assert_eq!(Meta::from_text(&meta.to_text()).expect("parses"), meta);
}

#[test]
fn the_flags_have_the_shape_of_a_comparison_pair() {
    let flags = meta().reproduce.flags;
    assert_eq!(
        &flags[..4],
        ["--race", "--no-audio", "--track", "Data\\Tracks\\Foo.vex"]
    );
    for flag in ["--pose", "--camera-pose", "--camera-fov", "--size"] {
        assert!(flags.iter().any(|f| f == flag), "{flag} in {flags:?}");
    }
    assert!(flags.contains(&"1,2,3,0,0,-1,0,1,0".to_string()));
}

#[test]
fn a_png_carries_both_chunks_and_still_decodes() {
    let pixels = [10, 20, 30, 0, 40, 50, 60, 7, 70, 80, 90, 255, 1, 2, 3, 4];
    let png = encode(2, 2, &pixels, &meta(), None);
    let found = read(&png).expect("reads");
    assert_eq!(found.meta, Some(meta()));
    assert!(found.record.is_none());
    let image = oag_texture::replacer::decode_png(&png).expect("decodes");
    assert_eq!(&image.rgba[..3], &[10, 20, 30]);
    assert_eq!(
        image.rgba[3], 255,
        "the alpha is made opaque, as every capture's is"
    );
}

#[test]
fn nothing_private_is_in_the_text() {
    let text = meta().to_text();
    assert!(!text.contains("/home"), "{text}");
    assert!(Meta::from_text(&text).is_ok());
}

#[test]
fn a_png_without_our_chunks_reads_as_nothing() {
    let png = oag_texture::png::encode_rgba(1, 1, &[0; 4]);
    let found = read(&png).expect("reads");
    assert!(found.meta.is_none() && found.record.is_none());
    assert!(read(b"not a png").is_err());
}

#[test]
fn the_command_line_loses_every_path() {
    let args = [
        "data/images/pulse.chd",
        "--race",
        "--track",
        "Data\\Tracks\\Foo.vex",
        "--screenshot",
        "/home/me/out.png",
        "--dump-audio",
        "/tmp/a.wav",
        "--ticks",
        "700",
        "--hold",
        "cross",
        "~/x.chd",
    ];
    let kept = own_flags(args.iter().map(ToString::to_string));
    assert_eq!(
        kept,
        [
            "--race",
            "--track",
            "Data\\Tracks\\Foo.vex",
            "--ticks",
            "700",
            "--hold",
            "cross"
        ]
    );
}

fn graded() -> Effects {
    let mut effects = Effects::new();
    effects.set(Row::Exposure, 60);
    effects.set(Row::Saturation, 5);
    effects.set(Row::SpeedBlur, 30);
    effects.set(Row::DepthOfField, 1);
    effects.set(Row::Aperture, 70);
    effects.set(Row::Focus, 45);
    effects
}

#[test]
fn a_picture_at_its_opening_values_is_recorded_as_them() {
    let picture = Picture::of(&Effects::new(), None);
    assert_eq!(picture.exposure, Effects::EXPOSURE_OPENS);
    assert_eq!(picture.saturation, Effects::SATURATION_OPENS);
    assert!(!picture.depth_of_field && picture.motion_blur.is_empty());
    assert!(picture.effects().is_neutral());
    let mut meta = meta();
    meta.picture = Some(picture);
    assert!(meta.to_text().contains("[picture]"));
}

#[test]
fn the_picture_controls_survive_the_text_and_draw_the_same_effects() {
    let picture = Picture::of(&graded(), None);
    let mut meta = meta();
    meta.picture = Some(picture.clone());
    let read = Meta::from_text(&meta.to_text()).expect("parses");
    assert_eq!(read.picture.as_ref(), Some(&picture));
    assert_eq!(picture.effects(), graded());
    // The text names the controls, not a bare table.
    assert!(meta.to_text().contains("[picture]"), "{}", meta.to_text());
}

#[test]
fn the_picture_flag_round_trips_and_a_bad_one_is_refused() {
    let picture = Picture::of(&graded(), None);
    let flag = picture.to_flag();
    assert_eq!(
        flag,
        "exposure=60,saturation=5,speed_blur=30,aperture=70,focus=45"
    );
    assert_eq!(
        Picture::from_flag(&flag).expect("reads").effects(),
        graded()
    );
    assert!(Picture::from_flag("exposure").is_err());
    assert!(Picture::from_flag("exposure=x").is_err());
    assert!(Picture::from_flag("brightness=4").is_err());
    let race = Race::default();
    assert!(flags(&race, None, None, Some(&picture), (2, 2)).contains(&"--picture".to_string()));
    assert!(!flags(&race, None, None, None, (2, 2)).contains(&"--picture".to_string()));
}

#[test]
fn a_picture_section_that_leaves_controls_out_has_them_at_their_opening_values() {
    let meta = Meta::from_text("[picture]\nspeed_blur = 50\n").expect("parses");
    let picture = meta.picture.expect("the section");
    assert_eq!(picture.speed_blur, 50);
    assert_eq!(picture.exposure, Effects::EXPOSURE_OPENS);
    assert_eq!(picture.saturation, Effects::SATURATION_OPENS);
    let effects = picture.effects();
    assert_eq!(effects.value(Row::Exposure), Effects::EXPOSURE_OPENS);
    assert_eq!(effects.value(Row::Saturation), Effects::SATURATION_OPENS);
}
