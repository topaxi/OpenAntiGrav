use super::*;

const TITLE: &Title = oag_pulse::TITLE;

#[test]
fn a_title_directory_is_its_lowercase_hyphenated_name() {
    assert_eq!(slug(TITLE), "wipeout-pulse");
    assert_eq!(slug(oag_pure::TITLE), "wipeout-pure");
}

#[test]
fn no_directory_is_no_override() {
    let scratch = oag_testdata::scratch::dir("overrides-none");
    assert!(override_layer_in(&[scratch.join("missing")], TITLE).is_none());
}

#[test]
fn files_under_the_title_directory_are_the_override_and_another_title_s_are_not() {
    let base = oag_testdata::scratch::dir("overrides");
    let mine = base.join("wipeout-pulse/Data/HUD");
    std::fs::create_dir_all(&mine).unwrap();
    std::fs::write(mine.join("Foo.mip"), b"mod").unwrap();

    let layer = override_layer_in(&[base.to_path_buf()], TITLE).expect("an override");
    assert!(layer.contains("Data\\HUD\\foo.mip"));
    assert!(override_layer_in(&[base.to_path_buf()], oag_pure::TITLE).is_none());
}

/// The fallback carries the indicator's layout and art under the entry names the
/// layout itself uses, so a reader asking the archives for them is answered.
#[test]
fn the_fallback_holds_the_assist_indicator_and_the_art_its_layout_names() {
    let layer = fallback_layer();
    let fragment = layer.read("Data\\XML\\HUD_assist_indicator.xml").unwrap();
    let text = String::from_utf8(fragment).unwrap();
    assert!(text.contains("CHOSEN, NOT MEASURED"), "labelled as chosen");
    let art = layer
        .read("Data\\HUD\\Textures\\AssistIndicator.pix")
        .unwrap();
    assert!(layer.contains("/data/hud/textures/assistindicator.pix"));
    assert!(
        art.starts_with(b"OAGMASK 112 144\n"),
        "the atlas the layout's UVs index"
    );
}

/// A title that names a split-HUD file in its `HudLayouts` has it in the fallback under that
/// name, and a split race of it reads a file rather than scaling the whole-screen layout.
/// Dropping either half, or a title's row, fails here.
#[test]
fn every_title_with_a_split_hud_row_has_its_file_in_the_fallback() {
    let layer = fallback_layer();
    for title in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_hd::TITLE,
        oag_2048::TITLE,
    ] {
        let entry = title
            .hud
            .split_layout
            .unwrap_or_else(|| panic!("{} names no split-HUD file", title.name));
        assert!(layer.contains(entry), "{}: {entry}", title.name);
        let text = String::from_utf8(layer.read(entry).unwrap()).unwrap();
        assert!(
            text.contains("CHOSEN, NOT MEASURED"),
            "{}: labelled",
            title.name
        );
    }
}

#[test]
fn the_fallback_carries_every_rumble_pattern_in_its_directory() {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/fallback/Data/XML/Rumble"
    );
    let layer = fallback_layer();
    let mut seen = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        let entry_name = format!("Data\\XML\\Rumble\\{name}");
        assert!(layer.contains(&entry_name), "{entry_name} is not embedded");
        seen += 1;
    }
    assert_eq!(seen, RUMBLE_PATTERNS.len());
}

/// A sorted table is what keeps two additions on separate lines, and a repeated
/// path would let the later insert silently win.
#[test]
fn the_embedded_table_is_sorted_with_no_path_twice() {
    let names: Vec<&str> = EMBEDDED.iter().map(|(name, _)| *name).collect();
    assert!(names.windows(2).all(|pair| pair[0] < pair[1]), "{names:?}");
    let layer = fallback_layer();
    for name in names {
        assert!(layer.contains(name), "{name} is not in the fallback layer");
    }
}

