//! Every HUD asset a disc ships, by family, and which of them a layout
//! reaches: the census behind `docs/ui/hud-assets.md`.
//!
//! **`#[ignore]`d and never run in CI.** It needs the game images, which this
//! project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-hud --run-ignored all \
//!     -E 'test(/^hud_asset_census_ground_truth::/)'
//! OAG_SWEEP=1 ... prints the tables the doc is built from
//! ```
//!
//! # Counting rule
//!
//! A path is **lower-cased, `\` read as `/`, with a leading `/` and a leading
//! `data/` dropped**, and counted **once however many archives carry it**
//! (base, patch, DLC): Omega ships `Data/xml/2048_hud/` in the patch and
//! `data/xml/2048_hud/` in the base, and they are the same files. Per-archive
//! entry counts are printed beside the union.
//!
//! # What "reached" means
//!
//! Every XML in a HUD directory is composed with [`oag_hud::compose`]. An XML
//! some other XML includes is a **fragment**; one nothing includes is a
//! **root**. Every quoted attribute value in every file read, ending in a
//! texture, font, model or material extension, is a **reference**. A root is
//! **wired** when [`oag_title::Title::hud`] names it, and a texture, font or
//! model is reached when a wired root's closure references it. What is left is
//! reached only by an unwired root, or by nothing at all.
//!
//! The PSP and PS2 WADs store no names, so those titles are walked from the
//! names this repository knows and counted as "found by name" against the
//! archive's entry count; no WAD is ever reported as shipping none.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use oag_title::Title;

/// The shared counting rule: lower case, forward slashes, no leading `/` or `data/`.
fn norm(path: &str) -> String {
    let lower = path.to_lowercase().replace('\\', "/");
    let trimmed = lower.trim_start_matches('/');
    trimmed.strip_prefix("data/").unwrap_or(trimmed).to_string()
}

struct Census {
    paths: BTreeSet<String>,
    /// A path as an archive spells it, by its normalised form: what to hand `read_name`.
    spelled: BTreeMap<String, String>,
    per_archive: Vec<(String, usize)>,
}

fn census(archives: &oag_assets::Archives) -> Census {
    let mut paths = BTreeSet::new();
    let mut spelled = BTreeMap::new();
    let mut per_archive = Vec::new();
    let mut take = |container: &oag_assets::Container| {
        if let oag_assets::Container::Psarc(psarc) = container {
            per_archive.push((container.label().to_string(), psarc.paths().len()));
            for path in psarc.paths() {
                paths.insert(norm(path));
                spelled.entry(norm(path)).or_insert_with(|| path.clone());
            }
        }
    };
    take(&archives.data);
    archives.patch.iter().for_each(&mut take);
    if let Some(fe) = &archives.fe {
        take(fe);
    }
    archives.extra.iter().for_each(&mut take);
    Census {
        paths,
        spelled,
        per_archive,
    }
}

/// The skin and split-screen directories under `xml/`.
const XML_DIRS: &[&str] = &[
    "2097_hud",
    "wo3_hud",
    "2048_hud",
    "duel_hud",
    "splitscreen_hud",
    "splitscreenzone_hud",
];

/// Which family a normalised path belongs to, `None` for everything that is
/// not a HUD asset by location.
fn family(path: &str) -> Option<String> {
    if let Some(rest) = path.strip_prefix("xml/") {
        return match rest.split_once('/') {
            None if rest.contains("hud") && rest.ends_with(".xml") => Some(
                if rest.starts_with("hud_") {
                    "xml/fragments"
                } else {
                    "xml/roots"
                }
                .to_string(),
            ),
            Some((dir, tail)) if XML_DIRS.contains(&dir) => {
                let sub = tail
                    .split_once('/')
                    .map_or("", |(sub, _)| sub)
                    .trim_end_matches('/');
                Some(if sub.is_empty() {
                    format!("xml/{dir}")
                } else {
                    format!("xml/{dir}/{sub}")
                })
            }
            _ => None,
        };
    }
    if let Some(rest) = path.strip_prefix("hud/") {
        let dir = rest.split_once('/').map_or("models", |(d, _)| d);
        return Some(format!("hud/{dir}"));
    }
    if let Some(rest) = path.strip_prefix("fe/fonts/") {
        let stem = rest.split('.').next().unwrap_or("");
        if stem.contains("hud") || stem.starts_with("small") {
            return Some("fe/fonts (HUD)".to_string());
        }
    }
    None
}

fn families(census: &Census) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for path in &census.paths {
        if let Some(family) = family(path) {
            *out.entry(family).or_insert(0) += 1;
        }
    }
    out
}

/// What the layouts say, once every HUD XML is composed.
#[derive(Default)]
struct Closure {
    roots: BTreeSet<String>,
    fragments: BTreeSet<String>,
    /// Every reference, with the roots whose closure carries it.
    refs: BTreeMap<String, BTreeSet<String>>,
    /// References no census path answers.
    dangling: BTreeSet<String>,
    /// Front-end plugin XML files carrying each [`SELECTOR_NEEDLES`] entry.
    selectors: BTreeMap<&'static str, BTreeSet<String>>,
    /// Includes the composer could not read, as `(root, include)`.
    missing: BTreeSet<(String, String)>,
}

/// Marks a reference made by a front-end plugin's XML (`plugins/**.xml`: the
/// font slots in `definition.xml` and `skin.xml`, screens that draw a HUD
/// texture), not by a HUD root.
const FRONT_END: &str = "front-end:";

/// Marks a texture named inside a `hud/` model or material file, found by
/// matching the stem against the file's bytes (the `.vex` and `.rcsmodel`
/// containers are not parsed here).
const MODEL: &str = "model:";

/// Text a plugin carries when it offers or branches on a HUD skin: the
/// `HUD Style` setting, the `PI_HUD` unlock rows and the `WIP3OUT` value.
const SELECTOR_NEEDLES: &[&str] = &["HUD Style", "PI_HUD", "WIP3OUT"];

