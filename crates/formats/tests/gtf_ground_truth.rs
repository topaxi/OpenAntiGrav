//! Parses **every `.gtf` on the Wipeout HD / Fury disc** and checks what came
//! out.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`. The image must be
//! layer-1 decrypted first - `docs/formats/ps3-disc.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-formats --run-ignored all \
//!     -E 'binary(gtf_ground_truth)'
//! ```
//!
//! # Why the sweep is the test
//!
//! [`oag_formats::gtf::Gtf::parse`] refuses a blob whose declared texel length
//! is not what its own descriptor implies, and that length is a function of the
//! format, both dimensions, the mip count, the cubemap flag and the pitch. So
//! "all 7,333 parse" is not a claim that nothing crashed: it is five independent
//! fields agreeing with a sixth, 7,333 times, over a corpus that runs 3x1 to
//! 2048x2048 with ten distinct format bytes and 131 non-power-of-two textures.
//!
//! The decode is checked a different way, because there is no length invariant
//! for pixels. [`the_dxt_endpoints_are_little_endian_inside_a_big_endian_file`]
//! decodes both readings of a large sample and compares how smooth each comes
//! out - the test that settles the question, and the same one
//! `oag_formats::ps2_texture` uses for its swizzle permutation.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oag_formats::gtf::{self, Format, Gtf};

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
    "PS3_GAME/USRDIR/DATA04.PSARC",
    "PS3_GAME/USRDIR/DATA05.PSARC",
    "PS3_GAME/USRDIR/DATA06.PSARC",
];

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

/// Calls `visit` with every `.gtf` on the disc.
fn for_every_gtf(image: &Path, mut visit: impl FnMut(&str, &[u8])) {
    for archive in ARCHIVES {
        let spec = format!("{}:{archive}", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("opening the archive");
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|path| path.ends_with(".gtf"))
            .cloned()
            .collect();
        for path in paths {
            let blob = open
                .read_path(&path)
                .unwrap_or_else(|e| panic!("reading {path}: {e}"));
            visit(&path, &blob);
        }
    }
}

