//! Parses every `.gxt` Wipeout 2048's base package ships and checks what came
//! out.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-texture --run-ignored all \
//!     -E 'binary(gxt_ground_truth)'
//! ```
//!
//! # Why the sweep is the test
//!
//! [`oag_texture::gxt::Gxt::parse`] refuses a texture whose declared texel length
//! is not what its width, height, format and mip count imply, the argument
//! [`oag_texture::gtf::Gtf::parse`] rests on in `gtf_ground_truth.rs`.
//! `oag_2048::hud::ART` reaches only the nine `.gxt` files
//! [`docs/formats/2048-hud.md`] names; this sweeps the whole corpus.
//!
//! **That check only runs on a format whose unit size is known**, so `PVRTII4BPP`
//! put 8,430 more textures under it in this package alone, and they pass only
//! because a mip level is floored at [`oag_texture::gxt::MIN_LEVEL_LEN`] bytes
//! (measured).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oag_texture::gxt::{Format, Gxt};

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
    ubc1: usize,
    ubc3: usize,
    pvrtc: usize,
    argb8888: usize,
    rgb888: usize,
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
                Some(Format::Ubc1) => out.ubc1 += 1,
                Some(Format::Ubc3) => out.ubc3 += 1,
                Some(Format::Pvrtii4bpp) => out.pvrtc += 1,
                Some(Format::Argb8888) => out.argb8888 += 1,
                Some(Format::Rgb888) => out.rgb888 += 1,
                None => *out.unsupported.entry(texture.format_byte).or_default() += 1,
            }
        }
        check(&entry, &parsed, &blob);
    }
    out
}

/// Every shipped `.gxt` parses, its declared length agrees with the arithmetic
/// its own descriptor implies, and every texture in a format this crate
/// decodes turns into exactly `width * height` texels.
///
/// **The counts are pinned, not just printed.** `checked == decoded + ubc1 +
/// ubc3 + pvrtc + argb8888 + rgb888` alone would hold even if `Gxt::parse`
/// silently skipped nine thousand files. `9910`/`370`/`8430`/`99` are what the
/// corpus is (measured 2026-08-26/27/28); `505`/`493`/`13`
/// (`UBC1`/`UBC3`/`Rgb888`) were measured 2026-09-16, closing out every format
/// byte the corpus carries - `unsupported` is empty from here on. A regression
/// changes one of these numbers.
///
/// All 8,430 `PVRTII4BPP` textures satisfy the length arithmetic that was
/// previously trusted. `Argb8888`'s 99 and `Rgb888`'s 13 take no
/// `MIN_LEVEL_LEN` floor (see `oag_texture::gxt::Texture::level_len`): all nine
/// `(width, height, mip count)` `Argb8888` shapes agree with the unfloored
/// formula, as does `Rgb888`'s one shape (512x64, one level). `UBC1`/`UBC3` reuse
/// `UBC2`'s twiddled walk and floored arithmetic - see `docs/formats/gxt.md`.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn every_shipped_gxt_parses_and_every_known_format_decodes() {
    let mut checked = 0usize;
    let found = survey(&mut |name, parsed, blob| {
        for texture in &parsed.textures {
            if texture.format().is_none() {
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
        "{} files, {} UBC2 + {} UBC1 + {} UBC3 + {} PVRTII4BPP + {} Argb8888 + {} Rgb888 \
         textures decoded, unsupported formats: {:?}",
        found.files,
        found.decoded,
        found.ubc1,
        found.ubc3,
        found.pvrtc,
        found.argb8888,
        found.rgb888,
        found.unsupported
    );
    assert_eq!(found.files, 9910, "shipped .gxt files");
    assert_eq!(found.decoded, 370, "UBC2 textures across them");
    assert_eq!(found.ubc1, 505, "UBC1 textures across them");
    assert_eq!(found.ubc3, 493, "UBC3 textures across them");
    assert_eq!(found.pvrtc, 8430, "PVRTII4BPP textures across them");
    assert_eq!(found.argb8888, 99, "Argb8888 textures across them");
    assert_eq!(found.rgb888, 13, "Rgb888 textures across them");
    assert!(found.unsupported.is_empty(), "{:?}", found.unsupported);
    assert_eq!(
        checked,
        found.decoded + found.ubc1 + found.ubc3 + found.pvrtc + found.argb8888 + found.rgb888
    );
}

/// Decodes a named entry to a checkerboard-composited PNG so a human can check
/// it looks like authored art, not noise or a solid fill: there is no
/// ground-truth frame, so the check is "does it look like something a game would
/// ship".
///
/// `.gxt`'s block grid is **twiddled (Morton/Z-order), not raster** (see
/// `oag_texture::gxt::blocks`), and this test is what that rests on: the wrong
/// order renders noise with a clean band; the twiddled order renders four
/// recognisable reticle pieces (`missile_reticule.gxt`) and a coherent sprite
/// atlas (`hud_2048.gxt`, the non-square 1024x512 case).
fn render_checkerboard(
    archive: &mut oag_assets::psarc::Archive,
    path: &str,
    out_name: &str,
) -> Vec<[u8; 4]> {
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
    let png = oag_texture::png::encode_rgba(width, height, &packed);

    let out = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/shots")
        .join(out_name);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).expect("creating data/shots");
    }
    std::fs::write(&out, &png).unwrap_or_else(|e| panic!("writing {}: {e}", out.display()));
    println!("wrote {}", out.display());

    // Not blank, not one flat colour (`opaque > 0` alone would miss a solid fill).
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
    rgba
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_played_skin_s_own_textures_decode_to_something_a_human_can_check() {
    let Some(path) = package() else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
        .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));

    // Square (256x256) and non-square (1024x512): the non-square path needs its own witness.
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

