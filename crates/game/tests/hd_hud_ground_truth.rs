//! Composes **Wipeout HD / Fury's own eighteen HUD layouts** and checks what
//! came out.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`. The image must be
//! layer-1 decrypted first - `docs/formats/ps3-disc.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(hd_hud_ground_truth)'
//! ```
//!
//! # What makes these checks rather than restatements
//!
//! `crates/game/tests/hud_layout_ground_truth.rs` pins Pulse's five layouts to
//! widget counts taken by `grep`. That trick does not survive the move to HD,
//! because a mode's HUD is 5 to 17 files and `grep` cannot follow an include.
//! So the counts here are cross-checked against a **second walker written in
//! this file** - [`count_widgets`] follows the same includes and counts raw
//! `<Image>`/`<Text>` elements, knowing nothing about `<Values>` carriers,
//! sprites-versus-fills, or offsets. Two independent readings of the same tree
//! agreeing on 2,320 widgets is a real check; one of them agreeing with itself
//! is not.
//!
//! The load-bearing assertions need no external count at all:
//!
//! - [`every_layout_composes_with_nothing_missing_and_nothing_skipped`] - every
//!   include resolves and every widget element is modelled, on all eighteen.
//! - [`every_texture_a_layout_names_is_on_the_disc`] - the extension rule
//!   [`oag_hd::hud::texture_entry`] states, checked against the manifests.
//!
//! Both fail loudly the day a reading drifts, and neither can be satisfied by a
//! parser that quietly drops what it does not understand.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use oag_formats::fexml::{self, Node};
use oag_game::hud;