const REF_EXTENSIONS: &[&str] = &[
    ".gtf",
    ".gnf",
    ".gxt",
    ".mip",
    ".fnt",
    ".vex",
    ".rcsmodel",
    ".rcsmaterial",
];

fn references(bytes: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(bytes);
    text.split('"')
        .skip(1)
        .step_by(2)
        .filter(|v| {
            let lower = v.to_lowercase();
            REF_EXTENSIONS.iter().any(|e| lower.ends_with(e))
        })
        .map(str::to_string)
        .collect()
}

fn closure(
    archives: &mut oag_assets::Archives,
    census: &Census,
    texture_extension: Option<&str>,
) -> Closure {
    let mut out = Closure::default();
    let xml: Vec<String> = census
        .paths
        .iter()
        .filter(|p| {
            p.ends_with(".xml") && matches!(family(p).as_deref(), Some(f) if f.starts_with("xml/"))
        })
        .cloned()
        .collect();
    let mut included = BTreeSet::new();
    for root in &xml {
        let seen = RefCell::new(Vec::<Vec<u8>>::new());
        let composed = {
            let mut read = |path: &str| {
                let bytes = archives.read_name(path).ok()?;
                seen.borrow_mut().push(bytes.clone());
                Some(bytes)
            };
            oag_hud::compose(&census.spelled[root], &mut read)
        };
        let Some(composed) = composed else {
            out.missing.insert((root.clone(), String::new()));
            continue;
        };
        for file in composed.files.iter().skip(1) {
            included.insert(norm(file));
        }
        for include in &composed.missing {
            out.missing.insert((root.clone(), include.clone()));
        }
        for bytes in seen.borrow().iter() {
            for reference in references(bytes) {
                let mut candidates = vec![norm(&reference)];
                if let Some(ext) = texture_extension {
                    candidates.push(norm(&oag_title::hud::replace_extension(&reference, ext)));
                }
                match candidates.iter().find(|c| census.paths.contains(*c)) {
                    Some(hit) => {
                        out.refs
                            .entry(hit.clone())
                            .or_default()
                            .insert(root.clone());
                    }
                    None => {
                        out.dangling.insert(norm(&reference));
                    }
                }
            }
        }
    }
    let declarations: Vec<String> = census
        .paths
        .iter()
        .filter(|p| p.starts_with("plugins/") && p.ends_with(".xml"))
        .cloned()
        .collect();
    for path in declarations {
        for (_, bytes) in archives.read_every_name(&census.spelled[&path]) {
            let lower = String::from_utf8_lossy(&bytes).to_lowercase();
            for needle in SELECTOR_NEEDLES {
                if lower.contains(&needle.to_lowercase()) {
                    out.selectors
                        .entry(needle)
                        .or_default()
                        .insert(path.clone());
                }
            }
            for reference in references(&bytes) {
                let mut candidates = vec![norm(&reference)];
                if let Some(ext) = texture_extension {
                    candidates.push(norm(&oag_title::hud::replace_extension(&reference, ext)));
                }
                if let Some(hit) = candidates.iter().find(|c| census.paths.contains(*c)) {
                    out.refs
                        .entry(hit.clone())
                        .or_default()
                        .insert(format!("{FRONT_END}{path}"));
                }
            }
        }
    }
    let hud_files: Vec<String> = census
        .paths
        .iter()
        .filter(|p| p.starts_with("hud/") && !p.contains("/textures/") && !p.contains("/debug/"))
        .cloned()
        .collect();
    let stems: Vec<(String, String)> = census
        .paths
        .iter()
        .filter(|p| p.starts_with("hud/textures/"))
        .map(|p| {
            let file = p.rsplit('/').next().unwrap_or(p);
            (
                p.clone(),
                file.split('.').next().unwrap_or(file).to_string(),
            )
        })
        .collect();
    for path in hud_files {
        let Ok(bytes) = archives.read_name(&census.spelled[&path]) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes).to_lowercase();
        for (texture, stem) in &stems {
            let found = ["\\", "/", "\0"]
                .iter()
                .any(|lead| text.contains(&format!("{lead}{stem}.")));
            if found {
                out.refs
                    .entry(texture.clone())
                    .or_default()
                    .insert(format!("{MODEL}{path}"));
            }
        }
    }
    let fonts: Vec<(String, BTreeSet<String>)> = out
        .refs
        .iter()
        .filter(|(path, _)| path.ends_with(".fnt"))
        .map(|(path, by)| (path.clone(), by.clone()))
        .collect();
    for (font, by) in fonts {
        let stem = font.trim_end_matches(".fnt");
        for sibling in census.paths.range(format!("{stem}.")..) {
            if !sibling.starts_with(&format!("{stem}.")) {
                break;
            }
            if !sibling.ends_with(".fnt") && !sibling.ends_with(".xml") {
                out.refs
                    .entry(sibling.clone())
                    .or_default()
                    .extend(by.iter().cloned());
            }
        }
    }
    for path in xml {
        if included.contains(&path) {
            out.fragments.insert(path);
        } else {
            out.roots.insert(path);
        }
    }
    out
}

/// Every root a title's [`Title::hud`] names, normalised.
fn wired_roots(title: &Title) -> BTreeSet<String> {
    let hud = title.hud;
    let mut out: BTreeSet<String> = [
        hud.arcade,
        hud.time_trial,
        hud.speed_lap,
        hud.zone,
        hud.elimination,
    ]
    .iter()
    .map(|p| norm(p))
    .collect();
    if let Some(split) = hud.split {
        out.extend(split.arcade.iter().map(|p| norm(p)));
        out.extend(split.elimination.iter().map(|p| norm(p)));
        if let Some(vertical) = split.vertical {
            out.extend(vertical.arcade.iter().map(|p| norm(p)));
            out.extend(vertical.elimination.iter().map(|p| norm(p)));
        }
    }
    out
}