/// The same picture check on `UBC3` (BC3), which reuses `UBC2`'s twiddled walk
/// (see `oag_texture::gxt::blocks`) with BC3's block math. `UBC1`'s check
/// (`ubc1_decodes_to_something_a_human_can_check`) cannot reuse this helper:
/// BC1 has no alpha and `render_checkerboard`'s flatness assertions assume one.
/// No HD `.gtf` twin exists for either format (`vita_gxt_ubc13_hd_oracle.rs`,
/// `crates/game/examples/`, finds 0 same-name same-size pairs across all three EU
/// packages), so a legible sample is the oracle: a front-end callout with a text
/// label and a punch-through alpha edge. The raster-order control in that probe
/// decodes to noise.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn ubc3_decodes_to_something_a_human_can_check() {
    let Some(path) = package() else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
        .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));

    render_checkerboard(
        &mut archive,
        "data/FE/NewImages/TinyCallout_MP.gxt",
        "2048_ubc3_callout.png",
    );
}

/// `UBC1` (BC1)'s check, as `UBC3`'s but without the alpha-based flatness
/// assertions: every texel measured on this sample decodes opaque, so "some
/// transparent" is the wrong invariant. A wrong block order or palette math would
/// garble the colour, so the check is on distinct colours and the picture.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn ubc1_decodes_to_something_a_human_can_check() {
    let Some(path) = package() else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
        .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));

    let entry = "data/Books/Manual/Pages/02/001.gxt";
    let blob = archive
        .read_path(entry)
        .unwrap_or_else(|e| panic!("{entry}: {e}"));
    let parsed = Gxt::parse(&blob).unwrap_or_else(|e| panic!("{entry}: {e}"));
    let texture = parsed.only().expect("one texture");
    let rgba = texture
        .to_rgba(&blob)
        .unwrap_or_else(|e| panic!("{entry}: {e}"));
    assert!(
        rgba.iter().all(|p| p[3] == 255),
        "{entry}: BC1 has no alpha ramp, every texel should decode fully opaque"
    );
    let distinct: std::collections::BTreeSet<[u8; 4]> = rgba.iter().copied().collect();
    assert!(
        distinct.len() > 4,
        "{entry}: only {} distinct texels - too flat to be authored art",
        distinct.len()
    );

    let width = u32::from(texture.width);
    let height = u32::from(texture.height);
    let packed: Vec<u8> = rgba.iter().flatten().copied().collect();
    let png = oag_texture::png::encode_rgba(width, height, &packed);
    let out = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data")
        .join("shots")
        .join("2048_ubc1_manual_page.png");
    std::fs::create_dir_all(out.parent().expect("has a parent")).expect("creating data/shots");
    std::fs::write(&out, &png).unwrap_or_else(|e| panic!("writing {}: {e}", out.display()));
    println!("wrote {}", out.display());
}