/// The census, by format byte. Counted 2026-08-17.
///
/// The four with the `0x20` bit set on a compressed format are the ones that
/// prove a pitch can be declared for block-compressed texels; see
/// `docs/formats/gtf.md`.
const FORMATS: &[(u8, usize)] = &[
    (0x81, 9),
    (0x85, 37),
    (0x86, 2485),
    (0x87, 527),
    (0x88, 4137),
    (0x9e, 7),
    (0xa5, 126),
    (0xa6, 1),
    (0xa7, 1),
    (0xa8, 3),
];

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_gtf_on_the_disc_parses_and_the_census_is_what_it_was() {
    let Some(image) = image() else {
        return;
    };
    let mut census: BTreeMap<u8, usize> = BTreeMap::new();
    let mut versions: BTreeMap<u32, usize> = BTreeMap::new();
    let (mut files, mut cubemaps, mut non_power_of_two) = (0usize, 0usize, 0usize);

    for_every_gtf(&image, |path, blob| {
        let parsed = Gtf::parse(blob).unwrap_or_else(|e| panic!("{path}: {e}"));
        files += 1;
        *versions.entry(parsed.version).or_default() += 1;
        assert_eq!(parsed.textures.len(), 1, "{path}: not one texture");
        let texture = parsed.only().expect("one");
        *census.entry(texture.format_byte).or_default() += 1;
        cubemaps += usize::from(texture.cubemap);
        non_power_of_two += usize::from(
            !u32::from(texture.width).is_power_of_two()
                || !u32::from(texture.height).is_power_of_two(),
        );
        // Fields that are constant across the whole disc. Each is a place a
        // reading could differ on another title and quietly not be noticed.
        assert_eq!(texture.dimension, 2, "{path}");
        assert_eq!(texture.depth, 1, "{path}");
        assert_eq!(texture.location, 0, "{path}");
    });

    assert_eq!(files, 7333);
    assert_eq!(cubemaps, 23);
    assert_eq!(non_power_of_two, 131);
    assert_eq!(
        census.into_iter().collect::<Vec<_>>(),
        FORMATS.to_vec(),
        "the format census moved"
    );
    assert_eq!(
        versions,
        BTreeMap::from([(0x0105_0000, 6980), (0x0201_0100, 353)])
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_texture_the_reader_accepts_decodes_to_its_own_dimensions() {
    let Some(image) = image() else {
        return;
    };
    let (mut decoded, mut swizzled, mut cubemaps) = (0usize, 0usize, 0usize);
    for_every_gtf(&image, |path, blob| {
        let parsed = Gtf::parse(blob).expect("parses");
        let texture = parsed.only().expect("one");
        // The whole corpus is 2.4 GiB; decoding the base level of everything
        // over 256x256 is where the interesting formats are and keeps this a
        // couple of minutes rather than twenty.
        if u32::from(texture.width) * u32::from(texture.height) > 256 * 256 {
            return;
        }
        match texture.to_rgba(blob) {
            Ok(rgba) => {
                let (w, h) = texture.level_size(0);
                assert_eq!(rgba.len(), (w * h) as usize, "{path}");
                decoded += 1;
            }
            Err(gtf::Error::Swizzled { .. }) => swizzled += 1,
            Err(gtf::Error::Cubemap) => cubemaps += 1,
            Err(e) => panic!("{path}: {e}"),
        }
    });

    // The refusals are the 53 Morton-order textures and the 23 cubemaps, minus
    // whatever is over the size cut.
    println!("{decoded} decoded, {swizzled} swizzled, {cubemaps} cubemaps");
    assert_eq!((decoded, swizzled, cubemaps), (4479, 34, 8));
}

/// Mean absolute difference between horizontally adjacent RGB texels.
///
/// Real art is smooth across a block boundary; a byte-swapped `R5G6B5` endpoint
/// moves five bits of red into the low bits of blue, and is not.
fn roughness(rgba: &[[u8; 4]], width: usize) -> f64 {
    let mut total = 0u64;
    let mut count = 0u64;
    for row in rgba.chunks_exact(width) {
        for pair in row.windows(2) {
            for (left, right) in pair[0].iter().zip(&pair[1]).take(3) {
                total += u64::from(left.abs_diff(*right));
                count += 1;
            }
        }
    }
    total as f64 / count.max(1) as f64
}

/// Swaps each colour block's two `R5G6B5` endpoints, which is the other reading.
fn swap_endpoints(blob: &[u8], texture: &gtf::Texture) -> Vec<u8> {
    let mut out = blob.to_vec();
    let unit = texture.format.unit_len();
    let colour_at = if unit == 16 { 8 } else { 0 };
    let range = texture.level_range(0);
    let mut at = range.start;
    while at + unit <= range.end {
        out.swap(at + colour_at, at + colour_at + 1);
        out.swap(at + colour_at + 2, at + colour_at + 3);
        at += unit;
    }
    out
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_dxt_endpoints_are_little_endian_inside_a_big_endian_file() {
    let Some(image) = image() else {
        return;
    };
    // `DATA00` alone is a large enough sample to settle this and keeps the test
    // to about a minute; the full-disc figure is on the docs page.
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", image.display());
    let mut open = oag_assets::psarc::Archive::open(&spec).expect("opening the archive");
    let paths: Vec<String> = open
        .paths()
        .iter()
        .filter(|path| path.ends_with(".gtf"))
        .cloned()
        .collect();

    let (mut little, mut swapped_wins, mut flat) = (0usize, 0usize, 0usize);
    for path in paths {
        let blob = open.read_path(&path).expect("entry");
        let parsed = Gtf::parse(&blob).expect("parses");
        let texture = parsed.only().expect("one");
        let (width, height) = texture.level_size(0);
        if !texture.format.is_block_compressed()
            || texture.cubemap
            || width < 16
            || height < 16
            || width * height > 512 * 512
        {
            continue;
        }
        let native = texture.to_rgba(&blob).expect("decodes");
        let other_blob = swap_endpoints(&blob, texture);
        let other = texture.to_rgba(&other_blob).expect("decodes");
        let (a, b) = (
            roughness(&native, width as usize),
            roughness(&other, width as usize),
        );
        // A mask whose RGB is uniformly white - and there are many, the retro
        // HUD skins among them - is flat either way and says nothing.
        if a < 0.005 && b < 0.005 {
            flat += 1;
        } else if a <= b {
            little += 1;
        } else {
            swapped_wins += 1;
        }
    }

    let judged = little + swapped_wins;
    assert!(judged > 300, "only {judged} textures had anything to say");
    assert!(flat > 0, "expected some all-white masks");
    let share = little as f64 / judged as f64;
    assert!(
        share > 0.9,
        "little-endian endpoints smoother on only {little} of {judged} ({share:.3})"
    );
}

/// The HUD's own twelve textures, which is what this format was read for.
///
/// Names as `oag_hd::hud::TEXTURES` spells them cannot be used here -
/// `oag-formats` does not depend on `oag-hd` - so these are the resolved entry
/// paths, and `crates/game/tests/hd_hud_ground_truth.rs` is what ties the two
/// spellings together.
const HUD_TEXTURES: &[(&str, &str, u16, u16, Format)] = &[
    (
        "DATA02",
        "/data/hud/textures/hud_components.gtf",
        1024,
        1024,
        Format::Dxt45,
    ),
    (
        "DATA02",
        "/data/hud/textures/hud_components_01.gtf",
        1024,
        512,
        Format::Dxt23,
    ),
    (
        "DATA02",
        "/data/hud/textures/hud_components_02.gtf",
        128,
        128,
        Format::Dxt23,
    ),
    (
        "DATA02",
        "/data/hud/textures/zonedamage.gtf",
        1024,
        256,
        Format::Dxt23,
    ),
    (
        "DATA00",
        "/data/hud/textures/detonator_hud2.gtf",
        1024,
        1024,
        Format::Dxt45,
    ),
    (
        "DATA00",
        "/data/hud/textures/fury_hud.gtf",
        1024,
        1024,
        Format::Dxt45,
    ),
    (
        "DATA02",
        "/data/hud/textures/hdhud.gtf",
        1024,
        1024,
        Format::Dxt45,
    ),
    (
        "DATA02",
        "/data/hud/textures/missile_reticule.gtf",
        256,
        256,
        Format::Dxt23,
    ),
    (
        "DATA00",
        "/data/hud/textures/nitro_hud.gtf",
        1024,
        1024,
        Format::Dxt45,
    ),
    (
        "DATA03",
        "/data/xml/wo3_hud/texture/wo3_hud.gtf",
        512,
        512,
        Format::Dxt23,
    ),
    (
        "DATA00",
        "/data/xml/2097_hud/texture/wo2097_hud.gtf",
        1024,
        1024,
        Format::Dxt45,
    ),
    (
        "DATA02",
        "/data/fe/images/voicecom.gtf",
        64,
        64,
        Format::Dxt45,
    ),
];

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_huds_own_textures_decode() {
    let Some(image) = image() else {
        return;
    };
    for &(archive, path, width, height, format) in HUD_TEXTURES {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("opening the archive");
        let blob = open
            .read_path(path)
            .unwrap_or_else(|e| panic!("reading {path}: {e}"));
        let parsed = Gtf::parse(&blob).unwrap_or_else(|e| panic!("{path}: {e}"));
        let texture = parsed.only().expect("one texture");
        assert_eq!(
            (texture.width, texture.height, texture.format),
            (width, height, format),
            "{path}"
        );
        let rgba = texture
            .to_rgba(&blob)
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(rgba.len(), width as usize * height as usize, "{path}");
        // An atlas of HUD sprites is mostly empty: the alpha channel has to
        // carry real structure rather than being uniformly opaque, which is what
        // a wrong alpha reading would produce.
        let clear = rgba.iter().filter(|t| t[3] == 0).count();
        assert!(
            clear > rgba.len() / 20,
            "{path}: only {clear} of {} texels are transparent",
            rgba.len()
        );
    }
}
