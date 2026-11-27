//! Every material drawn in an ordinary race on HD/Fury, Omega and 2048, classified by the
//! fragment program it draws with: claimed by a family matcher, drawn by the generic reading,
//! or unread (`docs/rendering/shader-census.md`).
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-mesh --run-ignored all \
//!     -E 'test(/^shader_census_ground_truth::/)' --no-capture
//! ```
//!
//! Each title's `(program key, status)` set is committed beside this file under
//! `shader_census/`. A new program, one that stops being claimed by its matcher, or one that
//! starts being claimed, fails the comparison. `OAG_CENSUS_OUT=<dir>` writes the new set and
//! the full table there to regenerate the baseline and the page.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::PathBuf;

use oag_assets::psarc::Archive;
use oag_mesh::mesh::rcs::census::{self, Material, Outcome};
use oag_rcs::rcsmaterial::{self, fragment, names};
use oag_rcs::{gcn, gxp};

/// Which of a title's models one test walks. Omega is three tests, or one would run past the
/// per-test ceiling.
#[derive(Clone, Copy)]
enum Part {
    All,
    /// `data/environments/`: the circuits of HD lineage.
    HdCircuits,
    /// `data/environments2048/`.
    Circuits2048,
    /// Everything that is not a circuit: craft, weapons, billboards, crowd and furniture.
    Rest,
}

impl Part {
    fn holds(self, path: &str) -> bool {
        let path = format!("/{path}");
        let hd = path.contains("/environments/");
        let new = path.contains("/environments2048/");
        match self {
            Self::All => true,
            Self::HdCircuits => hd,
            Self::Circuits2048 => new,
            Self::Rest => !hd && !new,
        }
    }
}

/// One title's archives as one name space, earlier archives shadowing later ones.
struct Archives {
    open: Vec<Archive>,
    index: BTreeMap<String, (usize, String)>,
    cache: HashMap<String, Option<Vec<u8>>>,
}

fn norm(path: &str) -> String {
    path.trim_start_matches('/').to_ascii_lowercase()
}

impl Archives {
    fn new(open: Vec<Archive>) -> Self {
        let mut index = BTreeMap::new();
        for (n, archive) in open.iter().enumerate() {
            for path in archive.paths() {
                index.entry(norm(path)).or_insert((n, path.clone()));
            }
        }
        Self {
            open,
            index,
            cache: HashMap::new(),
        }
    }

    fn read(&mut self, name: &str) -> Option<Vec<u8>> {
        let key = norm(name);
        if let Some(hit) = self.cache.get(&key) {
            return hit.clone();
        }
        let bytes = self.index.get(&key).and_then(|(n, path)| {
            let path = path.clone();
            self.open[*n].read_path(&path).ok()
        });
        // Material files are asked for again and again; textures once per model.
        if key.ends_with(".rcsmaterial") {
            self.cache.insert(key, bytes.clone());
        }
        bytes
    }

    /// The `.rcsmodel`s an ordinary race draws.
    fn race_models(&self) -> Vec<String> {
        self.index
            .keys()
            .filter(|p| p.ends_with(".rcsmodel") && in_a_race(p))
            .cloned()
            .collect()
    }
}

/// What an ordinary race loads: not the front end, not Zone's or Detonator's own craft,
/// circuits and furniture, not the occluder meshes that are never drawn.
fn in_a_race(path: &str) -> bool {
    let path = format!("/{path}");
    ![
        "/fe/",
        "/environments/zone_",
        "/ships/zone",
        "/hdships/zone",
        "/ships/detonator",
        "/hdships/detonator",
        "/zone_battle",
        "/new_mode_pads/",
        "/pvsblocker/",
        "/aurora/",
        "/environments/zone",
        "/dlc1/environments/zone_",
    ]
    .iter()
    .any(|skip| path.contains(skip))
}

/// Which part of a race a model belongs to: its circuit, or `craft`, `weapons`, ...
fn place(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    let after = |name: &str| {
        parts
            .iter()
            .position(|p| *p == name)
            .and_then(|i| parts.get(i + 1))
    };
    if let Some(circuit) = after("environments").or_else(|| after("environments2048")) {
        return (*circuit).to_string();
    }
    if path.contains("ships/") {
        return "craft".to_string();
    }
    parts
        .iter()
        .find(|p| !matches!(**p, "data" | "art" | "published" | "dlc1"))
        .map_or_else(String::new, |p| (*p).to_string())
}

/// A material instance's status.
fn status(material: &Material, has_file: bool) -> String {
    let families = |wanted: Outcome| -> Vec<&str> {
        material
            .claims
            .iter()
            .filter(|(_, o)| *o == wanted)
            .map(|(f, _)| *f)
            .collect()
    };
    let drawn = families(Outcome::Drawn);
    let not = families(Outcome::NotDrawn);
    if !drawn.is_empty() {
        format!("matcher:{}", drawn.join("+"))
    } else if !not.is_empty() {
        format!("unread:{}", not.join("+"))
    } else if material.key.is_none() {
        let why = if has_file { "no-variant" } else { "no-file" };
        format!("unread:{why}")
    } else {
        "generic".to_string()
    }
}

