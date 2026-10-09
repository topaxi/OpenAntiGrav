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
use std::path::PathBuf;

use oag_hud as hud;
use oag_tables::fexml::{self, Node};

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
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
/// is in. `crate::render::Renderer` starts at `Space::PSP` and `hud_overlay::Overlay`
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
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        ..oag_raceplay::Options::default()
    })
    .expect("loading the race");
    assert_eq!(
        loaded.hud.space,
        oag_display::space::Space::HD,
        "an HD source authors its HUD in 1920x1080"
    );
    assert_ne!(
        loaded.hud.space,
        oag_display::space::Space::PSP,
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
                    let parsed = oag_texture::gtf::Gtf::parse(&blob)
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

/// **Every sprite that reaches a draw samples its own texture, not a
/// neighbour's.**
///
/// The check the multi-atlas change is worth having. A layout's textures go
/// into one sheet stacked vertically, so a sprite offset by the wrong one's
/// placement lands in a *different image* at a plausible-looking rectangle - the
/// right patch of the wrong picture, which reads as art rather than as an error
/// and cannot be seen in a screenshot without knowing what to look for. Before
/// 2026-08-25 every sprite in a layout took the first texture's placement, so
/// 45 of the arcade HUD's 138 were wrong that way.
///
/// This is deliberately about the **composed uv**, after `sprite_draw` has
/// offset it, rather than about the authored rectangle -
/// [`every_sprite_s_source_rectangle_fits_inside_the_texture_it_names`] already
/// covers that half, and passing it is exactly what let the bug hide.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_drawn_sprite_lands_inside_the_texture_its_own_layout_names() {
    let Some(mut archives) = open() else {
        return;
    };
    let mut checked = 0usize;

    for &(root, ..) in EXPECTED {
        let layout = composed(&mut archives, root).layout;
        let references = layout.textures();
        if references.is_empty() {
            continue;
        }

        // The same read the race loader performs: declared name first, then the
        // title's own extension rule.
        let blobs: Vec<(String, Vec<u8>)> = references
            .iter()
            .map(|reference| {
                let entry = oag_hd::hud::texture_entry(reference);
                let blob = archives
                    .read_name(&entry)
                    .unwrap_or_else(|e| panic!("{root}: {reference} -> {entry}: {e}"));
                ((*reference).to_string(), blob)
            })
            .collect();
        let mut notes = Vec::new();
        let sheet = oag_hud::sprite::Sheet::build(&blobs, &mut notes);
        assert_eq!(
            sheet.len(),
            references.len(),
            "{root}: {} of {} textures decoded: {notes:?}",
            sheet.len(),
            references.len()
        );

        for sprite in &layout.sprites {
            // Either variant: a widget carrying a `RotationTheta` comes back as
            // `RotatedSprite`, and Zone's thirteen do. The uv is the same
            // composition either way, which is what this checks.
            let uv = match hud::sprite_draw(sprite, &sheet) {
                Some(oag_ui::frontend::Draw::Sprite { uv, .. })
                | Some(oag_ui::frontend::Draw::RotatedSprite { uv, .. }) => uv,
                _ => panic!("{root}: {} draws nothing", sprite.name),
            };
            let placed = sheet.get(&sprite.src).expect("just built");
            // A negative width or height is a mirror - 26 of HD's sprites author
            // one - so the patch runs backwards from `uv.xy` and the edges are
            // whichever way round they came out.
            let (left, right) = order(uv[0], uv[0] + uv[2]);
            let (top, bottom) = order(uv[1], uv[1] + uv[3]);
            let inside = left >= placed.x as f32
                && top >= placed.y as f32
                && right <= (placed.x + placed.width) as f32
                // One row of slack for the disc's own `VoiceCom` overhang, which
                // the test above pins at exactly one texel on 32 sprites.
                && bottom <= (placed.y + placed.height + 1) as f32;
            assert!(
                inside,
                "{root}: {} samples {left},{top}..{right},{bottom}, outside {:?} - the \
                 placement of {}",
                sprite.name, placed, sprite.src
            );
            checked += 1;
        }
    }

    println!("{checked} composed sprite uv(s) checked");
    assert_eq!(checked, 1029);
}

