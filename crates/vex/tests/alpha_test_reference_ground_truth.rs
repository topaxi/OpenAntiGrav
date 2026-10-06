//! Which alpha-test reference each batch on a real disc asks for, and what
//! applying it costs in texels.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! # What this is the gate for
//!
//! [`vex::Batch::alpha_test_reference`] reads a per-batch value out of two flag
//! bits, and the value it can return is `0x7f` - **half**, against the
//! `1/255` this project drew every cutout at until it was recovered. Moving a
//! batch to a stricter reference is the direction where geometry *disappears*,
//! so the reading does not get to land on the strength of the decompile alone.
//!
//! Three things are measured here, on every corpus this project can reach:
//!
//! 1. **Three of the four selector-bit combinations occur, and the third one
//!    discriminates.** `(0x0800, 0x00)` and `(0x0880, 0x20)` are the bulk, and
//!    every corpus also authors `(0x0880, 0x00)` - `pass_mask & 0x80` set with
//!    `header_flags & 0x20` clear. That is the case the two candidate readings
//!    disagree on: the recovered branch gives it `0`, and a selector keyed on
//!    `header_flags & 0x20` alone would give it `0x7f`. It is Wipeout Pure's
//!    `Speedup Pad`, whose glow texture tops out at `58/255` - so the wrong
//!    reading discards the pad entirely, which is the exact regression
//!    `crates/render/tests/pad_alpha_test_ground_truth.rs` renders and catches.
//! 2. **Nothing vanishes.** No cutout batch's texture is discarded whole by
//!    the reference the batch itself asks for, on Pulse either platform.
//! 3. **The `0x7f` bucket is inert in *texel* space, and that is not the same
//!    as inert on screen.** Its textures are binary cutouts - leaves, crowds,
//!    railings - so `> 0x7f` and `> 0` keep exactly the same texels of the
//!    decoded image. The sampler does not read the decoded image: it filters
//!    and mips it, which turns a `{0, 255}` edge into intermediate alphas, and
//!    `0x7f` moves the cut there from "any coverage" to "half coverage". So a
//!    leaf's silhouette tightens by a pixel or so. That is what the original's
//!    own filtered sampler does too, and it is the whole of the measured
//!    on-screen delta - see `crates/render/examples/threshold_probe.rs`, which
//!    isolates the two buckets. **This assertion is a texel count and must not
//!    be quoted as a claim about the picture.**
//!
//! The counts are frozen rather than merely bounded because they are what a
//! future reading has to move *deliberately*: a parser change that silently
//! reclassifies 7,823 batches is exactly what this catches.

use std::collections::BTreeMap;
use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::vex;

/// One corpus' worth of counts.
#[derive(Debug, Default, PartialEq, Eq)]
struct Census {
    /// `reference -> batches`, over batches the renderer routes into its cutout
    /// list (`!is_transparent() && is_alpha_tested()`).
    cutout: BTreeMap<u8, usize>,
    /// `(pass_mask & 0x880, header_flags & 0x30) -> batches`, same population.
    bits: BTreeMap<(u16, u8), usize>,
    /// `reference -> (texels with alpha > 0, of those discarded at that
    /// reference)`, over cutout batches whose texture is embedded in the same
    /// file and decodes.
    texels: BTreeMap<u8, (u64, u64)>,
    /// Cutout batches whose texture loses *every* texel at its own reference.
    vanishing: BTreeMap<u8, usize>,
}

fn blobs(disc: &mut DiscImage, archive_name: &str) -> Vec<Vec<u8>> {
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path.ends_with(archive_name))
        .unwrap_or_else(|| panic!("{archive_name} present"))
        .clone();
    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, Directory::directory_len(count))
        .expect("directory");
    let dir = Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    let mut out = Vec::new();
    for entry in &dir.entries {
        if entry.size == 0 {
            continue;
        }
        let raw = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let bytes = match entry.compression {
            Compression::None => raw,
            Compression::Lzss => {
                oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
            }
            Compression::Zlib => continue,
        };
        if !vex::has_magic(&bytes) {
            continue;
        }
        out.push(bytes);
    }
    out
}