fn sweep() -> bool {
    std::env::var_os("OAG_SWEEP").is_some()
}

/// How a non-XML HUD asset is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum State {
    /// A root `Title::hud` names (or its closure) references it.
    Wired,
    /// Only roots nothing in the engine names reference it.
    UnwiredRoot,
    /// Only a front-end plugin's XML (`plugins/**.xml`) names it.
    Declared,
    /// Only a `hud/` model or material names it (a byte match, not a parse).
    ByModel,
    /// No layout or declaration names it, but a crate's source spells its file name.
    NamedByCode,
    /// Nothing names it.
    Unreferenced,
}

/// Every `crates/*/src/**.rs`, lower-cased, read once per process.
fn crate_sources() -> &'static str {
    static SOURCES: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCES.get_or_init(|| {
        let mut out = String::new();
        let mut stack = vec![oag_testdata::repo_root().join("crates")];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = entry.file_name();
                    if name != "tests" && name != "target" && name != "examples" {
                        stack.push(path);
                    }
                } else if path.extension().is_some_and(|e| e == "rs")
                    && path.to_string_lossy().contains("/src/")
                    && let Ok(text) = std::fs::read_to_string(&path)
                {
                    out.push_str(&text.to_lowercase());
                }
            }
        }
        out
    })
}

/// Whether some crate spells `path`'s file stem as a name (`\stem.`, `/stem.` or `"stem.`).
fn named_by_code(path: &str) -> bool {
    let file = path.rsplit('/').next().unwrap_or(path);
    let stem = file.split('.').next().unwrap_or(file);
    let sources = crate_sources();
    ["\\", "/", "\""]
        .iter()
        .any(|lead| sources.contains(&format!("{lead}{stem}.")))
}

fn state(path: &str, refs: Option<&BTreeSet<String>>, wired: &BTreeSet<String>) -> State {
    let Some(refs) = refs else {
        return if named_by_code(path) {
            State::NamedByCode
        } else {
            State::Unreferenced
        };
    };
    if refs.iter().any(|r| wired.contains(r)) {
        State::Wired
    } else if refs
        .iter()
        .any(|r| !r.starts_with(FRONT_END) && !r.starts_with(MODEL))
    {
        State::UnwiredRoot
    } else if refs.iter().any(|r| r.starts_with(FRONT_END)) {
        State::Declared
    } else {
        State::ByModel
    }
}

/// `(family, state) -> count` over every non-XML HUD asset.
fn asset_states(
    census: &Census,
    closure: &Closure,
    wired: &BTreeSet<String>,
) -> BTreeMap<(String, State), usize> {
    let mut out = BTreeMap::new();
    for path in &census.paths {
        let Some(family) = family(path) else { continue };
        if path.ends_with(".xml") && family.starts_with("xml/") {
            continue;
        }
        *out.entry((family, state(path, closure.refs.get(path), wired)))
            .or_insert(0) += 1;
    }
    out
}

/// What one title's census measured: the numbers the tests pin and the doc quotes.
#[derive(Debug, PartialEq, Eq)]
struct Summary {
    paths: usize,
    families: Vec<(String, usize)>,
    roots: usize,
    fragments: usize,
    /// Roots `Title::hud` names that are in the census.
    wired: usize,
    /// Roots `Title::hud` names that no archive carries.
    wired_absent: Vec<String>,
    dangling: Vec<String>,
    missing: usize,
    /// Plugin files carrying each of [`SELECTOR_NEEDLES`], by count.
    selectors: Vec<(String, usize)>,
    states: Vec<(String, String, usize)>,
}

fn summary(title: &Title, census: &Census, closure: &Closure) -> Summary {
    let named = wired_roots(title);
    let wired = named.iter().filter(|p| census.paths.contains(*p)).count();
    Summary {
        paths: census.paths.len(),
        families: families(census).into_iter().collect(),
        roots: closure.roots.len(),
        fragments: closure.fragments.len(),
        wired,
        wired_absent: named
            .iter()
            .filter(|p| !census.paths.contains(*p))
            .cloned()
            .collect(),
        dangling: closure.dangling.iter().cloned().collect(),
        missing: closure.missing.len(),
        selectors: SELECTOR_NEEDLES
            .iter()
            .map(|n| {
                (
                    (*n).to_string(),
                    closure.selectors.get(n).map_or(0, BTreeSet::len),
                )
            })
            .collect(),
        states: asset_states(census, closure, &named)
            .into_iter()
            .map(|((family, state), n)| (family, format!("{state:?}"), n))
            .collect(),
    }
}

/// What a `Summary` is pinned to.
struct Expect {
    paths: usize,
    families: &'static [(&'static str, usize)],
    roots: usize,
    fragments: usize,
    wired: usize,
    wired_absent: &'static [&'static str],
    dangling: &'static [&'static str],
    selectors: &'static [(&'static str, usize)],
    states: &'static [(&'static str, &'static str, usize)],
}

fn check(name: &str, got: &Summary, want: &Expect) {
    assert_eq!(got.paths, want.paths, "{name}: unique paths");
    let families: Vec<(&str, usize)> = got.families.iter().map(|(f, n)| (f.as_str(), *n)).collect();
    assert_eq!(families, want.families, "{name}: family counts");
    assert_eq!(got.roots, want.roots, "{name}: roots");
    assert_eq!(got.fragments, want.fragments, "{name}: fragments");
    assert_eq!(got.wired, want.wired, "{name}: wired roots present");
    assert_eq!(
        got.wired_absent, want.wired_absent,
        "{name}: wired roots absent"
    );
    assert_eq!(got.dangling, want.dangling, "{name}: dangling references");
    assert_eq!(got.missing, 0, "{name}: every include resolves");
    let selectors: Vec<(&str, usize)> = got
        .selectors
        .iter()
        .map(|(n, c)| (n.as_str(), *c))
        .collect();
    assert_eq!(
        selectors, want.selectors,
        "{name}: plugin files naming a HUD selector"
    );
    let states: Vec<(&str, &str, usize)> = got
        .states
        .iter()
        .map(|(f, s, n)| (f.as_str(), s.as_str(), *n))
        .collect();
    assert_eq!(states, want.states, "{name}: asset states");
}

