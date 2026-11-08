//! How many of a PPSSPP texture pack's `textures.ini` keys are reproduced from
//! the texture bytes this engine loads off the disc.
//!
//! **`#[ignore]`d and never run in CI.** Needs a disc image and the two packs
//! (`just fetch-texture-packs`); skips when either is absent, and when a pack's
//! checkout is not the commit `scripts/texture-packs.pin` names (the counts are
//! of that commit). `OAG_REQUIRE_GAME_DATA=1` makes a missing image a failure.
//!
//! ```sh
//! just fetch-texture-packs && just test-data
//! ```
//!
//! Every `.mip`, `.fnt` atlas and `.vex` `Texture` node of every WAD on the disc
//! is keyed ([`Stored::replacement`]) in one pass and the pack's `[hashes]` are
//! classified against the result. `docs/formats/ppsspp-texture-pack.md` has the
//! hash, the classes and the numbers.
//!
//! The optional oracle test compares the census against a PPSSPP "Save new
//! textures" dump (`OAG_PPSSPP_DUMP=<TEXTURES/<serial>/new>`), which names every
//! texture the emulator sampled by the key it would look up.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::lzss;
use oag_formats::wad::{self, Compression, Directory};
use oag_texture::fnt;
use oag_texture::replacement::{Key, Replacement, Stored};
use oag_texture::replacement_pack::{Line, Pack};