/// The same picture check on the format that is 85% of the corpus: `PVRTII4BPP`.
///
/// **A font atlas is the strongest oracle for this codec**, which is why
/// `RussianHud.gxt` is rendered here. A wrong word order, bit layout or the 4bpp
/// path's internal transposition garbles glyph shapes while leaving the image
/// smooth enough that a smoothness metric barely separates them (an untwiddled
/// control scores only 1.4x rougher). It renders the full Latin and Cyrillic
/// alphabets, crisp and upright, on a 1024x1024 surface.
///
/// The assertions are what a font atlas *is*, so a decode that merely looks busy
/// fails:
///
/// - **Near-monochrome.** The face is white; a bit layout that mixed channels
///   would tint it (the three channels come from three different bit fields).
/// - **Bimodal alpha.** Glyph or background with anti-aliasing between, not the
///   smeared middle a wrong modulation decode gives.
fn font_atlas_looks_like_a_font(rgba: &[[u8; 4]]) {
    let monochrome = rgba
        .iter()
        .filter(|t| {
            let (lo, hi) = (t[..3].iter().min().unwrap(), t[..3].iter().max().unwrap());
            hi - lo <= 8
        })
        .count() as f64
        / rgba.len() as f64;
    let bimodal =
        rgba.iter().filter(|t| t[3] == 0 || t[3] >= 250).count() as f64 / rgba.len() as f64;
    let clear = rgba.iter().filter(|t| t[3] == 0).count() as f64 / rgba.len() as f64;
    println!(
        "font atlas: {monochrome:.4} monochrome, {bimodal:.4} bimodal alpha, {clear:.4} clear"
    );
    // Measured 2026-08-27: 1.0000 monochrome, 0.8221 bimodal, 0.6390 clear. The
    // thresholds sit below with room for rounding but not for a wrong decode,
    // which loses the monochrome property outright.
    assert!(
        monochrome > 0.99,
        "font is white, got {monochrome:.4} monochrome"
    );
    assert!(bimodal > 0.75, "glyph or background, got {bimodal:.4}");
    assert!(
        (0.2..0.95).contains(&clear),
        "an atlas is mostly but not entirely background, got {clear:.4} clear"
    );
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_pvrtc_textures_decode_to_something_a_human_can_check() {
    let Some(path) = package() else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
        .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));

    let font = render_checkerboard(
        &mut archive,
        "data/fe/fonts/russianhud.gxt",
        "2048_pvrtc_font.png",
    );
    font_atlas_looks_like_a_font(&font);

    // A game-mode icon: two flat tones on transparency, exercising the alpha path.
    render_checkerboard(
        &mut archive,
        "data/fe/images/detonator.gxt",
        "2048_pvrtc_detonator.png",
    );
}

/// `Argb8888`'s picture check: 2048's Zone/Detonator "Track" speed-class art, the
/// corpus `docs/formats/gxt.md` left the tiling and channel order open on.
///
/// Raw bytes showed `data/Tex/zoneModeTrack{0..14}.gxt` changing shape across the
/// stage ladder while `data/Tex/zoneMode{0..14}.gxt` (the "general" set, swept
/// below) is a flat, identical blank, so a correct decode has a real prediction
/// to check against.
///
/// Two invariants a wrong tiling or channel order would not both survive:
///
/// - **Opaque fraction is `2048 / 65536` texels on all three stages** (measured
///   2026-08-28): the stencil mask's *area* is constant, only its *shape*
///   changes. A wrong tiling scrambles which texels are opaque, not how many,
///   so this is a sanity check the shape assertion does not cover.
/// - **No two stages decode to the same texel multiset.** `zoneMode0.gxt`'s
///   ground truth below is the control: a flat set decodes to one solid colour.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_zone_track_art_decodes_to_a_shape_that_escalates_across_stages() {
    let Some(path) = package() else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
        .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));

    let mut stages = Vec::new();
    for stage in [0, 7, 14] {
        let rgba = render_checkerboard(
            &mut archive,
            &format!("data/tex/zonemodetrack{stage}.gxt"),
            &format!("2048_zone_track_stage{stage}.png"),
        );
        assert_eq!(rgba.len(), 256 * 256, "zoneModeTrack{stage}: 256x256");
        let opaque = rgba.iter().filter(|t| t[3] == 255).count();
        assert_eq!(
            opaque, 2048,
            "zoneModeTrack{stage}: opaque texel count, measured 2026-08-28"
        );
        stages.push(rgba);
    }
    for a in 0..stages.len() {
        for b in (a + 1)..stages.len() {
            assert_ne!(
                stages[a], stages[b],
                "two sampled stages decoded identically - the escalation this set is named for is missing"
            );
        }
    }

    // The control: the "general" set is the flat placeholder `docs/formats/gxt.md`
    // found by byte pattern (fifteen identical files, one solid colour), checked
    // here by decoding.
    let blank = archive
        .read_path("data/tex/zonemode0.gxt")
        .expect("zoneMode0.gxt");
    let parsed = Gxt::parse(&blank).expect("parses");
    let texture = parsed.only().expect("one texture");
    let rgba = texture.to_rgba(&blank).expect("decodes");
    let distinct: std::collections::BTreeSet<[u8; 4]> = rgba.iter().copied().collect();
    assert_eq!(
        distinct.len(),
        1,
        "zoneMode0.gxt (the 'general' set): expected one flat colour, got {} distinct texels",
        distinct.len()
    );
}

