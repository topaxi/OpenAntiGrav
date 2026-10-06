//! Which materials compile which Zone shader, counted off the disc.
//!
//! # What this file is the evidence for
//!
//! `crates/render/tests/zone_recolour.rs` proves the rule this engine draws
//! reaches a pixel. This proves the rule is the *right* one to draw - which is
//! the question that actually needed settling, because a shader that matches a
//! screenshot by over-applying is indistinguishable from a correct one in a
//! screenshot.
//!
//! Wipeout HD's Zone effect is not an engine program: it is a variant compiled
//! into the materials themselves, and the disc carries **two different Zone
//! shaders**. `docs/ghidra/functions/ps3-hdfury-eu/zone-shader.md` documented
//! one of them from three materials and generalised it; this counts both over
//! the whole corpus and finds that the one it documented is 0.6% of the disc
//! and appears on **no racing circuit at all**.
//!
//! ```text
//! zoneBase   20,084 blocks   rim^10 and rim^5 in all   no black mask in any
//! zoneAniso     130 blocks   neither in any            a black mask in all
//! ```
//!
//! The discriminator is not "textured versus untextured", which was the
//! hypothesis: 18,032 of the `zoneBase` blocks sample a `zoneTex*` too. It is
//! the *environment* - see
//! [`each_racing_circuit_compiles_only_the_zone_base_shape`].
//!
//! `#[ignore]`d because it needs `data/images/hdfury-ps3-eu-dec.iso`; run with
//! `just test-data`.

mod rcsmodel_common;

use std::collections::{BTreeMap, BTreeSet};

use oag_rcs::rcsmaterial::{Declared, fragment, name_hash};
use rcsmodel_common::image;

/// Every archive on the disc that holds a `.rcsmaterial`. The whole disc
/// rather than one archive, because the circuits are split across DATA02 and
/// DATA03 while DATA00 holds only the four unnumbered ones - a census over
/// DATA00 alone would miss every numbered circuit and is how an earlier count
/// of this went wrong.
const ALL_ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

/// Which of the disc's two Zone shaders a fragment block compiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Shape {
    /// `surface = zoneTex * zoneEffect + zoneBase * rim^10 + zoneBaseAlt * rim^5`.
    /// The circuit shape, and what `mesh.wgsl` draws.
    Base,
    /// `surface = albedo + zoneCol * (1 - blackMask)`, with the rim term a
    /// `zoneAnisoPalette` lookup. The Zone-arena shape.
    Aniso,
}

/// One Zone-bearing fragment block, reduced to what the census asks of it.
struct Block {
    path: String,
    shape: Shape,
    /// The block's code raises something to the 10th and the 5th power - a
    /// `MUL` by each literal between the `LG2` and the `EX2`.
    rim_exponents: bool,
    /// The block's code carries the `100000` of the hand-rolled
    /// `step(0, albedo.r + albedo.g + albedo.b)`.
    black_mask: bool,
    /// The block samples at least one `zoneTex*`.
    samples_zone_texture: bool,
    /// The block declares `zoneTexVis` **and** its code fetches from that
    /// unit - so it draws the audio-spectrum glow rather than merely naming
    /// the lookup.
    samples_vis: bool,
    /// Every *negative* literal a saturating `ADD` in this block carries.
    ///
    /// The visualiser glow opens with `saturate(N.y - t)`, so `-t` is here.
    /// See [`the_visualiser_glow_is_gated_to_up_facing_surfaces_everywhere`]
    /// for why this is a list rather than one value: the assertion is that a
    /// visualiser block carries exactly one, from a two-element set.
    negative_saturating_literals: Vec<f32>,
}

