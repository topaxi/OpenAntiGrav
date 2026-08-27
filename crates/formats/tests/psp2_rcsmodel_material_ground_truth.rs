//! Validates [`rcsmodel::psp2`](oag_formats::rcsmodel::psp2)'s texture
//! coordinate decode and material table reader against every `.rcsmodel`
//! Wipeout 2048 ships - the corpus-wide argument
//! `psp2_rcsmodel_ground_truth.rs` already makes for the geometry, extended
//! to the two axes `docs/formats/2048-rcsmodel.md` records as newly read:
//! the diffuse texture coordinate and the material table.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```

use std::path::{Path, PathBuf};

use oag_formats::rcsmodel::psp2;

const PACKAGES: [&str; 3] = [
    "vita/PCSF00007/base/PSP2/data.psarc",
    "vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

fn package(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted")
        .join(name);
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

fn every_model(mut check: impl FnMut(&str, &psp2::Model)) -> usize {
    let mut files = 0usize;
    for name in PACKAGES {
        let Some(path) = package(name) else { continue };
        let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
            .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
        let mut entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmodel"))
            .cloned()
            .collect();
        entries.sort();
        for entry in entries {
            let blob = archive
                .read_path(&entry)
                .unwrap_or_else(|e| panic!("reading {entry}: {e}"));
            let decoded = psp2::parse(&blob).unwrap_or_else(|e| panic!("{entry}: {e}"));
            files += 1;
            check(&entry, &decoded);
        }
    }
    files
}

/// Every `Uv1`-declared submesh decodes a texcoord per vertex, in the same
/// proportion the census that found the attribute measured.
///
/// The real confirmation is the index-exact oracle against Wipeout HD's own
/// diffuse UV (`crates/game/examples/vita_rcsmodel_uv_oracle.rs`, 1,432
/// vertices, mean squared distance indistinguishable from zero for `f16`
/// against 0.41 - chance level - for `unorm16`). What this test adds is
/// reach: every file in the corpus, not just the twelve HD-ported circuits.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn texcoords_decode_for_the_same_fraction_the_census_found() {
    let mut submeshes = 0usize;
    let mut with_texcoords = 0usize;
    let mut finite = 0usize;
    let mut substituted = 0usize;
    let mut total_vertices = 0usize;
    let files = every_model(|name, decoded| {
        for mesh in &decoded.submeshes {
            submeshes += 1;
            if mesh.texcoords.is_empty() {
                continue;
            }
            with_texcoords += 1;
            assert_eq!(
                mesh.texcoords.len(),
                mesh.positions.len(),
                "{name}: a submesh with a texcoord has one per vertex, not a subset"
            );
            substituted += mesh.non_finite_texcoords;
            for uv in &mesh.texcoords {
                total_vertices += 1;
                // psp2::one() substitutes the origin for a non-finite
                // decode before it ever reaches SubMesh::texcoords - see
                // SubMesh::non_finite_texcoords, counted separately below.
                assert!(
                    uv[0].is_finite() && uv[1].is_finite(),
                    "{name}: a non-finite texcoord reached SubMesh::texcoords \
                     rather than being substituted"
                );
                finite += 1;
            }
        }
    });
    if files == 0 {
        return;
    }
    println!(
        "{with_texcoords}/{submeshes} submeshes carry a decoded Uv1 ({:.1}%), \
         {finite}/{total_vertices} finite ({substituted} substituted)",
        100.0 * with_texcoords as f64 / submeshes as f64
    );
    assert_eq!(files, 993, "the three EU packages ship 993 .rcsmodel");
    // The census (vita_rcsmodel_texture_probe.rs) measured 92.5%; assert the
    // same order of magnitude rather than the exact figure, so an unrelated
    // change to submesh acceptance does not make this test flaky.
    assert!(
        with_texcoords * 100 >= submeshes * 85,
        "only {with_texcoords} of {submeshes} submeshes carry a texcoord"
    );
    // Every texcoord SubMesh hands out is finite by construction (asserted
    // above), but not every one is a *real* decode: a stride is shared by
    // every chunk that declares it, and two chunks can share a stride while
    // packing genuinely different attributes into it - `find_by_stride`
    // picks one declaration per stride (preferring one that names `Uv1`),
    // so a chunk whose own layout disagrees decodes noise at that offset,
    // substituted with the origin rather than reaching a GPU buffer as
    // NaN/Inf. Measured at ~21% substituted - asserted as a ceiling well
    // above that, so an unrelated corpus change does not make this test
    // flaky, but a regression that poisoned most of a file would still
    // fail it. See `docs/formats/2048-rcsmodel.md`.
    assert!(
        substituted * 100 <= total_vertices * 40,
        "{substituted} of {total_vertices} decoded texcoords needed substituting - \
         more than expected"
    );
}

/// Every material the table names resolves to real paths, corpus-wide.
///
/// [`psp2::material::read`] refuses a header whose name does not end
/// `.rcsmaterial` and a sampler whose texture does not end `.gxt` - so a
/// material that is read at all is one whose pointers were followed
/// correctly, the same argument [`psp2::parse`]'s section arithmetic makes
/// for the container itself.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn every_material_resolves_to_real_paths() {
    let mut files_with_materials = 0usize;
    let mut single_material = 0usize;
    let mut materials = 0usize;
    let mut with_diffuse = 0usize;
    let files = every_model(|name, decoded| {
        if decoded.materials.is_empty() {
            return;
        }
        files_with_materials += 1;
        if decoded.has_one_material() {
            single_material += 1;
        }
        for material in &decoded.materials {
            materials += 1;
            assert!(
                material.name.to_ascii_lowercase().ends_with(".rcsmaterial"),
                "{name}: material name {:?} does not end .rcsmaterial",
                material.name
            );
            if let Some(texture) = material.diffuse_texture() {
                with_diffuse += 1;
                assert!(
                    texture.to_ascii_lowercase().ends_with(".gxt"),
                    "{name}: diffuse texture {texture:?} does not end .gxt"
                );
            }
            for texture in &material.textures {
                assert!(
                    texture.to_ascii_lowercase().ends_with(".gxt"),
                    "{name}: material texture {texture:?} does not end .gxt"
                );
            }
        }
    });
    if files == 0 {
        return;
    }
    println!(
        "{files_with_materials}/{files} files carry a material table, \
         {single_material} single-material, {materials} material(s) total, \
         {with_diffuse} with a diffuse texture"
    );
    assert_eq!(files, 993, "the three EU packages ship 993 .rcsmodel");
    // Measured 950/993 files with a readable table, 522 single-material and
    // 34,388 materials total (a handful of large circuits carry hundreds
    // each) - assert order of magnitude, not the exact counts, for the same
    // reason as above.
    assert!(
        files_with_materials > 900,
        "only {files_with_materials} files carry a material table"
    );
    assert!(
        single_material * 100 >= files_with_materials * 50,
        "only {single_material} of {files_with_materials} are single-material"
    );
    // Measured 83.1% - the rest are materials whose input table this
    // reading's word-aligned scan does not reach a `.gxt` from within (a
    // lookup-only material, or one whose texture pointer sits at an offset
    // this scan's own extent bound misses). See `psp2::material`'s module
    // doc.
    assert!(
        with_diffuse * 100 >= materials * 75,
        "only {with_diffuse} of {materials} materials resolve a diffuse texture"
    );
}

