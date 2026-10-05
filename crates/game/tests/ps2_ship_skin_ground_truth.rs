//! PS2 Pulse's alternative skins, off the real PS2 disc: a whole sibling atlas.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(ps2_ship_skin_ground_truth)'
//! ```
//!
//! Reported from play: "on PS2 pulse, the craft alternative textures are
//! selectable, but do not work/load (works on PSP pulse)". The PS2 port's
//! `Skin_SwapAtlasSibling` (`0x001dfb70` in `SCES_547.48`) never reads the
//! `.dat`: it maps its file name to a sibling of the hull's one
//! `ALL_Textures.tga` atlas and copies that sibling over it whole. See
//! `docs/ghidra/functions/ps2-pulse-eu/ship-skin.md`.

use std::path::PathBuf;

use oag_mesh::mesh;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-ps2-eu.chd")
}

/// All twelve teams the PS2 disc ships, the roster
/// `ps2_shield_ground_truth.rs` walks.
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

fn rgba(texture: &mesh::ModelTexture) -> &[u8] {
    texture.rgba().expect("a PS2 atlas decodes to RGBA8")
}

/// For every team, the skin file named `skin_file` replaces the hull's atlas
/// with exactly the decoded sibling `sibling` names, and that is a different
/// picture from the atlas the hull wore before.
fn every_team_swaps_in(skin_file: &str, sibling: fn(&str) -> String) {
    let Some(image) = image() else {
        return;
    };
    let mut archives =
        oag_pulse::open(&image.display().to_string()).expect("opening the PS2 archives");
    for team in PS2_TEAMS {
        let hull_name = format!(r"Data\Ships\{team}\Ship.vex");
        let blob = archives
            .read_name(&hull_name)
            .unwrap_or_else(|e| panic!("reading {hull_name}: {e}"));
        let set = mesh::Ps2TextureSet::parse(&archives.read_preceding(&hull_name).unwrap())
            .unwrap_or_else(|e| panic!("{hull_name}: texture set: {e}"));
        let mut hull = mesh::build_with_textures(&hull_name, &blob, Some(&set))
            .unwrap_or_else(|e| panic!("building {hull_name}: {e}"));
        let atlas_at = hull
            .textures
            .iter()
            .position(|slot| {
                slot.as_ref()
                    .is_some_and(|t| mesh::ship_skin::is_ps2_atlas(&t.label))
            })
            .unwrap_or_else(|| panic!("{hull_name}: no ALL_Textures.tga slot"));
        let baseline = hull.textures[atlas_at].clone().unwrap();

        let atlas_path = mesh::ship_skin::ps2_atlas_path(&blob).expect("the atlas node's path");
        let dir = atlas_path.rsplit_once('\\').unwrap().0;
        let stem = atlas_path
            .rsplit('\\')
            .next()
            .unwrap()
            .rsplit_once('.')
            .unwrap()
            .0;
        let expected_name = format!(r"{dir}\{}", sibling(stem));
        let expected = oag_texture::ps2_texture::parse(
            &archives
                .read_name(&expected_name)
                .unwrap_or_else(|e| panic!("{expected_name} is not on the disc: {e}")),
        )
        .unwrap_or_else(|e| panic!("{expected_name} does not decode: {e}"));

        let entry = format!(r"Data\Ships\{team}\{skin_file}");
        let report = oag_game::preview::paint(&mut archives, &entry, &hull_name, &mut hull);
        println!("{team}: {report:?}");
        let painted = hull.textures[atlas_at].as_ref().unwrap();
        assert_eq!(
            (painted.width, painted.height),
            (baseline.width, baseline.height),
            "{team}: {report:?}"
        );
        assert_eq!(
            (u32::from(expected.width), u32::from(expected.height)),
            (baseline.width, baseline.height),
            "{team}: {expected_name} is not the atlas's size"
        );
        assert_eq!(
            rgba(painted),
            expected.to_rgba(),
            "{team}: the atlas is not {expected_name}: {report:?}"
        );
        assert_ne!(
            rgba(painted),
            rgba(&baseline),
            "{team}: {expected_name} is the same picture as the baseline atlas"
        );
    }
}

#[test]
#[ignore = "needs a disc image"]
fn ship_alt_swaps_in_every_teams_livery_atlas() {
    every_team_swaps_in("ship_alt.dat", |_| "livery.pct".to_string());
}

#[test]
#[ignore = "needs a disc image"]
fn ship_eliminator_swaps_in_every_teams_eliminator_atlas() {
    every_team_swaps_in("ship_eliminator.dat", |stem| {
        format!("{stem}_eliminator.pct")
    });
}

/// The selection screen's preview hull takes the same swap: `ship_FE.vex`
/// carries the atlas too, and the original's front end applies the skin to it
/// (`FUN_001bed60`'s call into `Skin_SwapAtlasSibling`). Some preview hulls
/// name the atlas from two nodes; both share one cached image in the original,
/// so the one `memcpy` repaints both, and every such slot is replaced here.
#[test]
#[ignore = "needs a disc image"]
fn the_front_end_preview_hull_is_repainted_too() {
    let Some(image) = image() else {
        return;
    };
    let mut archives =
        oag_pulse::open(&image.display().to_string()).expect("opening the PS2 archives");
    for team in PS2_TEAMS {
        let name = format!(r"Data\Ships\{team}\ship_FE.vex");
        let mut model = oag_game::preview::model(&mut archives, &name)
            .unwrap_or_else(|e| panic!("{name}: {e:#}"));
        let entry = format!(r"Data\Ships\{team}\ship_alt.dat");
        let report = oag_game::preview::paint(&mut archives, &entry, &name, &mut model);
        println!("{team}: {report:?}");
        assert!(
            report
                .iter()
                .any(|line| line.contains("atlas slot(s) repainted")
                    && !line.contains("(0 atlas slot(s)")),
            "{team}: {report:?}"
        );
    }
}
