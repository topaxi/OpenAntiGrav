use super::*;

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
            flags: flags(&race, Some(&camera), Some(&craft), (2, 2)),
        },
        race: Some(race),
        camera: Some(camera),
        craft: Some(craft),
        mods: vec![Mod {
            id: "oag.extras".into(),
            version: "1.0.0".into(),
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