/// Every fragment block on one archive that names a Zone parameter or sampler.
fn zone_blocks(archive: &str) -> Vec<Block> {
    let image = image().expect("checked by the caller");
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
    let Ok(mut open) = oag_assets::psarc::Archive::open(&spec) else {
        return Vec::new();
    };
    let paths: Vec<String> = open
        .paths()
        .iter()
        .filter(|p| p.ends_with(".rcsmaterial"))
        .cloned()
        .collect();

    let base_inner = name_hash("zoneBaseInner");
    let vis = name_hash("zoneTexVis");
    let aniso_power = name_hash("zoneAnisoPower");
    let aniso_palette = [
        name_hash("zoneAnisoPalette"),
        name_hash("zoneAnisoPaletteOuter"),
    ];
    let zone_textures: Vec<u32> = [
        "zoneTexInner",
        "zoneTexOuter",
        "zoneTexInnerNearest",
        "zoneTexOuterNearest",
        "zoneTexVis",
    ]
    .iter()
    .map(|n| name_hash(n))
    .collect();

    let mut out = Vec::new();
    for path in paths {
        let Ok(bytes) = open.read_path(&path) else {
            continue;
        };
        // Every `SHO` block in the file, fragment ones only. Walked by search
        // rather than through the variant table so a block the table does not
        // reach is still counted - the census is about what the file holds.
        let mut at = 0usize;
        while let Some(found) = find_block(&bytes, at) {
            at = found + 4;
            if bytes.get(found + 4..found + 8) != Some(&[0, 0, 0, 1]) {
                continue;
            }
            let Some(declared) = Declared::parse(&bytes, found) else {
                continue;
            };
            let names_zone = declared.parameters.iter().any(|h| is_zone_parameter(*h))
                || declared.samplers.iter().any(|(h, _)| is_zone_parameter(*h));
            if !names_zone {
                continue;
            }
            let Some(program) = fragment::Program::parse(&bytes, found) else {
                continue;
            };

            // The units the code actually fetches from, so a sampler the block
            // declares but never reads does not count as used.
            let fetched: BTreeSet<u32> = program
                .instructions
                .iter()
                .filter(|i| i.is_texture())
                .map(|i| u32::from(i.unit))
                .collect();
            let sampled = |wanted: &[u32]| {
                declared
                    .samplers
                    .iter()
                    .any(|(h, unit)| wanted.contains(h) && fetched.contains(unit))
            };

            let is_base = declared.parameters.contains(&base_inner);
            let is_aniso = declared.parameters.contains(&aniso_power) || sampled(&aniso_palette);
            let shape = match (is_base, is_aniso) {
                (true, false) => Shape::Base,
                (false, true) => Shape::Aniso,
                // Both or neither: left out of the census rather than forced
                // into a bucket. The disjointness assertion below is what says
                // "both" never happens.
                _ => {
                    assert!(
                        !(is_base && is_aniso),
                        "{path} block at {found:#x} declares both Zone shapes: \
                         the two are supposed to be mutually exclusive"
                    );
                    continue;
                }
            };

            let literals = |want: f32| {
                program.instructions.iter().any(|i| {
                    i.constant
                        .is_some_and(|c| c.iter().any(|v| (*v - want).abs() < 1e-3))
                })
            };
            let mut negative_saturating_literals: Vec<f32> = program
                .instructions
                .iter()
                .filter(|i| i.saturate && i.name() == Some("ADD"))
                .filter_map(|i| i.constant)
                .flatten()
                .filter(|v| *v < 0.0)
                .collect();
            negative_saturating_literals.sort_by(|a, b| a.total_cmp(b));
            negative_saturating_literals.dedup();
            out.push(Block {
                path: path.clone(),
                shape,
                rim_exponents: literals(10.0) && literals(5.0),
                black_mask: literals(100_000.0),
                samples_zone_texture: sampled(&zone_textures),
                samples_vis: sampled(&[vis]),
                negative_saturating_literals,
            });
        }
    }
    out
}

/// The next `SHO\x08` at or after `from`.
fn find_block(data: &[u8], from: usize) -> Option<usize> {
    data.get(from..)?
        .windows(4)
        .position(|w| w == b"SHO\x08")
        .map(|i| from + i)
}

