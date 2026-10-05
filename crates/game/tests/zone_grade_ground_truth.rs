//! A Zone race reads its title's own stage table and grades the circuit with it.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-game \
//!     --run-ignored all -E 'binary(zone_grade_ground_truth)'
//! ```
//!
//! # What this file is the evidence for
//!
//! `zone_ground_truth.rs` proves a Zone race opens the right *circuit*. This
//! one proves the escalation laid over it is the disc's: HD/Fury's
//! `/data/environments/zonemode.effectsettings` reaches
//! `oag_raceplay::Loaded` through a real race load, its fifteen stages are
//! the file's own, and two adjacent stages produce two visibly different fogs
//! whose numbers are the ones the file authors - not a pair this test made up.
//!
//! # What is deliberately not asserted
//!
//! **Nothing about when HD's own stage changes.** HD/Fury's Zone stage index
//! comes from a per-craft field (`craftArray[n]->+0x640`) whose writer is not
//! found, so an HD race still rests on stage `0` and the HD tests below drive
//! the stage by hand to check the plumbing. **2048 is different**: its
//! zone-number ladder *is* recovered
//! (`oag_2048::race::ZONE_STAGES`), and the last test in this file drives a
//! real 2048 table through it, asserting the disc's own colours at the
//! recovered band boundaries. See `oag_raceplay::zone_grade`'s module docs.

use std::path::{Path, PathBuf};

use oag_raceplay as race;

/// One image, or `None` with a printed reason when it is not present.
fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

fn load(mode: oag_race::Mode) -> Option<race::Loaded> {
    let path = image("data/images/hdfury-ps3-eu-dec.iso")?;
    Some(
        race::load(&race::Options {
            source: path.display().to_string(),
            class: "VENOM".to_string(),
            mode,
            ..race::Options::default()
        })
        .expect("loading the race"),
    )
}

/// **The table reaches a real race load**, with the ladder the file names, and
/// the load report says so.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_zone_race_loads_the_titles_own_stage_table() {
    let Some(loaded) = load(oag_race::Mode::Zone) else {
        return;
    };
    for line in &loaded.report {
        println!("{line}");
    }
    let grade = loaded
        .zone_grade
        .as_ref()
        .expect("HD ships /data/environments/zonemode.effectsettings");
    // Fifteen stages, `0` through `14` - `Start` to `Supersonic`.
    assert_eq!(grade.last_stage(), 14);
    // **And the race opens on stage 1, not stage 0.** `Start` is the pre-race
    // state: it authors a flat black `Track.Base Colour` where `Sub Venom`
    // authors the cyan the original visibly shows at a start line. Opening on
    // `Start` is what this port did until 2026-08-31, and it drew a black
    // scene where the original draws cyan.
    //
    // This asserts the *opening* stage, which is inferred - see
    // `race::load::environment::ZONE_OPENING_STAGE` for the three sources.
    // What advances the stage during the race is still unrecovered on this
    // title, so nothing here asserts movement.
    assert_eq!(grade.blend().current, 1);
    assert_eq!(grade.blend().requested, 1);
    // Shown whole rather than fading up out of `Start`.
    assert_eq!(grade.blend().weight, 1.0);
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("the Zone colour grade")),
        "the load report should say what it read"
    );
}

