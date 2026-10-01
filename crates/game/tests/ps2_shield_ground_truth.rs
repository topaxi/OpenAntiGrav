//! The PS2 shield shell, off the real PS2 disc: textured and additive.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(ps2_shield_ground_truth)'
//! ```
//!
//! Reported from play on 2026-09-17: on the PS2 source the Shield pickup's
//! shell "renders solid and not animated". Two causes, then a third found by
//! a PCSX2 GS dump on 2026-10-01. The shell's one `Texture` node carries no
//! pixels on PS2, so it bound the white 1x1 until
//! `livery::shield::shield_model` learned the external texture set. And the
//! model was the wrong file: `ShipShield_Construct` (`0x00169168`) builds
//! `%s\%sshield.vex` with the literal prefix `extra`, so the PS2 shell is
//! `extrashield.vex` (batches `0x1232`, additive in their own words, a
//! `pulse_shield_extra_ADD` texture), not the PSP's `shipshield.vex`. See
//! `docs/ghidra/functions/ps2-pulse-eu/shield-pickup.md`.

use std::path::PathBuf;

use oag_game::race;
use oag_render::mesh;
use oag_vex::vex::BlendClass;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-ps2-eu.chd")
}

fn load() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// Every team's shell has its texture decoded, and not one of its draws is
/// left in the opaque or cutout lists, where it would hide the craft.
#[test]
#[ignore = "needs a disc image"]
fn every_ps2_shell_is_textured_and_additive() {
    let Some(loaded) = load() else {
        return;
    };
    assert!(!loaded.liveries.is_empty(), "no grid was fielded");
    for livery in &loaded.liveries {
        let shell = livery
            .shield
            .as_ref()
            .unwrap_or_else(|| panic!("{}: no shield shell", livery.team));
        assert!(
            !shell.textures.is_empty() && shell.textures.iter().all(Option::is_some),
            "{}: the shell's texture did not decode, so it binds the white 1x1",
            livery.team
        );
        assert!(
            shell.draws.is_empty() && shell.alpha_tested_draws.is_empty(),
            "{}: {} opaque and {} cutout draw(s) left, so the shell hides the craft",
            livery.team,
            shell.draws.len(),
            shell.alpha_tested_draws.len()
        );
        assert!(
            !shell.transparent_draws.is_empty()
                && shell
                    .transparent_draws
                    .iter()
                    .all(|draw| draw.blend == Some(BlendClass::Additive)),
            "{}: a shell draw is not on the additive class",
            livery.team
        );
    }
}

/// The cockpit sphere already carries `0x200` on PS2, so it only needed its
/// texture - and must come out of the load untouched otherwise.
#[test]
#[ignore = "needs a disc image"]
fn the_ps2_cockpit_sphere_is_textured() {
    let Some(loaded) = load() else {
        return;
    };
    let sphere = loaded
        .shield_cockpit
        .as_ref()
        .expect("vr_shield_cockpit.vex is on the PS2 disc");
    assert!(
        !sphere.textures.is_empty() && sphere.textures.iter().all(Option::is_some),
        "the cockpit sphere's texture did not decode"
    );
    assert!(sphere.draws.is_empty() && sphere.alpha_tested_draws.is_empty());
    assert!(
        sphere
            .transparent_draws
            .iter()
            .all(|draw| draw.blend == Some(BlendClass::Additive))
    );
}

/// All twelve teams the PS2 disc ships, not only the eight a race fields -
/// the other four are reachable from the team picker, and a shell that missed
/// the preceding-entry rule would draw as a white additive wash. The same
/// roster `boost_plume_ground_truth.rs` walks.
const PS2_TEAMS: [&str; 12] = [
    "AG_Systems",
    "Assegai",
    "Auricom",
    "EGX",
    "Feisar",
    "Goteki",
    "Harimau",
    "Icaras",
    "Mantis",
    "Piranha",
    "Qirex",
    "Triakis",
];

