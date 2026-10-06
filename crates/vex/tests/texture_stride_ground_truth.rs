//! Establishes that a PSP `.vex` texture's rows are padded to a 16-byte stride,
//! against every `.vex` file on the Pulse PSP disc.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! # What this is for
//!
//! [`vex::textures`] used to read a texture's rows back to back, and that is
//! right for the overwhelming majority of them: a 4-bit texture 32 pixels wide
//! already fills a 16-byte row, so the padding is a no-op and nothing looks
//! wrong. It is **not** right for the narrow ones, and there the picture comes
//! out built from the wrong bytes entirely - which is how the start-line
//! gantry's advertising board on `16_Track` rendered as a blown-out white block
//! (`docs/formats/vex.md`, "Texture rows are padded to 16 bytes").
//!
//! The claim under test is arithmetic and needs no picture:
//!
//! > Every `Texture` node declares its own total texel-block size at payload
//! > `+0x0c`. Summing `align_up(width * bpp / 8, 16) * height` over the node's
//! > declared mip levels reproduces that number exactly; summing the unpadded
//! > `width * bpp / 8 * height` does not.
//!
//! That is a closure argument over a field the decoder does not otherwise use,
//! so a wrong stride cannot satisfy it by coincidence across thousands of
//! textures - and it is checked here on every `.vex` on the disc rather than on
//! the one circuit that exposed the fault.
//!
//! The second test measures the blast radius: which textures the padding
//! actually changes, i.e. which ones the old decoder built out of the wrong
//! bytes. It is the list a reviewer wants when deciding what a screenshot
//! should be re-checked against.

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::vex::{self, texture_row_bytes, texture_row_stride};

/// The archive holding every circuit and every ship.
const PSP_DATA: &str = "PSP_GAME/USRDIR/Data.wad";

/// Offset of a `Texture` payload's declared texel-block size.
const TEXEL_SIZE_AT: usize = 0x0c;

/// Smallest number of textures the sweep must reach before its agreement means
/// anything. Well under the real figure; a count that collapses is a broken
/// walk, not a disc with fewer textures on it.
const MIN_TEXTURES: usize = 2_000;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// One `Texture` node, reduced to the five numbers this file is about.
struct TextureHeader {
    label: String,
    width: u16,
    height: u16,
    bits_per_pixel: u8,
    mip_count: u8,
    texel_size: usize,
}

/// Total texel bytes a texture occupies, summed over its mip levels.
///
/// `align` chooses between the padded stride this file establishes and the
/// tightly-packed one the decoder used to assume, so the two readings are
/// measured by the same code against the same numbers.
fn texel_bytes(t: &TextureHeader, align: bool) -> usize {
    (0..u32::from(t.mip_count))
        .map(|level| {
            let width = (t.width >> level).max(1);
            let height = usize::from((t.height >> level).max(1));
            let row = if align {
                texture_row_stride(width, t.bits_per_pixel)
            } else {
                texture_row_bytes(width, t.bits_per_pixel)
            };
            row * height
        })
        .sum()
}

/// Every `Texture` node header in every version-6 `.vex` of one archive.
fn texture_headers(disc: &mut DiscImage) -> Vec<TextureHeader> {
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == PSP_DATA)
        .unwrap_or_else(|| panic!("{PSP_DATA} present"))
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
    for (index, entry) in dir.entries.iter().enumerate() {
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
        if !vex::has_magic(&bytes) || vex::version(&bytes) != Ok(6) {
            continue;
        }
        let Ok(nodes) = vex::nodes(&bytes) else {
            continue;
        };
        for node in nodes.iter().filter(|n| n.class_id == vex::CLASS_TEXTURE) {
            let p = &bytes[node.payload()];
            if p.len() < TEXEL_SIZE_AT + 4 {
                continue;
            }
            let u16_at = |at: usize| u16::from_le_bytes([p[at], p[at + 1]]);
            let texel_size = u32::from_le_bytes([
                p[TEXEL_SIZE_AT],
                p[TEXEL_SIZE_AT + 1],
                p[TEXEL_SIZE_AT + 2],
                p[TEXEL_SIZE_AT + 3],
            ]) as usize;
            let bits_per_pixel = p[4];
            if !matches!(bits_per_pixel, 4 | 8) {
                continue;
            }
            out.push(TextureHeader {
                label: format!(
                    "entry {index} {}",
                    vex::texture_asset_path(p).unwrap_or_else(|| "?".to_string())
                ),
                width: u16_at(0),
                height: u16_at(2),
                bits_per_pixel,
                mip_count: p[5],
                texel_size,
            });
        }
    }
    out
}