fn order(a: f32, b: f32) -> (f32, f32) {
    if a <= b { (a, b) } else { (b, a) }
}

/// **Every widget HD's title package calls always-on is a widget HD authors.**
///
/// `oag_hd::hud::ALWAYS_ON` is an allow-list matched by name, so a misspelling
/// costs that widget silently: the HUD simply draws one thing fewer, which
/// looks exactly like a layout that does not carry it. Each name has to be in at
/// least one of the eighteen.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_always_on_widget_is_authored_by_at_least_one_layout() {
    let Some(mut archives) = open() else {
        return;
    };
    let mut authored: BTreeSet<String> = BTreeSet::new();
    for &(root, ..) in EXPECTED {
        for sprite in composed(&mut archives, root).layout.sprites {
            authored.insert(sprite.name);
        }
    }
    let missing: Vec<&&str> = oag_hd::hud::ALWAYS_ON
        .iter()
        .filter(|name| !authored.contains(**name))
        .collect();
    assert!(
        missing.is_empty(),
        "not authored by any layout: {missing:?}"
    );
}

/// **One name, one widget - and the one layout where that is false.**
///
/// `oag_hud::draw_list` matches the always-on set by name and draws the
/// *first* sprite carrying each, which is a rule worth exactly as much as its
/// exception list. Across all eighteen composed layouts, `BestLapImage` in the
/// two speed-lap ones is the only always-on name authored more than once - the
/// second copy is the ghost's lap-time row, and this build loads no ghost. Any
/// other duplicate appearing here means a widget silently stopped being drawn.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_always_on_name_is_authored_once_bar_the_one_that_is_not() {
    let Some(mut archives) = open() else {
        return;
    };
    let mut duplicated: Vec<(&str, String, usize)> = Vec::new();
    for &(root, ..) in EXPECTED {
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        for sprite in composed(&mut archives, root).layout.sprites {
            if oag_hd::hud::ALWAYS_ON.contains(&sprite.name.as_str()) {
                *seen.entry(sprite.name).or_default() += 1;
            }
        }
        duplicated.extend(
            seen.into_iter()
                .filter(|&(_, n)| n > 1)
                .map(|(name, n)| (root, name, n)),
        );
    }
    assert_eq!(
        duplicated,
        vec![("/data/xml/speedlap_hud.xml", "BestLapImage".to_string(), 2)],
        "always-on names authored more than once"
    );
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

/// **Zone's ladder is authored by the Zone layouts and by nothing else.**
///
/// `oag_hd::hud::ALWAYS_ON` is title-wide rather than per-mode, so a name added
/// to it for one layout draws in every layout that happens to author it too.
/// The thirteen Zone widgets added on 2026-08-31 are safe on those terms only
/// because no other mode carries them - and this is the assertion that keeps
/// that true, in the direction that matters: a future name that *is* shared
/// fails here rather than appearing quietly in an arcade race.
///
/// **The two retro skins carry fewer, and that is a finding rather than a gap.**
/// Both write `<Image name="CurrentZonePanel">` with no `<Values>` at all - a
/// bare grouping element holding the eleven rows and carrying no art of its
/// own, where the default skin authors it as a 476x54 patch of
/// `HUD_Components_01.gtf`; `2097_hud` does the same to `ZoneBG`. So the retro
/// ladders are eleven ticks and eleven numbers with no column behind them and no
/// highlighted current row, which is what a Wipeout 3 or 2097 HUD looks like.
/// This build draws the default skin only (see `oag_hd::hud::skins`), so nothing
/// turns on it; it is asserted because a silent rise in either count would mean
/// the parser had started reading a bare group as a sprite.
///
/// `/data/xml/splitscreenzone_hud/` authors the same names and is deliberately
/// not among the eighteen roots - see `oag_hd::hud::ROOTS`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_zone_ladder_widget_is_authored_by_the_zone_layouts_alone() {
    let Some(mut archives) = open() else {
        return;
    };
    let ladder: Vec<&str> = oag_hd::hud::ALWAYS_ON
        .iter()
        .filter(|name| name.starts_with("Zone") || **name == "CurrentZonePanel")
        .copied()
        .collect();
    assert_eq!(ladder.len(), 13, "{ladder:?}");

    let mut counts: Vec<(&str, usize)> = Vec::new();
    for &(root, ..) in EXPECTED {
        let names: BTreeSet<String> = composed(&mut archives, root)
            .layout
            .sprites
            .into_iter()
            .map(|sprite| sprite.name)
            .collect();
        let found = ladder.iter().filter(|n| names.contains(**n)).count();
        if found > 0 {
            counts.push((root, found));
        }
    }
    assert_eq!(
        counts,
        vec![
            ("/data/xml/zone_hud.xml", 13),
            ("/data/xml/wo3_hud/zone_hud.xml", 12),
            ("/data/xml/2097_hud/zone_hud.xml", 11),
        ],
        "Zone ladder widgets, by layout"
    );
}

