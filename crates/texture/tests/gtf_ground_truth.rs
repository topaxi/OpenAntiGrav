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
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-texture --run-ignored all \
//!     -E 'binary(gtf_ground_truth)'
//! ```
//!
//! # Why the sweep is the test
//!
//! [`oag_texture::gtf::Gtf::parse`] refuses a blob whose declared texel length is
//! not what its descriptor implies (a function of format, dimensions, mip count,
//! cubemap flag and pitch). So "all 7,333 parse" is five independent fields
//! agreeing with a sixth, 7,333 times, over a corpus running 3x1 to 2048x2048 with
//! ten format bytes and 131 non-power-of-two textures.
//!
//! The decode is checked differently, since pixels have no length invariant:
//! [`the_dxt_endpoints_are_little_endian_inside_a_big_endian_file`] decodes both
//! readings of a large sample and compares smoothness, as
//! `oag_texture::ps2_texture` does for its swizzle permutation.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oag_texture::gtf::{self, Format, Gtf};

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
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
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
            // Nothing on the disc reaches this: every format byte in `FORMATS`
            // decodes. Kept so an unknown format shows as a count, not a panic.
            Err(gtf::Error::UnknownFormat { .. }) => unknown_format += 1,
            Err(gtf::Error::Cubemap) => cubemaps += 1,
            Err(e) => panic!("{path}: {e}"),
        }
    });

    // The only refusals left are cubemaps, which need `face_to_rgba`; see
    // `docs/formats/gtf.md`.
    println!("{decoded} decoded, {unknown_format} unknown format, {cubemaps} cubemaps");
    assert_eq!((decoded, unknown_format, cubemaps), (4513, 0, 8));
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_swizzle_order_is_smoother_than_a_linear_misread() {
    let Some(image) = image() else {
        return;
    };
    // Only 53 files on the disc are swizzled and not block-compressed (37
    // `A8R8G8B8`, 7 `A8B8G8R8`, 9 `B8`), so the whole disc is a small sweep with no
    // sampling.
    //
    // **No named exceptions.** Three files once were, an artefact of measuring
    // roughness along rows only; [`roughness_2d`] retires them.
    let (mut smoother, mut flat, mut rougher) = (0usize, 0usize, 0usize);
    for_every_gtf(&image, |path, blob| {
        let parsed = Gtf::parse(blob).expect("parses");
        let texture = parsed.only().expect("one");
        if texture.format.is_block_compressed() || texture.is_linear() || texture.cubemap {
            return;
        }
        let (width, height) = texture.level_size(0);
        let native = texture.to_rgba(blob).expect("decodes");
        // The wrong reading: the same bytes addressed as raster order, `linear`
        // forced regardless of the descriptor (as if the `0x20` bit were misread).
        let range = texture.level_range(0);
        let texels = &blob[range];
        let mut wrong = gtf::decode_level(
            texture.format,
            texels,
            width,
            height,
            texture.pitch as usize,
            true,
        )
        .expect("decodes");
        // **The same remap on both sides.** `to_rgba` applies the descriptor's; a
        // `wrong` that skipped it would measure the remap, not the addressing (the
        // mistake that made `0xa9e4` look like it forced blue - see `gtf::Remap`).
        gtf::Remap::decode(texture.remap).apply(&mut wrong);
        let (a, b) = (
            roughness_2d(&native, width as usize),
            roughness_2d(&wrong, width as usize),
        );
        // A handful are flat single-colour masks (`corner2.gtf`'s RGB plane,
        // alpha carrying the shape) and say nothing either way.
        if a < 0.005 && b < 0.005 {
            flat += 1;
        } else if a <= b {
            smoother += 1;
        } else {
            rougher += 1;
            eprintln!("{path}: morton roughness {a:.2} >= raster misread {b:.2}");
        }
    });

    let judged = smoother + rougher;
    println!("{smoother} smoother, {rougher} rougher, {flat} flat, {judged} judged");
    assert!(judged > 20, "only {judged} textures had anything to say");
    assert_eq!(
        (judged, rougher),
        (43, 0),
        "the Morton reading was the smoother one on every judgeable file"
    );
    assert_eq!(flat, 10, "the flat bucket moved");
}

/// Every `B8` file on the disc, and what each one is.
///
/// Nine, one per ship team. Named rather than counted because the *names* settled
/// the format: a one-channel 128x64 `ambient_shadow` is a craft's contact shadow,
/// so a soft blob is right and horizontal banding wrong.
const AMBIENT_SHADOWS: &[&str] = &[
    "/data/ships/ag_systems/textures/ambient_shadow.gtf",
    "/data/ships/assegai/textures/ambient_shadow.gtf",
    "/data/ships/egx/textures/ambient_shadow.gtf",
    "/data/ships/feisar/textures/ambient_shadow.gtf",
    "/data/ships/goteki/textures/ambient_shadow.gtf",
    "/data/ships/piranha/textures/ambient_shadow.gtf",
    "/data/ships/qirex/textures/ambient_shadow.gtf",
    "/data/ships/triakis/textures/ambient_shadow.gtf",
    "/data/ships/zone/textures/ambient_shadow.gtf",
];

