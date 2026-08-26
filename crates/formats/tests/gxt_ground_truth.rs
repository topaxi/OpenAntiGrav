//! Parses every `.gxt` Wipeout 2048's base package ships and checks what came
//! out.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-formats --run-ignored all \
//!     -E 'binary(gxt_ground_truth)'
//! ```
//!
//! # Why the sweep is the test
//!
//! [`oag_formats::gxt::Gxt::parse`] refuses a `UBC2` texture whose declared
//! texel length is not what its own width, height and mip count imply - the
//! same "the length is a function of everything else in the descriptor"
//! argument [`oag_formats::gtf::Gtf::parse`] rests its own confidence on, in
//! `gtf_ground_truth.rs`. `oag_2048::hud::ART`'s reticle only reaches the nine
//! `.gxt` files [`docs/formats/2048-hud.md`] names, and this sweeps the whole
//! corpus rather than just those nine.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oag_formats::gxt::{Format, Gxt};

fn package() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
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

/// What one sweep of the corpus found.
#[derive(Default)]
struct Survey {
    files: usize,
    decoded: usize,
    unsupported: BTreeMap<u32, usize>,
}

fn survey(check: &mut impl FnMut(&str, &Gxt, &[u8])) -> Survey {
    let mut out = Survey::default();
    let Some(path) = package() else {
        return out;
    };
    let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
        .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut entries: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".gxt"))
        .cloned()
        .collect();
    entries.sort();

    for entry in entries {
        let blob = archive
            .read_path(&entry)
            .unwrap_or_else(|e| panic!("reading {entry}: {e}"));
        let parsed = Gxt::parse(&blob).unwrap_or_else(|e| panic!("{entry}: {e}"));
        out.files += 1;
        for texture in &parsed.textures {
            match texture.format() {
                Some(Format::Ubc2) => out.decoded += 1,
                None => *out.unsupported.entry(texture.format_byte).or_default() += 1,
            }
        }
        check(&entry, &parsed, &blob);
    }
    out
}

/// Every shipped `.gxt` parses, and its declared length agrees with the
/// arithmetic its own descriptor implies wherever the format is `UBC2`.
///
/// **The counts are pinned, not just printed.** `checked` and `found.decoded`
/// both increment on the same `Some(Format::Ubc2)` textures inside `survey`'s
/// callback, so `checked == found.decoded` alone would hold even if `Gxt::parse`
/// silently skipped nine thousand files - it says nothing about whether the
/// *right* files were seen. `9910`/`370` are what the corpus actually is,
/// measured 2026-08-26; a real regression changes one of these numbers, which
/// only a pinned value can catch.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn every_shipped_gxt_parses_and_ubc2_textures_decode() {
    let mut checked = 0usize;
    let found = survey(&mut |name, parsed, blob| {
        for texture in &parsed.textures {
            if texture.format() != Some(Format::Ubc2) {
                continue;
            }
            let rgba = texture
                .to_rgba(blob)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(
                rgba.len(),
                usize::from(texture.width) * usize::from(texture.height),
                "{name}: decoded pixel count"
            );
            checked += 1;
        }
    });

    if found.files == 0 {
        return;
    }

    println!(
        "{} files, {} UBC2 textures decoded, unsupported formats: {:?}",
        found.files, found.decoded, found.unsupported
    );
    assert_eq!(found.files, 9910, "shipped .gxt files");
    assert_eq!(found.decoded, 370, "UBC2 textures across them");
    assert_eq!(checked, found.decoded);
}

/// Decodes a named entry and writes it to a checkerboard-composited PNG so a
/// human (or a later `Read` of the file) can check it looks like authored art
/// rather than noise or a solid fill. This format's version of `gtf`'s own
/// smoothness check: there is no ground-truth frame to compare against, so
/// the check is "does the picture look like something a game would ship".
///
/// `.gxt`'s block grid is **twiddled (Morton/Z-order), not raster** - see
/// `oag_formats::gxt::blocks`'s own doc comment for how that was measured.
/// This test is what the measurement rests on: a checkerboard-composited
/// render of the wrong block order was noise with an anomalous clean band;
/// the current, twiddled order renders four recognisable reticle pieces
/// (`missile_reticule.gxt`) and a coherent-looking sprite atlas
/// (`hud_2048.gxt`, the non-square 1024x512 case the same block-order rule
/// has to hold for).
fn render_checkerboard(archive: &mut oag_assets::psarc::Archive, path: &str, out_name: &str) {
    let blob = archive
        .read_path(path)
        .unwrap_or_else(|e| panic!("{path}: {e}"));
    let parsed = Gxt::parse(&blob).unwrap_or_else(|e| panic!("{path}: {e}"));
    let texture = parsed.only().expect("one texture");
    let rgba = texture
        .to_rgba(&blob)
        .unwrap_or_else(|e| panic!("{path}: {e}"));

    let width = u32::from(texture.width);
    let height = u32::from(texture.height);
    let mut packed = Vec::with_capacity(rgba.len() * 4);
    for (index, texel) in rgba.iter().enumerate() {
        let x = index as u32 % width;
        let y = index as u32 / width;
        let checker: f32 = if (x / 16 + y / 16).is_multiple_of(2) {
            60.0
        } else {
            90.0
        };
        let a = f32::from(texel[3]) / 255.0;
        for &channel in &texel[..3] {
            let blended = f32::from(channel) * a + checker * (1.0 - a);
            packed.push(blended.round() as u8);
        }
        packed.push(255);
    }
    let png = oag_formats::png::encode_rgba(width, height, &packed);

    let out = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/shots")
        .join(out_name);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).expect("creating data/shots");
    }
    std::fs::write(&out, &png).unwrap_or_else(|e| panic!("writing {}: {e}", out.display()));
    println!("wrote {}", out.display());

    // Not blank, not one flat colour, not a decode that collapsed to a solid
    // fill (which `opaque > 0` alone would not catch).
    let opaque = rgba.iter().filter(|p| p[3] > 0).count();
    assert!(opaque > 0, "{path}: nothing decoded opaque at all");
    assert!(
        opaque < rgba.len(),
        "{path}: every texel opaque - a decode that came out as a solid fill would too"
    );
    let distinct: std::collections::BTreeSet<[u8; 4]> = rgba.iter().copied().collect();
    assert!(
        distinct.len() > 4,
        "{path}: only {} distinct texels - too flat to be authored art",
        distinct.len()
    );
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_played_skin_s_own_textures_decode_to_something_a_human_can_check() {
    let Some(path) = package() else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
        .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));

    // Square (256x256, one level) and non-square (1024x512) - the twiddle
    // rule's general, non-square path needs its own witness.
    render_checkerboard(
        &mut archive,
        "data/xml/2048_hud/texture/missile_reticule.gxt",
        "2048_missile_reticule.png",
    );
    render_checkerboard(
        &mut archive,
        "data/xml/2048_hud/texture/hud_2048.gxt",
        "2048_hud_2048.png",
    );
}