#[derive(Default)]
struct Row {
    chunks: usize,
    materials: BTreeSet<String>,
    places: BTreeMap<String, usize>,
    /// An example `.rcsmaterial` path and, on HD, its block offset.
    example: String,
    reads: String,
}

type Rows = BTreeMap<(String, String), Row>;

fn key_of(material: &Material) -> String {
    material.key.map_or_else(
        || format!("nokey:{}", stem(&material.name)),
        |k| format!("{k:016x}"),
    )
}

fn stem(name: &str) -> String {
    name.rsplit('/')
        .next()
        .unwrap_or(name)
        .trim_end_matches(".rcsmaterial")
        .to_ascii_lowercase()
}

/// What the program declares it reads, for the page's ranked table.
fn reads(title: &str, file: &[u8], block: Option<usize>) -> String {
    let mut names: BTreeSet<String> = BTreeSet::new();
    match title {
        "hd" => {
            if let Some(d) = block.and_then(|at| rcsmaterial::Declared::parse(file, at)) {
                names.extend(d.samplers.iter().map(|&(h, _)| {
                    names::sampler_name(h).map_or_else(|| format!("tex:{h:08x}"), str::to_string)
                }));
                names.extend(d.parameters.iter().map(|&h| {
                    names::parameter_name(h).map_or_else(|| format!("{h:08x}"), str::to_string)
                }));
            }
            let _ = block.map(|at| fragment::Program::parse(file, at));
        }
        "omega" => {
            for (_, program) in gcn::scan(file).iter() {
                let Ok(program) = program else { continue };
                if program.stage != gcn::Stage::Pixel {
                    continue;
                }
                if let Ok(r) = &program.reflection {
                    names.extend(r.samplers.iter().map(|s| s.name.clone()));
                    names.extend(r.constants.iter().map(|c| c.name.clone()));
                }
            }
        }
        _ => {
            for (_, program) in gxp::programs(file) {
                let Ok(program) = program else { continue };
                if program.is_fragment() {
                    names.extend(program.parameters.iter().map(|p| p.name.clone()));
                }
            }
        }
    }
    names
        .into_iter()
        .filter(|n| !n.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn census(title: &str, mut archives: Archives, part: Part) -> (Rows, usize) {
    let models: Vec<String> = archives
        .race_models()
        .into_iter()
        .filter(|p| part.holds(p))
        .collect();
    let count = models.len();
    let mut rows = Rows::new();
    for path in models {
        let blob = archives.read(&path).unwrap_or_else(|| panic!("{path}"));
        let materials = if title == "hd" {
            let (materials, report) = census::hd(&blob, &mut |n| archives.read(n))
                .unwrap_or_else(|e| panic!("{path}: {e:#}"));
            let with_program: usize = materials
                .iter()
                .filter(|m| m.block.is_some())
                .map(|m| m.chunks)
                .sum();
            assert_eq!(
                with_program, report.variant_chunks,
                "{path}: census chunks are the variant resolution's"
            );
            materials
        } else {
            census::psp2(&path, &blob, &mut |n| archives.read(n))
                .unwrap_or_else(|e| panic!("{path}: {e:#}"))
                .0
        };
        let here = place(&path);
        for material in materials {
            let has_file = archives.read(&material.name).is_some();
            let key = (key_of(&material), status(&material, has_file));
            let row = rows.entry(key).or_default();
            row.chunks += material.chunks;
            row.materials.insert(stem(&material.name));
            *row.places.entry(here.clone()).or_default() += material.chunks;
            if row.example.is_empty() {
                row.example = match material.block {
                    Some(at) => format!("{} @{at:#x}", norm(&material.name)),
                    None => norm(&material.name),
                };
                if let Some(file) = archives.read(&material.name) {
                    row.reads = reads(title, &file, material.block);
                }
            }
        }
    }
    (rows, count)
}

fn baseline_text(rows: &Rows) -> String {
    rows.keys()
        .map(|(key, status)| format!("{key}\t{status}\n"))
        .collect()
}

fn table(rows: &Rows) -> String {
    let mut sorted: Vec<_> = rows.iter().collect();
    sorted.sort_by(|a, b| b.1.chunks.cmp(&a.1.chunks).then(a.0.cmp(b.0)));
    let mut out = String::from("key\tstatus\tchunks\tmaterials\tplaces\texample\treads\n");
    for ((key, status), row) in sorted {
        let mut places: Vec<_> = row.places.iter().collect();
        places.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        let _ = writeln!(
            out,
            "{key}\t{status}\t{}\t{}\t{}\t{}\t{}",
            row.chunks,
            row.materials.iter().cloned().collect::<Vec<_>>().join(","),
            places
                .iter()
                .map(|(p, n)| format!("{p}:{n}"))
                .collect::<Vec<_>>()
                .join(","),
            row.example,
            row.reads,
        );
    }
    out
}

fn check(
    title: &str,
    archives: Archives,
    (name, part): (&str, Part),
    expected_models: usize,
    baseline: &str,
) {
    let (rows, models) = census(title, archives, part);
    let total: usize = rows.values().map(|r| r.chunks).sum();
    let share = |prefix: &str| -> usize {
        rows.iter()
            .filter(|((_, s), _)| s.starts_with(prefix))
            .map(|(_, r)| r.chunks)
            .sum()
    };
    println!(
        "{name}: {models} models, {} (key, status) rows, {total} chunks: {} matcher, {} generic, {} unread",
        rows.len(),
        share("matcher"),
        share("generic"),
        share("unread"),
    );
    if let Some(dir) = std::env::var_os("OAG_CENSUS_OUT") {
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).expect("the output directory");
        std::fs::write(dir.join(format!("{name}.tsv")), baseline_text(&rows)).expect("write");
        std::fs::write(dir.join(format!("{name}-table.tsv")), table(&rows)).expect("write");
    }
    assert_eq!(
        models, expected_models,
        "{name}: models an ordinary race draws"
    );
    let got: BTreeSet<&str> = Box::leak(baseline_text(&rows).into_boxed_str())
        .lines()
        .collect();
    let want: BTreeSet<&str> = baseline.lines().filter(|l| !l.is_empty()).collect();
    let new: Vec<_> = got.difference(&want).take(20).collect();
    let gone: Vec<_> = want.difference(&got).take(20).collect();
    assert!(
        new.is_empty() && gone.is_empty(),
        "{name}: (program, status) differs from shader_census/{name}.tsv\n\
         now and not in the baseline: {new:#?}\nin the baseline and not now: {gone:#?}"
    );
}

fn hd_archives() -> Option<Archives> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    Some(Archives::new(
        (0..7)
            .map(|n| {
                let spec = format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", image.display());
                Archive::open(&spec).expect("the archive opens")
            })
            .collect(),
    ))
}