/// **All fifteen per-stage Zone textures decode off the disc, and they are
/// fifteen different pictures.**
///
/// The distinctness is the point: HD ships *two* fifteen-entry sets, and the
/// one the obvious publisher binds is fifteen byte-identical flat whites. This
/// asserts the port loaded the other one - `zonemodetrack{0..14}.gtf` - by
/// checking that no two stages carry the same texels. A regression that
/// re-pointed the loader at the general set would pass every other test in
/// this file and fail this one.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn all_fifteen_per_stage_zone_textures_decode_and_differ() {
    let Some(loaded) = load(oag_race::Mode::Zone) else {
        return;
    };
    let mut grade = loaded.zone_grade.expect("the table loads");
    let (decoded, numbered) = grade.stage_art_counts();
    assert_eq!(numbered, 15, "HD numbers fifteen stages of texture");
    assert_eq!(decoded, 15, "every one of them decodes");

    let mut seen: Vec<Vec<u8>> = Vec::new();
    for stage in 0..=grade.last_stage() {
        grade.request_stage(stage);
        grade.commit();
        let art = grade
            .stage_art()
            .unwrap_or_else(|| panic!("stage {stage} has a texture"));
        // 256x256 on every stage, per the set's own header.
        assert_eq!((art.width, art.height), (256, 256), "stage {stage} size");
        let texels = art
            .to_rgba()
            .unwrap_or_else(|| panic!("stage {stage} decodes to pixels"))
            .into_owned();
        assert!(
            !seen.contains(&texels),
            "stage {stage} repeats an earlier stage's texture - the flat-white \
             'general' set was loaded instead of the 'track' set"
        );
        seen.push(texels);
    }
}

/// **Two adjacent stages grade the circuit differently, off the file's own
/// numbers.** `Start` authors no fog of its own and leaves the circuit's
/// standing; `Sub Venom` authors the cyan the file states, at the density the
/// file states.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn two_stages_of_the_real_table_produce_two_different_fogs() {
    let Some(loaded) = load(oag_race::Mode::Zone) else {
        return;
    };
    let mut grade = loaded.zone_grade.expect("the table loads");
    // The circuit's own fog, whatever this circuit authors - the thing a stage
    // lays over. A `zone_N` circuit authors one; if it ever stops, this test
    // says so rather than silently comparing two `None`s.
    let base = loaded.authored_fog.expect("a zone_N circuit fogs");

    // Driven to `Start` explicitly: a race opens on stage 1 now, because
    // `Start` is the pre-race state. It is still the stage that proves a
    // zero-density stage leaves the circuit's own fog standing.
    grade.request_stage(0);
    grade.commit();
    grade.set_weight(1.0);
    let start = grade.fog(Some(base)).expect("stage 0 leaves the circuit's");
    assert_eq!(
        start.colour, base.colour,
        "Start authors Fog density 0, so the circuit's own fog stands"
    );

    grade.request_stage(1);
    assert!(grade.commit());
    grade.set_weight(1.0);
    let sub_venom = grade.fog(Some(base)).expect("stage 1 authors its own");
    // Straight out of `/data/environments/zonemode.effectsettings`:
    // `"1 Sub Venom.Lighting.Fog colour"=0.000000 1.305882 1.800000 0.000000`
    // and `"1 Sub Venom.Lighting.Fog density"=0.002100`.
    assert_eq!(sub_venom.colour, [0.0, 1.305_882, 1.8]);
    assert_eq!(sub_venom.density, 0.002_1);
    assert_ne!(
        sub_venom.colour, start.colour,
        "a stage change has to be visible, or the table is doing nothing"
    );

    // Halfway between them is halfway between the two authored colours - the
    // recovered cross-fade, on real data.
    grade.set_weight(0.5);
    let half = grade.fog(Some(base)).expect("still over zero density");
    assert_eq!(half.colour, [0.0, 1.305_882 / 2.0, 0.9]);
}

/// **A stage recolours the circuit's rig without aiming it.** The schema has no
/// `Sun direction` key, so a graded rig keeps the circuit's own direction and
/// takes the stage's own ambient.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_stage_tints_the_circuits_own_light_rig() {
    let Some(loaded) = load(oag_race::Mode::Zone) else {
        return;
    };
    let mut grade = loaded.zone_grade.expect("the table loads");
    let base = loaded.light;
    grade.request_stage(1);
    grade.commit();
    grade.set_weight(1.0);
    let lit = grade.light(base);
    assert_eq!(lit.direction, base.direction, "the circuit aims the sun");
    // `"1 Sub Venom.Lighting.Constant Ambient Colour"=1.500000 x3`.
    assert_eq!(lit.ambient, [1.5, 1.5, 1.5]);
}

