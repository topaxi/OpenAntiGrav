//! Validates the [`rcsmodel::psp2`](oag_rcs::rcsmodel::psp2) decoder against
//! every `.rcsmodel` Wipeout 2048 ships.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! The container was settled by arithmetic that cannot come out even unless the
//! reading is right, and these tests are that argument over **993 files**:
//!
//! - **every file's sections add up to its own length**, which is what says the
//!   header's section table is read correctly;
//! - **every submesh's index buffer is exactly its own contents**, `count * 2`
//!   bytes rounded up to four, with the count divisible by three;
//! - **every vertex buffer's length divides by its own vertex count**, which is
//!   the only thing that gives a stride - no field states one;
//! - **no index names a vertex outside its own submesh**, over 33 million of
//!   them;
//! - **the diffuse texture coordinate and the material table**
//!   (`docs/formats/2048-rcsmodel.md`) decode for the same fraction of the
//!   corpus the census found, and every submesh names a material.
//!
//! # One walk
//!
//! Decoding the 993 files costs seconds and nextest runs every `#[test]` in its
//! own process, so the corpus claims are one [`Claim`] each and
//! [`every_claim_holds_over_the_corpus`] walks it once, feeding every model to
//! every claim. A claim keeps its own assertion messages, so a failure still
//! names what it was about.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oag_rcs::rcsmodel::psp2;

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

/// What one sweep of the corpus found.
#[derive(Default)]
struct Survey {
    files: usize,
    without_geometry: usize,
    submeshes: usize,
    triangles: usize,
    vertices: usize,
    strides: BTreeMap<usize, usize>,
    unpaired: usize,
}

fn survey(check: &mut impl FnMut(&str, &psp2::Model)) -> Survey {
    let mut out = Survey::default();
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
            out.files += 1;
            if !decoded.has_geometry() {
                out.without_geometry += 1;
            }
            out.submeshes += decoded.submeshes.len();
            out.unpaired += decoded.unpaired_pointers;
            for mesh in &decoded.submeshes {
                out.triangles += mesh.triangle_count();
                out.vertices += mesh.positions.len();
                *out.strides.entry(mesh.stride).or_default() += 1;
            }
            check(&entry, &decoded);
        }
    }
    out
}

/// One property of the decoded corpus: fed every model once, then judged on
/// what it saw.
trait Claim {
    fn observe(&mut self, name: &str, decoded: &psp2::Model);
    fn conclude(&self, found: &Survey);
}

/// Every shipped file decodes, and the decode accounts for the whole file.
///
/// [`psp2::parse`] refuses sections that do not add up, so a file that parses at
/// all is one whose section table was read correctly.
struct SectionsAddUp;

impl Claim for SectionsAddUp {
    fn observe(&mut self, name: &str, decoded: &psp2::Model) {
        assert!(!decoded.sections.is_empty(), "{name}: no sections at all");
        assert!(
            decoded.sections.len() <= 2,
            "{name}: {} sections, and only the CPU/GPU pair is understood",
            decoded.sections.len()
        );
    }

    fn conclude(&self, found: &Survey) {
        println!(
            "{} file(s), {} with no GPU section, {} submesh(es), {} triangle(s), {} vertices",
            found.files, found.without_geometry, found.submeshes, found.triangles, found.vertices
        );
        println!("vertex strides: {:?}", found.strides);
        println!(
            "{} GPU pointer(s) this reading does not account for",
            found.unpaired
        );
        assert_eq!(found.files, 993, "the three EU packages ship 993 .rcsmodel");
        assert!(found.submeshes > 200_000, "{} submeshes", found.submeshes);
    }
}

/// Every submesh's buffers close on its own two counts.
///
/// The index buffer is the check that makes finding records through the
/// relocation table safe: it is `index_count * 2` bytes rounded up to four, and
/// the count divides by three. The vertex buffer's length divides by the vertex
/// count, which is the **only** source for a stride - the format states none.
struct SubmeshesClose;