/// Every submesh names a material this reading can resolve, and the *pattern*
/// of which ones it names is what says the field is a material index rather
/// than some other small number.
///
/// **The control group is the argument.** `psp2::material`'s module doc
/// records the sweep this pins: nineteen offsets in a `-0x60..+0x80` window
/// satisfy "always in range, always zero on a one-material model", and
/// eighteen of them average 3.2 or fewer distinct values per multi-material
/// model. This one averages 80.3. So the assertions below are not "the value
/// parsed" - they are the three conditions together, of which the third is the
/// only one an accidentally-small field cannot also pass.
///
/// Measured 2026-08-27 over the three EU packages: 244,889 submeshes, 0 out of
/// range, 0 non-zero on a one-material model, 86 of 414 multi-material models
/// naming every material they declare.
#[test]
#[ignore = "needs the decrypted Vita packages in data/extracted/vita/"]
fn every_submesh_names_a_material_and_multi_material_models_use_their_whole_table() {
    let mut submeshes = 0usize;
    let mut unresolved = 0usize;
    let mut single_material_nonzero = 0usize;
    let mut multi_models = 0usize;
    let mut full_coverage = 0usize;
    let mut distinct_total = 0usize;

    let files = every_model(|name, model| {
        if model.materials.is_empty() || model.submeshes.is_empty() {
            return;
        }
        let mut used = std::collections::BTreeSet::new();
        for submesh in &model.submeshes {
            submeshes += 1;
            match submesh.material {
                // `psp2::parse` already drops an index outside the table, so
                // a `None` here is exactly "the field did not resolve".
                None => unresolved += 1,
                Some(index) => {
                    used.insert(index);
                    if model.materials.len() == 1 && index != 0 {
                        single_material_nonzero += 1;
                    }
                }
            }
        }
        if model.materials.len() > 1 {
            multi_models += 1;
            distinct_total += used.len();
            if used.len() == model.materials.len() {
                full_coverage += 1;
            }
        }
        assert!(
            used.iter().all(|&i| i < model.materials.len()),
            "{name}: a material index outside the table survived parse"
        );
    });
    if files == 0 {
        return;
    }

    let mean = distinct_total as f64 / multi_models.max(1) as f64;
    println!(
        "{submeshes} submeshes, {unresolved} unresolved, {single_material_nonzero} non-zero on a \
         one-material model; {multi_models} multi-material models, {full_coverage} using their \
         whole table, {mean:.1} distinct materials each on average"
    );
    assert_eq!(files, 993, "the three EU packages ship 993 .rcsmodel");
    assert_eq!(submeshes, 244_889, "submeshes across them");
    assert_eq!(unresolved, 0, "submeshes with no resolvable material index");
    assert_eq!(
        single_material_nonzero, 0,
        "a one-material model's submesh named something other than material 0 - \
         the control group that separates this field from every other small integer"
    );
    assert!(
        mean > 50.0,
        "multi-material models average {mean:.1} distinct materials, measured at 80.3 - \
         below about 50 this stops being distinguishable from the eighteen offsets ruled out"
    );
    assert!(
        full_coverage >= 80,
        "only {full_coverage} models name every material they declare, measured at 86"
    );
}