const PULSE_PACK: &str = "WipEout-Pulse-Upscale-Version";
const PURE_PACK: &str = "WipEout-Pure-HD-Texture-Pack";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The pack's `textures.ini`, parsed, or `None` (with the reason printed) when
/// the pack is absent or not at the pinned commit.
fn pack(name: &str) -> Option<Pack> {
    let dir = repo_root().join("data/texture-packs").join(name);
    let Ok(ini) = std::fs::read_to_string(dir.join("textures.ini")) else {
        eprintln!(
            "skip: {} is absent (just fetch-texture-packs)",
            dir.display()
        );
        return None;
    };
    let pins = std::fs::read_to_string(repo_root().join("scripts/texture-packs.pin")).ok()?;
    let pinned = pins.lines().filter(|l| !l.starts_with('#')).find_map(|l| {
        let mut fields = l.split(' ');
        (fields.next() == Some(name)).then(|| fields.nth(1).map(str::to_string))?
    })?;
    let head = std::process::Command::new("git")
        .arg("-C")
        .arg(&dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    if head.as_deref() != Some(pinned.as_str()) {
        eprintln!("skip: {name} is at {head:?}, the pin is {pinned}");
        return None;
    }
    Some(Pack::parse(&ini))
}

/// One texture, keyed.
struct Source {
    kind: &'static str,
    replacement: Replacement,
}

/// Every WAD blob of the disc, decompressed, to `each(archive, index, blob)`.
fn walk(disc: &mut DiscImage, mut each: impl FnMut(&str, usize, &[u8])) {
    let entries = disc.entries().expect("entries").to_vec();
    for e in entries
        .iter()
        .filter(|e| e.path.to_lowercase().ends_with(".wad"))
    {
        let header = disc
            .read_entry_range(e, 0, wad::HEADER_LEN as u64)
            .expect("header");
        let count = Directory::peek_entry_count(&header).expect("count");
        let bytes = disc
            .read_entry_range(e, 0, Directory::directory_len(count))
            .expect("directory");
        let dir = Directory::parse(&bytes, Some(e.size)).expect("parse");
        for (i, ent) in dir.entries.iter().enumerate() {
            if ent.size == 0 {
                continue;
            }
            let raw = disc
                .read_entry_range(e, u64::from(ent.offset), u64::from(ent.size))
                .expect("blob");
            let blob = match ent.compression {
                Compression::None => raw,
                Compression::Lzss => {
                    lzss::decompress(&raw, ent.size_uncompressed as usize).expect("lzss")
                }
                Compression::Zlib => continue,
            };
            each(&e.path, i, &blob);
        }
    }
}

/// What the census saw besides keys.
#[derive(Default, Debug, PartialEq, Eq)]
struct Walked {
    vex_files_with_textures: usize,
    vex_unreadable_nodes: usize,
    ship_skins: usize,
    dlc_archives: usize,
}

/// Keys one blob if it is a texture of a kind the census reads.
fn visit(blob: &[u8], sources: &mut Vec<Source>, walked: &mut Walked) {
    if oag_vex::vex::has_magic(blob) {
        let Ok(tree) = oag_vex::vex::nodes(blob) else {
            return;
        };
        let Ok(nodes) = oag_vex::vex::stored_textures(blob, &tree) else {
            return;
        };
        let mut any = false;
        for node in &nodes {
            let Some(t) = node else {
                walked.vex_unreadable_nodes += 1;
                continue;
            };
            any = true;
            let stored = Stored {
                width: u32::from(t.width),
                height: u32::from(t.height),
                bits_per_pixel: u32::from(t.bits_per_pixel),
                mip_levels: t.mip_count,
                swizzled: t.flags & oag_formats::swizzle::FLAG_SWIZZLED != 0,
                palette: t.palette,
                texels: t.texels,
                pad_level0: true,
            };
            if let Some(replacement) = stored.replacement() {
                sources.push(Source {
                    kind: "vex",
                    replacement,
                });
            }
        }
        walked.vex_files_with_textures += usize::from(any);
    } else if let Ok(stored) = Stored::from_mip(blob) {
        if let Some(replacement) = stored.replacement() {
            sources.push(Source {
                kind: "mip",
                replacement,
            });
        }
    } else if fnt::looks_like_font(blob) {
        if let Some(replacement) = Stored::from_font_atlas(blob)
            .ok()
            .and_then(|s| s.replacement())
        {
            sources.push(Source {
                kind: "fnt",
                replacement,
            });
        }
    } else if blob.len() == oag_texture::ship_skin::FILE_LEN
        && oag_texture::ship_skin::parse(blob).is_ok()
    {
        walked.ship_skins += 1;
    }
}

/// The downloadable-content archives of the title `image` is, unpacked from
/// `data/dlc` (Pure's through `data/keys/pure-dlc-keys.txt`).
fn dlc_archives(image: &str) -> Vec<oag_assets::Archive> {
    let roots = [repo_root().join("data/dlc")];
    let cache = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("dlc-cache");
    let keys = std::fs::read_to_string(repo_root().join("data/keys/pure-dlc-keys.txt"))
        .map(|text| oag_formats::pure_dlc::parse_keys(&text))
        .unwrap_or_default();
    let (packs, problems) = if image.starts_with("pure") {
        oag_source::dlc::pure_packs(&roots, &cache, &keys)
    } else {
        oag_source::dlc::packs(&roots, &cache, &keys)
    };
    assert!(
        problems.is_empty(),
        "DLC that would not unpack: {problems:?}"
    );
    packs.into_iter().flat_map(|p| p.archives).collect()
}

fn census(image: &str) -> Option<(Vec<Source>, Walked)> {
    let path = oag_testdata::image(image)?;
    let mut disc = DiscImage::open(&path).expect("open disc");
    let (mut sources, mut walked) = (Vec::new(), Walked::default());
    walk(&mut disc, |_, _, blob| {
        visit(blob, &mut sources, &mut walked)
    });
    for mut archive in dlc_archives(image) {
        walked.dlc_archives += 1;
        for index in 0..archive.len() as usize {
            if let Ok(blob) = archive.read(index) {
                visit(&blob, &mut sources, &mut walked);
            }
        }
    }
    Some((sources, walked))
}

/// How one `[hashes]` line fares against the census.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Class {
    /// The full key is one a texture of ours produces.
    Exact,
    /// A data-hash wildcard (`0000000000000000dddddddd`) whose data hash is ours.
    Wildcard,
    /// Our data hash matches; the palette hash reads past the texture's own
    /// block into the heap, so the key is not knowable from the file.
    DataOnlyHeapTail,
    /// Our data hash matches and the palette window is inside the block, but the
    /// key still differs.
    DataOnlyOther,
    /// No texture of ours has this data hash.
    NoData,
}

/// Every data hash a census produced, mapped to whether **any** texture with
/// that hash has a palette window running past its own block.
fn data_hashes(sources: &[Source]) -> HashMap<u32, bool> {
    let mut data: HashMap<u32, bool> = HashMap::new();
    for s in sources {
        let tail = s.replacement.key.is_none();
        data.entry(s.replacement.data)
            .and_modify(|t| *t |= tail)
            .or_insert(tail);
    }
    data
}

fn classify(line: &Line, keys: &HashSet<Key>, data: &HashMap<u32, bool>) -> Option<Class> {
    let key = line.key()?;
    let tail_only = data.get(&key.data).copied();
    Some(if key.dim_clut == 0 {
        if tail_only.is_some() {
            Class::Wildcard
        } else {
            Class::NoData
        }
    } else if keys.contains(&key) {
        Class::Exact
    } else {
        match tail_only {
            Some(true) => Class::DataOnlyHeapTail,
            Some(false) => Class::DataOnlyOther,
            None => Class::NoData,
        }
    })
}

/// The directory a pack path is reported under: `in_game/<sub>` or the top one.
fn group(target: &str) -> String {
    let mut parts = target.split('/');
    match (parts.next(), parts.next()) {
        (Some("in_game"), Some(sub)) if target.matches('/').count() > 1 => {
            format!("in_game/{sub}")
        }
        (Some(top), _) => top.to_string(),
        _ => String::new(),
    }
}