/// A mod's `fallback/<slug>/` folders are one layer per title, and an open keeps only its own
/// title's, so a mod's Pulse entries never answer on Pure.
#[test]
fn a_mod_folder_is_one_layer_per_title_and_an_open_keeps_its_own() {
    let root = oag_testdata::scratch::dir("mod-fallback-layers");
    for (slug, file) in [
        ("wipeout-pulse", "Data/XML/Ours.xml"),
        ("wipeout-pure", "Data/XML/Other.xml"),
    ] {
        let path = root.join("fallback").join(slug).join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"x").unwrap();
    }
    std::fs::create_dir_all(root.join("fallback/wipeout-hd")).unwrap();
    let mods = ModFallback::in_folder(&root, "test.mod");
    assert_eq!(
        mods.iter().map(|m| m.slug.as_str()).collect::<Vec<_>>(),
        ["wipeout-pulse", "wipeout-pure"],
        "an empty folder is no layer"
    );
    let layering = Layering {
        mods,
        ..Layering::none()
    };
    let pulse = layering.mods_for(TITLE);
    assert_eq!(pulse.len(), 1);
    assert!(pulse[0].contains("Data\\XML\\Ours.xml") && !pulse[0].contains("Data\\XML\\Other.xml"));
    assert_eq!(pulse[0].kind(), Kind::Mod);
    assert_eq!(pulse[0].label(), "test.mod");
    assert!(layering.mods_for(oag_hd::TITLE).is_empty());
    assert!(Layering::none().mods_for(TITLE).is_empty());
    // A mod whose entries serve only single-player races is not mounted in a network race.
    let mut single = layering.clone();
    single.mods.iter_mut().for_each(|m| m.network = false);
    assert!(single.clone().for_network().mods_for(TITLE).is_empty());
    assert_eq!(
        single.mods_for(TITLE).len(),
        1,
        "a single-player race mounts it"
    );
    assert_eq!(layering.for_network().mods_for(TITLE).len(), 1);
    std::fs::remove_dir_all(&root).unwrap();
}

/// What [`register_mods`] registers is what [`Layering::player`] mounts and what
/// [`registered_mods_hold`] answers, until it is replaced.
#[test]
fn the_registered_mods_are_the_players_until_replaced() {
    let mut layer = Layer::new(Kind::Mod, "test.mod");
    layer.insert_bytes("Data\\XML\\Ours.xml", b"x");
    register_mods(vec![ModFallback {
        slug: slug(TITLE),
        layer,
        network: true,
    }]);
    assert_eq!(Layering::player().mods_for(TITLE).len(), 1);
    assert!(registered_mods_hold(TITLE, "data/xml/ours.xml"));
    assert!(!registered_mods_hold(oag_pure::TITLE, "data/xml/ours.xml"));
    register_mods(Vec::new());
    assert!(!registered_mods_hold(TITLE, "data/xml/ours.xml"));
}

/// An enabled mod is listed for a race on a title it admits, a PPSSPP pack on a PSP source
/// alone, and a single-player mod in no network race.
#[test]
fn an_enabled_mod_is_listed_for_its_titles_platform_and_network() {
    let any = EnabledMod {
        id: "a.any".to_string(),
        version: "1.0.0".to_string(),
        titles: Vec::new(),
        psp_only: false,
        network: true,
    };
    let pack = EnabledMod {
        id: "b.pack".to_string(),
        titles: vec![slug(TITLE)],
        psp_only: true,
        network: false,
        ..any.clone()
    };
    assert!(any.is_for(&[oag_hd::TITLE], Platform::Ps3));
    assert!(pack.is_for(&[TITLE], Platform::Psp));
    assert!(
        pack.is_for(&[oag_hd::TITLE, TITLE], Platform::Psp),
        "a craft's title counts"
    );
    assert!(!pack.is_for(&[TITLE], Platform::Ps2));
    assert!(!pack.is_for(&[oag_hd::TITLE], Platform::Psp));
    let layering = Layering {
        enabled: vec![any.clone(), pack],
        ..Layering::none()
    };
    assert_eq!(layering.for_network().enabled, [any]);
    assert!(Layering::none().enabled.is_empty());
}
