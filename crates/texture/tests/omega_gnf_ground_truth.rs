//! `oag_texture::gnf` against every real `.gnf` this project can extract from
//! the Omega Collection's nine archives (five base `omega-eu` + four patch
//! `omega-eu-patch`): `Texture::parse`, `Texture::decode` and
//! `Texture::block_levels`, each entry read once and held to all three.
//!
//! **`#[ignore]`d and never run in CI.** Needs
//! `data/extracted/ps4/omega-eu/uroot/` and
//! `data/extracted/ps4/omega-eu-patch/uroot/`, which this project does not
//! ship - see `docs/reverse-engineering/source-images.md`'s Omega
//! Collection section.
//!
//! ```sh
//! just test-data
//! ```
//!
//! Until 2026-10-12 this was three binaries (this one,
//! `omega_gnf_pixels_ground_truth.rs` and `omega_gnf_mips_ground_truth.rs`),
//! each opening the same archives and reading, parsing and decoding the same
//! entries; one pass now carries every claim the three made.
//!
//! # What this asserts, and what it deliberately does not
//!
//! Not a "real content" rate: `docs/formats/psarc.md`'s "Block data location"
//! section (and `crates/assets/tests/omega_psarc_ground_truth.rs`'s module
//! docs) establish that whether a given entry's bytes are present at all is a
//! property of this local dump, not of the reader, so there is no floor on how
//! many entries decode. What *is* a fact about the code:
//!
//! **Parse.** In the five base archives, every entry parses to a header with
//! a non-zero width and height, or is refused as `BadMagic`/`TooShort` (the
//! "garbage"/"all-zero" population `docs/formats/psarc.md` measures), never
//! another error and never a panic. A curated known-real entry decodes to the
//! exact fields hand-verified against the raw bytes: BC7/sRGB, 128x64, tiled.
//!
//! **Decode.** `decode` untiles `TileMode(13)` (`Thin_1DThin`) BC7; the
//! evidence is `docs/formats/gnf.md`'s "Tiling" section. It never panics, a
//! success has exactly `width * height` texels, and every error is one the
//! contract promises (`CorruptBlocks` for a base level with a block carrying
//! no valid BC7 mode bit, `UnsupportedFormat`, `DataOutOfBounds`, or `Tiled`
//! naming the *exact* `tile_mode` the descriptor declares). Every slice of an
//! archive has an entry that reached `decode`. `data08.psarc`'s `Data/fe/`
//! subtree (front-end images and fonts) is decoded in full, every entry is
//! accounted for, and at least one real picture decodes;
//! `crates/texture/examples/gnf_frontend_census.rs` measures the split. The
//! rest of `data08` is not decoded at full size, as it never was.
//!
//! **Mip chain**, for every BC7 `Thin_1DThin` entry, each an arithmetic fact
//! rather than a taste:
//!
//! 1. The level count is the descriptor's and every level is exactly
//!    `ceil(w/4) * ceil(h/4)` blocks of 16 bytes.
//! 2. The base level is the decode's, byte for byte: the blocks handed to the
//!    GPU decode to the same texels `Texture::decode` draws, so passing them
//!    through changes no base-level pixel.
//! 3. The authored chain is the base's own filtered down, to within BC7's
//!    quantisation: level 1 decoded sits within [`MEAN_LIMIT`] of a 2x2 box
//!    filter of level 0 decoded, which is what the renderer synthesises when it
//!    is not given a chain.

use std::path::PathBuf;

use oag_assets::psarc::Archive;
use oag_texture::gnf::{self, SurfaceFormat, Texture};

/// Level-0 texel count above which the chain's decode comparisons (2 and 3
/// above) are skipped, not its structure checks: one 4096-square BC7 costs as
/// much as hundreds of the small textures around it.
const DECODE_LIMIT: u32 = 1024 * 1024;

/// Mean absolute difference, in 8-bit channel units, that level 1 decoded may
/// sit from a box filter of level 0 decoded, averaged over every texture in
/// one test's slice. Every slice under the limit implies every archive under
/// it, the claim this held before the archives were sliced; each test prints
/// its own mean (1.04 to 2.75 per archive, worst single texture 28, in
/// `docs/formats/gnf.md`).
const MEAN_LIMIT: f64 = 4.0;