/// Every PS2 shell, and the cockpit sphere, is skinned by the entry directly
/// before it: the empty-block signature holds, the set there decodes as many
/// entries as the model declares slots, and rebuilding leaves none empty.
#[test]
#[ignore = "needs a disc image"]
fn every_ps2_shield_model_is_skinned_by_the_entry_before_it() {
    let Some(image) = image() else {
        return;
    };
    let mut archives =
        oag_pulse::open(&image.display().to_string()).expect("opening the PS2 archives");
    let names = PS2_TEAMS
        .iter()
        .map(|team| format!(r"Data\Ships\{team}\extrashield.vex"))
        .chain([oag_pulse::race::COCKPIT_SHIELD.to_string()]);
    for name in names {
        let blob = archives
            .read_name(&name)
            .unwrap_or_else(|e| panic!("reading {name}: {e}"));
        let bare = mesh::build_with_textures(&name, &blob, None)
            .unwrap_or_else(|e| panic!("decoding {name}: {e}"));
        assert!(
            !bare.textures.is_empty() && bare.textures.iter().all(Option::is_none),
            "{name}: not the empty-block PS2 signature the shield's re-skin gates on"
        );
        let preceding = archives
            .read_preceding(&name)
            .unwrap_or_else(|e| panic!("{name}: reading the entry before it: {e}"));
        let set = mesh::Ps2TextureSet::parse(&preceding)
            .unwrap_or_else(|e| panic!("{name}: the entry before it is not a texture set ({e})"));
        assert_eq!(
            set.entry_count(),
            bare.textures.len(),
            "{name}: preceding set and declared slots disagree"
        );
        let skinned = mesh::build_with_textures(&name, &blob, Some(&set))
            .unwrap_or_else(|e| panic!("re-skinning {name}: {e}"));
        assert!(
            skinned.textures.iter().all(Option::is_some),
            "{name}: a slot is still empty after re-skinning"
        );
    }
}

/// The shell is the model the executable names, on every team, and has the
/// shape the GS dump of the original's draw showed: a `u` span of one half
/// texture (the PS2 draw's `u` was `0.114` and `0.614`, a scroll on `{0, 0.5}`),
/// `v` within one texture, and every batch additive by its own class.
#[test]
#[ignore = "needs a disc image"]
fn the_ps2_shell_is_extrashield_and_not_shipshield() {
    let Some(image) = image() else {
        return;
    };
    let mut archives =
        oag_pulse::open(&image.display().to_string()).expect("opening the PS2 archives");
    for team in PS2_TEAMS {
        let names = race::shield_entry_names(
            oag_title::race::SHIP_DIR,
            team,
            oag_assets::Platform::Ps2,
            None,
        );
        assert_eq!(
            names[0],
            format!(r"Data\Ships\{team}\extrashield.vex"),
            "{team}: the PS2 prefix is the literal `extra`"
        );
        let blob = archives
            .read_name(&names[0])
            .unwrap_or_else(|e| panic!("{team}: reading {}: {e}", names[0]));
        let bare = mesh::build_with_textures(&names[0], &blob, None)
            .unwrap_or_else(|e| panic!("{team}: decoding {}: {e}", names[0]));
        assert!(
            bare.draws.is_empty() && bare.alpha_tested_draws.is_empty(),
            "{team}: extrashield.vex names its own blend class on every batch"
        );
        assert!(
            bare.transparent_draws
                .iter()
                .all(|draw| draw.blend == Some(BlendClass::Additive)),
            "{team}: a batch is not on the additive class"
        );
        let (mut max_u, mut max_v) = (0.0_f32, 0.0_f32);
        for vertex in &bare.vertices {
            max_u = max_u.max(vertex.texcoord[0]);
            max_v = max_v.max(vertex.texcoord[1]);
        }
        assert!(
            max_u <= 0.5 + 1e-3 && max_v <= 1.0 + 1e-3,
            "{team}: uv reaches ({max_u}, {max_v}); the original's draw stayed within (0.5, 1)"
        );
    }
}

/// The PS2 source loads the palette that leaves the shell's vertices alone, and
/// a PSP source does not: a GS dump of the raised PS2 shell found its vertex
/// colours at the authored values through the fade-up, the flicker and the
/// fade-out.
#[test]
#[ignore = "needs a disc image"]
fn the_ps2_source_does_not_tint_the_shell() {
    let Some(loaded) = load() else {
        return;
    };
    assert!(!loaded.setup.shield_palette.tints_shell);
    assert_eq!(
        loaded.setup.shield_palette,
        oag_render::shield::PS2_PULSE_PALETTE
    );
}