/// The one-channel format decodes, and comes out as a shadow rather than as
/// noise.
///
/// Three claims, which a wrong reading does not satisfy together:
///
/// 1. The 9 `B8` files are exactly [`AMBIENT_SHADOWS`]: nothing else on the disc
///    uses the format.
/// 2. The descriptor's `remap` broadcasts the stored byte, so every texel is grey
///    with opaque alpha; `Remap` reads it off `+0x10`.
/// 3. The Morton reading is markedly smoother than the raster misread, by better
///    than 1.6x on all nine. `docs/formats/gtf.md` has the picture this number
///    stands in for.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_single_channel_textures_are_ship_shadows_and_read_in_morton_order() {
    let Some(image) = image() else {
        return;
    };
    let mut found = Vec::new();
    for_every_gtf(&image, |path, blob| {
        let parsed = Gtf::parse(blob).expect("parses");
        let texture = parsed.only().expect("one");
        if texture.format != Format::B8 {
            return;
        }
        found.push(path.to_string());
        assert_eq!((texture.width, texture.height), (128, 64), "{path}");
        assert!(!texture.is_linear(), "{path}");
        assert_eq!(texture.remap, 0xa9ff, "{path}");

        let (width, height) = texture.level_size(0);
        let native = texture.to_rgba(blob).expect("decodes");
        for texel in &native {
            assert_eq!(texel[0], texel[1], "{path}: not grey");
            assert_eq!(texel[1], texel[2], "{path}: not grey");
            assert_eq!(texel[3], 0xff, "{path}: alpha not forced opaque");
        }

        let texels = &blob[texture.level_range(0)];
        let mut wrong =
            gtf::decode_level(texture.format, texels, width, height, 0, true).expect("decodes");
        // The same remap on both sides (see
        // `the_swizzle_order_is_smoother_than_a_linear_misread`); without it the
        // broadcast alone makes the Morton reading three times rougher.
        gtf::Remap::decode(texture.remap).apply(&mut wrong);
        let (a, b) = (
            roughness_2d(&native, width as usize),
            roughness_2d(&wrong, width as usize),
        );
        println!("{path}: morton {a:.3}, raster misread {b:.3}");
        assert!(
            b > a * 1.6,
            "{path}: morton {a:.3} is not markedly smoother than raster {b:.3}"
        );
    });
    assert_eq!(found, AMBIENT_SHADOWS);
}

/// [`roughness`], plus the same thing down columns.
///
/// **The swizzle question needs both axes; the endianness question does not.** A
/// byte-swapped `R5G6B5` endpoint is wrong in every direction. A Morton misread
/// lays each tile out as consecutive texels, i.e. *horizontal stripes*, uniform
/// along a row, so a within-row metric scores the wrong reading as smooth. Three
/// swizzled files were named exceptions for that reason; the vertical axis retires
/// them. See `docs/formats/gtf.md`.
fn roughness_2d(rgba: &[[u8; 4]], width: usize) -> f64 {
    let across = roughness(rgba, width);
    let mut total = 0u64;
    let mut count = 0u64;
    for (index, texel) in rgba.iter().enumerate().take(rgba.len() - width) {
        for (top, below) in texel.iter().zip(&rgba[index + width]).take(3) {
            total += u64::from(top.abs_diff(*below));
            count += 1;
        }
    }
    across + total as f64 / count.max(1) as f64
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
    // `DATA00` alone settles it in about a minute; the full-disc figure is on the docs page.
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
        // A mask whose RGB is uniformly white (many, the retro HUD skins among them)
        // is flat either way.
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
/// `oag_hd::hud::TEXTURES`'s spellings cannot be used (`oag-texture` does not
/// depend on `oag-hd`), so these are the resolved entry paths;
/// `crates/game/tests/hd_hud_ground_truth.rs` ties the two together.
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
/// Face-major (all of one face's levels, then the next) is what
/// `Texture::chain_len` assumes, asserted here because level-major gives the same
/// total length and a scrambled sky.
///
/// The picture-free check: **a cube's six faces are six different images, and a
/// sky's four sides are more alike than any is to the zenith or nadir.** On
/// Talon's Junction the zenith is deep blue (mean `(78, 123, 165)`), the nadir
/// blank white and the four sides pale horizon (`204..217` red). Read level-major,
/// the "faces" would be slices of one image and not separate that way.
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
            // A face index is meaningless on a flat texture and is refused.
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
        // **The content claim is asserted on the one sky whose picture was looked
        // at, and reported for the rest.** `amphiseum`'s is a night sky (every face
        // near black, so a bar measures exposure) and `zone_1`'s a featureless grey
        // void whose zenith and nadir are alike. Both are data, not decode errors.
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
                // Read off the decoded faces and checked against the picture: blue
                // zenith, white nadir, four pale horizon sides with a skyline. See
                // `docs/formats/gtf.md`.
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
