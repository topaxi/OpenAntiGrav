//! Mods know the titles they are for: a texture pack takes them from its `[games]` serials, and
//! a mod that excludes a title is neither loaded nor offered on it (the MODS page, the menu
//! tree, the texture packs a race or the front end opens).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use oag_game::mods::Mods;
use oag_ui::menu::{BUILT_IN, For};

fn base(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-pack-titles-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A pack folder with no manifest, as a player drops one in.
fn pack(base: &Path, name: &str, games: &str) {
    let dir = base.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("textures.ini"),
        format!("[options]\nversion = 1\nhash = xxh64\nignoreAddress = true\n[games]\n{games}\n[hashes]\n"),
    )
    .unwrap();
}

fn load(base: &Path) -> Mods {
    let on: BTreeSet<String> = [
        "ppsspp.pure-pack",
        "ppsspp.pulse-pack",
        "ppsspp.unknown-pack",
    ]
    .map(String::from)
    .into();
    let switches = oag_game::settings::Mods {
        enabled: on,
        ..Default::default()
    };
    Mods::load_from(&[base.to_path_buf()], &switches).unwrap()
}

fn titles_of(mods: &Mods, id: &str) -> Vec<String> {
    mods.player()
        .find(|m| m.manifest.id == id)
        .unwrap_or_else(|| panic!("{id} not loaded"))
        .manifest
        .titles
        .clone()
}

fn names(
    packs: &[Result<std::sync::Arc<oag_texture::replacer::Replacements>, String>],
) -> Vec<String> {
    packs
        .iter()
        .map(|pack| pack.as_ref().expect("a pack opens").name().to_string())
        .collect()
}

#[test]
fn a_pure_pack_is_a_pure_mod_and_is_never_a_candidate_on_pulse() {
    let base = base("packs");
    pack(
        &base,
        "Pure Pack",
        "UCES00001 = textures.ini\nUCUS98612 = textures.ini",
    );
    pack(
        &base,
        "Pulse Pack",
        "UCES00465 = textures.ini\nUCUS98712 = textures.ini",
    );
    pack(&base, "Unknown Pack", "UCJS10007 = textures.ini");
    let mods = load(&base);

    assert_eq!(titles_of(&mods, "ppsspp.pure-pack"), ["wipeout-pure"]);
    assert_eq!(titles_of(&mods, "ppsspp.pulse-pack"), ["wipeout-pulse"]);
    assert!(
        titles_of(&mods, "ppsspp.unknown-pack").is_empty(),
        "serials no title lists leave the pack for every title"
    );

    let on_pulse =
        oag_game::texture_pack::packs_for(&mods, oag_pulse::TITLE, oag_assets::Platform::Psp);
    assert_eq!(names(&on_pulse), ["Pulse Pack", "Unknown Pack"]);
    let on_pure =
        oag_game::texture_pack::packs_for(&mods, oag_pure::TITLE, oag_assets::Platform::Psp);
    assert_eq!(names(&on_pure), ["Pure Pack", "Unknown Pack"]);
}

#[test]
fn an_authors_own_titles_win_over_the_serials() {
    let base = base("author");
    pack(&base, "Pure Pack", "UCES00001 = textures.ini");
    std::fs::write(
        base.join("Pure Pack").join("oag-mod.toml"),
        "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/mod.tosd\", version = \"1.0.0\" }\n\n[mod]\nid = \"ppsspp.pure-pack\"\nname = \"Pure Pack\"\nversion = \"1.0.0\"\nauthors = [\"a\"]\ndescription = \"d\"\ntitles = [\"wipeout-hd\"]\n\n[texture-pack]\nformat = \"ppsspp\"\n",
    )
    .unwrap();
    let mods = load(&base);
    assert_eq!(titles_of(&mods, "ppsspp.pure-pack"), ["wipeout-hd"]);
}

#[test]
fn a_mod_for_another_title_is_not_in_the_menu_the_strings_or_the_mods_page() {
    let base = base("menu");
    let dir = base.join("pulse-only");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("oag-mod.toml"),
        "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/mod.tosd\", version = \"1.0.0\" }\n\n[mod]\nid = \"test.pure-only\"\nname = \"Pure Only\"\nversion = \"0.1.0\"\nauthors = [\"a\"]\nlicense = \"CC0-1.0\"\ndescription = \"d\"\ntitles = [\"wipeout-pure\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("menu.toml"),
        "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/mod-menu.tosd\", version = \"1.0.0\" }\n\n[[entry]]\nid = \"purity\"\nparent = \"options\"\nkind = \"submenu\"\nlabel = \"PURITY\"\ntarget = \"records\"\n",
    )
    .unwrap();
    let mods = Mods::load_from(&[base], &oag_game::settings::Mods::default()).unwrap();
    let build = |title| {
        mods.build(
            BUILT_IN,
            None,
            Some(For {
                title,
                platform: oag_assets::Platform::Psp,
                updated: false,
            }),
        )
        .unwrap()
    };

    let pulse = build(oag_pulse::TITLE);
    assert!(
        !pulse.listing.iter().any(|l| l.id == "test.pure-only"),
        "not offered on the MODS page"
    );
    assert!(!pulse.added.iter().any(|a| a.mod_id == "test.pure-only"));
    assert_eq!(mods.player_for(oag_pulse::TITLE).count(), 0);

    let pure = build(oag_pure::TITLE);
    assert!(pure.listing.iter().any(|l| l.id == "test.pure-only"));
    assert!(pure.added.iter().any(|a| a.mod_id == "test.pure-only"));
}

#[test]
fn a_ppsspp_pack_is_the_psps_alone_and_is_not_offered_on_a_ps2_source() {
    use oag_assets::Platform;

    let base = base("platform");
    pack(&base, "Pulse Pack", "UCES00465 = textures.ini");
    pack(&base, "Unknown Pack", "UCJS10007 = textures.ini");
    let mods = load(&base);
    let build = |platform| {
        mods.build(
            BUILT_IN,
            None,
            Some(For {
                title: oag_pulse::TITLE,
                platform,
                updated: false,
            }),
        )
        .unwrap()
    };
    let listed = |platform, id: &str| build(platform).listing.iter().any(|l| l.id == id);

    for id in ["ppsspp.pulse-pack", "ppsspp.unknown-pack"] {
        assert!(listed(Platform::Psp, id), "{id} is offered on a PSP");
        for platform in [Platform::Ps2, Platform::Ps3, Platform::Vita, Platform::Ps4] {
            assert!(!listed(platform, id), "{id} is not offered on {platform:?}");
        }
    }

    let found = |platform| {
        names(&oag_game::texture_pack::packs_for(
            &mods,
            oag_pulse::TITLE,
            platform,
        ))
    };
    assert_eq!(found(Platform::Psp), ["Pulse Pack", "Unknown Pack"]);
    assert!(found(Platform::Ps2).is_empty(), "no pack is opened on PS2");
    assert_eq!(
        mods.player_on(oag_pulse::TITLE, Platform::Ps2).count(),
        0,
        "the same filter the page uses"
    );
}
