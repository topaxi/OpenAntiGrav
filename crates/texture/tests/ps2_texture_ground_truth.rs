//! Validates the PS2 texture decoder against every blob on the PS2 disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The tests skip with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves nothing.
//!
//! # What this is for
//!
//! A PS2 texture blob is a GS upload packet, so almost every field is declared
//! twice and the declarations must agree:
//!
//! - The **packed log2 byte** at `+0x00` and the dimension words at `+0x04` say
//!   the same thing in two encodings.
//! - Both **`GIFtag`s** declare their payload in quadwords, and those counts have
//!   to be exactly the texel and palette byte counts the dimensions imply.
//! - The **total size closes**: header, setup, texels, setup, palette, with each
//!   transfer block padded to 256 bytes, must be the blob's actual length.
//! - **`TRXREG`** must name one of three transfer rectangles, each a fixed
//!   function of the dimensions. It identifies the pixel layout and caught the
//!   dimension pair being height-first: a width-first reading makes `TRXREG`
//!   nonsense on every non-square texture.
//!
//! None of those hold on a non-texture or on fields read in the wrong order.
//!
//! # None of that evidences the *permutation*
//!
//! Framing arithmetic says where the texels are, not that they were unpicked
//! correctly, and plenty of wrong permutations are bijections. So the test scores
//! every decoded texture's **total variation** against the same texels under the
//! opposite layout: a wrong permutation scatters pixels that belong together.
//! Decoding is smoother on 4,856 of 4,956 swizzled textures and 185 of 185 linear
//! ones; the hundred exceptions are flat or noise-like blobs.

use std::collections::BTreeMap;
use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_texture::ps2_texture::{self, Layout};

/// The PS2 archives that hold textures.
const PS2_ARCHIVES: [&str; 2] = ["54748/WADS2.WAD", "54748/WADSP.WAD"];

/// Fewer textures than this means the walk stopped finding them.
const MIN_TEXTURES: usize = 5000;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

#[derive(Default)]
struct Survey {
    blobs: usize,
    textures: usize,
    decoded: usize,
    pixels: usize,
    layouts: BTreeMap<&'static str, usize>,
    shapes: BTreeMap<(u16, u16, u8), usize>,
    flags: BTreeMap<u16, usize>,
    /// Blobs whose header parses but whose layout is not implemented.
    refused: usize,
    /// Decoded textures big enough to test, per layout.
    testable: BTreeMap<&'static str, usize>,
    /// Of those, how many are smoother decoded than under the other reading.
    smoother: BTreeMap<&'static str, usize>,
}

fn survey(disc: &mut DiscImage, archive_path: &str, into: &mut Survey) {
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == archive_path)
        .unwrap_or_else(|| panic!("{archive_path} present"))
        .clone();

    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, Directory::directory_len(count))
        .expect("directory");
    let dir = Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    for (index, entry) in dir.entries.iter().enumerate() {
        if entry.size == 0 {
            continue;
        }
        let raw = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let blob = match entry.compression {
            Compression::None => raw,
            Compression::Lzss => {
                oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
            }
            Compression::Zlib => panic!("{archive_path} entry {index}: unexpected zlib entry"),
        };
        into.blobs += 1;

        let Ok(head) = ps2_texture::header(&blob) else {
            continue;
        };
        into.textures += 1;
        *into.layouts.entry(layout_name(head.layout)).or_default() += 1;
        *into
            .shapes
            .entry((head.width, head.height, head.bits_per_pixel))
            .or_default() += 1;
        *into.flags.entry(head.flags).or_default() += 1;

        assert_eq!(
            head.total_bytes,
            blob.len(),
            "{archive_path} entry {index}: header implies {} bytes, blob is {}",
            head.total_bytes,
            blob.len()
        );

        let texture = match ps2_texture::parse(&blob) {
            Ok(texture) => texture,
            Err(ps2_texture::Error::UnsupportedLayout(Layout::Psmt4)) => {
                into.refused += 1;
                continue;
            }
            Err(error) => panic!("{archive_path} entry {index}: {error}"),
        };

        let pixels = usize::from(texture.width) * usize::from(texture.height);
        assert_eq!(
            texture.indices.len(),
            pixels,
            "{archive_path} entry {index}: {} indices for {pixels} pixels",
            texture.indices.len()
        );
        assert_eq!(
            texture.palette.len(),
            1usize << texture.bits_per_pixel,
            "{archive_path} entry {index}: wrong palette length"
        );
        // Every index must name a real palette entry; this bites only if the
        // palette is short, as a mis-sized CLUT block would make it.
        assert!(
            texture
                .indices
                .iter()
                .all(|&i| usize::from(i) < texture.palette.len()),
            "{archive_path} entry {index}: index out of palette"
        );
        // Alpha is on the GS's 0-128 scale; above that, the palette block starts
        // in the wrong place.
        assert!(
            texture.palette.iter().all(|c| c[3] <= 128),
            "{archive_path} entry {index}: palette alpha above the GS maximum of 128"
        );

        // The size arithmetic does not say the permutation is right (many wrong
        // ones are bijections). Scoring the texels under the *opposite* reading
        // does: a wrong permutation raises local discontinuity.
        if texture.width >= 32 && texture.height >= 16 {
            let name = layout_name(head.layout);
            let (w, h) = (usize::from(texture.width), usize::from(texture.height));
            *into.testable.entry(name).or_default() += 1;

            let texels =
                &blob[ps2_texture::TEXEL_OFFSET..ps2_texture::TEXEL_OFFSET + head.texel_bytes];
            let control: Vec<u8> = match head.layout {
                Layout::Psmt8 => texels.to_vec(),
                Layout::Linear => (0..w * h)
                    .map(|i| texels[ps2_texture::psmt8_offset(i % w, i / w, w)])
                    .collect(),
                // The nibbles straight off the blob: what a reader that missed the
                // swizzle would draw.
                Layout::Psmt4 => (0..w * h)
                    .map(|i| (texels[i / 2] >> ((i & 1) * 4)) & 0x0f)
                    .collect(),
            };
            if texture.roughness() < ps2_texture::roughness_of(&control, w, h) {
                *into.smoother.entry(name).or_default() += 1;
            }
        }

        into.decoded += 1;
        into.pixels += pixels;
    }
}