struct Outcome {
    by_group: BTreeMap<String, BTreeMap<Class, usize>>,
    keys: usize,
    blank: usize,
    levels: usize,
    cache_key_wildcards: usize,
    no_data: BTreeMap<String, usize>,
}

fn measure(pack: &Pack, sources: &[Source]) -> Outcome {
    let keys: HashSet<Key> = sources.iter().filter_map(|s| s.replacement.key).collect();
    let data = data_hashes(sources);
    let mut out = Outcome {
        by_group: BTreeMap::new(),
        keys: 0,
        blank: 0,
        levels: 0,
        cache_key_wildcards: 0,
        no_data: BTreeMap::new(),
    };
    for line in &pack.hashes {
        if line.level > 0 {
            out.levels += 1;
        } else if line.data_hash.is_none() {
            out.cache_key_wildcards += 1;
        } else if line.is_blacklist() {
            out.blank += 1;
        } else if let Some(class) = classify(line, &keys, &data) {
            out.keys += 1;
            if class == Class::NoData {
                let deep: Vec<&str> = line.target.split('/').take(4).collect();
                *out.no_data.entry(deep.join("/")).or_default() += 1;
            }
            *out.by_group
                .entry(group(&line.target))
                .or_default()
                .entry(class)
                .or_default() += 1;
        }
    }
    out
}

/// `[exact, wildcard, heap-tail, other, no-data]` over every group.
fn totals(o: &Outcome) -> [usize; 5] {
    let mut t = [0; 5];
    for classes in o.by_group.values() {
        for (class, n) in classes {
            t[*class as usize] += n;
        }
    }
    t
}

fn report(image: &str, sources: &[Source], walked: &Walked, o: &Outcome) {
    let mut by_kind: BTreeMap<&str, usize> = BTreeMap::new();
    for s in sources {
        *by_kind.entry(s.kind).or_default() += 1;
    }
    let keyed = sources
        .iter()
        .filter(|s| s.replacement.key.is_some())
        .count();
    println!("== {image}: {by_kind:?} ({keyed} keyed) {walked:?}");
    println!(
        "   {} keys + {} blank + {} mip-level + {} cache-key-wildcard lines",
        o.keys, o.blank, o.levels, o.cache_key_wildcards
    );
    for (group, classes) in &o.by_group {
        let c = |k: Class| classes.get(&k).copied().unwrap_or(0);
        println!(
            "   {group:24} {:5}  exact {:5} wildcard {:4} heap-tail {:4} other {:4} no-data {:4}",
            classes.values().sum::<usize>(),
            c(Class::Exact),
            c(Class::Wildcard),
            c(Class::DataOnlyHeapTail),
            c(Class::DataOnlyOther),
            c(Class::NoData)
        );
    }
    let mut deep: Vec<_> = o.no_data.iter().collect();
    deep.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    for (dir, n) in deep.iter().take(16) {
        println!("   no-data {n:4} {dir}");
    }
    println!(
        "   TOTAL [exact, wildcard, heap-tail, other, no-data] {:?}",
        totals(o)
    );
}

/// What a pressing's census must come to.
struct Expect {
    sources: usize,
    /// Of those, the ones whose key is knowable from their own bytes.
    keyed: usize,
    keys: usize,
    /// `[exact, wildcard, heap-tail, other, no-data]` per reported directory.
    groups: &'static [(&'static str, [usize; 5])],
}

const PULSE_GROUPS: &[(&str, [usize; 5])] = &[
    ("font", [3, 2, 0, 0, 0]),
    ("in_game", [1, 0, 0, 0, 0]),
    ("in_game/adverts", [105, 1, 1, 0, 0]),
    ("in_game/effects", [9, 4, 1, 0, 16]),
    ("in_game/ships", [276, 2, 0, 1, 167]),
    ("in_game/tracks", [1376, 235, 0, 0, 8]),
    ("intro", [2, 0, 0, 0, 5]),
    ("main_menu", [164, 1, 0, 0, 0]),
];

const PURE_GROUPS: &[(&str, [usize; 5])] = &[
    ("font", [2, 0, 0, 22, 2]),
    ("in_game/adverts", [103, 0, 0, 0, 0]),
    ("in_game/effects", [2, 0, 0, 0, 6]),
    ("in_game/hud", [3, 0, 0, 0, 0]),
    ("in_game/ships", [105, 0, 0, 0, 60]),
    ("in_game/tracks", [79, 0, 0, 0, 0]),
    ("intro", [4, 0, 0, 0, 5]),
    ("main_menu", [352, 0, 0, 0, 151]),
];

