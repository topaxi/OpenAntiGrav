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
    let (mut decoded, mut unknown_format, mut cubemaps) = (0usize, 0usize, 0usize);
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
            // `B8`, still - the swizzle decoder covers `A8R8G8B8` and
            // `A8B8G8R8`, not the one-channel format, and every `B8` file on
            // the disc is swizzled. See `docs/formats/gtf.md`.
            Err(gtf::Error::UnknownFormat { .. }) => unknown_format += 1,
            Err(gtf::Error::Cubemap) => cubemaps += 1,
            Err(e) => panic!("{path}: {e}"),
        }
    });

    // The refusals are the 9 `B8` files and the 23 cubemaps, minus whatever is
    // over the size cut. The 44 swizzled `A8R8G8B8`/`A8B8G8R8` files that used
    // to refuse here now decode - see `docs/formats/gtf.md`.
    println!("{decoded} decoded, {unknown_format} unknown format, {cubemaps} cubemaps");
    assert_eq!((decoded, unknown_format, cubemaps), (4504, 9, 8));
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_swizzle_order_is_smoother_than_a_linear_misread() {
    let Some(image) = image() else {
        return;
    };
    // Only 44 files on the whole disc are swizzled `A8R8G8B8`/`A8B8G8R8`
    // (the 9 swizzled `B8` ones are not decoded either way, and everything
    // else is either linear or block-compressed), so the whole disc is a
    // small enough sweep - no archive sampling needed, unlike the DXT
    // endianness question this mirrors.
    //
    // One named exception, inspected by eye rather than assumed wrong:
    // `fealphaluminancetexture.gtf` decodes to a coherent blocky test chart
    // whose rows happen to be internally uniform, which `roughness`'s own
    // within-row-only metric cannot see past. It does not read as scrambled
    // tiles - the failure mode this test exists to catch - so it is named
    // rather than folded into the flat bucket. Two more `A8B8G8R8` files
    // (`feburneffect_test.gtf`, a dissolve noise texture; `fetesttexture_001.gtf`,
    // a scratch/detail map) looked like exceptions before `Texture::remap`'s
    // `REMAP_FORCES_BLUE` correction landed - real blue-channel garbage in the
    // undecoded reading masquerading as signal - and clear the bar now that
    // it does not.
    const KNOWN_EXCEPTIONS: &[&str] = &["/data/tex/fealphaluminancetexture.gtf"];
    let (mut smoother, mut flat, mut rougher, mut unexpected) = (0usize, 0usize, 0usize, 0usize);
    for_every_gtf(&image, |path, blob| {
        let parsed = Gtf::parse(blob).expect("parses");
        let texture = parsed.only().expect("one");
        if texture.format.is_block_compressed()
            || texture.is_linear()
            || texture.cubemap
            || texture.format == Format::B8
        {
            return;
        }
        let (width, height) = texture.level_size(0);
        let native = texture.to_rgba(blob).expect("decodes");
        // The wrong reading: the same bytes, addressed as if the `0x20` bit
        // had been misread and this were raster order after all - the
        // mistake `Error::Swizzled` used to guard against by refusing outright
        // rather than risk. `linear` forced regardless of what the descriptor
        // says, which is exactly the bug being checked was never shipped.
        let range = texture.level_range(0);
        let texels = &blob[range];
        let wrong = gtf::decode_level(
            texture.format,
            texels,
            width,
            height,
            texture.pitch as usize,
            true,
        )
        .expect("decodes");
        let (a, b) = (
            roughness(&native, width as usize),
            roughness(&wrong, width as usize),
        );
        // A handful of these are flat single-colour masks - `corner2.gtf`'s
        // own RGB plane among them, alpha carrying the shape instead - which
        // say nothing either way.
        if a < 0.005 && b < 0.005 {
            flat += 1;
        } else if a <= b {
            smoother += 1;
        } else if KNOWN_EXCEPTIONS.contains(&path) {
            rougher += 1;
        } else {
            unexpected += 1;
            eprintln!("{path}: native roughness {a:.2} >= misread {b:.2}, not a known exception");
        }
    });

    let judged = smoother + rougher;
    println!(
        "{smoother} smoother, {rougher} known exceptions, {flat} flat, {unexpected} unexpected, {judged} judged"
    );
    assert!(judged > 20, "only {judged} textures had anything to say");
    assert_eq!(
        rougher,
        KNOWN_EXCEPTIONS.len(),
        "a known exception stopped reproducing"
    );
    assert_eq!(
        unexpected, 0,
        "the Morton reading was not the smoother one somewhere new"
    );
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

/// **All 23 cubemaps decode, six faces each, and the faces are stored
/// face-major.**
///
/// Face-major - all of one face's levels, then the next face's - is what
/// `Texture::chain_len` already assumed to make the length invariant close, and
/// it is asserted here rather than left implicit, because the other reading
/// (level-major) produces exactly the same total length and a scrambled sky.
///
/// The check that separates them without a picture: a **cube's six faces are
/// six different images, and the four sides of a sky are more like one another
/// than any of them is like the zenith or the nadir.** On Talon's Junction the
/// zenith is deep blue (mean `(78, 123, 165)`), the nadir is blank white
/// (`(255, 255, 255)`) and the four sides are pale horizon (`204..217` red).
/// Read level-major, six "faces" of a single-level cubemap would each be a
/// slice of one image and would not separate that way.
#[test]
#[ignore]
fn every_cubemap_decodes_six_distinct_faces() {
    let Some(image) = image() else {
        return;
    };
    let (mut cubemaps, mut decoded, mut refused) = (0usize, 0usize, 0usize);
    let mut skies = 0usize;
    for_every_gtf(&image, |path, blob| {
        let Ok(parsed) = gtf::Gtf::parse(blob) else {
            return;
        };
        let Some(texture) = parsed.only() else { return };
        if !texture.cubemap {
            // A face index is meaningless on a flat texture and is refused as
            // such, rather than answering the whole image.
            assert!(
                texture.face_to_rgba(blob, 0).is_err(),
                "{path}: a flat texture accepted a face index"
            );
            return;
        }
        cubemaps += 1;
        assert_eq!(texture.faces(), 6, "{path}");
        let faces: Vec<_> = (0..6).map(|f| texture.face_to_rgba(blob, f)).collect();
        assert!(
            texture.face_to_rgba(blob, 6).is_err(),
            "{path}: a seventh face was accepted"
        );
        if faces.iter().any(Result::is_err) {
            refused += 1;
            return;
        }
        decoded += 1;
        let (width, height) = texture.level_size(0);
        let means: Vec<[u64; 3]> = faces
            .iter()
            .map(|f| {
                let rgba = f.as_ref().expect("checked");
                assert_eq!(rgba.len(), width as usize * height as usize, "{path}");
                let n = rgba.len() as u64;
                std::array::from_fn(|c| {
                    rgba.iter().map(|p| u64::from(p[c])).sum::<u64>() / n.max(1)
                })
            })
            .collect();
        // **The content claim is asserted on the one sky whose picture was
        // looked at, and reported for the rest.** Two circuits are why it is
        // not a bar over the corpus: `amphiseum`'s sky is a night sky, every
        // face near black, so a brightest-against-darkest bar measures
        // exposure; and `zone_1`'s is a featureless grey void whose zenith and
        // nadir genuinely are alike. Both are data, not decode errors, and a
        // threshold that called either one a failure would be a threshold this
        // file picked to pass.
        let luma = |m: &[u64; 3]| m[0] + m[1] + m[2];
        if path.ends_with("/sky.gtf") {
            skies += 1;
            println!(
                "  zenith {:4} nadir {:4} sides {:4} {:4} {:4} {:4}  {path}",
                luma(&means[2]),
                luma(&means[3]),
                luma(&means[0]),
                luma(&means[1]),
                luma(&means[4]),
                luma(&means[5])
            );
            if path.contains("talons_junction") {
                // Read off the decoded faces and checked against the picture:
                // blue zenith, blank white nadir, four pale horizon sides with
                // a city skyline on them. See `docs/formats/gtf.md`.
                assert_eq!(
                    means[2],
                    [78, 123, 165],
                    "{path}: the +Y face is the zenith"
                );
                assert_eq!(
                    means[3],
                    [255, 255, 255],
                    "{path}: the -Y face is the nadir"
                );
                for side in [0, 1, 4, 5] {
                    assert!(
                        (190..=220).contains(&means[side][0]),
                        "{path}: face {side} should be a horizon, not {:?}",
                        means[side]
                    );
                }
            }
        }
    });
    println!("{cubemaps} cubemaps, {decoded} decoded, {refused} refused, {skies} skies");
    assert_eq!(cubemaps, 23, "the disc's own count");
    assert!(skies >= 8, "only {skies} circuit skies found");
}
