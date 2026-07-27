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
//! A PS2 texture blob is a GS upload packet, so almost every field in it is
//! declared twice and the two declarations have to agree:
//!
//! - The **packed log2 byte** at `+0x00` and the dimension words at `+0x04` say
//!   the same thing in two encodings.
//! - Both **`GIFtag`s** declare their payload in quadwords, and those counts have
//!   to be exactly the texel and palette byte counts the dimensions imply.
//! - The **total size closes**: header, setup, texels, setup, palette, with each
//!   transfer block padded to 256 bytes, must be the blob's actual length.
//! - **`TRXREG`** has to name one of three transfer rectangles, each a fixed
//!   function of the dimensions. It is what identifies the pixel layout, and it
//!   is also what caught the dimension pair being stored height-first: a
//!   width-first reading makes `TRXREG` nonsense on every non-square texture.
//!
//! None of those come out even on a blob that is not a texture, or on a texture
//! read with the fields in the wrong order.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oag_disc::DiscImage;
use oag_formats::ps2_texture::{self, Layout};
use oag_formats::wad::{self, Compression, Directory};

/// The PS2 archives that hold textures.
const PS2_ARCHIVES: [&str; 2] = ["54748/WADS2.WAD", "54748/WADSP.WAD"];

/// Fewer textures than this means the walk stopped finding them.
const MIN_TEXTURES: usize = 5000;

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
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
        // Every index has to name a real palette entry. It always does for a
        // 256-entry palette, so this only bites if the palette is short, which
        // is exactly the failure a mis-sized CLUT block would produce.
        assert!(
            texture
                .indices
                .iter()
                .all(|&i| usize::from(i) < texture.palette.len()),
            "{archive_path} entry {index}: index out of palette"
        );
        // Alpha is on the GS's 0-128 scale; anything above that would mean the
        // palette block starts in the wrong place.
        assert!(
            texture.palette.iter().all(|c| c[3] <= 128),
            "{archive_path} entry {index}: palette alpha above the GS maximum of 128"
        );

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
    // Both transfer shapes are in use; if one vanished the classifier would be
    // silently collapsing them.
    assert!(survey.layouts.contains_key("psmt8"));
    assert!(survey.layouts.contains_key("linear"));
    // Every texture is power-of-two in both dimensions, which the packed log2
    // byte already forces, and no dimension exceeds the GS maximum.
    for &(width, height, bits) in survey.shapes.keys() {
        assert!(width.is_power_of_two() && height.is_power_of_two());
        assert!(width <= 1024 && height <= 1024, "{width}x{height}");
        assert!(matches!(bits, 4 | 8));
    }
}