/// The craft a race actually loads, read out in full - the case a human can
/// check by eye, and the one that shows the binding is not a name match.
///
/// `feisar2048\3`'s sixteen submeshes resolve to five of its six materials,
/// and **exactly one** of them takes `2048_ship_glass_dg`. That submesh is
/// the one whose own authored name is `GlassShape` - which this reading never
/// looks at. `psp2::material`'s module doc explains why that distinction is
/// the whole point: matching a submesh's name to a material's is the invented-
/// not-measured failure `CLAUDE.md` forbids, and it is exactly what the index
/// independently agrees with here.
#[test]
#[ignore = "needs the decrypted Vita packages in data/extracted/vita/"]
fn the_raced_crafts_submeshes_resolve_to_a_sensible_set_of_materials() {
    let Some(path) = package(PACKAGES[0]) else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
        .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let blob = archive
        .read_path("data/art/published/Ships/feisar2048/3/ship.rcsmodel")
        .expect("the raced craft");
    let model = psp2::parse(&blob).expect("parses");

    assert_eq!(model.submeshes.len(), 16);
    assert_eq!(model.materials.len(), 6);

    let mut by_material: std::collections::BTreeMap<usize, usize> =
        std::collections::BTreeMap::new();
    for submesh in &model.submeshes {
        *by_material
            .entry(submesh.material.expect("every submesh binds"))
            .or_default() += 1;
    }
    println!("{by_material:?}");
    assert_eq!(by_material.len(), 5, "five of the six materials are used");

    let named = |index: usize| model.materials[index].name.to_ascii_lowercase();
    let glass: Vec<usize> = by_material
        .keys()
        .copied()
        .filter(|&i| named(i).contains("glass"))
        .collect();
    assert_eq!(glass.len(), 1, "one glass material is bound");
    assert_eq!(
        by_material[&glass[0]], 1,
        "and exactly one submesh binds it - the canopy"
    );
    assert!(
        by_material.keys().any(|&i| named(i).contains("paint")),
        "the livery material is bound too"
    );
}