impl Claim for SubmeshesClose {
    fn observe(&mut self, name: &str, decoded: &psp2::Model) {
        for mesh in &decoded.submeshes {
            assert_eq!(
                mesh.indices.len() % 3,
                0,
                "{name}: {} indices is not a triangle list",
                mesh.indices.len()
            );
            assert!(
                mesh.stride >= 12 && mesh.stride % 4 == 0,
                "{name}: stride {} is not a word-aligned vertex",
                mesh.stride
            );
            for &index in &mesh.indices {
                assert!(
                    usize::from(index) < mesh.positions.len(),
                    "{name}: index {index} names one of {} vertices",
                    mesh.positions.len()
                );
            }
        }
    }

    fn conclude(&self, found: &Survey) {
        // 16 to 64 in steps of four, on the whole corpus. A stride outside that is
        // not forbidden by the format - it would just be new, and worth seeing.
        for stride in found.strides.keys() {
            assert!(
                (16..=64).contains(stride),
                "stride {stride} is outside the range the corpus showed"
            );
        }
    }
}

/// Positions decode as a finite `f32[3]` at each vertex's `+0`.
///
/// Asserted as a **rate** rather than absolutely: 456 of 33,335,682 vertices
/// across the corpus land outside a generous world box, and those are recorded
/// rather than explained - see `docs/formats/2048-rcsmodel.md`. A reading that
/// was wrong about the offset would not miss by 0.0014 %.
#[derive(Default)]
struct PositionsAreFinite {
    sane: usize,
    total: usize,
}

impl Claim for PositionsAreFinite {
    fn observe(&mut self, _: &str, decoded: &psp2::Model) {
        for position in decoded.positions() {
            self.total += 1;
            if position.iter().all(|c| c.is_finite() && c.abs() < 1.0e5) {
                self.sane += 1;
            }
        }
    }

    fn conclude(&self, _: &Survey) {
        let (sane, total) = (self.sane, self.total);
        println!("{sane}/{total} vertices decode as a finite in-range f32[3]");
        assert!(
            sane * 10_000 >= total * 9_999,
            "only {sane} of {total} positions are in range"
        );
    }
}

/// Normals decode as unit vectors - a self-consistency check the whole
/// corpus can run, not just the twelve HD-ported circuits.
///
/// **The real confirmation is elsewhere**: an index-exact vertex
/// correspondence against Wipeout HD (1,504 vertices, zero ambiguity) scores
/// this exact decode - three signed bytes, `byte/127.0`, x/y/z at
/// [`psp2::NORMAL_OFFSET`] - at 100% within 18 degrees, mean dot 0.994,
/// against every other candidate tried at or below chance. See
/// `docs/formats/2048-rcsmodel.md`. What this test adds is reach: the ported
/// circuits are 12 of 993 files, and unit length is a property every decoded
/// normal should have regardless of which file it came from.
#[derive(Default)]
struct NormalsAreUnit {
    unit: usize,
    total: usize,
}

impl Claim for NormalsAreUnit {
    fn observe(&mut self, _: &str, decoded: &psp2::Model) {
        for normal in decoded.normals() {
            self.total += 1;
            let len = (0..3).map(|a| normal[a] * normal[a]).sum::<f32>().sqrt();
            if (len - 1.0).abs() < 0.02 {
                self.unit += 1;
            }
        }
    }

    fn conclude(&self, _: &Survey) {
        let (unit, total) = (self.unit, self.total);
        println!("{unit}/{total} normals are unit-length within 2%");
        assert!(total > 0, "no normals decoded at all");
        assert!(
            unit * 100 >= total * 99,
            "only {unit} of {total} normals are unit-length"
        );
    }
}