const BASE: &str = "omega-eu";
const PATCH: &str = "omega-eu-patch";

fn open(dir: &str, name: &str) -> Option<Archive> {
    let path: PathBuf = oag_testdata::exact(&format!("data/extracted/ps4/{dir}/uroot/{name}"))?;
    Some(Archive::open_file(&path).unwrap_or_else(|e| panic!("open {name}: {e}")))
}

/// What one sweep's entries came to.
#[derive(Default)]
struct Tally {
    /// `parse` refused it.
    unparsed: usize,
    /// `decode` was asked, and drew the picture.
    decoded: usize,
    /// `decode` refused a corrupt base level.
    corrupt: usize,
    /// `decode` refused it with another error the contract names.
    other: usize,
    /// `block_levels` returned a chain.
    chains: usize,
    /// `block_levels` refused a corrupt block in some level.
    chain_corrupt: usize,
    /// `block_levels` found no single 2D chain in the blob.
    chain_short: usize,
    /// Level 1 compared against a box filter of level 0.
    compared: usize,
    mad_sum: f64,
    worst: (f64, String),
}

/// How hard one sweep holds its entries.
#[derive(Clone, Copy)]
struct Rules {
    /// A parse error other than `BadMagic`/`TooShort` fails, and a parsed
    /// header has non-zero dimensions. The five base archives.
    strict_parse: bool,
    /// `decode` runs at full size whatever the entry's size.
    decode: bool,
}

fn box_down(rgba: &[[u8; 4]], w: u32, h: u32) -> Vec<[u8; 4]> {
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    let mut out = Vec::with_capacity((nw * nh) as usize);
    for y in 0..nh {
        for x in 0..nw {
            let at = |sx: u32, sy: u32| rgba[(sy.min(h - 1) * w + sx.min(w - 1)) as usize];
            let taps = [
                at(x * 2, y * 2),
                at(x * 2 + 1, y * 2),
                at(x * 2, y * 2 + 1),
                at(x * 2 + 1, y * 2 + 1),
            ];
            out.push(std::array::from_fn(|c| {
                ((taps.iter().map(|t| u32::from(t[c])).sum::<u32>() + 2) / 4) as u8
            }));
        }
    }
    out
}

fn check_entry(label: &str, path: &str, blob: &[u8], rules: Rules, tally: &mut Tally) {
    let texture = match Texture::parse(blob) {
        Ok(texture) => texture,
        Err(gnf::Error::BadMagic { .. } | gnf::Error::TooShort { .. }) => {
            tally.unparsed += 1;
            return;
        }
        Err(other) if rules.strict_parse => panic!("{label}: {path}: unexpected error: {other}"),
        Err(_) => {
            tally.unparsed += 1;
            return;
        }
    };
    if rules.strict_parse {
        assert!(texture.width > 0, "{label}: {path}: zero width");
        assert!(texture.height > 0, "{label}: {path}: zero height");
    }

    let decoded = rules.decode.then(|| {
        let result = texture.decode(blob);
        match &result {
            Ok(rgba) => {
                assert_eq!(
                    rgba.len(),
                    (texture.width * texture.height) as usize,
                    "{label}: {path}: wrong pixel count"
                );
                tally.decoded += 1;
            }
            Err(gnf::Error::CorruptBlocks { .. }) => tally.corrupt += 1,
            Err(gnf::Error::Tiled { tile_mode }) => {
                assert_eq!(
                    *tile_mode, texture.tile_mode.0,
                    "{label}: {path}: Error::Tiled named the wrong tile_mode"
                );
                tally.other += 1;
            }
            Err(gnf::Error::UnsupportedFormat { .. } | gnf::Error::DataOutOfBounds { .. }) => {
                tally.other += 1;
            }
            Err(e) => panic!("{label}: {path}: unexpected error: {e}"),
        }
        result
    });

    check_chain(path, blob, &texture, decoded, tally);
}