/// **The Zone HUD's left-hand ladder, as a frame rather than as a layout.**
///
/// The gap this closes was two-sided and either half alone draws nothing
/// visible: the sprites were missing from `ALWAYS_ON`, and the eleven
/// `ZonePlus<N>` labels carry no `idstring`, so `oag_hud::draw_list`'s
/// text allow-list dropped every one of them. Checked against the reference
/// frame (a maintainer capture, gitignored), which is a Zone
/// race on zone 1: `SUB-VENOM` on the current row, `1` beside it, and the ten
/// rows below it numbered `2` to `11`.
///
/// The class name is asked for by **rung**, not by zone - `zone_stage` here
/// stands in for what `crate::race::Scene::zone_stage` supplies in a real race,
/// and what puts a zone on a rung is unrecovered on this title. See
/// `oag_hd::hud::ZONE_SPEED_CLASSES`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_zone_race_draws_the_ladder_and_numbers_it_from_the_current_zone() {
    let Some(mut archives) = open() else {
        return;
    };
    let layout = composed(&mut archives, oag_hd::hud::layouts::ZONE).layout;

    // The strings the ladder's class names resolve through: HD's own English
    // plugin, read the same way a race reads it.
    let blob = archives
        .read_name(r"Data\Plugins\Languages\English\entries.xml")
        .expect("English entries.xml");
    let strings =
        oag_ui::language::StringTable::from_xml(&fexml::text(&blob).expect("entries.xml is text"));

    let blobs: Vec<(String, Vec<u8>)> = layout
        .textures()
        .iter()
        .map(|reference| {
            let entry = oag_hd::hud::texture_entry(reference);
            (
                (*reference).to_string(),
                archives.read_name(&entry).expect("a HUD texture"),
            )
        })
        .collect();
    let mut notes = Vec::new();
    let sheet = oag_hud::sprite::Sheet::build(&blobs, &mut notes);

    let cx = hud::Context {
        layout: &layout,
        strings: &strings,
        sheet: &sheet,
        art: oag_hd::hud::ART,
        hud_line_height: 92.0,
        small_line_height: 34.0,
        default_line_height: 34.0,
        default_border: layout.default_border(),
    };

    // Zone 1 on rung 1, which is the state the reference frame is in.
    let readout = oag_hud::Readout {
        zone: 1,
        zone_stage: 1,
        score: 1452,
        ..Default::default()
    };
    let frame = hud::draw_list(&cx, &readout);
    let text: Vec<String> = frame
        .hud_text
        .iter()
        .chain(&frame.small_text)
        .filter_map(|draw| match draw {
            oag_ui::frontend::Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();

    // The current zone and the ten coming up, and what the frame reads beside
    // them.
    for expected in ["ZONE", "SUB-VENOM", "1452"] {
        assert!(text.contains(&expected.to_string()), "{expected}: {text:?}");
    }
    for zone in 1..=11u32 {
        assert!(
            text.contains(&zone.to_string()),
            "row {zone} missing from {text:?}"
        );
    }

    // Rung 0 is the one with no name, so the class row is blank there while the
    // ladder still counts - the state an HD Zone race actually rests in, its
    // own trigger being unrecovered.
    let unnamed = hud::draw_list(
        &cx,
        &oag_hud::Readout {
            zone: 1,
            ..Default::default()
        },
    );
    let named = unnamed.hud_text.iter().chain(&unnamed.small_text).any(
        |draw| matches!(draw, oag_ui::frontend::Draw::Text { text, .. } if text == "SUB-VENOM"),
    );
    assert!(!named, "rung 0 named a speed class");

    // The column, the current row's panel and the eleven ticks: thirteen quads
    // that were absent before 2026-08-31.
    let rotated = frame
        .sprites
        .iter()
        .filter(|d| matches!(d, oag_ui::frontend::Draw::RotatedSprite { .. }))
        .count();
    // `ZoneBG`'s quarter turn plus ten tilted ticks; `ZonePlusLight5` authors no
    // `RotationTheta` at all, being the row on the arc's own axis.
    assert_eq!(rotated, 11, "rotated Zone widgets");
}

/// **The zone-8 reference frame, row for row.**
///
/// The zone-8 frame (a maintainer capture, gitignored) is the capture that
/// settled two things at once: the recovered threshold table's widest band so
/// far, and where the next class's name goes. It reads
///
/// ```text
///  8  SUB-RAPIER
///  9
/// 10
/// 11
/// 12  RAPIER
/// 13
/// ```
///
/// so zones `7`-`11` are Sub Rapier and the bump is at `12` - `oag_hd::race::ZONE_STAGES`'
/// own band, which no one-rung-per-zone reading could produce - and `RAPIER`
/// sits **level with its own `12`**, which is what `hud::draw::zone_next_row`
/// implements.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_zone_eight_frame_is_reproduced_row_for_row() {
    let Some(mut archives) = open() else {
        return;
    };
    let layout = composed(&mut archives, oag_hd::hud::layouts::ZONE).layout;
    let blob = archives
        .read_name(r"Data\Plugins\Languages\English\entries.xml")
        .expect("English entries.xml");
    let strings =
        oag_ui::language::StringTable::from_xml(&fexml::text(&blob).expect("entries.xml is text"));
    let mut notes = Vec::new();
    let sheet = oag_hud::sprite::Sheet::build(&[], &mut notes);
    let cx = hud::Context {
        layout: &layout,
        strings: &strings,
        sheet: &sheet,
        art: oag_hd::hud::ART,
        hud_line_height: 92.0,
        small_line_height: 34.0,
        default_line_height: 34.0,
        default_border: layout.default_border(),
    };

    // Zone 8: rung 5 (`5 Sub Rapier`), next bump four rows down at zone 12.
    let zone = 8u16;
    let stages = oag_hd::race::ZONE_STAGES;
    let readout = oag_hud::Readout {
        zone: u32::from(zone),
        zone_stage: stages.stage_for(zone).expect("a band"),
        zone_next_in: stages.next_zone(zone).map(|next| u32::from(next - zone)),
        ..Default::default()
    };
    assert_eq!(readout.zone_stage, 5);
    assert_eq!(readout.zone_next_in, Some(4));

    let frame = hud::draw_list(&cx, &readout);
    let text: Vec<(String, f32, f32)> = frame
        .hud_text
        .iter()
        .chain(&frame.small_text)
        .filter_map(|draw| match draw {
            oag_ui::frontend::Draw::Text { text, x, y, .. } => Some((text.clone(), *x, *y)),
            _ => None,
        })
        .collect();
    let at = |want: &str| -> (f32, f32) {
        text.iter()
            .find(|(t, ..)| t == want)
            .map(|&(_, x, y)| (x, y))
            .unwrap_or_else(|| panic!("{want} missing from {text:?}"))
    };

    // The two class names the frame shows, and the ladder around them.
    at("SUB-RAPIER");
    let rapier = at("RAPIER");
    for n in 8..=18u32 {
        at(&n.to_string());
    }

    // `RAPIER` is on `12`'s own line and to its right - the frame's arrangement,
    // and the reason this test exists rather than a comment.
    let twelve = at("12");
    assert!(
        (rapier.1 - twelve.1).abs() < 1.0,
        "RAPIER at y={} is not on row 12's line (y={})",
        rapier.1,
        twelve.1
    );
    assert!(
        rapier.0 > twelve.0,
        "RAPIER at x={} is not right of its own number (x={})",
        rapier.0,
        twelve.0
    );
}