/// `tangent` decodes to a real signal, not noise - the corpus-wide version of
/// the check `crates/game/examples/vita_rcsmodel_tangent_bytesearch.rs` ran as
/// a scratch probe. There is no Wipeout HD twin to check `tangent`'s content
/// against (unlike `normal`/`Uv1`), so orthogonality to the already-cracked
/// `normal` at the same vertex is the strongest evidence this field has - see
/// `docs/formats/2048-rcsmodel.md#tangent-is-cracked-too-at-a-lower-confidence`.
///
/// Both floors sit well inside what that page reports (86.5% orthogonal-ish,
/// 82.6% of the fourth byte exactly `+-127`/`-128`, both restricted to the
/// 58.1% of vertices whose `(x, y, z)` passes a unit-length band) so this
/// tolerates ordinary corpus drift without masking a real regression.
///
/// **The control is what makes this evidence rather than a number that could
/// be coincidence**: a deliberately wrong byte assignment (`x` and `y`
/// swapped) has the same vector length by construction, so it passes the
/// same unit-length filter - and then has to fail the *orthogonality* floor,
/// or the floor is not discriminating anything. The bytesearch probe measured
/// this exact permutation at 44.0% orthogonal-ish unrestricted, far below the
/// winning candidate's 71.1%.
#[derive(Default)]
struct TangentsAreOrthogonal {
    unit_total: usize,
    orth_within_20deg: usize,
    sign_at_extreme: usize,
    wrong_unit_total: usize,
    wrong_orth_within_20deg: usize,
}

impl Claim for TangentsAreOrthogonal {
    fn observe(&mut self, _: &str, decoded: &psp2::Model) {
        for mesh in &decoded.submeshes {
            for (tangent, normal) in mesh.tangents.iter().zip(&mesh.normals) {
                let len = (0..3).map(|a| tangent[a] * tangent[a]).sum::<f32>().sqrt();
                if (len - 1.0).abs() < 0.1 {
                    self.unit_total += 1;
                    let unit = [tangent[0] / len, tangent[1] / len, tangent[2] / len];
                    let dot: f32 = (0..3).map(|a| unit[a] * normal[a]).sum();
                    self.orth_within_20deg += usize::from(dot.abs() < 0.342);
                    let w = (tangent[3] * 127.0).round() as i32;
                    self.sign_at_extreme += usize::from(matches!(w, 127 | -127 | -128));
                }

                // CONTROL: x and y swapped. Length is invariant under
                // permutation, so this passes the identical unit-length
                // filter on the identical vertices - only the orthogonality
                // floor can tell the two apart.
                let wrong = [tangent[1], tangent[0], tangent[2]];
                let wrong_len = (0..3).map(|a| wrong[a] * wrong[a]).sum::<f32>().sqrt();
                if (wrong_len - 1.0).abs() < 0.1 {
                    self.wrong_unit_total += 1;
                    let wrong_unit = [
                        wrong[0] / wrong_len,
                        wrong[1] / wrong_len,
                        wrong[2] / wrong_len,
                    ];
                    let wrong_dot: f32 = (0..3).map(|a| wrong_unit[a] * normal[a]).sum();
                    self.wrong_orth_within_20deg += usize::from(wrong_dot.abs() < 0.342);
                }
            }
        }
    }

    fn conclude(&self, _: &Survey) {
        let Self {
            unit_total,
            orth_within_20deg,
            sign_at_extreme,
            wrong_unit_total,
            wrong_orth_within_20deg,
        } = *self;
        println!(
            "{orth_within_20deg}/{unit_total} unit-length tangents orthogonal-ish to normal \
             (within 20deg); {sign_at_extreme}/{unit_total} fourth byte exactly +-1"
        );
        println!(
            "CONTROL (x/y swapped): {wrong_orth_within_20deg}/{wrong_unit_total} orthogonal-ish"
        );
        assert!(unit_total > 1_000_000, "{unit_total} unit-length tangents");
        assert!(
            orth_within_20deg * 100 >= unit_total * 80,
            "only {orth_within_20deg} of {unit_total} orthogonal-ish"
        );
        assert!(
            sign_at_extreme * 100 >= unit_total * 75,
            "only {sign_at_extreme} of {unit_total} read as a clean +-1 sign"
        );
        assert!(
            wrong_unit_total == 0 || wrong_orth_within_20deg * 100 < wrong_unit_total * 80,
            "the x/y-swapped control also clears the 80% floor - it is not discriminating anything"
        );
    }
}