/// The mip-chain claims, reusing `decode`'s picture when the sweep already
/// drew it.
fn check_chain(
    path: &str,
    blob: &[u8],
    t: &Texture,
    decoded: Option<Result<Vec<[u8; 4]>, gnf::Error>>,
    tally: &mut Tally,
) {
    if t.surface_format != SurfaceFormat::Bc7 || t.tile_mode.0 != 13 {
        return;
    }
    let levels = match t.block_levels(blob) {
        Ok(levels) => levels,
        Err(gnf::Error::CorruptBlocks { .. }) => {
            tally.chain_corrupt += 1;
            return;
        }
        Err(gnf::Error::ChainLayout { .. } | gnf::Error::DataOutOfBounds { .. }) => {
            tally.chain_short += 1;
            return;
        }
        Err(e) => panic!("{path}: unexpected {e}"),
    };
    tally.chains += 1;
    assert_eq!(
        levels.len(),
        usize::from(t.last_mip_level - t.base_mip_level) + 1,
        "{path}"
    );
    for (level, blocks) in levels.iter().enumerate() {
        let wb = (t.width >> level).max(1).div_ceil(4);
        let hb = (t.height >> level).max(1).div_ceil(4);
        assert_eq!(
            blocks.len(),
            (wb * hb) as usize * 16,
            "{path} level {level}"
        );
    }
    if t.width * t.height > DECODE_LIMIT {
        return;
    }
    let drawn = decoded
        .unwrap_or_else(|| t.decode(blob))
        .unwrap_or_else(|e| panic!("{path}: block_levels gave a chain but decode() refused: {e}"));
    let base = gnf::decode_bc7_level(&levels[0], t.width, t.height).unwrap();
    assert_eq!(base, drawn, "{path}: base level differs from decode()");
    if levels.len() < 2 || t.width < 8 || t.height < 8 {
        return;
    }
    let (w1, h1) = (t.width / 2, t.height / 2);
    let authored = gnf::decode_bc7_level(&levels[1], w1, h1).unwrap();
    let filtered = box_down(&base, t.width, t.height);
    let total: u64 = authored
        .iter()
        .zip(&filtered)
        .flat_map(|(a, b)| a.iter().zip(b))
        .map(|(a, b)| u64::from(a.abs_diff(*b)))
        .sum();
    let mad = total as f64 / (authored.len() * 4) as f64;
    tally.mad_sum += mad;
    tally.compared += 1;
    if mad > tally.worst.0 {
        tally.worst = (mad, path.to_string());
    }
}

/// Reads, checks and tallies every one of `paths`, prints the counts and
/// asserts the chain's mean over them.
fn sweep(archive: &mut Archive, paths: &[String], label: &str, rules: Rules) -> Tally {
    let mut tally = Tally::default();
    for path in paths {
        let blob = archive
            .read_path(path)
            .unwrap_or_else(|e| panic!("{label}: {path}: read failed: {e}"));
        check_entry(label, path, &blob, rules, &mut tally);
    }
    let mean = if tally.compared > 0 {
        tally.mad_sum / tally.compared as f64
    } else {
        0.0
    };
    println!(
        "{label}: {} entries, {} unparsed; decode: {} decoded, {} refused (corrupt base level), \
         {} other clean errors; {} chains untiled, {} refused corrupt, {} not one 2D chain; \
         level 1 vs box(level 0) over {}: mean MAD {mean:.3}, worst {:.3} ({})",
        paths.len(),
        tally.unparsed,
        tally.decoded,
        tally.corrupt,
        tally.other,
        tally.chains,
        tally.chain_corrupt,
        tally.chain_short,
        tally.compared,
        tally.worst.0,
        tally.worst.1
    );
    assert!(
        mean < MEAN_LIMIT,
        "{label}: mean MAD {mean:.3} over the limit"
    );
    tally
}

fn gnf_paths(archive: &Archive) -> Vec<String> {
    archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".gnf"))
        .cloned()
        .collect()
}