fn loose(paths: &[&str]) -> Option<Archives> {
    let mut open = Vec::new();
    for path in paths {
        let path = oag_testdata::exact(path)?;
        open.push(Archive::open_file(&path).expect("the archive opens"));
    }
    Some(Archives::new(open))
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn hd_race_materials_are_the_baselines_programs() {
    let Some(archives) = hd_archives() else {
        return;
    };
    check(
        "hd",
        archives,
        ("hd", Part::All),
        499,
        include_str!("shader_census/hd.tsv"),
    );
}

/// The patch's archives shadow the base package's, newest first.
fn omega_check(name: &str, part: Part, expected_models: usize, baseline: &str) {
    let Some(archives) = loose(&[
        "data/extracted/ps4/omega-eu-patch/uroot/data09.psarc",
        "data/extracted/ps4/omega-eu-patch/uroot/data08.psarc",
        "data/extracted/ps4/omega-eu-patch/uroot/data07.psarc",
        "data/extracted/ps4/omega-eu-patch/uroot/data05.psarc",
        "data/extracted/ps4/omega-eu/uroot/data00.psarc",
        "data/extracted/ps4/omega-eu/uroot/data01.psarc",
        "data/extracted/ps4/omega-eu/uroot/data02.psarc",
        "data/extracted/ps4/omega-eu/uroot/data03.psarc",
        "data/extracted/ps4/omega-eu/uroot/data04.psarc",
    ]) else {
        return;
    };
    check("omega", archives, (name, part), expected_models, baseline);
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu and omega-eu-patch"]
fn omega_hd_circuit_materials_are_the_baselines_programs() {
    omega_check(
        "omega-hd-circuits",
        Part::HdCircuits,
        36,
        include_str!("shader_census/omega-hd-circuits.tsv"),
    );
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu and omega-eu-patch"]
fn omega_2048_circuit_materials_are_the_baselines_programs() {
    omega_check(
        "omega-2048-circuits",
        Part::Circuits2048,
        50,
        include_str!("shader_census/omega-2048-circuits.tsv"),
    );
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu and omega-eu-patch"]
fn omega_craft_and_furniture_materials_are_the_baselines_programs() {
    omega_check(
        "omega-rest",
        Part::Rest,
        1018,
        include_str!("shader_census/omega-rest.tsv"),
    );
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn w2048_race_materials_are_the_baselines_programs() {
    // The 1.04 patch is mounted over the base package; DLC adds its own paths.
    let Some(archives) = loose(&[
        "data/extracted/vita/PCSF00007/patch-v104/PSP2/data2.psarc",
        "data/extracted/vita/PCSF00007/patch-v104/PSP2/data1.psarc",
        "data/extracted/vita/PCSF00007/base/PSP2/data.psarc",
        "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
        "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
    ]) else {
        return;
    };
    check(
        "2048",
        archives,
        ("2048", Part::All),
        975,
        include_str!("shader_census/2048.tsv"),
    );
}