/// Whether a name hash is one of the sixteen Zone parameters or samplers.
fn is_zone_parameter(hash: u32) -> bool {
    const NAMES: [&str; 16] = [
        "zoneColourTint",
        "zoneEffectInner",
        "zoneEffectOuter",
        "zoneBaseInner",
        "zoneBaseOuter",
        "zoneBaseAltInner",
        "zoneBaseAltOuter",
        "zoneOrigin",
        "zoneTexInner",
        "zoneTexOuter",
        "zoneTexInnerNearest",
        "zoneTexOuterNearest",
        "zoneTexVis",
        "zoneAnisoPalette",
        "zoneAnisoPaletteOuter",
        "zoneAnisoPower",
    ];
    NAMES.iter().any(|n| name_hash(n) == hash)
}

/// **The disc carries two Zone shaders, and the one this engine draws is
/// 99.3% of them.**
///
/// The counts are the load-bearing numbers. `zone-shader.md` recorded the
/// `zoneAniso` shape as the disc's Zone rule, read from three materials; if
/// that were right, this ratio would be the other way round.
#[test]
#[ignore]
fn the_zone_base_shape_is_the_disc_s_zone_shader_and_the_aniso_one_is_the_exception() {
    let Some(_) = image() else {
        return;
    };
    let blocks: Vec<Block> = ALL_ARCHIVES.iter().flat_map(|a| zone_blocks(a)).collect();
    let base = blocks.iter().filter(|b| b.shape == Shape::Base).count();
    let aniso = blocks.iter().filter(|b| b.shape == Shape::Aniso).count();

    assert_eq!(
        (base, aniso),
        (20_084, 130),
        "the two-shape census moved; {} Zone blocks over the whole disc",
        blocks.len()
    );
}

/// **Each shape carries its own arithmetic, and never the other's.**
///
/// This is what makes the two genuinely different shaders rather than one
/// shader with two parameter sets: `rim^10`/`rim^5` and the `100000` black
/// mask are perfectly anti-correlated across 20,214 blocks. It is also the
/// specific retraction owed to `zone-shader.md`, whose `surface = albedo +
/// zoneCol * (1 - blackMask)` line does not apply to a single one of the
/// 20,084 blocks a racing circuit compiles.
#[test]
#[ignore]
fn the_rim_exponents_and_the_black_mask_never_appear_in_the_same_block() {
    let Some(_) = image() else {
        return;
    };
    for archive in ALL_ARCHIVES {
        for block in zone_blocks(archive) {
            match block.shape {
                Shape::Base => {
                    assert!(
                        block.rim_exponents,
                        "{archive} {} declares zoneBaseInner but its code has no \
                         rim^10/rim^5 pair",
                        block.path
                    );
                    assert!(
                        !block.black_mask,
                        "{archive} {} carries the black-mask literal in the \
                         zoneBase shape",
                        block.path
                    );
                }
                Shape::Aniso => {
                    assert!(
                        block.black_mask,
                        "{archive} {} is the aniso shape but has no black mask",
                        block.path
                    );
                    assert!(
                        !block.rim_exponents,
                        "{archive} {} carries the rim exponents in the aniso shape",
                        block.path
                    );
                }
            }
        }
    }
}

/// **The two families are not "textured" and "untextured".**
///
/// That was the hypothesis this work set out to test, and the disc refutes it:
/// most `zoneBase` blocks sample a zone texture in the same program. So a port
/// cannot pick the shape by asking whether a material has a texture, and the
/// selection rule had to come from somewhere else - it came from
/// [`each_racing_circuit_compiles_only_the_zone_base_shape`].
#[test]
#[ignore]
fn most_zone_base_blocks_sample_a_zone_texture_in_the_same_program() {
    let Some(_) = image() else {
        return;
    };
    let blocks: Vec<Block> = ALL_ARCHIVES.iter().flat_map(|a| zone_blocks(a)).collect();
    let textured = blocks
        .iter()
        .filter(|b| b.shape == Shape::Base && b.samples_zone_texture)
        .count();
    assert_eq!(
        textured, 18_032,
        "the overlap moved; `zoneBase` and `zoneTex*` are supposed to coexist \
         in most of the corpus"
    );
}