fn report(name: &str, title: &Title, census: &Census, closure: &Closure) {
    if !sweep() {
        return;
    }
    println!("=== {name}: {} unique paths", census.paths.len());
    for (archive, count) in &census.per_archive {
        println!("archive {count:6} {archive}");
    }
    let wired = wired_roots(title);
    for root in &closure.roots {
        println!(
            "root {} {root}",
            if wired.contains(root) {
                "WIRED"
            } else {
                "-----"
            }
        );
    }
    for fragment in &closure.fragments {
        println!("fragment {fragment}");
    }
    for (root, include) in &closure.missing {
        println!("missing {root} -> {include}");
    }
    for path in &census.paths {
        let Some(f) = family(path) else { continue };
        if f.starts_with("xml/") && path.ends_with(".xml") {
            continue;
        }
        let refs = closure.refs.get(path);
        let n = refs.map_or(0, BTreeSet::len);
        println!(
            "asset {f} {path} {:?} refd_by={n}",
            state(path, refs, &wired)
        );
    }
    for (needle, files) in &closure.selectors {
        println!("selector {needle}: {files:?}");
    }
    println!("summary {:?}", summary(title, census, closure));
}

const HD_FURY: Expect = Expect {
    paths: 11139,
    families: &[
        ("fe/fonts (HUD)", 12),
        ("hud/materials", 4),
        ("hud/models", 20),
        ("hud/textures", 44),
        ("xml/2097_hud", 27),
        ("xml/2097_hud/font", 4),
        ("xml/2097_hud/splitscreen_hud", 23),
        ("xml/2097_hud/texture", 2),
        ("xml/duel_hud", 5),
        ("xml/duel_hud/duel_splitscreen_hud", 5),
        ("xml/fragments", 32),
        ("xml/roots", 8),
        ("xml/splitscreen_hud", 31),
        ("xml/splitscreenzone_hud", 10),
        ("xml/wo3_hud", 27),
        ("xml/wo3_hud/font", 4),
        ("xml/wo3_hud/splitscreen_hud", 23),
        ("xml/wo3_hud/texture", 2),
    ],
    roots: 57,
    fragments: 134,
    wired: 13,
    wired_absent: &[],
    dangling: &[],
    selectors: &[("HUD Style", 4), ("PI_HUD", 1), ("WIP3OUT", 18)],
    states: &[
        ("fe/fonts (HUD)", "Declared", 12),
        ("hud/materials", "Unreferenced", 4),
        ("hud/models", "Wired", 4),
        ("hud/models", "UnwiredRoot", 6),
        ("hud/models", "NamedByCode", 4),
        ("hud/models", "Unreferenced", 6),
        ("hud/textures", "Wired", 8),
        ("hud/textures", "UnwiredRoot", 1),
        ("hud/textures", "ByModel", 8),
        ("hud/textures", "NamedByCode", 2),
        ("hud/textures", "Unreferenced", 25),
        ("xml/2097_hud/font", "UnwiredRoot", 2),
        ("xml/2097_hud/font", "Declared", 2),
        ("xml/2097_hud/texture", "UnwiredRoot", 1),
        ("xml/2097_hud/texture", "Unreferenced", 1),
        ("xml/wo3_hud/font", "UnwiredRoot", 2),
        ("xml/wo3_hud/font", "Declared", 2),
        ("xml/wo3_hud/texture", "UnwiredRoot", 1),
        ("xml/wo3_hud/texture", "Declared", 1),
    ],
};

const HD_PSN: Expect = Expect {
    paths: 6504,
    families: &[
        ("fe/fonts (HUD)", 12),
        ("hud/models", 8),
        ("hud/textures", 32),
        ("xml/2097_hud", 23),
        ("xml/2097_hud/font", 6),
        ("xml/2097_hud/splitscreen_hud", 18),
        ("xml/2097_hud/texture", 2),
        ("xml/fragments", 20),
        ("xml/roots", 7),
        ("xml/splitscreen_hud", 18),
        ("xml/splitscreenzone_hud", 16),
        ("xml/wo3_hud", 23),
        ("xml/wo3_hud/font", 6),
        ("xml/wo3_hud/splitscreen_hud", 18),
        ("xml/wo3_hud/texture", 2),
    ],
    roots: 41,
    fragments: 102,
    wired: 9,
    wired_absent: &[
        "xml/splitscreen_hud/splitscreen_elimination_hud_0.xml",
        "xml/splitscreen_hud/splitscreen_elimination_hud_1.xml",
        "xml/splitscreen_hud/vert_splitscreen_elimination_hud_0.xml",
        "xml/splitscreen_hud/vert_splitscreen_elimination_hud_1.xml",
    ],
    dangling: &[],
    selectors: &[("HUD Style", 4), ("PI_HUD", 1), ("WIP3OUT", 18)],
    states: &[
        ("fe/fonts (HUD)", "Declared", 12),
        ("hud/models", "Wired", 4),
        ("hud/models", "NamedByCode", 4),
        ("hud/textures", "Wired", 6),
        ("hud/textures", "UnwiredRoot", 1),
        ("hud/textures", "ByModel", 3),
        ("hud/textures", "NamedByCode", 1),
        ("hud/textures", "Unreferenced", 21),
        ("xml/2097_hud/font", "UnwiredRoot", 2),
        ("xml/2097_hud/font", "Declared", 4),
        ("xml/2097_hud/texture", "UnwiredRoot", 1),
        ("xml/2097_hud/texture", "Unreferenced", 1),
        ("xml/wo3_hud/font", "UnwiredRoot", 2),
        ("xml/wo3_hud/font", "Declared", 4),
        ("xml/wo3_hud/texture", "UnwiredRoot", 1),
        ("xml/wo3_hud/texture", "Declared", 1),
    ],
};