/// Every submesh whose own declaration names a diffuse coordinate decodes one per vertex.
///
/// The real confirmation is the index-exact oracle against Wipeout HD's own
/// diffuse UV (`crates/game/examples/vita_rcsmodel_uv_oracle.rs`, 1,432
/// vertices, mean squared distance indistinguishable from zero for `f16`
/// against 0.41 - chance level - for `unorm16`). What this test adds is
/// reach: every file in the corpus, not just the twelve HD-ported circuits.
#[derive(Default)]
struct TexcoordsDecode {
    submeshes: usize,
    with_texcoords: usize,
    finite: usize,
    substituted: usize,
    total_vertices: usize,
}

impl Claim for TexcoordsDecode {
    fn observe(&mut self, name: &str, decoded: &psp2::Model) {
        for mesh in &decoded.submeshes {
            self.submeshes += 1;
            if mesh.texcoords.is_empty() {
                continue;
            }
            self.with_texcoords += 1;
            assert_eq!(
                mesh.texcoords.len(),
                mesh.positions.len(),
                "{name}: a submesh with a texcoord has one per vertex, not a subset"
            );
            self.substituted += mesh.non_finite_texcoords;
            for uv in &mesh.texcoords {
                self.total_vertices += 1;
                // psp2::one() substitutes the origin for a non-finite
                // decode before it ever reaches SubMesh::texcoords - see
                // SubMesh::non_finite_texcoords, counted separately below.
                assert!(
                    uv[0].is_finite() && uv[1].is_finite(),
                    "{name}: a non-finite texcoord reached SubMesh::texcoords \
                     rather than being substituted"
                );
                self.finite += 1;
            }
        }
    }

    fn conclude(&self, found: &Survey) {
        let Self {
            submeshes,
            with_texcoords,
            finite,
            substituted,
            total_vertices,
        } = *self;
        println!(
            "{with_texcoords}/{submeshes} submeshes carry a decoded Uv1 ({:.1}%), \
             {finite}/{total_vertices} finite ({substituted} substituted)",
            100.0 * with_texcoords as f64 / submeshes as f64
        );
        assert_eq!(found.files, 993, "the three EU packages ship 993 .rcsmodel");
        // Read through each record's own declaration, 80.7% of the corpus's submeshes carry a
        // diffuse coordinate; the rest are Zone models and other geometry whose declaration names
        // none (`psp2_flat_census_ground_truth` pins the per-archive counts). Before 2026-10-27,
        // keyed by stride, 92.5% did - but a quarter of those read the wrong attribute, 21% of
        // vertices decoded noise, and the substituted count was 5.7 million vertices.
        assert!(
            with_texcoords * 100 >= submeshes * 80,
            "only {with_texcoords} of {submeshes} submeshes carry a texcoord"
        );
        // Every texcoord SubMesh hands out is finite by construction (asserted above). Read from
        // the record's own declaration a non-finite value is rare (520 of 27.3 million vertices);
        // a regression back to a shared layout would put it at ~21% again.
        assert!(
            substituted * 10_000 <= total_vertices,
            "{substituted} of {total_vertices} decoded texcoords needed substituting - \
             more than expected"
        );
    }
}

/// Every material the table names resolves to real paths, corpus-wide.
///
/// [`psp2::material::read`] refuses a header whose name does not end
/// `.rcsmaterial` and a sampler whose texture does not end `.gxt` - so a
/// material that is read at all is one whose pointers were followed
/// correctly, the same argument [`psp2::parse`]'s section arithmetic makes
/// for the container itself.
#[derive(Default)]
struct MaterialsResolve {
    files_with_materials: usize,
    single_material: usize,
    materials: usize,
    with_diffuse: usize,
}