/// Wipeout HD's `.gtf` decode is ground truth for 2048's `PVRTII4BPP` one,
/// over the 2,284 textures the two titles share.
///
/// **The strongest evidence this format's decode has.** 2048's DLC re-ships
/// HD/Fury's circuits and fourteen-team roster, so the *same authored texture*
/// is a BC `.gtf` on the PS3 disc and a `PVRTII4BPP` `.gxt` in the Vita package:
/// the HD-as-oracle method that settled the `WO Track` point tail, 2048's vertex
/// normal and its `Uv1` (`docs/formats/2048-rcsmodel.md`).
///
/// Two lossy codecs never agree bit for bit, so the measurement is a *comparison
/// of comparisons*. Measured 2026-08-27, mean absolute per-channel difference of
/// 255, median over 2,284 pairs:
///
/// | Compared against HD's own decode | Median |
/// | --- | ---: |
/// | this decode | **3.83** |
/// | this decode, HD flipped vertically | 10.01 |
/// | the same payload read in raster word order | 34.60 |
/// | a different texture of the same size (chance) | 59.74 |
///
/// The flipped row says the Vita's rows are top-down where the PS3's are
/// bottom-up (measured). The raster row is the wrong answer most likely to
/// occur, nine times further away; the last row is "no relationship".
///
/// The threshold is on the *median*: a quarter of the pairs are different art
/// sharing a basename (several circuits ship their own `billboard3`), so a mean
/// or worst case would measure the pairing, not the decode.
#[test]
#[ignore = "needs both the Vita package and the decrypted PS3 disc"]
fn the_pvrtc_decode_agrees_with_wipeout_hd_s_own_copy_of_the_same_art() {
    let packages = vita_packages();
    let Some(disc) = hd_disc() else { return };
    if packages.is_empty() {
        return;
    }

    // Collect the basenames worth decoding on the HD side first, so the sweep does
    // not decode 3,588 PS3 textures to use 2,284.
    let mut wanted: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
    let mut archives: Vec<oag_assets::psarc::Archive> = packages
        .iter()
        .map(|path| {
            oag_assets::psarc::Archive::open(&path.display().to_string())
                .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()))
        })
        .collect();
    for (index, archive) in archives.iter_mut().enumerate() {
        for entry in archive.paths().to_vec() {
            if entry.to_ascii_lowercase().ends_with(".gxt") {
                wanted
                    .entry(basename(&entry))
                    .or_default()
                    .push((index, entry));
            }
        }
    }

    let mut truth: BTreeMap<String, (u32, u32, Vec<[u8; 4]>)> = BTreeMap::new();
    for name in HD_ARCHIVES {
        let spec = format!("{}:{name}", disc.display());
        let Ok(mut open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".gtf"))
            .filter(|p| wanted.contains_key(&basename(p)))
            .cloned()
            .collect();
        for path in paths {
            let key = basename(&path);
            if truth.contains_key(&key) {
                continue;
            }
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            let Ok(parsed) = oag_texture::gtf::Gtf::parse(&blob) else {
                continue;
            };
            let Some(texture) = parsed.textures.first() else {
                continue;
            };
            let Ok(rgba) = texture.to_rgba(&blob) else {
                continue;
            };
            if rgba.len() == usize::from(texture.width) * usize::from(texture.height) {
                truth.insert(
                    key,
                    (u32::from(texture.width), u32::from(texture.height), rgba),
                );
            }
        }
    }

    let mut matched = Vec::new();
    let mut flipped_scores = Vec::new();
    for (name, entries) in &wanted {
        let Some((width, height, hd)) = truth.get(name) else {
            continue;
        };
        for (index, entry) in entries {
            let Ok(blob) = archives[*index].read_path(entry) else {
                continue;
            };
            let Ok(parsed) = Gxt::parse(&blob) else {
                continue;
            };
            let Some(texture) = parsed.only() else {
                continue;
            };
            if texture.format() != Some(Format::Pvrtii4bpp)
                || u32::from(texture.width) != *width
                || u32::from(texture.height) != *height
            {
                continue;
            }
            let Ok(rgba) = texture.to_rgba(&blob) else {
                continue;
            };
            matched.push(mean_difference(&rgba, hd));
            flipped_scores.push(mean_difference(
                &rgba,
                &flip_rows(hd, *width as usize, *height as usize),
            ));
        }
    }

    if matched.is_empty() {
        return;
    }
    let median = |mut v: Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v[v.len() / 2]
    };
    let same = median(matched.clone());
    let upside_down = median(flipped_scores);
    println!(
        "{} shared textures: median difference {same:.2}, {upside_down:.2} against HD flipped",
        matched.len()
    );
    assert!(
        matched.len() > 2000,
        "only {} shared textures found - the pairing itself regressed",
        matched.len()
    );
    assert!(
        same < 8.0,
        "median difference against HD's own decode is {same:.2}, measured at 3.83"
    );
    assert!(
        upside_down > same * 2.0,
        "HD flipped ({upside_down:.2}) should be far worse than as-decoded ({same:.2}) - \
         if it is not, this pairing is not sensitive to orientation and proves less than it looks"
    );
}