fn census(blobs: &[Vec<u8>]) -> Census {
    let mut out = Census::default();
    for bytes in blobs {
        let Ok(nodes) = vex::nodes(bytes) else {
            continue;
        };
        let Ok(classes) = vex::classes_of(bytes) else {
            continue;
        };
        // A `Speedup Pad`/`Weapon Pad` node's payload is a mesh payload - see
        // `oag_mesh::mesh::build_pads` - and the pads are where the third
        // selector pattern lives, so leaving them out of the walk is what made
        // an earlier version of this file report only two.
        let drawn: Vec<u32> = [classes.mesh, classes.speedup_pad, classes.weapon_pad]
            .into_iter()
            .flatten()
            .collect();
        // Each texture's alpha histogram, decoded once per file.
        let alpha: Vec<Option<BTreeMap<u8, u64>>> = vex::textures(bytes)
            .map(|ts| {
                ts.into_iter()
                    .map(|t| {
                        t.map(|t| {
                            let mut h: BTreeMap<u8, u64> = BTreeMap::new();
                            for texel in t.to_rgba().as_chunks::<4>().0 {
                                *h.entry(texel[3]).or_default() += 1;
                            }
                            h
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        for node in nodes.iter().filter(|n| drawn.contains(&n.class_id)) {
            let range = node.payload();
            if range.end > bytes.len() {
                continue;
            }
            let payload = &bytes[range];
            let materials = vex::mesh_materials(payload);
            for list in 0..2 {
                let Ok(batches) = vex::mesh_batches(payload, list) else {
                    continue;
                };
                for batch in &batches {
                    if batch.is_transparent() || !batch.is_alpha_tested() {
                        continue;
                    }
                    let Some(reference) = batch.alpha_test_reference() else {
                        continue;
                    };
                    *out.cutout.entry(reference).or_default() += 1;
                    *out.bits
                        .entry((batch.pass_mask & 0x0880, batch.header_flags & 0x30))
                        .or_default() += 1;
                    let Some(Some(hist)) = materials
                        .get(usize::from(batch.material_index))
                        .copied()
                        .flatten()
                        .map(|m| m.texture as usize)
                        .and_then(|t| alpha.get(t))
                    else {
                        continue;
                    };
                    let drawn: u64 = hist.iter().filter(|(a, _)| **a > 0).map(|(_, n)| n).sum();
                    let kept: u64 = hist
                        .iter()
                        .filter(|(a, _)| **a > reference)
                        .map(|(_, n)| n)
                        .sum();
                    let e = out.texels.entry(reference).or_default();
                    e.0 += drawn;
                    e.1 += drawn - kept;
                    if kept == 0 {
                        *out.vanishing.entry(reference).or_default() += 1;
                    }
                }
            }
        }
    }
    out
}

fn of(image: &str, archive_name: &str) -> Option<Census> {
    let path: PathBuf = oag_testdata::image(image)?;
    let mut disc = DiscImage::open(&path).expect("open");
    Some(census(&blobs(&mut disc, archive_name)))
}

/// **PSP Pulse, the corpus the selector was read against.**
///
/// 8,561 cutout batches, two patterns, and the strict reference removes
/// **nothing at all**: every one of the 1,500 `0x7f` batches is on a texture
/// whose alpha is binary, so `> 0x7f` and `> 0` keep the same texels.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn psp_pulses_cutout_batches_split_two_ways_and_the_strict_reference_hides_nothing() {
    let Some(c) = of("data/images/pulse-psp-usa.chd", "PSP_GAME/USRDIR/Data.wad") else {
        return;
    };
    assert_eq!(
        c.cutout,
        BTreeMap::from([(0x10, 7_823), (0x7f, 1_500)]),
        "the disc's own cutout population, by the reference each batch asks for"
    );
    assert_eq!(
        c.bits,
        BTreeMap::from([((0x0800, 0x00), 1_500), ((0x0880, 0x20), 7_823)]),
        "Pulse's PSP build authors only two of the four combinations - the \
         discriminating third is Pure's, below"
    );
    assert_eq!(
        c.texels.get(&0x7f),
        Some(&(3_016_396, 0)),
        "the 0x7f batches are binary cutouts: the strictest reference discards none of their texels"
    );
    assert_eq!(
        c.texels.get(&0x10),
        Some(&(17_005_223, 431_576)),
        "the 0x10 batches lose their sub-16 fringe, which the cutout pipeline was drawing fully opaque"
    );
    assert_eq!(
        c.vanishing,
        BTreeMap::new(),
        "no batch loses its whole texture"
    );
}

/// **PSP Pure, a second title, and the discriminating case.** From a different
/// executable and a different `.vex` version - so the authoring convention is
/// not a Pulse-only accident.
///
/// 13 batches do lose their whole texture, and they are all
/// `Z3_whiteblue_cloud_GLOW.tga`, whose alpha is a **uniform 3**. Under this
/// project's old `1/255` those 13 draw through the *cutout* pipeline, which
/// forces alpha to `1.0` and writes depth - so a 3/255 cloud was being painted
/// solid. Discarding it is the fix, not the regression.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn psp_pure_authors_the_discriminating_third_pattern_on_its_speedup_pads() {
    let Some(c) = of("data/images/pure-psp-usa.chd", "PSP_GAME/USRDIR/Data.wad") else {
        return;
    };
    assert_eq!(
        c.cutout,
        BTreeMap::from([(0, 349), (0x10, 1_749), (0x7f, 956)])
    );
    assert_eq!(
        c.bits,
        BTreeMap::from([
            ((0x0800, 0x00), 956),
            // The discriminating pattern: `pass_mask & 0x80` set, `header_flags
            // & 0x20` clear. Pure's `Speedup Pad`, and the reason the branch
            // order is not a free choice.
            ((0x0880, 0x00), 349),
            ((0x0880, 0x20), 1_749),
        ])
    );
    assert_eq!(
        c.texels.get(&0x7f),
        Some(&(2_751_124, 5_323)),
        "0.19% of texels - see the module docs on what this does and does not say"
    );
    assert_eq!(
        c.vanishing,
        BTreeMap::from([(0x10, 13)]),
        "only Z3_whiteblue_cloud_GLOW, uniform alpha 3, which the original discards too"
    );
}

/// **PS2 Pulse, a third corpus and a second platform.** The selector was read
/// in the PSP executable and this one has not been read - what this asserts is
/// that the PS2 build authors the same two bit patterns in the same fields, so
/// the shared `.vex` path is not being handed a value its own engine would
/// have derived differently.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn ps2_pulse_authors_the_same_three_patterns() {
    let Some(c) = of("data/images/pulse-ps2-eu.chd", "WADS2.WAD") else {
        return;
    };
    assert_eq!(
        c.cutout,
        BTreeMap::from([(0, 20), (0x10, 14_046), (0x7f, 1_313)])
    );
    assert_eq!(
        c.bits,
        BTreeMap::from([
            ((0x0800, 0x00), 1_313),
            ((0x0880, 0x00), 20),
            ((0x0880, 0x20), 14_046),
        ])
    );
    assert_eq!(
        c.vanishing,
        BTreeMap::new(),
        "PS2 model textures live outside the .vex, so only the embedded ones are measured here"
    );
}