const OMEGA: Expect = Expect {
    paths: 27077,
    families: &[
        ("fe/fonts (HUD)", 16),
        ("hud/materials", 31),
        ("hud/models", 24),
        ("hud/textures", 45),
        ("xml/2048_hud", 119),
        ("xml/2048_hud/font", 7),
        ("xml/2048_hud/texture", 8),
        ("xml/2097_hud", 27),
        ("xml/2097_hud/font", 11),
        ("xml/2097_hud/splitscreen_hud", 23),
        ("xml/2097_hud/texture", 3),
        ("xml/duel_hud", 15),
        ("xml/duel_hud/duel_splitscreen_hud", 5),
        ("xml/fragments", 83),
        ("xml/roots", 22),
        ("xml/splitscreen_hud", 31),
        ("xml/splitscreenzone_hud", 16),
        ("xml/wo3_hud", 27),
        ("xml/wo3_hud/font", 8),
        ("xml/wo3_hud/splitscreen_hud", 23),
        ("xml/wo3_hud/texture", 3),
    ],
    roots: 122,
    fragments: 269,
    wired: 13,
    wired_absent: &[],
    dangling: &[
        "hud/zone_battle_hud_bars_full.vex",
        "hud/zone_battle_hud_bars_full_rays.vex",
        "hud/zone_battle_hud_boost.vex",
        "hud/zone_battle_hud_boost_rays.vex",
        "hud/zone_battle_hud_hex_full.vex",
    ],
    selectors: &[("HUD Style", 4), ("PI_HUD", 1), ("WIP3OUT", 25)],
    states: &[
        ("fe/fonts (HUD)", "Declared", 16),
        ("hud/materials", "Unreferenced", 31),
        ("hud/models", "Wired", 4),
        ("hud/models", "UnwiredRoot", 1),
        ("hud/models", "NamedByCode", 4),
        ("hud/models", "Unreferenced", 15),
        ("hud/textures", "Wired", 8),
        ("hud/textures", "UnwiredRoot", 1),
        ("hud/textures", "ByModel", 5),
        ("hud/textures", "NamedByCode", 4),
        ("hud/textures", "Unreferenced", 27),
        ("xml/2048_hud/font", "Declared", 7),
        ("xml/2048_hud/texture", "UnwiredRoot", 6),
        ("xml/2048_hud/texture", "Unreferenced", 2),
        ("xml/2097_hud/font", "UnwiredRoot", 2),
        ("xml/2097_hud/font", "Declared", 7),
        ("xml/2097_hud/font", "Unreferenced", 2),
        ("xml/2097_hud/texture", "UnwiredRoot", 1),
        ("xml/2097_hud/texture", "NamedByCode", 1),
        ("xml/2097_hud/texture", "Unreferenced", 1),
        ("xml/wo3_hud/font", "UnwiredRoot", 2),
        ("xml/wo3_hud/font", "Declared", 4),
        ("xml/wo3_hud/font", "Unreferenced", 2),
        ("xml/wo3_hud/texture", "UnwiredRoot", 1),
        ("xml/wo3_hud/texture", "Declared", 1),
        ("xml/wo3_hud/texture", "NamedByCode", 1),
    ],
};

const WIPEOUT_2048_EU: Expect = Expect {
    paths: 22147,
    families: &[
        ("fe/fonts (HUD)", 6),
        ("hud/debug", 3),
        ("hud/materials", 6),
        ("hud/models", 44),
        ("hud/textures", 42),
        ("xml/2048_hud", 56),
        ("xml/2048_hud/font", 4),
        ("xml/2048_hud/texture", 5),
        ("xml/2097_hud", 27),
        ("xml/2097_hud/font", 6),
        ("xml/2097_hud/splitscreen_hud", 23),
        ("xml/2097_hud/texture", 2),
        ("xml/duel_hud", 5),
        ("xml/duel_hud/duel_splitscreen_hud", 5),
        ("xml/fragments", 32),
        ("xml/roots", 8),
        ("xml/splitscreen_hud", 31),
        ("xml/splitscreenzone_hud", 16),
        ("xml/wo3_hud", 27),
        ("xml/wo3_hud/font", 6),
        ("xml/wo3_hud/splitscreen_hud", 23),
        ("xml/wo3_hud/texture", 2),
    ],
    roots: 84,
    fragments: 169,
    wired: 5,
    wired_absent: &[],
    dangling: &[],
    selectors: &[("HUD Style", 4), ("PI_HUD", 0), ("WIP3OUT", 17)],
    states: &[
        ("fe/fonts (HUD)", "Unreferenced", 6),
        ("hud/debug", "Unreferenced", 3),
        ("hud/materials", "Unreferenced", 6),
        ("hud/models", "UnwiredRoot", 10),
        ("hud/models", "NamedByCode", 4),
        ("hud/models", "Unreferenced", 30),
        ("hud/textures", "UnwiredRoot", 9),
        ("hud/textures", "ByModel", 8),
        ("hud/textures", "NamedByCode", 2),
        ("hud/textures", "Unreferenced", 23),
        ("xml/2048_hud/font", "Declared", 4),
        ("xml/2048_hud/texture", "Wired", 5),
        ("xml/2097_hud/font", "UnwiredRoot", 2),
        ("xml/2097_hud/font", "Unreferenced", 4),
        ("xml/2097_hud/texture", "UnwiredRoot", 1),
        ("xml/2097_hud/texture", "Unreferenced", 1),
        ("xml/wo3_hud/font", "UnwiredRoot", 2),
        ("xml/wo3_hud/font", "Unreferenced", 4),
        ("xml/wo3_hud/texture", "UnwiredRoot", 1),
        ("xml/wo3_hud/texture", "Unreferenced", 1),
    ],
};