/// **Only Zone reads it.** A single race on the same title loads no grade at
/// all, which is what keeps a table meant for one mode out of the other three.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_race_outside_zone_loads_no_grade() {
    let Some(loaded) = load(oag_race::Mode::SingleRace) else {
        return;
    };
    assert!(loaded.zone_grade.is_none());
}

/// **The 2048 branch resolves against a real 2048 source, not just against a
/// hand-written path.** Its table sits beside the circuit rather than at a
/// title-wide entry, and its `DEFAULT_TRACK` is spelled with backslashes and a
/// capital `Data` where the shipped entry is lowercase with forward slashes -
/// so this is a check that the sibling rewrite plus the archive's own fold
/// actually reach the file, which is the one thing an HD-only test cannot say.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_2048_branch_resolves_its_per_circuit_table() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007/base");
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        println!("skipping: {} not present", path.display());
        return;
    }
    let mut archives = oag_2048::open(&path.display().to_string()).expect("opening 2048");
    let palette = oag_2048::race::DEFAULTS
        .zone_palette
        .expect("2048 ships a table");
    let name = palette
        .entry_for(oag_2048::race::DEFAULT_TRACK)
        .expect("the circuit's own directory");
    let blob = archives
        .read_name(&name)
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    let text = String::from_utf8(blob).expect("UTF-8");
    let table = oag_tables::effectsettings::EffectSettings::parse(&text).expect("it parses");
    // 2048's own thirteen-stage ladder: HD's fifteen with `Sub Venom` and
    // `Venom` dropped.
    assert_eq!(table.stages.len(), 13);
    assert_eq!(table.stages[&0].name, "Start");
    assert_eq!(table.stages[&12].name, "Supersonic");
}

/// **The recovered ladder moves a real 2048 table, at the recovered zone
/// boundaries, to the colours the disc itself authors.**
///
/// This is the end-to-end check that separates "the plumbing works when driven
/// by hand" from "a race escalates on its own": nothing here picks a stage.
/// Every stage comes from `ZONE_STAGES::stage_for`, which is
/// `Hud_UpdateZoneSpeedClassWidget`'s own `0x11 - i` against the seventeen
/// thresholds read out of `0x8151faf8`.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_2048_zone_race_escalates_on_the_recovered_ladder() {
    let Some(mut archives) = open_2048() else {
        return;
    };
    let palette = oag_2048::race::DEFAULTS
        .zone_palette
        .expect("2048 ships a table");
    let name = palette
        .entry_for(oag_2048::race::DEFAULT_TRACK)
        .expect("the circuit's own directory");
    let text = String::from_utf8(archives.read_name(&name).expect("the table")).expect("UTF-8");
    let table = oag_tables::effectsettings::EffectSettings::parse(&text).expect("it parses");
    let mut grade = race::zone_grade::ZoneGrade::new(
        name,
        table,
        oag_2048::race::DEFAULTS.zone_stages,
        Vec::new(),
        Vec::new(),
    )
    .expect("it names stages");

    // A race *opens* on stage 1, not on `Start`: zone 0 matches the table's
    // last record (threshold `0`), which is class 1.
    assert!(grade.show_zone(0));
    assert_eq!(grade.blend().current, 1);
    let opening = grade.palette();

    // Every band boundary the thresholds state, and nothing between them.
    let boundaries = [
        (0u16, 1u32),
        (1, 1),
        (2, 2),
        (8, 2),
        (9, 3),
        (16, 3),
        (17, 4),
        (33, 5),
        (40, 6),
        (70, 12),
        // Past the file's own last row the clamp holds, exactly as
        // `Zone_UpdateStage` clamps to `0xc` against this same thirteen-stage
        // table.
        (90, 12),
        (200, 12),
    ];
    for (zone, expected) in boundaries {
        grade.show_zone(zone);
        assert_eq!(
            grade.blend().current,
            expected,
            "zone {zone} should show stage {expected}"
        );
    }

    // And the picture really differs, in the file's own numbers. Straight out
    // of `ZoneMode2048.effectSettings`:
    //   "Zone 1 Sub Flash.Fog.Environment Fog Colour"=0.592157 x3 0.001500
    //   "Zone 12 Supersonic.Fog.Environment Fog Colour"
    //       =0.129412 0.968627 1.000000 0.002500
    // The fourth lane is the density this file has no separate key for - see
    // `oag_tables::effectsettings::key::ENVIRONMENT_FOG_COLOUR_2048`.
    assert_eq!(opening.fog_colour, Some([0.592_157; 3]));
    assert_eq!(opening.fog_density, Some(0.001_5));
    grade.show_zone(70);
    let top = grade.palette();
    assert_eq!(top.fog_colour, Some([0.129_412, 0.968_627, 1.0]));
    assert_eq!(top.fog_density, Some(0.002_5));
    assert_ne!(
        (opening.fog_colour, opening.fog_density),
        (top.fog_colour, top.fog_density),
        "stage 1 and stage 12 must not author the same fog"
    );
}