/// The seven archives Wipeout HD's disc carries, as
/// `crates/texture/tests/gtf_ground_truth.rs` names them.
const HD_ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
    "PS3_GAME/USRDIR/DATA04.PSARC",
    "PS3_GAME/USRDIR/DATA05.PSARC",
    "PS3_GAME/USRDIR/DATA06.PSARC",
];

/// Every Wipeout 2048 package present: the base and both DLC archives.
/// **The DLC halves are load-bearing**: the re-shipped HD/Fury circuits, and most
/// of the shared art, live there.
fn vita_packages() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007");
    [
        "base/PSP2/data.psarc",
        "dlc1/PSP2/dlc1.psarc",
        "dlc2/PSP2/dlc2.psarc",
    ]
    .iter()
    .map(|tail| root.join(tail))
    .filter(|path| path.exists())
    .collect()
}

fn hd_disc() -> Option<PathBuf> {
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

/// A path's filename without its extension, lowercased - how the two titles'
/// otherwise unrelated directory trees are matched up.
fn basename(path: &str) -> String {
    let lower = path.to_ascii_lowercase().replace('\\', "/");
    let file = lower.rsplit('/').next().unwrap_or(&lower).to_string();
    file.rsplit_once('.')
        .map_or(file.clone(), |(stem, _)| stem.to_string())
}

/// Mean absolute per-channel difference over RGB, 0 to 255.
fn mean_difference(a: &[[u8; 4]], b: &[[u8; 4]]) -> f64 {
    let mut total = 0u64;
    for (x, y) in a.iter().zip(b) {
        for channel in 0..3 {
            total += u64::from(x[channel].abs_diff(y[channel]));
        }
    }
    total as f64 / (a.len() * 3) as f64
}

fn flip_rows(rgba: &[[u8; 4]], width: usize, height: usize) -> Vec<[u8; 4]> {
    let mut out = Vec::with_capacity(rgba.len());
    for y in (0..height).rev() {
        out.extend_from_slice(&rgba[y * width..(y + 1) * width]);
    }
    out
}