const WIPEOUT_2048_USA: Expect = Expect {
    paths: 20467,
    families: &[
        ("fe/fonts (HUD)", 6),
        ("hud/debug", 3),
        ("hud/materials", 6),
        ("hud/models", 44),
        ("hud/textures", 42),
        ("xml/2048_hud", 56),
        ("xml/2048_hud/font", 4),
        ("xml/2048_hud/texture", 5),
        ("xml/2097_hud", 27),
        ("xml/2097_hud/font", 6),
        ("xml/2097_hud/splitscreen_hud", 23),
        ("xml/2097_hud/texture", 2),
        ("xml/duel_hud", 5),
        ("xml/duel_hud/duel_splitscreen_hud", 5),
        ("xml/fragments", 32),
        ("xml/roots", 8),
        ("xml/splitscreen_hud", 31),
        ("xml/splitscreenzone_hud", 16),
        ("xml/wo3_hud", 27),
        ("xml/wo3_hud/font", 6),
        ("xml/wo3_hud/splitscreen_hud", 23),
        ("xml/wo3_hud/texture", 2),
    ],
    roots: 84,
    fragments: 169,
    wired: 5,
    wired_absent: &[],
    dangling: &[],
    selectors: &[("HUD Style", 4), ("PI_HUD", 0), ("WIP3OUT", 17)],
    states: &[
        ("fe/fonts (HUD)", "Unreferenced", 6),
        ("hud/debug", "Unreferenced", 3),
        ("hud/materials", "Unreferenced", 6),
        ("hud/models", "UnwiredRoot", 10),
        ("hud/models", "NamedByCode", 4),
        ("hud/models", "Unreferenced", 30),
        ("hud/textures", "UnwiredRoot", 9),
        ("hud/textures", "ByModel", 8),
        ("hud/textures", "NamedByCode", 2),
        ("hud/textures", "Unreferenced", 23),
        ("xml/2048_hud/font", "Declared", 4),
        ("xml/2048_hud/texture", "Wired", 5),
        ("xml/2097_hud/font", "UnwiredRoot", 2),
        ("xml/2097_hud/font", "Unreferenced", 4),
        ("xml/2097_hud/texture", "UnwiredRoot", 1),
        ("xml/2097_hud/texture", "Unreferenced", 1),
        ("xml/wo3_hud/font", "UnwiredRoot", 2),
        ("xml/wo3_hud/font", "Unreferenced", 4),
        ("xml/wo3_hud/texture", "UnwiredRoot", 1),
        ("xml/wo3_hud/texture", "Unreferenced", 1),
    ],
};

fn open_hd() -> Option<oag_assets::Archives> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    oag_hd::open(&image.display().to_string()).ok()
}