fn image() -> Option<PathBuf> {
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

fn open() -> Option<oag_assets::Archives> {
    let image = image()?;
    oag_hd::open(&image.display().to_string()).ok()
}

/// `(root, sprites, fills, labels, files)` for each shipped layout.
///
/// Measured 2026-08-17 on `hdfury-ps3-eu-dec.iso`. `files` counts the root
/// itself, so `mptag_hud.xml`'s 1 is a layout with no includes at all - the one
/// HD HUD that is shaped like a Pulse one.
///
/// **These are the composition this build performs**, and which archive each
/// root is read out of is part of it: six of the eighteen ship in more than one
/// archive at different sizes, and `oag_assets::Archives` searches `DATA00`,
/// then `DATA02`, then the rest. **Whether the original picks the same copy is
/// not read** - see `docs/formats/hd-hud.md`. So a future change to the archive
/// order moves these numbers legitimately, and this table is where that shows up
/// rather than in a screenshot.
const EXPECTED: &[(&str, usize, usize, usize, usize)] = &[
    ("/data/xml/arcade_hud.xml", 138, 4, 70, 17),
    ("/data/xml/elimination_hud.xml", 132, 2, 70, 15),
    ("/data/xml/timetrial_hud.xml", 51, 1, 43, 14),
    ("/data/xml/speedlap_hud.xml", 50, 1, 40, 15),
    ("/data/xml/zone_hud.xml", 36, 1, 32, 5),
    ("/data/xml/detonator_hud.xml", 71, 2, 40, 8),
    ("/data/xml/mptag_hud.xml", 0, 0, 8, 1),
    ("/data/xml/duel_hud/duel_hud.xml", 62, 1, 52, 9),
    ("/data/xml/wo3_hud/arcade_hud.xml", 93, 55, 77, 17),
    ("/data/xml/wo3_hud/elimination_hud.xml", 91, 53, 63, 16),
    ("/data/xml/wo3_hud/timetrial_hud.xml", 26, 27, 43, 13),
    ("/data/xml/wo3_hud/speedlap_hud.xml", 23, 27, 37, 13),
    ("/data/xml/wo3_hud/zone_hud.xml", 37, 20, 33, 5),
    ("/data/xml/2097_hud/arcade_hud.xml", 71, 74, 79, 17),
    ("/data/xml/2097_hud/elimination_hud.xml", 78, 67, 64, 16),
    ("/data/xml/2097_hud/timetrial_hud.xml", 17, 33, 45, 13),
    ("/data/xml/2097_hud/speedlap_hud.xml", 17, 31, 39, 13),
    ("/data/xml/2097_hud/zone_hud.xml", 36, 23, 34, 5),
];

fn composed(archives: &mut oag_assets::Archives, root: &str) -> hud::Composed {
    let mut read = |path: &str| archives.read_name(path).ok();
    hud::compose(root, &mut read).unwrap_or_else(|| panic!("composing {root}"))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_disc_ships_the_eighteen_root_layouts_the_title_package_names() {
    let Some(image) = image() else {
        return;
    };
    // Re-derived from the manifests rather than trusting `ROOTS`: every
    // `<mode>_hud.xml` directly inside one of the four single-screen
    // directories, across all seven archives.
    const WHERE: &[&str] = &[
        "/data/xml/",
        "/data/xml/wo3_hud/",
        "/data/xml/2097_hud/",
        "/data/xml/duel_hud/",
    ];
    let mut found = BTreeSet::new();
    let mut split_screen = BTreeSet::new();
    let mut copies: BTreeMap<String, usize> = BTreeMap::new();
    for archive in oag_hd::archives::ALL {
        let spec = format!("{}:{archive}", image.display());
        let Ok(open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        for path in open.paths() {
            if !path.ends_with("_hud.xml") {
                continue;
            }
            let directory = &path[..=path.rfind('/').expect("psarc paths are absolute")];
            if WHERE.contains(&directory) {
                found.insert(path.clone());
                *copies.entry(path.clone()).or_default() += 1;
            } else if path.contains("splitscreen") || path.contains("split_screen") {
                split_screen.insert(path.clone());
            }
        }
    }

    // Which archive a root is read out of is a decision this project makes and
    // the original's is unread, so the size of the ambiguity is pinned here.
    let shared: Vec<&String> = copies
        .iter()
        .filter(|&(_, &n)| n > 1)
        .map(|(p, _)| p)
        .collect();
    assert_eq!(
        shared.len(),
        6,
        "roots shipping in more than one archive: {shared:?}"
    );

    let named: BTreeSet<String> = oag_hd::hud::ROOTS
        .iter()
        .map(|p| (*p).to_string())
        .collect();
    assert_eq!(
        found, named,
        "the manifests and oag_hd::hud::ROOTS disagree"
    );
    assert_eq!(named.len(), 18);

    // The split-screen family is real, out of scope, and excluded on purpose -
    // asserted so the exclusion is a recorded decision rather than a rule that
    // happens to miss them. `splitscreenzone_hud/zone_hud.xml` is the one that
    // reaches for `ROOTS` hardest: same basename, different directory.
    assert!(
        split_screen.contains("/data/xml/splitscreenzone_hud/zone_hud.xml"),
        "expected the split-screen Zone HUD to exist and be left out: {split_screen:?}"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_layout_composes_with_nothing_missing_and_nothing_skipped() {
    let Some(mut archives) = open() else {
        return;
    };
    for &(root, ..) in EXPECTED {
        let out = composed(&mut archives, root);
        assert!(
            out.missing.is_empty(),
            "{root}: includes that did not resolve: {:?}",
            out.missing
        );
        assert!(
            out.layout.skipped.is_empty(),
            "{root}: widgets the parser did not model: {:?}",
            out.layout.skipped
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_layout_composes_to_the_widget_counts_the_disc_has() {
    let Some(mut archives) = open() else {
        return;
    };
    for &(root, sprites, fills, labels, files) in EXPECTED {
        let out = composed(&mut archives, root);
        assert_eq!(
            (
                out.layout.sprites.len(),
                out.layout.fills.len(),
                out.layout.labels.len(),
                out.files.len(),
            ),
            (sprites, fills, labels, files),
            "{root}"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_second_walker_agrees_on_how_many_widgets_there_are() {
    let Some(mut archives) = open() else {
        return;
    };
    let mut total = 0;
    for &(root, ..) in EXPECTED {
        let out = composed(&mut archives, root);
        let mine = out.layout.sprites.len() + out.layout.fills.len() + out.layout.labels.len();
        let theirs = count_widgets(&mut archives, root);
        assert_eq!(mine, theirs, "{root}: two walkers, two answers");
        total += mine;
    }
    assert_eq!(total, 2320, "the whole HUD, both ways");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_texture_a_layout_names_is_on_the_disc_once_the_extension_is_replaced() {
    let Some(mut archives) = open() else {
        return;
    };
    let mut uses: BTreeMap<String, usize> = BTreeMap::new();
    for &(root, ..) in EXPECTED {
        for sprite in composed(&mut archives, root).layout.sprites {
            *uses.entry(sprite.src).or_default() += 1;
        }
    }

    let named: BTreeSet<&str> = uses.keys().map(String::as_str).collect();
    let expected: BTreeSet<&str> = oag_hd::hud::TEXTURES.iter().copied().collect();
    assert_eq!(
        named, expected,
        "the layouts and oag_hd::hud::TEXTURES disagree"
    );

    for (reference, count) in &uses {
        let entry = oag_hd::hud::texture_entry(reference);
        assert!(
            archives.locate(&entry).is_some(),
            "{reference} ({count} sprites) resolves to {entry}, which is not on the disc"
        );
    }

    // The two that need the rule, and the size of what would be lost without it.
    let literal: usize = uses
        .iter()
        .filter(|(reference, _)| !reference.to_ascii_lowercase().ends_with(".gtf"))
        .map(|(_, n)| n)
        .sum();
    assert_eq!(
        literal, 192,
        "sprites whose src is not itself a shipped name"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn offsets_compose_through_three_levels_and_across_two_files() {
    let Some(mut archives) = open() else {
        return;
    };
    let out = composed(&mut archives, "/data/xml/elimination_hud.xml");

    // `HUD_Elim_positions.xml` opens
    //   <Text name="PositionsParent" OffsetX="1545" OffsetY="40">
    //     <Item OffsetX="290" OffsetY="50">
    //       <Image name="HighlightBar"><Values x="0" y="0" .../></Image>
    // and is reached from a root that adds no offset of its own, so the bar sits
    // at 1545 + 290 and 40 + 50. Read with the offsets *not* composing - the
    // rule this parser had until 2026-08-17 - it lands at (290, 50) instead, on
    // the far side of a 1920-wide screen from where the position list belongs.
    let bar = out
        .layout
        .sprites
        .iter()
        .find(|s| s.name == "HighlightBar")
        .expect("HighlightBar");
    assert_eq!(bar.rect[0], 1835.0);
    assert_eq!(bar.rect[1], 90.0);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_anonymous_widgets_are_kept_rather_than_dropped() {
    let Some(mut archives) = open() else {
        return;
    };
    let mut anonymous = 0;
    for &(root, ..) in EXPECTED {
        let layout = composed(&mut archives, root).layout;
        anonymous += layout.sprites.iter().filter(|s| s.name.is_empty()).count();
        anonymous += layout.fills.iter().filter(|f| f.name.is_empty()).count();
        anonymous += layout.labels.iter().filter(|t| t.name.is_empty()).count();
    }
    // Pulse authors none of these; HD's are mostly `BackgroundLayer="1"` panels
    // behind a named readout, so dropping them leaves the numbers unbacked.
    assert_eq!(anonymous, 106);
}

/// The grid a race hands the HUD renderer is the **source's**, not the PSP's.
///
/// The defect this pins was invisible from the layout side, because a layout
/// carries bare numbers and every one of them parses the same whatever grid it
/// is in. `crate::render::Renderer` starts at `Space::PSP` and `hud::Overlay`
/// never told it otherwise, so HD's 1920x1080 layouts were drawn four times
/// oversized: the lap digit landed a quarter of the way into the picture as a
/// solid yellow slab that reads as scenery, and the speed bar, the total time
/// and the lap times fell off the right and bottom edges.
///
/// Asserted against `Space::PSP` as well as for `Space::HD`, because equality
/// with the right value and inequality with the default are different claims -
/// the second is the one that fails if `Space::of` ever stops being consulted.
/// See `docs/ui/hud.md`, "And the grid is the source's, not the PSP's".
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_race_hands_the_hud_hds_own_grid_rather_than_the_psps() {
    let Some(image) = image() else {
        return;
    };
    let loaded = oag_game::race::load(&oag_game::race::Options {
        source: image.display().to_string(),
        class: oag_physics::SpeedClass::Venom,
        mode: oag_race::Mode::TimeTrial,
        ..oag_game::race::Options::default()
    })
    .expect("loading the race");
    assert_eq!(
        loaded.hud.space,
        oag_game::frontend::Space::HD,
        "an HD source authors its HUD in 1920x1080"
    );
    assert_ne!(
        loaded.hud.space,
        oag_game::frontend::Space::PSP,
        "the PSP's 480x272 is the default a `Renderer` starts at, and drawing \
         HD's layouts in it is the bug this test exists for"
    );
}

/// The check that ties the two readings together.
///
/// A sprite carries `U`, `V`, `TxtrWidth` and `TxtrHeight` - a rectangle inside
/// the texture its `src` names - and until `.gtf` was read there was nothing to
/// check that rectangle against. Now there is, and it tests **both** halves at
/// once: a layout misread would put a sub-rectangle outside its texture, and a
/// texture whose dimensions were misread would fail to contain rectangles that
/// really do fit. 1,029 sprites over 12 textures, and every one lands.
///
/// It is also the check that would catch the extension rule
/// [`oag_hd::hud::texture_entry`] states being wrong in the one way that would
/// otherwise be invisible: resolving to *some* texture that happens to exist.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_sprite_s_source_rectangle_fits_inside_the_texture_it_names() {
    let Some(mut archives) = open() else {
        return;
    };
    let mut sizes: BTreeMap<String, (f32, f32)> = BTreeMap::new();
    let mut overhanging: Vec<(String, String, f32)> = Vec::new();
    let mut checked = 0usize;

    for &(root, ..) in EXPECTED {
        for sprite in composed(&mut archives, root).layout.sprites {
            let size = match sizes.get(&sprite.src) {
                Some(size) => *size,
                None => {
                    let entry = oag_hd::hud::texture_entry(&sprite.src);
                    let blob = archives
                        .read_name(&entry)
                        .unwrap_or_else(|e| panic!("{} -> {entry}: {e}", sprite.src));
                    let parsed = oag_formats::gtf::Gtf::parse(&blob)
                        .unwrap_or_else(|e| panic!("{entry}: {e}"));
                    let texture = parsed.only().expect("one texture");
                    let size = (f32::from(texture.width), f32::from(texture.height));
                    sizes.insert(sprite.src.clone(), size);
                    size
                }
            };

            let [u, v, w, h] = sprite.uv;
            let over = (u + w - size.0).max(v + h - size.1).max(0.0);
            assert!(u >= 0.0 && v >= 0.0, "{root}: {} at {u},{v}", sprite.name);
            if over > 0.0 {
                overhanging.push((sprite.name.clone(), sprite.src.clone(), over));
            }
            checked += 1;
        }
    }

    assert_eq!(sizes.len(), 12, "textures named: {:?}", sizes.keys());
    assert_eq!(checked, 1029);

    // **One widget in the disc's own data samples off the edge of its texture**,
    // and it is authored that way rather than misread: `VoiceCom0`-`VoiceCom7`
    // take a 62x64 patch from `V="1"` of a 64x64 image, so the last row is one
    // texel past the bottom. 32 of the 56 voice-chat sprites do it - the copies
    // in the fragments that were edited later use `V="0"` - which is how a
    // duplicated fragment diverges. Pinned rather than tolerated, because the
    // check is only worth having if its exceptions are named.
    let worst = overhanging
        .iter()
        .map(|(.., over)| *over)
        .fold(0.0f32, f32::max);
    assert_eq!(
        worst, 1.0,
        "an overhang bigger than one texel: {overhanging:?}"
    );
    assert!(
        overhanging
            .iter()
            .all(|(name, src, _)| name.starts_with("VoiceCom") && src.ends_with(r"voiceCom.gtf")),
        "something other than the voice-chat icon overhangs: {overhanging:?}"
    );
    assert_eq!(overhanging.len(), 32);
}

/// A second reading of the same include graph, deliberately naive.
///
/// Counts `<Image>` and `<Text>` elements at any depth, following `SrcRel` by
/// hand. It knows nothing about `<Values>`, `Src`, offsets or fills, so it can
/// only agree with [`hud::compose`] by both being right about the file set.
fn count_widgets(archives: &mut oag_assets::Archives, root: &str) -> usize {
    fn tally(node: &Node) -> usize {
        node.children
            .iter()
            .map(|c| {
                usize::from(
                    c.name.eq_ignore_ascii_case("Image") || c.name.eq_ignore_ascii_case("Text"),
                ) + tally(c)
            })
            .sum()
    }

    fn walk(archives: &mut oag_assets::Archives, path: &str) -> usize {
        let Ok(blob) = archives.read_name(path) else {
            return 0;
        };
        let Ok(text) = fexml::text(&blob) else {
            return 0;
        };
        let tree = fexml::parse(&text);
        let mut total = tally(&tree);
        for include in includes(&tree) {
            let mut parts: Vec<&str> = path.split(['/', '\\']).collect();
            parts.pop();
            for part in include.split(['/', '\\']) {
                match part {
                    "." | "" => {}
                    ".." => {
                        parts.pop();
                    }
                    other => parts.push(other),
                }
            }
            total += walk(archives, &parts.join("/"));
        }
        total
    }

    fn includes(node: &Node) -> Vec<String> {
        let mut out = Vec::new();
        for child in &node.children {
            if child.name.eq_ignore_ascii_case("LoadXML")
                && let Some(rel) = child.value("SrcRel")
            {
                out.push(rel.trim().to_string());
            }
            out.extend(includes(child));
        }
        out
    }

    walk(archives, root)
}
