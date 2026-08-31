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

/// **The stage table is a real colour grade, read off the disc**: `Start` and
/// `Sub Venom` differ in every field that has somewhere to go, and both ends
/// of the recovered cross-fade land on the file's own authored numbers.
///
/// Every value asserted here was read out of
/// `/data/environments/zonemode.effectsettings` itself, not typed from a
/// guess - the same numbers `effectsettings/tests.rs` blends offline, so a
/// drift in either fails both.
#[test]
#[ignore]
fn hd_stage_zero_and_stage_one_are_two_different_authored_palettes() {
    use oag_formats::effectsettings::StagePalette;

    let Some(image) = hd_image() else { return };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", image.display());
    let mut archive = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
    let bytes = archive
        .read_path("/data/environments/zonemode.effectsettings")
        .expect("the entry reads");
    let text = String::from_utf8(bytes).expect("UTF-8");
    let effect = EffectSettings::parse(&text).expect("it parses");

    // `Start`: no fog of its own, a neutral rig, a black sky reflection.
    let start = StagePalette {
        // The three fields HD's own cross-fade reads, plus the scalar that
        // rides in the first one's fourth lane.
        scene_texture_colour: Some([0.0, 0.0, 0.0]),
        scene_eq_brightness: Some(0.0),
        scene_base_colour: Some([3.0, 3.0, 3.0]),
        scene_base_colour_highlight: Some([0.0, 0.0, 0.0]),
        // **The file authors no `Scene.Aniso Power` on any of its fifteen
        // stages**, though HD's own schema registers the key. So the Zone
        // shader's `zoneAnisoPower` keeps the `(6, 6)` its constructor sets
        // and the rim term it exponentiates has no per-stage source.
        scene_aniso_power: None,
        fog_colour: Some([0.0, 0.0, 0.0]),
        fog_density: Some(0.0),
        sun_colour: Some([1.0, 1.0, 1.0]),
        ambient_colour: Some([1.0, 1.0, 1.0]),
        prelit_scale: Some([1.0, 1.0, 1.0]),
        prelit_power: Some([1.0, 1.0, 1.0]),
        sky_reflection_colour: Some([0, 0, 0, 255]),
        sky_horizon_colour: Some([0.0, 0.0, 0.0]),
        sky_zenith_colour: Some([0.0, 0.0, 0.0]),
    };
    // `Sub Venom`: a cyan fog at a real density, a brighter ambient, a sunless
    // rig and a white sky reflection - the escalation this table exists for.
    let sub_venom = StagePalette {
        // `Scene.Texture Colour` is what reaches the shader as `fogColour`,
        // and `Scene.EQ brightness` is the `20.0` that rides in its `.w`.
        scene_texture_colour: Some([0.721_569, 0.909_804, 0.964_706]),
        scene_eq_brightness: Some(20.0),
        scene_base_colour: Some([0.003_922, 0.847_059, 1.0]),
        scene_base_colour_highlight: Some([0.0, 1.694_118, 2.0]),
        scene_aniso_power: None,
        fog_colour: Some([0.0, 1.305_882, 1.8]),
        fog_density: Some(0.002_1),
        sun_colour: Some([0.0, 0.0, 0.0]),
        ambient_colour: Some([1.5, 1.5, 1.5]),
        prelit_scale: Some([1.0, 1.0, 1.0]),
        prelit_power: Some([1.0, 1.0, 1.0]),
        sky_reflection_colour: Some([255, 255, 255, 255]),
        sky_horizon_colour: Some([0.0, 0.211_765, 0.247_059]),
        sky_zenith_colour: Some([0.003_922, 0.286_275, 0.466_667]),
    };
    assert_eq!(effect.stage_palette(0), Some(start));
    assert_eq!(effect.stage_palette(1), Some(sub_venom));
    assert_ne!(start, sub_venom, "the two stages are a visible grade apart");

    // Stage 0 at rest is exactly `Start` - it pairs with itself, so nothing
    // fades while a race sits on it.
    assert_eq!(effect.blended_palette(0, 1.0), Some(start));
    // Stage 1 fully arrived is exactly `Sub Venom`; stage 1 not yet begun is
    // exactly `Start`, which is the stage it fades from.
    assert_eq!(effect.blended_palette(1, 1.0), Some(sub_venom));
    assert_eq!(effect.blended_palette(1, 0.0), Some(start));
    // Halfway: the float keys in floats, `1.8` intact rather than clipped to
    // white, and the one byte-written key through the recovered byte
    // arithmetic.
    let half = effect.blended_palette(1, 0.5).expect("stage 1 is named");
    assert_eq!(half.fog_colour, Some([0.0, 1.305_882 / 2.0, 0.9]));
    assert_eq!(half.fog_density, Some(0.002_1 / 2.0));
    assert_eq!(half.sky_reflection_colour, Some([127, 127, 127, 254]));
    // And the field that actually reaches a shader fades on the same terms:
    // `Start` authors black, so half of `Sub Venom` is half its own value.
    assert_eq!(
        half.scene_texture_colour,
        Some([0.721_569 / 2.0, 0.909_804 / 2.0, 0.964_706 / 2.0])
    );
    assert_eq!(half.scene_eq_brightness, Some(10.0));
}

/// **The whole ladder is populated, not just its first rungs** - every one of
/// HD's fifteen stages authors the fog, rig and sky-reflection keys this
/// project reads, so a stage index anywhere on the ladder has a palette to
/// apply rather than a hole.
#[test]
#[ignore]
fn every_hd_stage_authors_the_keys_this_project_reads() {
    let Some(image) = hd_image() else { return };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", image.display());
    let mut archive = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
    let bytes = archive
        .read_path("/data/environments/zonemode.effectsettings")
        .expect("the entry reads");
    let text = String::from_utf8(bytes).expect("UTF-8");
    let effect = EffectSettings::parse(&text).expect("it parses");
    for stage in 0..HD_STAGES.len() as u32 {
        let palette = effect
            .stage_palette(stage)
            .unwrap_or_else(|| panic!("stage {stage} is named"));
        assert!(palette.fog_colour.is_some(), "stage {stage} fog colour");
        assert!(palette.fog_density.is_some(), "stage {stage} fog density");
        assert!(palette.sun_colour.is_some(), "stage {stage} sun colour");
        assert!(palette.ambient_colour.is_some(), "stage {stage} ambient");
        assert!(
            palette.sky_reflection_colour.is_some(),
            "stage {stage} sky reflection"
        );
        // The three the original cross-fades, and the scalar that shares the
        // first one's storage - a hole in any of these would mean a stage
        // whose traced consumer has nothing to consume.
        assert!(
            palette.scene_texture_colour.is_some(),
            "stage {stage} Scene.Texture Colour"
        );
        assert!(
            palette.scene_eq_brightness.is_some(),
            "stage {stage} Scene.EQ brightness"
        );
        assert!(
            palette.scene_base_colour.is_some(),
            "stage {stage} Scene.Base Colour"
        );
        assert!(
            palette.scene_base_colour_highlight.is_some(),
            "stage {stage} Scene.Base Colour Highlight"
        );
    }
}
