//! What HD/Fury and Wipeout 2048 actually say in `.effectSettings`/`.effectsettings`.
//!
//! `#[ignore]`d because HD needs `data/images/hdfury-ps3-eu-dec.iso` and 2048
//! needs `data/extracted/vita/PCSF00007/base/PSP2/data.psarc` (`just
//! vita-self-decrypt` per `data/README.md`); run with `just test-data`.

use std::path::{Path, PathBuf};

use oag_formats::effectsettings::EffectSettings;

/// HD's own 15-stage speed-class ladder, `zonemode.effectsettings`'s own
/// order - shared verbatim by all four HD files measured here.
const HD_STAGES: &[&str] = &[
    "Start",
    "Sub Venom",
    "Venom",
    "Sub Flash",
    "Flash",
    "Sub Rapier",
    "Rapier",
    "Sub Phantom",
    "Phantom",
    "Super Phantom",
    "Zen",
    "Super Zen",
    "Subsonic",
    "Mach 1",
    "Supersonic",
];

/// 2048's 13-stage ladder - HD's own list with `Sub Venom`/`Venom` missing,
/// every later index shifted down by exactly 2.
const TWOK48_STAGES: &[&str] = &[
    "Start",
    "Sub Flash",
    "Flash",
    "Sub Rapier",
    "Rapier",
    "Sub Phantom",
    "Phantom",
    "Super Phantom",
    "Zen",
    "Super Zen",
    "Subsonic",
    "Mach 1",
    "Supersonic",
];

/// 2048's ten circuits, each shipping a byte-identical copy of the table.
const TWOK48_CIRCUITS: &[&str] = &[
    "altima",
    "arena",
    "bridge",
    "cathedral",
    "mall",
    "park",
    "sol",
    "square",
    "subway",
    "tower",
];

fn hd_image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/hdfury-ps3-eu-dec.iso");
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn twok48_psarc() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn stage_names(effect: &EffectSettings) -> Vec<String> {
    effect.stages.values().map(|s| s.name.clone()).collect()
}

/// **All four of HD's files parse and share the same 15-stage ladder** -
/// `zonemode`, its DLC3 revision, and `detonatormode`/`detonatormodedlc3`,
/// which reuses Zone's own speed-class list wholesale rather than authoring
/// one of its own.
#[test]
#[ignore]
fn hd_files_parse_and_share_the_fifteen_stage_ladder() {
    let Some(image) = hd_image() else { return };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", image.display());
    let mut archive = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");

    for name in [
        "zonemode",
        "zonemodedlc3",
        "detonatormode",
        "detonatormodedlc3",
    ] {
        let path = format!("/data/environments/{name}.effectsettings");
        let bytes = archive
            .read_path(&path)
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        let text = String::from_utf8(bytes)
            .unwrap_or_else(|_| panic!("{path} is not UTF-8, so it is not this format"));
        let effect = EffectSettings::parse(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(
            stage_names(&effect),
            HD_STAGES,
            "{path} should carry the same 15-stage ladder"
        );
    }
}

/// One stage's own float+alpha colour, read off the real disc rather than a
/// hand-typed excerpt - `Start`'s constant ambient is `(1, 1, 1, 0)`.
#[test]
#[ignore]
fn hd_stage_zero_reads_its_own_ambient_colour() {
    let Some(image) = hd_image() else { return };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", image.display());
    let mut archive = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
    let bytes = archive
        .read_path("/data/environments/zonemode.effectsettings")
        .expect("the entry reads");
    let text = String::from_utf8(bytes).expect("UTF-8");
    let effect = EffectSettings::parse(&text).expect("it parses");
    assert_eq!(
        effect.stage_vec4(0, "Lighting.Constant Ambient Colour"),
        Some([1.0, 1.0, 1.0, 0.0])
    );
}

/// **All ten of 2048's per-circuit copies parse identically to the same
/// 13-stage ladder**, and are byte-identical to each other - one shared
/// table duplicated per circuit directory rather than looked up once.
#[test]
#[ignore]
fn twok48_circuits_share_one_byte_identical_thirteen_stage_table() {
    let Some(psarc) = twok48_psarc() else { return };
    let mut archive = oag_assets::psarc::Archive::open(psarc.to_str().expect("utf-8 path"))
        .expect("the archive opens");

    let mut first_bytes: Option<Vec<u8>> = None;
    for circuit in TWOK48_CIRCUITS {
        let path = format!("data/art/published/environments/{circuit}/ZoneMode2048.effectSettings");
        let bytes = archive
            .read_path(&path)
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        match &first_bytes {
            None => first_bytes = Some(bytes.clone()),
            Some(first) => assert_eq!(
                &bytes, first,
                "{circuit}'s copy should be byte-identical to altima's"
            ),
        }
        let text = String::from_utf8(bytes)
            .unwrap_or_else(|_| panic!("{path} is not UTF-8, so it is not this format"));
        let effect = EffectSettings::parse(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(
            stage_names(&effect),
            TWOK48_STAGES,
            "{path} should carry the same 13-stage ladder"
        );
    }
}

/// A title-wide key with no stage prefix reads straight off `table` - 2048's
/// `"Sky Radius"`, a global rather than a per-stage value.
#[test]
#[ignore]
fn twok48_title_wide_keys_read_with_no_stage() {
    let Some(psarc) = twok48_psarc() else { return };
    let mut archive = oag_assets::psarc::Archive::open(psarc.to_str().expect("utf-8 path"))
        .expect("the archive opens");
    let bytes = archive
        .read_path("data/art/published/environments/altima/ZoneMode2048.effectSettings")
        .expect("the entry reads");
    let text = String::from_utf8(bytes).expect("UTF-8");
    let effect = EffectSettings::parse(&text).expect("it parses");
    assert_eq!(effect.table.scalar("Sky Radius"), Some(4000.0));
}
