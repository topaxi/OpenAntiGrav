//! Wipeout 2048's `alpha_emissive` families add the picture's alpha times `glowtinrt` after the
//! light, as HD's do.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. Run it with `just test-data`.
//!
//! Every lit fragment program of `diffuse_emissive_alpha`, `diffuse_emissive_alpha_glow`,
//! `mr_diffuse_emissive_alpha_glow_specular` and `dn_diffuse_specular_emiss` that declares
//! `glowtinrt` ends `mad.f16 _, texel.wwww, glowtinrt.xyzw, lit`
//! (`scripts/vita-alpha-emissive-sweep.py`: 171 of 171; `docs/formats/rcsmaterial.md`, "The
//! picture's alpha times a colour"). The loader resolves such a material to a glow-table entry
//! of `PROGRAM_PICTURE_ALPHA_EMISSIVE` carrying the authored colour, and a material that binds
//! the sampler without authoring the colour (Sol 2's `dn_diffuse_specular_emiss_glow`) gets none.
//! Drop `psp2::alpha_emissive::plan` and every assertion here fails.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use oag_mesh::mesh::rcs::psp2;
use oag_mesh::mesh::{Model, PROGRAM_PICTURE_ALPHA_EMISSIVE, slots};

fn package(rel: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007")
        .join(rel);
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

fn build(rel: &str, track: &str) -> Option<(Model, psp2::Report)> {
    let package = package(rel)?;
    let mut archive =
        oag_assets::psarc::Archive::open(package.to_str().expect("utf-8")).expect("opens");
    let model = archive.read_path(track).expect("model");
    Some(psp2::build("circuit", &model, None, &mut |p| archive.read_path(p).ok()).expect("builds"))
}

/// The first vertex each non-empty draw's indices reach.
fn first_vertices(model: &Model) -> impl Iterator<Item = oag_mesh::mesh::GpuVertex> + '_ {
    model
        .draws
        .iter()
        .filter(|d| !d.range.is_empty())
        .map(|d| model.vertices[model.indices[d.range.start as usize] as usize])
}

/// The authored `glowtinrt` of every `alpha_emissive` material of Queens Mall's circuit model
/// (`diffuse_emissive_alpha_glow` x4 submeshes, `diffuse_emissive_alpha` x4), as the census of
/// the model's material table reads them.
const MALL_TINTS: [[f32; 3]; 3] = [
    [1.0, 0.033_660_89, 0.001_626_968_4],
    [0.246_704_1, 0.173_706_05, 0.052_734_375],
    [1.0, 0.704_101_56, 0.213_867_19],
];

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn queens_malls_alpha_emissive_materials_add_their_authored_colour() {
    let Some((model, report)) = build(
        "base/PSP2/data.psarc",
        "data/art/published/environments/mall/track.rcsmodel",
    ) else {
        return;
    };
    let entries: BTreeSet<[u32; 3]> = model
        .emissive
        .iter()
        .filter(|e| e.program[0] == PROGRAM_PICTURE_ALPHA_EMISSIVE)
        .map(|e| e.tint.map(f32::to_bits))
        .collect();
    let want: BTreeSet<[u32; 3]> = MALL_TINTS.iter().map(|t| t.map(f32::to_bits)).collect();
    assert_eq!(entries, want, "{}", report.describe());
    let users = first_vertices(&model)
        .filter(|v| {
            let index = slots::material_index(v.slots) as usize;
            index != 0
                && model
                    .emissive
                    .get(index - 1)
                    .is_some_and(|e| e.program[0] == PROGRAM_PICTURE_ALPHA_EMISSIVE)
        })
        .count();
    assert_eq!(users, 8, "{}", report.describe());
}

/// A lightmapped submesh takes the term too: its program (`#14`, `#16`, `#20`, `#22` of
/// `diffuse_emissive_alpha_glow`) carries the same `mad`, so the role is not dropped where
/// `SECOND_IS_LIGHTMAP` is set.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_lightmapped_alpha_emissive_submesh_carries_the_entry() {
    let Some((model, report)) = build(
        "base/PSP2/data.psarc",
        "data/art/published/environments/park/track.rcsmodel",
    ) else {
        return;
    };
    let lightmapped = first_vertices(&model)
        .filter(|v| v.slots & slots::SECOND_IS_LIGHTMAP != 0)
        .filter(|v| {
            let index = slots::material_index(v.slots) as usize;
            index != 0
                && model
                    .emissive
                    .get(index - 1)
                    .is_some_and(|e| e.program[0] == PROGRAM_PICTURE_ALPHA_EMISSIVE)
        })
        .count();
    assert!(lightmapped > 0, "{}", report.describe());
}

/// Sol 2's `dn_diffuse_specular_emiss_glow` binds `alpha_emissive` and authors no `glowtinrt`:
/// its program does not declare the uniform, so it gets no entry.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_material_that_authors_no_glowtinrt_gets_no_entry() {
    let Some((model, report)) = build(
        "dlc1/PSP2/dlc1.psarc",
        "data/art/published/DLC1/environments/Sol_2/track.rcsmodel",
    ) else {
        return;
    };
    assert!(
        model
            .emissive
            .iter()
            .all(|e| e.program[0] != PROGRAM_PICTURE_ALPHA_EMISSIVE),
        "{}",
        report.describe()
    );
}

/// Omega re-ships the families: its GCN pixel programs of the same materials multiply the
/// sample's alpha by `glowtinrt` and add it to the lit sum (249 of 250 programs plain, the
/// 250th folding the colour into the alpha-weighted specular), so Metropia's glow rows load the
/// same way.
#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn omegas_metropia_alpha_emissive_materials_add_their_authored_colour() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/ps4/omega-eu/uroot");
    if !root.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            root.display()
        );
        return;
    }
    let mut archives: Vec<_> = ["data00.psarc", "data01.psarc", "data02.psarc"]
        .iter()
        .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
        .collect();
    let blob = archives
        .iter_mut()
        .find_map(|a| {
            a.read_path("Data/environments/02_metropia/track.final.rcsmodel")
                .ok()
        })
        .expect("Metropia's model is in a base archive");
    let (model, report) = psp2::build("metropia", &blob, None, &mut |p| {
        archives.iter_mut().find_map(|a| a.read_path(p).ok())
    })
    .expect("builds");
    let entries = model
        .emissive
        .iter()
        .filter(|e| e.program[0] == PROGRAM_PICTURE_ALPHA_EMISSIVE)
        .count();
    assert_eq!(entries, 6, "{}", report.describe());
    assert_eq!(report.alpha_emissive_materials, 18, "{}", report.describe());
    // `dn_diffuse_specular_emiss` lists its `SpecularTexture` first, and the build draws the
    // first non-lightmap sampler as the picture where no programs are read (Omega), so the
    // alpha of the drawn texel is the specular map's: those 14 stay with the generic reading.
    // The count may shrink, never grow.
    assert!(report.alpha_emissive_unread <= 14, "{}", report.describe());
}