fn layout_name(layout: Layout) -> &'static str {
    match layout {
        Layout::Linear => "linear",
        Layout::Psmt8 => "psmt8",
        Layout::Psmt4 => "psmt4",
    }
}

#[test]
#[ignore = "needs a PS2 disc image under data/images"]
fn every_ps2_texture_blob_decodes_and_its_declared_sizes_close() {
    let Some(path) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");

    let mut survey = Survey::default();
    for archive in PS2_ARCHIVES {
        self::survey(&mut disc, archive, &mut survey);
    }

    println!("blobs        {}", survey.blobs);
    println!("textures     {}", survey.textures);
    println!("decoded      {}", survey.decoded);
    println!("refused      {}", survey.refused);
    println!("pixels       {}", survey.pixels);
    println!("layouts      {:?}", survey.layouts);
    println!("flags        {:?}", survey.flags);
    println!("shapes       {}", survey.shapes.len());
    println!("testable     {:?}", survey.testable);
    println!("smoother     {:?}", survey.smoother);

    assert!(
        survey.textures >= MIN_TEXTURES,
        "only {} textures found, expected at least {MIN_TEXTURES}",
        survey.textures
    );
    assert_eq!(
        survey.decoded + survey.refused,
        survey.textures,
        "every recognised blob must either decode or be explicitly refused"
    );
    assert_eq!(
        survey.refused, 0,
        "nothing on this disc is refused any more: the five PSMT4 blobs are the \
         font atlases and they decode"
    );
    // All three transfer shapes are in use; if one vanished the classifier would
    // be collapsing them.
    assert!(survey.layouts.contains_key("psmt8"));
    assert!(survey.layouts.contains_key("linear"));
    assert!(survey.layouts.contains_key("psmt4"));
    // The permutation check: the other reading must be measurably rougher on
    // essentially every texture.
    for layout in ["psmt8", "linear", "psmt4"] {
        let testable = survey.testable.get(layout).copied().unwrap_or(0);
        let smoother = survey.smoother.get(layout).copied().unwrap_or(0);
        // Only five PSMT4 blobs exist, so that layout gets its own floor.
        let floor = if layout == "psmt4" { 5 } else { 100 };
        assert!(
            testable >= floor,
            "{layout}: only {testable} testable textures"
        );
        assert!(
            smoother * 100 >= testable * 97,
            "{layout}: only {smoother} of {testable} decode smoother than the opposite reading"
        );
    }

    // Every texture is power-of-two in both dimensions, which the packed log2
    // byte already forces, and no dimension exceeds the GS maximum.
    for &(width, height, bits) in survey.shapes.keys() {
        assert!(width.is_power_of_two() && height.is_power_of_two());
        assert!(width <= 1024 && height <= 1024, "{width}x{height}");
        assert!(matches!(bits, 4 | 8));
    }
}