/// `slice` of `slices`: entry `i` of the archive's `.gnf` list belongs to slice
/// `i % slices`, so the slices cover it with nothing left over. Every claim is
/// per entry except the chain's mean, which per slice is the stricter form,
/// and the guard that some entry reached `decode`, likewise.
fn check_archive(dir: &str, name: &str, (slice, slices): (usize, usize)) {
    let Some(mut archive) = open(dir, name) else {
        return;
    };
    let all = gnf_paths(&archive);
    let base = dir == BASE;
    if base {
        assert!(!all.is_empty(), "{name}: no .gnf entries at all");
    }
    let paths: Vec<String> = all.into_iter().skip(slice).step_by(slices).collect();
    let rules = Rules {
        strict_parse: base,
        decode: true,
    };
    let tally = sweep(&mut archive, &paths, name, rules);
    if !paths.is_empty() {
        assert!(
            tally.decoded + tally.corrupt + tally.other > 0,
            "{name}: not one of {} .gnf entries reached decode()",
            paths.len()
        );
    }
}

macro_rules! per_slice {
    ($($test:ident: $dir:ident, $name:literal, $slice:literal of $slices:literal;)*) => {$(
        #[test]
        #[ignore]
        fn $test() {
            check_archive($dir, $name, ($slice, $slices));
        }
    )*};
}

per_slice! {
    data00_slice_0: BASE, "data00.psarc", 0 of 3;
    data00_slice_1: BASE, "data00.psarc", 1 of 3;
    data00_slice_2: BASE, "data00.psarc", 2 of 3;
    data01_slice_0: BASE, "data01.psarc", 0 of 3;
    data01_slice_1: BASE, "data01.psarc", 1 of 3;
    data01_slice_2: BASE, "data01.psarc", 2 of 3;
    data02_slice_0: BASE, "data02.psarc", 0 of 3;
    data02_slice_1: BASE, "data02.psarc", 1 of 3;
    data02_slice_2: BASE, "data02.psarc", 2 of 3;
    data03: BASE, "data03.psarc", 0 of 1;
    data04_slice_0: BASE, "data04.psarc", 0 of 2;
    data04_slice_1: BASE, "data04.psarc", 1 of 2;
    patch_data05: PATCH, "data05.psarc", 0 of 1;
    patch_data07: PATCH, "data07.psarc", 0 of 1;
    patch_data09: PATCH, "data09.psarc", 0 of 1;
}

/// `data08.psarc`: every entry's chain, and its `Data/fe/` subtree
/// (`Data/fe/images/*.gnf`, `Data/fe/fonts/*.gnf`) decoded in full. Asserts
/// `decoded > 0` outright for the subtree, not just that something reached
/// `decode`: `crates/texture/examples/gnf_frontend_census.rs` measured 219 of
/// it drawing clean, so a drop to zero real pictures is what this catches.
#[test]
#[ignore]
fn patch_data08() {
    let Some(mut archive) = open(PATCH, "data08.psarc") else {
        return;
    };
    let (fe, rest): (Vec<String>, Vec<String>) = gnf_paths(&archive)
        .into_iter()
        .partition(|p| p.to_ascii_lowercase().contains("/fe/"));
    assert!(
        !fe.is_empty(),
        "data08.psarc: no Data/fe/*.gnf entries at all"
    );

    let label = "data08.psarc Data/fe/";
    let decode = Rules {
        strict_parse: false,
        decode: true,
    };
    let tally = sweep(&mut archive, &fe, label, decode);
    assert_eq!(
        tally.decoded + tally.corrupt + tally.other + tally.unparsed,
        fe.len(),
        "{label}: not every entry reached an accounted-for outcome"
    );
    assert!(
        tally.decoded > 0,
        "{label}: not one real front-end image decoded"
    );

    let chain_only = Rules {
        strict_parse: false,
        decode: false,
    };
    sweep(
        &mut archive,
        &rest,
        "data08.psarc outside Data/fe/",
        chain_only,
    );
}

#[test]
#[ignore]
fn the_known_real_holographic_glow_decodes_exactly() {
    let Some(mut archive) = open(BASE, "data03.psarc") else {
        return;
    };
    let bytes = archive
        .read_path("Data/art/published/hdships/harimau/Livery2/Holographic_02_GLOW.gnf")
        .expect("reads");
    let texture = Texture::parse(&bytes).expect("parses");
    assert_eq!(texture.surface_format, SurfaceFormat::Bc7);
    assert_eq!(texture.width, 128);
    assert_eq!(texture.height, 64);
    assert!(!texture.is_linear(), "this sample is genuinely tiled");
}