impl Claim for MaterialsResolve {
    fn observe(&mut self, name: &str, decoded: &psp2::Model) {
        if decoded.materials.is_empty() {
            return;
        }
        self.files_with_materials += 1;
        if decoded.has_one_material() {
            self.single_material += 1;
        }
        for material in &decoded.materials {
            self.materials += 1;
            assert!(
                material.name.to_ascii_lowercase().ends_with(".rcsmaterial"),
                "{name}: material name {:?} does not end .rcsmaterial",
                material.name
            );
            if let Some(texture) = material.diffuse_texture() {
                self.with_diffuse += 1;
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
    }

    fn conclude(&self, found: &Survey) {
        let Self {
            files_with_materials,
            single_material,
            materials,
            with_diffuse,
        } = *self;
        println!(
            "{files_with_materials}/{} files carry a material table, \
             {single_material} single-material, {materials} material(s) total, \
             {with_diffuse} with a diffuse texture",
            found.files
        );
        assert_eq!(found.files, 993, "the three EU packages ship 993 .rcsmodel");
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
/// naming every material they declare. **Re-measured 2026-09-02** after
/// `oag_rcs::rcsmodel::psp2::SKY_BUFFER_POINTER_GAP` found 2,647 more
/// submeshes the single-gap reading missed entirely (see
/// `docs/formats/2048-rcsmodel.md`): 247,536 submeshes, still 0 out of range
/// and 0 non-zero on a one-material model, and now 428 of 428 multi-material
/// models naming every material they declare - the newly-found submeshes are
/// disproportionately on multi-material models, which is what moves the
/// second number so much further than the first.
#[derive(Default)]
struct SubmeshesNameAMaterial {
    submeshes: usize,
    unresolved: usize,
    single_material_nonzero: usize,
    multi_models: usize,
    full_coverage: usize,
    distinct_total: usize,
}

impl Claim for SubmeshesNameAMaterial {
    fn observe(&mut self, name: &str, model: &psp2::Model) {
        if model.materials.is_empty() || model.submeshes.is_empty() {
            return;
        }
        let mut used = std::collections::BTreeSet::new();
        for submesh in &model.submeshes {
            self.submeshes += 1;
            match submesh.material {
                // `psp2::parse` already drops an index outside the table, so
                // a `None` here is exactly "the field did not resolve".
                None => self.unresolved += 1,
                Some(index) => {
                    used.insert(index);
                    if model.materials.len() == 1 && index != 0 {
                        self.single_material_nonzero += 1;
                    }
                }
            }
        }
        if model.materials.len() > 1 {
            self.multi_models += 1;
            self.distinct_total += used.len();
            if used.len() == model.materials.len() {
                self.full_coverage += 1;
            }
        }
        assert!(
            used.iter().all(|&i| i < model.materials.len()),
            "{name}: a material index outside the table survived parse"
        );
    }

    fn conclude(&self, found: &Survey) {
        let Self {
            submeshes,
            unresolved,
            single_material_nonzero,
            multi_models,
            full_coverage,
            distinct_total,
        } = *self;
        let mean = distinct_total as f64 / multi_models.max(1) as f64;
        println!(
            "{submeshes} submeshes, {unresolved} unresolved, {single_material_nonzero} non-zero on a \
             one-material model; {multi_models} multi-material models, {full_coverage} using their \
             whole table, {mean:.1} distinct materials each on average"
        );
        assert_eq!(found.files, 993, "the three EU packages ship 993 .rcsmodel");
        assert_eq!(submeshes, 247_536, "submeshes across them");
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
}

/// All the corpus claims over one walk of the three packages.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn every_claim_holds_over_the_corpus() {
    let mut claims: Vec<Box<dyn Claim>> = vec![
        Box::new(SectionsAddUp),
        Box::new(SubmeshesClose),
        Box::new(PositionsAreFinite::default()),
        Box::new(NormalsAreUnit::default()),
        Box::new(TangentsAreOrthogonal::default()),
        Box::new(TexcoordsDecode::default()),
        Box::new(MaterialsResolve::default()),
        Box::new(SubmeshesNameAMaterial::default()),
    ];
    let found = survey(&mut |name, decoded| {
        for claim in &mut claims {
            claim.observe(name, decoded);
        }
    });
    if found.files == 0 {
        return;
    }
    for claim in &claims {
        claim.conclude(&found);
    }
}

/// A circuit's positions are world space and a craft's are its own.
///
/// The check that says a caller composes no transform onto either: `altima`'s
/// track model spans the box its `track_col.col` sibling states, and `Assegai`'s
/// hull sits inside a few units of its own origin.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn a_circuit_is_world_space_and_a_craft_is_not() {
    let Some(path) = package(PACKAGES[0]) else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
        .expect("opening the base package");
    let mut bounds = |entry: &str| {
        let blob = archive
            .read_path(entry)
            .unwrap_or_else(|e| panic!("{entry}: {e}"));
        let decoded = psp2::parse(&blob).unwrap_or_else(|e| panic!("{entry}: {e}"));
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for position in decoded.positions() {
            for a in 0..3 {
                lo[a] = lo[a].min(position[a]);
                hi[a] = hi[a].max(position[a]);
            }
        }
        (lo, hi)
    };

    let (lo, hi) = bounds("data/art/published/environments/altima/track.rcsmodel");
    // Altima's own collision file states a box centred near the origin and
    // roughly 2,000 x 770 x 2,500 units across; the render geometry is the
    // scenery too, so it is larger and in the same place.
    assert!(lo[0] < -900.0 && hi[0] > 700.0, "x {}..{}", lo[0], hi[0]);
    assert!(
        hi[1] > 500.0,
        "a circuit reaches well above the road: y {}",
        hi[1]
    );
    assert!(lo[2] < -700.0 && hi[2] > 1400.0, "z {}..{}", lo[2], hi[2]);

    let (lo, hi) = bounds("data/art/published/hdships/Assegai/ship.rcsmodel");
    for a in 0..3 {
        assert!(
            lo[a] > -20.0 && hi[a] < 20.0,
            "a craft is authored about its own origin: axis {a} spans {}..{}",
            lo[a],
            hi[a]
        );
    }
}

/// The craft a race actually loads, read out in full - the case a human can
/// check by eye, and the one that shows the binding is not a name match.
///
/// `feisar2048\3`'s submeshes resolve to five of its six materials, and
/// **exactly one** of them takes `2048_ship_glass_dg`. That submesh is
/// the one whose own authored name is `GlassShape` - which this reading never
/// looks at. `psp2::material`'s module doc explains why that distinction is
/// the whole point: matching a submesh's name to a material's is the invented-
/// not-measured failure `CLAUDE.md` forbids, and it is exactly what the index
/// independently agrees with here.
///
/// **Re-measured 2026-09-02**: `SKY_BUFFER_POINTER_GAP` finds six more
/// submeshes on this same craft (16 to 22) than the single-gap reading did -
/// among them the canopy, which is why "five of six materials" above is now
/// six of six. See `docs/formats/2048-rcsmodel.md`.
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

    assert_eq!(model.submeshes.len(), 22);
    assert_eq!(model.materials.len(), 6);

    let mut by_material: std::collections::BTreeMap<usize, usize> =
        std::collections::BTreeMap::new();
    for submesh in &model.submeshes {
        *by_material
            .entry(submesh.material.expect("every submesh binds"))
            .or_default() += 1;
    }
    println!("{by_material:?}");
    assert_eq!(by_material.len(), 6, "all six materials are used");

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