/// The base Vita package, or `None` with a printed reason.
fn open_2048() -> Option<oag_assets::Archives> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007/base");
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        println!("skipping: {} not present", path.display());
        return None;
    }
    Some(oag_2048::open(&path.display().to_string()).expect("opening 2048"))
}

/// **The Zone shader's own parameters assemble off the disc, and they come
/// from the half of HD's paired publication whose textures this build binds.**
///
/// `Texture U/V scale` are the two title-wide, prefix-free keys
/// `Environment_RegisterStageSchema` (`0x003d0b98`) registers straight into
/// shader parameter 52 - `zoneColourTint.xy`. `Track.Texture Colour` is what
/// `Environment_UpdateStageBlend` (`0x003da540`) copies into
/// `zoneEffectInner`/`zoneEffectOuter` **in the block that also binds the
/// `zoneModeTrack*` textures**, which are the ones this build loads. Both are
/// read here off the real file, not from a constant this test carries.
///
/// **Both halves travel, each with its own colours.** `Start` authors the
/// two in opposite corners - `Scene.Texture Colour` pure black against
/// `Track.Texture Colour` at `9.0` - so a regression that filled either
/// group from the other's keys would draw the environment lit or the road
/// dark where the disc draws a dark environment around a lit road, and only
/// this pair catches it. Which chunk reads which is the chunk's own
/// render-block bit, asserted in `crates/rcs/tests/rcsmodel_ground_truth.rs`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_zone_shader_parameters_come_from_the_half_whose_textures_are_bound() {
    let Some(loaded) = load(oag_race::Mode::Zone) else {
        return;
    };
    let mut grade = loaded.zone_grade.expect("the table loads");

    // `Start` is driven to explicitly: a race no longer *opens* on it, because
    // it is the pre-race state and the original visibly is not on it at a
    // start line. It is still the stage worth asserting here - it is where the
    // `Scene`/`Track` halves sit in opposite corners.
    grade.request_stage(0);
    grade.commit();
    grade.set_weight(1.0);
    assert_eq!(grade.blend().current, 0);
    let start = grade.zone_uniform();
    // `"Texture U scale"=1.000000` / `"Texture V scale"=1.000000`, the only
    // two keys in the file with no stage prefix at all.
    assert_eq!(start.uv_scale, [1.0, 1.0], "zoneColourTint.xy off the file");
    assert_eq!(
        start.enabled, 1.0,
        "every input resolved, so the term is switched on"
    );
    // `"0 Start.Track.Texture Colour"=9.000000 9.000000 9.000000`, against
    // `"0 Start.Scene.Texture Colour"=0.000000 0.000000 0.000000`.
    assert_eq!(start.track.effect, [9.0, 9.0, 9.0, 0.0]);
    assert_eq!(
        start.scene.effect,
        [0.0, 0.0, 0.0, 0.0],
        "Start's Scene half is black: a dark environment around a lit road"
    );
    // `"0 Start.Scene.Base Colour"` and its highlight, the scene group's own
    // rim summands, come from the Scene keys and not the Track ones.
    assert_eq!(
        start.scene.base_alt[..3],
        grade.palette().scene_base_colour.unwrap_or([0.0; 3]),
        "the scene group's rim colour is the Scene key"
    );
    assert_eq!(
        start.track.base_alt[..3],
        grade.palette().track_base_colour.unwrap_or([0.0; 3]),
        "the track group's rim colour is the Track key"
    );

    // `"1 Sub Venom.Track.Texture Colour"=4.584567 6.537755 6.917541`,
    // `"1 Sub Venom.Track.EQ brightness"=20.000000`.
    grade.request_stage(1);
    assert!(grade.commit());
    let sub_venom = grade.zone_uniform();
    assert_eq!(sub_venom.uv_scale, [1.0, 1.0]);
    assert_eq!(
        sub_venom.track.effect,
        [4.584_567, 6.537_755, 6.917_541, 20.0],
        "the stage's own Track.Texture Colour, and the disc's own glow drive \
         scalar now that mesh.wgsl's zone_glow draws it"
    );
    // `"1 Sub Venom.Scene.Texture Colour"=0.721569 0.909804 0.964706`,
    // `"1 Sub Venom.Scene.EQ brightness"=20.000000` - the showing stage's
    // own, unblended, on the same terms as the track group below.
    assert_eq!(
        sub_venom.scene.effect,
        [0.721_569, 0.909_804, 0.964_706, 20.0],
        "the scene group's effect is Scene.Texture Colour with Scene.EQ brightness in .w"
    );
    assert_ne!(
        sub_venom.scene.effect[..3],
        sub_venom.track.effect[..3],
        "the two groups author different colours on Sub Venom"
    );

    // **The unblended stage, not the cross-fade.** `commit` zeroes the weight,
    // so `palette()` here reads stage 0's - and the Zone parameters must not,
    // because the original hands the shader both stages separately and picks
    // between them with a sphere test rather than mixing them.
    assert_eq!(grade.blend().weight, 0.0);
    assert_eq!(
        grade.palette().track_texture_colour,
        Some([9.0, 9.0, 9.0]),
        "the colour-space cross-fade is at stage 0's end"
    );
    assert_ne!(
        sub_venom.track.effect[0], 9.0,
        "zoneEffect must read the showing stage itself, not the cross-fade"
    );

    // Every one of the fifteen stages carries a texture and a track colour, so
    // `enabled` never drops - and none of the fifteen is black, which is why
    // an HD Zone race resting on stage 0 still shows something.
    for stage in 0..=grade.last_stage() {
        grade.request_stage(stage);
        grade.commit();
        let uniform = grade.zone_uniform();
        assert_eq!(uniform.enabled, 1.0, "stage {stage} has all three inputs");
        assert_ne!(
            uniform.track.effect[..3],
            [0.0; 3],
            "stage {stage} authors a Track.Texture Colour that draws"
        );
    }
}

/// **A title with no `.effectSettings` Zone table draws no Zone recolour**,
/// and says so rather than reaching for HD's numbers.
///
/// Wipeout Pulse ships no such table, so its Zone race has no grade at all and
/// the scene's Zone uniform stays at its all-zero default - which is the
/// identity on every albedo.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_title_with_no_zone_table_leaves_every_albedo_alone() {
    let Some(path) = image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Zone,
        ..race::Options::default()
    })
    .expect("loading the race");
    assert!(
        loaded.zone_grade.is_none(),
        "Pulse ships no .effectSettings table"
    );
    let zone = oag_mesh::mesh_render::Zone::default();
    assert_eq!(zone.enabled, 0.0, "so nothing is added to any albedo");
}