fn run(image: &str, pack_name: &str, expect: &Expect) {
    let Some(pack) = pack(pack_name) else { return };
    let Some((sources, walked)) = census(image) else {
        return;
    };
    assert_eq!(pack.hash_kind().as_deref(), Some("xxh64"));
    assert!(pack.flag("ignoreaddress"));
    assert!(
        pack.other_sections.is_empty(),
        "a section this reader would misread: {:?}",
        pack.other_sections
    );
    assert!(pack.unparsed.is_empty(), "{:?}", pack.unparsed);
    let outcome = measure(&pack, &sources);
    report(image, &sources, &walked, &outcome);
    assert_eq!(sources.len(), expect.sources, "textures read");
    let keyed = sources
        .iter()
        .filter(|s| s.replacement.key.is_some())
        .count();
    assert_eq!(keyed, expect.keyed, "textures whose key is knowable");
    assert_eq!(outcome.keys, expect.keys, "level-0 full keys classified");
    let got: Vec<(String, [usize; 5])> = outcome
        .by_group
        .iter()
        .map(|(g, classes)| {
            let mut row = [0; 5];
            for (class, n) in classes {
                row[*class as usize] = *n;
            }
            (g.clone(), row)
        })
        .collect();
    let want: Vec<(String, [usize; 5])> = expect
        .groups
        .iter()
        .map(|(g, row)| ((*g).to_string(), *row))
        .collect();
    assert_eq!(
        got, want,
        "per directory [exact, wildcard, heap-tail, other, no-data]"
    );
}

#[test]
#[ignore = "needs pulse-psp-eu.chd and the Pulse pack"]
fn pulse_eu_keys() {
    run(
        "pulse-psp-eu.chd",
        PULSE_PACK,
        &Expect {
            sources: 7633,
            keyed: 6169,
            keys: 2380,
            groups: PULSE_GROUPS,
        },
    );
}

#[test]
#[ignore = "needs pulse-psp-usa.chd and the Pulse pack"]
fn pulse_usa_keys() {
    run(
        "pulse-psp-usa.chd",
        PULSE_PACK,
        &Expect {
            sources: 7633,
            keyed: 6169,
            keys: 2380,
            groups: PULSE_GROUPS,
        },
    );
}

#[test]
#[ignore = "needs pure-psp-eu.chd and the Pure pack"]
fn pure_eu_keys() {
    run(
        "pure-psp-eu.chd",
        PURE_PACK,
        &Expect {
            sources: 4573,
            keyed: 4149,
            keys: 896,
            groups: PURE_GROUPS,
        },
    );
}

#[test]
#[ignore = "needs pure-psp-usa.chd and the Pure pack"]
fn pure_usa_keys() {
    run(
        "pure-psp-usa.chd",
        PURE_PACK,
        &Expect {
            sources: 4567,
            keyed: 4143,
            keys: 896,
            groups: PURE_GROUPS,
        },
    );
}

/// Every texture PPSSPP dumped for Pulse EU, against the census.
///
/// A dumped file is named by the key PPSSPP would look it up by, so this is the
/// oracle for the hash: among the dumped textures whose palette window lies
/// inside their own block, nearly every key must be one the census produced.
#[test]
#[ignore = "needs OAG_PPSSPP_DUMP, a PPSSPP `new/` directory from pulse-psp-eu.chd"]
fn pulse_eu_oracle() {
    let Ok(dump) = std::env::var("OAG_PPSSPP_DUMP") else {
        return;
    };
    let Some((sources, _)) = census("pulse-psp-eu.chd") else {
        return;
    };
    let keys: HashSet<Key> = sources.iter().filter_map(|s| s.replacement.key).collect();
    let data = data_hashes(&sources);
    let mut tally: BTreeMap<&str, [usize; 2]> = BTreeMap::new();
    for e in std::fs::read_dir(dump).expect("dump dir") {
        let name = e.expect("entry").file_name().to_string_lossy().into_owned();
        let Some(key) = Key::parse(name.trim_end_matches(".png")) else {
            continue;
        };
        let class = match data.get(&key.data) {
            None => "no-data",
            Some(true) => "heap-tail",
            Some(false) => "window-inside",
        };
        let t = tally.entry(class).or_default();
        t[0] += 1;
        t[1] += usize::from(keys.contains(&key));
    }
    println!("dumped keys by class [dumped, exact]: {tally:?}");
    let [dumped, exact] = tally.get("window-inside").copied().unwrap_or([0, 0]);
    assert!(
        dumped >= 100,
        "too few dumped textures to mean anything: {dumped}"
    );
    assert!(exact * 100 >= dumped * 95, "{exact} of {dumped} exact");
}