/// **Zone 0 numbers the rows ahead and leaves the current one blank**, which is
/// the state the layout's own placeholder strings spell out.
///
/// `ZonePlus0` is authored with no `string` and `ZonePlus1`-`ZonePlus10` with
/// `1` through `10` - `zone + N` at `zone = 0`. An earlier revision gated the
/// whole ladder on `zone > 0` the way `Zone` and `Score` are gated, which left
/// the column empty for the first ten seconds of every run; the disc says
/// otherwise, and this is that reading checked against the real layout.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_ladder_before_the_first_zone_is_the_layouts_own_placeholders() {
    let Some(mut archives) = open() else {
        return;
    };
    let layout = composed(&mut archives, oag_hd::hud::layouts::ZONE).layout;
    let strings = oag_ui::language::StringTable::default();
    let mut notes = Vec::new();
    let sheet = oag_hud::sprite::Sheet::build(&[], &mut notes);
    let cx = hud::Context {
        layout: &layout,
        strings: &strings,
        sheet: &sheet,
        art: oag_hd::hud::ART,
        hud_line_height: 92.0,
        small_line_height: 34.0,
        default_line_height: 34.0,
        default_border: layout.default_border(),
    };
    let frame = hud::draw_list(&cx, &oag_hud::Readout::default());
    // A bare digit string is no longer unique to the `ZonePlus*` ladder:
    // `ShieldBarText` on HD draws one too now that `oag_title::HudArt::
    // shield_percent` is `false` for this title (a `Readout::default()`'s
    // zero shield used to format as `0%`, which this all-digit filter never
    // matched; it now formats as a bare `0`). Narrowed to the ladder's own
    // range - `ZonePlus1`-`10`'s authored values are exactly `1`-`10` -
    // rather than to "any text that happens to be all digits", which stopped
    // being unique to this widget family.
    let numbers: Vec<String> = frame
        .hud_text
        .iter()
        .chain(&frame.small_text)
        .filter_map(|draw| match draw {
            oag_ui::frontend::Draw::Text { text, .. }
                if text.parse::<u32>().is_ok_and(|n| (1..=10).contains(&n)) =>
            {
                Some(text.clone())
            }
            _ => None,
        })
        .collect();

    // The ten rows ahead, and no eleventh: the current row carries no number
    // before the first zone.
    let authored: Vec<String> = (1..=10u32).map(|n| n.to_string()).collect();
    assert_eq!(numbers, authored, "the zone-0 ladder");

    // And they are the strings the disc itself authors on those widgets, which
    // is what makes this a check rather than a restatement.
    for (n, want) in (1..=10u32).zip(&authored) {
        assert_eq!(
            layout
                .label(&format!("ZonePlus{n}"))
                .and_then(|l| l.string.as_deref()),
            Some(want.as_str()),
            "ZonePlus{n}'s placeholder"
        );
    }
    assert_eq!(
        layout.label("ZonePlus0").and_then(|l| l.string.as_deref()),
        None,
        "ZonePlus0 authors no placeholder"
    );
}