/// The claim: the padded stride reproduces every declared texel-block size, and
/// the unpacked one does not.
///
/// Both directions matter. That the padded formula closes is what makes it the
/// right reading; that the unpadded formula *fails* on most textures is what
/// says the difference is real rather than a distinction the data never
/// exercises.
#[test]
#[ignore = "needs a disc image under data/images"]
fn every_texture_block_closes_on_a_sixteen_byte_row_stride() {
    let Some(path) = image() else { return };
    let mut disc = DiscImage::open(&path).expect("opening the disc image");
    let textures = texture_headers(&mut disc);

    assert!(
        textures.len() >= MIN_TEXTURES,
        "only {} texture(s) reached; the archive walk is broken",
        textures.len()
    );

    let mut unpadded_agrees = 0;
    for t in &textures {
        assert_eq!(
            texel_bytes(t, true),
            t.texel_size,
            "{}: {}x{} {}bpp {} mip(s) declares {} texel byte(s)",
            t.label,
            t.width,
            t.height,
            t.bits_per_pixel,
            t.mip_count,
            t.texel_size
        );
        if texel_bytes(t, false) == t.texel_size {
            unpadded_agrees += 1;
        }
    }

    println!(
        "{} textures: the padded stride closes on all of them; the unpadded one on {}",
        textures.len(),
        unpadded_agrees
    );
    assert!(
        unpadded_agrees < textures.len(),
        "the unpadded stride closes on every texture too, so this file measures nothing"
    );
}

/// Which textures the padding actually changes at the base level - the ones the
/// old decoder built out of the wrong bytes.
///
/// A 4-bit texture needs 32 pixels to fill a 16-byte row and an 8-bit one needs
/// 16, so this is exactly the set narrower than that. Asserted to be non-empty
/// rather than pinned to a count: the point is that the fault has real content
/// behind it, and a pinned number would break on the next disc region for no
/// reason worth a maintainer's time.
#[test]
#[ignore = "needs a disc image under data/images"]
fn the_narrow_textures_the_stride_changes_are_named() {
    let Some(path) = image() else { return };
    let mut disc = DiscImage::open(&path).expect("opening the disc image");
    let textures = texture_headers(&mut disc);

    let affected: Vec<&TextureHeader> = textures
        .iter()
        .filter(|t| {
            texture_row_stride(t.width, t.bits_per_pixel)
                != texture_row_bytes(t.width, t.bits_per_pixel)
        })
        .collect();

    for t in &affected {
        println!(
            "  {} {}x{} {}bpp: {} picture byte(s) per {}-byte row",
            t.label,
            t.width,
            t.height,
            t.bits_per_pixel,
            texture_row_bytes(t.width, t.bits_per_pixel),
            texture_row_stride(t.width, t.bits_per_pixel)
        );
        assert!(
            usize::from(t.width) * usize::from(t.bits_per_pixel) < 8 * 16,
            "{}: only a row narrower than 16 bytes can be padded",
            t.label
        );
    }
    println!(
        "{} of {} textures have a padded base level",
        affected.len(),
        textures.len()
    );
    assert!(
        !affected.is_empty(),
        "no texture on the disc is narrow enough for the stride to matter"
    );
}