fn run(name: &str, title: &Title, mut archives: oag_assets::Archives, ext: &str, want: &Expect) {
    let census = census(&archives);
    let closure = closure(&mut archives, &census, Some(ext));
    report(name, title, &census, &closure);
    check(name, &summary(title, &census, &closure), want);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_fury_census() {
    let Some(archives) = open_hd() else { return };
    run(
        "hd-fury",
        oag_hd::TITLE,
        archives,
        oag_hd::hud::TEXTURE_EXTENSION,
        &HD_FURY,
    );
}

#[test]
#[ignore = "needs the installed PSN download in data/extracted/ps3/"]
fn hd_psn_census() {
    let Some(dir) = oag_testdata::exact("data/extracted/ps3/hd-psn-eu") else {
        return;
    };
    let archives = oag_hd::open(&dir.display().to_string()).expect("HD PSN opens");
    run(
        "hd-psn",
        oag_hd::TITLE,
        archives,
        oag_hd::hud::TEXTURE_EXTENSION,
        &HD_PSN,
    );
}

#[test]
#[ignore = "needs the Omega packages in data/images/"]
fn omega_census() {
    let Some(pkg) = oag_testdata::image("data/images/omega-ps4-eu.pkg") else {
        return;
    };
    let archives = oag_omega::open(&pkg.display().to_string()).expect("Omega opens");
    run("omega", oag_omega::TITLE, archives, ".gnf", &OMEGA);
}

fn census_2048(name: &str, serial: &str, want: &Expect) {
    let Some(dir) = oag_testdata::exact(&format!("data/extracted/vita/{serial}")) else {
        return;
    };
    let archives = oag_2048::open(&dir.display().to_string()).expect("2048 opens");
    run(
        name,
        oag_2048::TITLE,
        archives,
        oag_2048::hud::TEXTURE_EXTENSION,
        want,
    );
}

#[test]
#[ignore = "needs the extracted Vita package in data/extracted/vita/"]
fn wipeout_2048_eu_census() {
    census_2048("2048-eu", "PCSF00007", &WIPEOUT_2048_EU);
}

#[test]
#[ignore = "needs the extracted Vita package in data/extracted/vita/"]
fn wipeout_2048_usa_census() {
    census_2048("2048-usa", "PCSA00015", &WIPEOUT_2048_USA);
}

/// The WADs store a CRC of each name and no names, so these are **probed**: a
/// name is "found" when some entry hashes to it. The seeds are the names the
/// layouts and the title crates spell, plus every HUD name HD, Omega and 2048
/// ship (they descend from Pulse's data), tried under Pulse's own directories.
mod wad {
    pub const FONTS: &[&str] = &[
        r"Data\FE\Fonts\PulseHud.fnt",
        r"Data\FE\Fonts\small.fnt",
        r"Data\FE\Fonts\pulse_text.fnt",
    ];

    /// Stems under `Data\HUD\Textures\`, as `.mip`.
    pub const TEXTURES: &[&str] = &[
        "PulseHUD",
        "hdhud",
        "fury_hud",
        "hud_components",
        "hud_bars",
        "nitro_hud",
        "speed",
        "zonedamage",
        "grid",
        "ghostship",
        "barcode",
        "memstick",
        "bullet",
        "bullet_back",
        "1st",
        "2nd",
        "3rd",
        "4th",
        "5th",
        "6th",
        "7th",
        "8th",
        "gunsight_add",
        "gunsightdot_add",
        "missile_reticule",
        "reticule_elements",
        "ready_go_nomip",
        "pulse_ready_go_nomip",
        "detonator_hud2",
        "duel_bars",
        "zone_outline_01",
        "bg_template",
        "bg_template_zone",
    ];

    /// Stems under `Data\HUD\`, as `.vex`.
    pub const MODELS: &[&str] = &[
        "Pulse_Ready_Go",
        "Cockpit_321GO",
        "Ready_GO",
        "missile_sight_outer",
        "missile_sight_inner",
        "leachbeam_sight",
        "grid",
        "ghostship",
        "memory_stick_anim",
        "Bar_1",
        "Bar_2",
        "Bar_3",
        "Bar_outline_1",
        "Speed",
        "Shield",
        "Lap",
        "Position",
        "Thrust",
        "Zone_Bar_1",
        "Zone_Bar_3",
        "Zone_outline_1",
        "emp_hud",
        "zone_battle_hud_boost",
        "zone_battle_hud_hex_full",
        "zone_battle_hud_bars_full",
    ];

    /// Layout names beyond the five a Pulse race reads: the other modes and the
    /// skin and split-screen directories the later titles ship.
    pub const LAYOUTS: &[&str] = &[
        r"Data\XML\SpeedLap_HUD.xml",
        r"Data\XML\Detonator_HUD.xml",
        r"Data\XML\Arcade_HUD_old.xml",
        r"Data\XML\hud_timers.xml",
        r"Data\XML\wo3_hud\Arcade_HUD.xml",
        r"Data\XML\2097_hud\Arcade_HUD.xml",
        r"Data\XML\2048_hud\Arcade_HUD.xml",
        r"Data\XML\SplitScreen_hud\SplitScreen_HUD_0.xml",
        r"Data\XML\Duel_HUD\Duel_HUD.xml",
    ];

    pub fn probe_names() -> Vec<String> {
        let mut out: Vec<String> = FONTS
            .iter()
            .chain(LAYOUTS)
            .map(|s| (*s).to_string())
            .collect();
        out.extend(
            TEXTURES
                .iter()
                .map(|s| format!(r"Data\HUD\Textures\{s}.mip")),
        );
        out.extend(MODELS.iter().map(|s| format!(r"Data\HUD\{s}.vex")));
        out
    }
}

/// The name, or the PS2 pressing's `.pct` spelling of it.
fn wad_spellings(name: &str) -> Vec<String> {
    let mut out = vec![name.to_string()];
    if let Some(pct) = oag_pulse::ps2_texture_name(&oag_pulse::ps2_strip_build_prefix(name)) {
        out.push(pct);
    }
    out
}

struct WadCensus {
    entries: Vec<(String, usize)>,
    /// Layout roots that composed, with their file counts.
    composed: Vec<(String, usize)>,
    /// Everything the composed layouts reference, and whether an entry answers it.
    referenced: BTreeMap<String, bool>,
    /// Probe names an entry answers, and how many were tried.
    probe_hits: Vec<String>,
    probed: usize,
    /// Front-end files (the title's root and its includes) that carry a HUD
    /// selector, as `(file, needle)`; the root's file count beside them.
    selectors: Vec<(String, &'static str)>,
    front_end_files: usize,
}

fn wad_census(archives: &mut oag_assets::Archives, title: &Title) -> WadCensus {
    let mut entries = vec![(
        archives.data.label().to_string(),
        archives.data.entry_count(),
    )];
    if let Some(fe) = &archives.fe {
        entries.push((fe.label().to_string(), fe.entry_count()));
    }
    let hud = title.hud;
    let mut roots: Vec<&str> = vec![
        hud.arcade,
        hud.time_trial,
        hud.speed_lap,
        hud.zone,
        hud.elimination,
    ];
    roots.sort_unstable();
    roots.dedup();
    let mp_tag = r"Data\XML\MPTag_HUD.xml";
    if archives.locate(mp_tag).is_some() {
        roots.push(mp_tag);
    }
    let mut composed = Vec::new();
    let mut referenced = BTreeMap::new();
    for root in roots {
        let seen = RefCell::new(Vec::<Vec<u8>>::new());
        let result = {
            let mut read = |path: &str| {
                let bytes = archives.read_name(path).ok()?;
                seen.borrow_mut().push(bytes.clone());
                Some(bytes)
            };
            oag_hud::compose(root, &mut read)
        };
        let Some(result) = result else { continue };
        composed.push((root.to_string(), result.files.len()));
        for bytes in seen.borrow().iter() {
            for reference in references(bytes) {
                let found = wad_spellings(&reference)
                    .iter()
                    .any(|n| archives.locate(n).is_some());
                referenced.insert(norm(&reference), found);
            }
        }
    }
    let probes = wad::probe_names();
    let probe_hits = probes
        .iter()
        .filter(|n| {
            wad_spellings(n)
                .iter()
                .any(|s| archives.locate(s).is_some())
        })
        .cloned()
        .collect();
    let mut selectors = Vec::new();
    let mut front_end_files = 0;
    if let Some(front_end) = title.front_end {
        let seen = RefCell::new(Vec::<(String, Vec<u8>)>::new());
        let composed = {
            let mut read = |path: &str| {
                let bytes = archives.read_name(path).ok()?;
                seen.borrow_mut().push((path.to_string(), bytes.clone()));
                Some(bytes)
            };
            oag_hud::compose(front_end.root, &mut read)
        };
        front_end_files = composed.map_or(0, |c| c.files.len());
        for (path, bytes) in seen.borrow().iter() {
            let lower = String::from_utf8_lossy(bytes).to_lowercase();
            for needle in SELECTOR_NEEDLES {
                if lower.contains(&needle.to_lowercase()) {
                    selectors.push((path.clone(), *needle));
                }
            }
        }
    }
    WadCensus {
        selectors,
        front_end_files,
        entries,
        composed,
        referenced,
        probe_hits,
        probed: probes.len(),
    }
}

fn report_wad(name: &str, census: &WadCensus) {
    if !sweep() {
        return;
    }
    println!("=== {name}");
    for (archive, count) in &census.entries {
        println!("wad {count:6} {archive}");
    }
    for (root, files) in &census.composed {
        println!("composed {files:2} {root}");
    }
    let resolved = census.referenced.values().filter(|f| **f).count();
    println!(
        "references {} resolved {resolved} unresolved {}",
        census.referenced.len(),
        census.referenced.len() - resolved
    );
    for (reference, found) in &census.referenced {
        println!(
            "ref {} {reference}",
            if *found { "found" } else { "ABSENT" }
        );
    }
    println!(
        "front-end files {} selectors {:?}",
        census.front_end_files, census.selectors
    );
    println!("probes {} hits {}", census.probed, census.probe_hits.len());
    for hit in &census.probe_hits {
        println!("probe-hit {hit}");
    }
}

/// What a WAD title's census is pinned to.
struct WadExpect {
    entries: &'static [usize],
    composed: usize,
    referenced: usize,
    resolved: usize,
    probe_hits: usize,
    front_end_files: usize,
}

fn check_wad(name: &str, got: &WadCensus, want: &WadExpect) {
    let entries: Vec<usize> = got.entries.iter().map(|(_, n)| *n).collect();
    assert_eq!(entries, want.entries, "{name}: entries per WAD");
    assert_eq!(
        got.composed.len(),
        want.composed,
        "{name}: layouts composed"
    );
    assert_eq!(got.referenced.len(), want.referenced, "{name}: references");
    let resolved = got.referenced.values().filter(|f| **f).count();
    assert_eq!(
        resolved, want.resolved,
        "{name}: references an entry answers"
    );
    assert_eq!(got.probed, wad::probe_names().len(), "{name}: probes tried");
    assert_eq!(got.probe_hits.len(), want.probe_hits, "{name}: probe hits");
    assert_eq!(
        got.front_end_files, want.front_end_files,
        "{name}: front-end files composed"
    );
    assert_eq!(
        got.selectors,
        [],
        "{name}: front-end files naming a HUD selector"
    );
}

fn wad_title(name: &str, image: &str, pure: bool, want: &WadExpect) {
    let Some(path) = oag_testdata::image(image) else {
        return;
    };
    let source = path.display().to_string();
    let (mut archives, title) = if pure {
        (
            oag_pure::open(&source).expect("Pure opens"),
            oag_pure::TITLE,
        )
    } else {
        (
            oag_pulse::open(&source).expect("Pulse opens"),
            oag_pulse::TITLE,
        )
    };
    let census = wad_census(&mut archives, title);
    report_wad(name, &census);
    check_wad(name, &census, want);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_psp_eu_census() {
    wad_title(
        "pulse-psp-eu",
        "data/images/pulse-psp-eu.chd",
        false,
        &WadExpect {
            entries: &[1138, 27],
            composed: 5,
            referenced: 6,
            resolved: 6,
            probe_hits: 25,
            front_end_files: 19,
        },
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_psp_usa_census() {
    wad_title(
        "pulse-psp-usa",
        "data/images/pulse-psp-usa.chd",
        false,
        &WadExpect {
            entries: &[1142, 27],
            composed: 5,
            referenced: 6,
            resolved: 6,
            probe_hits: 25,
            front_end_files: 19,
        },
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_ps2_eu_census() {
    wad_title(
        "pulse-ps2-eu",
        "data/images/pulse-ps2-eu.chd",
        false,
        &WadExpect {
            entries: &[7200, 193],
            composed: 5,
            referenced: 6,
            resolved: 6,
            probe_hits: 44,
            front_end_files: 14,
        },
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_psp_eu_census() {
    wad_title(
        "pure-psp-eu",
        "data/images/pure-psp-eu.chd",
        true,
        &WadExpect {
            entries: &[841, 158],
            composed: 4,
            referenced: 20,
            resolved: 20,
            probe_hits: 12,
            front_end_files: 11,
        },
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_psp_usa_census() {
    wad_title(
        "pure-psp-usa",
        "data/images/pure-psp-usa.chd",
        true,
        &WadExpect {
            entries: &[832, 157],
            composed: 4,
            referenced: 20,
            resolved: 20,
            probe_hits: 12,
            front_end_files: 11,
        },
    );
}

#[test]
#[ignore = "needs the DLC packs in data/dlc/"]
fn psp_dlc_packs_census() {
    let root = oag_testdata::repo_root().join("data/dlc");
    if !root.exists() {
        return;
    }
    let cache = oag_testdata::repo_root().join("data/cache/dlc");
    let (pulse, pure, problems) = oag_source::dlc::packs_from_defaults(&[root], &cache);
    if sweep() {
        println!(
            "=== psp-dlc: pulse packs {} pure packs {} problems {problems:?}",
            pulse.len(),
            pure.len()
        );
    }
    let probes = wad::probe_names();
    let mut archives = 0;
    let mut total_hits = 0;
    for (title, packs) in [("pulse", &pulse), ("pure", &pure)] {
        for pack in packs {
            for archive in &pack.archives {
                archives += 1;
                let hits: Vec<&String> = probes
                    .iter()
                    .filter(|n| wad_spellings(n).iter().any(|s| archive.contains(s)))
                    .collect();
                total_hits += hits.len();
                if sweep() {
                    println!(
                        "dlc {title} {} entries {} probe-hits {} {hits:?}",
                        pack.label,
                        archive.directory().entries.len(),
                        hits.len()
                    );
                }
            }
        }
    }
    assert!(problems.is_empty(), "DLC unpack problems: {problems:?}");
    assert_eq!((pulse.len(), pure.len()), (4, 8), "packs per title");
    assert_eq!(archives, 24, "WAD archives across every pack");
    assert_eq!(total_hits, 0, "no DLC pack ships a HUD name probed");
}