/// **The selection rule: it is the environment, and every racing circuit is
/// the same one.**
///
/// This is the assertion that says applying the `zoneBase` rule to every draw
/// of a Zone race is faithful rather than over-applied. The `zoneAniso` shape
/// exists only inside `zone_1`..`zone_4`, HD's four dedicated Zone arenas -
/// which this engine does not load - and two of the three materials
/// `zone-shader.md` read are from them.
#[test]
#[ignore]
fn each_racing_circuit_compiles_only_the_zone_base_shape() {
    let Some(_) = image() else {
        return;
    };
    let mut by_environment: BTreeMap<String, BTreeSet<Shape>> = BTreeMap::new();
    for block in ALL_ARCHIVES.iter().flat_map(|a| zone_blocks(a)) {
        let mut parts = block.path.trim_start_matches('/').split('/');
        let (Some("data"), Some("environments"), Some(name)) =
            (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        by_environment
            .entry(name.to_string())
            .or_default()
            .insert(block.shape);
    }

    let circuits: Vec<&String> = by_environment
        .keys()
        .filter(|name| !name.starts_with("zone_"))
        .collect();
    assert_eq!(
        circuits.len(),
        12,
        "expected twelve racing circuits, got {circuits:?}"
    );
    for circuit in &circuits {
        assert_eq!(
            by_environment[*circuit],
            BTreeSet::from([Shape::Base]),
            "{circuit} compiles a Zone shape other than the circuit one"
        );
    }

    // And the arenas are where the other shape lives, so its absence above is
    // a real split rather than this test failing to find it anywhere.
    let arenas: Vec<&String> = by_environment
        .keys()
        .filter(|name| name.starts_with("zone_"))
        .collect();
    assert_eq!(arenas.len(), 4, "expected four Zone arenas, got {arenas:?}");
    for arena in arenas {
        assert!(
            by_environment[arena].contains(&Shape::Aniso),
            "{arena} carries no aniso block; the split is not what it looks like"
        );
    }
}

/// **Craft carry no Zone variant at all**, which is why `race::scene::frame`
/// binds a disabled Zone to the ships.
///
/// The Zone surface replaces a material's albedo, so this is the difference
/// between craft that stay coloured against a monochrome circuit - what the
/// original does - and craft that go monochrome with it.
#[test]
#[ignore]
fn no_ship_or_weapon_material_compiles_a_zone_variant() {
    let Some(_) = image() else {
        return;
    };
    let mut seen = 0usize;
    for archive in ALL_ARCHIVES {
        for block in zone_blocks(archive) {
            let path = block.path.trim_start_matches('/');
            assert!(
                !path.starts_with("data/materials/ships/") && !path.starts_with("data/weapons/"),
                "{path} compiles a Zone variant; craft were excluded on the \
                 strength of there being none"
            );
            seen += 1;
        }
    }
    assert!(seen > 0, "no Zone block found at all; the sweep is broken");
}

/// **The audio-spectrum glow is gated to up-facing surfaces on every material
/// on the disc, and the threshold takes exactly two values.**
///
/// `zone-shader.md` reads the glow as `saturate(N.y - 0.5) * (1 -
/// windowDepth) * E.w * zoneTexVis[band].rgb`, from two materials. The
/// `saturate(N.y - 0.5)` half is what makes the visualiser a *floor* display,
/// which is how the microcode independently produced the maintainer's own
/// play observation - and it is also the part that could not explain the
/// other half of that observation, the billboards. So the obvious hypothesis
/// was a second glow shape hiding in a billboard material. **There is no
/// second shape.** Every one of the 18,050 blocks that actually fetches
/// `zoneTexVis` carries exactly one negative saturating `ADD` literal, and it
/// is `-0.5` in 16,834 of them and `-1` in the other 1,216 - including all 64
/// blocks of `billboarddiffuse`, all 56 of `cf_billboard1` and all 212 of
/// `nr_crowd_bustle`.
///
/// **`-1` reads as the same gate turned off**, at confidence 70 rather than
/// the count's own: `saturate(N.y - 1)` is zero for every unit normal, so
/// those blocks multiply the lookup by zero. That rests on the register
/// feeding the `ADD` being a normalised `N.y`, decoded through opcode `0x3b`,
/// named `DIVSQ` (`a / sqrt(b)`) 2026-09-25, confidence 84, see
/// `docs/formats/rcsmaterial.md`. That reading comes from its pairing with a
/// `DP3` of a vector against itself, twice in the same block: `DP3(v,v)->d;
/// DIVSQ(v,d) = v * rsqrt(d)`, the normalize idiom the naming itself
/// confirmed. What is *measured* here, and what this asserts, is the
/// two-value threshold set; the "turned off" reading is a separate
/// interpretation the opcode's own name does not itself settle.
///
/// The other half of the answer is geometry rather than microcode - see
/// `rcsmodel_material_ground_truth.rs`'s
/// `billboard_geometry_is_mixed_where_crowd_and_banner_geometry_is_not`.
#[test]
#[ignore]
fn the_visualiser_glow_is_gated_to_up_facing_surfaces_everywhere() {
    let Some(_) = image() else {
        return;
    };
    let blocks: Vec<Block> = ALL_ARCHIVES.iter().flat_map(|a| zone_blocks(a)).collect();
    let vis: Vec<&Block> = blocks.iter().filter(|b| b.samples_vis).collect();

    let mut half = 0usize;
    let mut one = 0usize;
    for block in &vis {
        match block.negative_saturating_literals.as_slice() {
            [v] if (*v + 0.5).abs() < 1e-6 => half += 1,
            [v] if (*v + 1.0).abs() < 1e-6 => one += 1,
            other => panic!(
                "{} fetches zoneTexVis with saturating-ADD literals {other:?}; \
                 the up-gate is supposed to be one of exactly two thresholds",
                block.path
            ),
        }
    }
    assert_eq!(
        (vis.len(), half, one),
        (18_050, 16_834, 1_216),
        "the visualiser gate census moved"
    );
}

/// **Every billboard, crowd, screen and scanline material carries the up-gate
/// too**, which is the specific refutation of "the billboard half must be a
/// second fragment block with a different gate".
///
/// Kept apart from the count above because it is the claim a reader of
/// `hd-zone-stage-textures-are-grounded.md`'s Open item needs, and it is
/// named by material family rather than by a total: a regression that dropped
/// one family out of the sweep would leave the count assertion to catch it by
/// arithmetic alone, which is not the same as naming it.
#[test]
#[ignore]
fn no_billboard_or_crowd_material_compiles_an_ungated_visualiser() {
    let Some(_) = image() else {
        return;
    };
    const FAMILIES: [&str; 5] = ["billboard", "crowd", "banner", "screen", "scanline"];
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for block in ALL_ARCHIVES.iter().flat_map(|a| zone_blocks(a)) {
        if !block.samples_vis {
            continue;
        }
        let leaf = block.path.rsplit('/').next().unwrap_or_default();
        let Some(family) = FAMILIES.iter().find(|f| leaf.contains(*f)) else {
            continue;
        };
        assert_eq!(
            block.negative_saturating_literals.len(),
            1,
            "{} draws the visualiser with no single up-gate threshold",
            block.path
        );
        *seen.entry(family).or_default() += 1;
    }
    for family in FAMILIES {
        assert!(
            seen.get(family).is_some_and(|n| *n > 0),
            "no material named *{family}* draws the visualiser at all; the \
             sweep is not finding the family it is supposed to refute"
        );
    }
}
