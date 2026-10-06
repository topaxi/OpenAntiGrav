//! What the recovered per-batch alpha-test reference costs on screen, measured
//! rather than eyeballed.
//!
//! Renders one model twice through the offscreen path - once with each batch's
//! own reference (`vex::Batch::alpha_test_reference`), once with every cutout
//! forced back to this project's old flat `1/255` - and reports the pixel
//! difference at several camera angles. **No window and no Xvfb**: this goes
//! through `capture_pixels_from`, so the numbers are the same on a headless
//! machine as on a desktop.
//!
//! "Lit pixels" is `r + g + b > 100`, the same measure the figures in
//! `docs/formats/vex.md` were taken with, so the two are comparable.
//!
//! ```sh
//! cargo run -q -p oag-render --example threshold_probe -- \
//!     data/images/pulse-psp-usa.chd 'Data\Environments\16_Track\track.vex'
//! ```

use oag_mesh::mesh;
use oag_mesh::mesh_render::Anisotropy;

const SIZE: u32 = 1024;

/// The yaw angles each model is measured at. More than one, because a cutout
/// that vanishes only from behind is exactly what a single frame misses.
const YAWS: [f32; 4] = [0.0, 1.2, 2.4, 3.6];

fn lit(pixels: &[u8]) -> usize {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]) > 100)
        .count()
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/pulse-psp-usa.chd".into());
    let name = args
        .next()
        .unwrap_or_else(|| r"Data\Environments\16_Track\track.vex".into());

    let data = oag_assets::Container::open(&spec)
        .and_then(|mut c| c.read_entry(&name))
        .or_else(|_| {
            oag_assets::Container::open(&format!("{spec}:PSP_GAME/USRDIR/Data.wad"))
                .and_then(|mut c| c.read_entry(&name))
        })?;

    let recovered = mesh::build(&name, &data)?;
    let references: std::collections::BTreeSet<String> = recovered
        .alpha_tested_draws
        .iter()
        .filter_map(|d| d.alpha_test_ref)
        .map(|r| format!("{:.4} ({:#04x})", r, (r * 255.0).round() as u32))
        .collect();
    println!(
        "{name}: {} cutout draw(s), references {:?}",
        recovered.alpha_tested_draws.len(),
        references
    );

    // The old behaviour, reproduced exactly: every cutout back on the flat
    // `1/255` this project compared against before the per-batch reference was
    // recovered. Named rather than left `None`, because `None` takes
    // `mesh.wesl`'s `ALPHA_TEST_THRESHOLD`, which is `0` now that the discard
    // is `<=` - the two are the same picture on 8-bit alpha but not on the
    // sampler's filtered edges, and this probe measures edges.
    let mut flat = recovered.clone();
    for draw in &mut flat.alpha_tested_draws {
        draw.alpha_test_ref = Some(1.0 / 255.0);
    }

    // A third argument writes the pair of frames out, so the delta can be
    // looked at rather than only counted.
    let png_prefix = args.next();

    // A third variant, to separate the two buckets' own costs: the strict
    // `0x7f` batches keep their reference, the permissive `0x10` ones go back
    // to the flat threshold. Rendered against `flat`, this isolates what the
    // strict bucket alone does to the picture - which a texel census cannot
    // answer, because the sampler filters and mips a binary-alpha cutout into
    // intermediate alphas at every edge.
    let mut strict_only = recovered.clone();
    for draw in &mut strict_only.alpha_tested_draws {
        if draw.alpha_test_ref.is_some_and(|r| r < 0.25) {
            draw.alpha_test_ref = Some(1.0 / 255.0);
        }
    }

    let mut total_before = 0usize;
    let mut total_after = 0usize;
    let mut total_changed = 0usize;
    for yaw in YAWS {
        let before = oag_mesh::mesh_render::capture_pixels_from(
            &flat,
            SIZE,
            SIZE,
            yaw,
            0.35,
            Anisotropy::default(),
            0.0,
        )?;
        let after = oag_mesh::mesh_render::capture_pixels_from(
            &recovered,
            SIZE,
            SIZE,
            yaw,
            0.35,
            Anisotropy::default(),
            0.0,
        )?;
        let changed = before
            .as_chunks::<4>()
            .0
            .iter()
            .zip(after.as_chunks::<4>().0)
            .filter(|(a, b)| a != b)
            .count();
        let strict = oag_mesh::mesh_render::capture_pixels_from(
            &strict_only,
            SIZE,
            SIZE,
            yaw,
            0.35,
            Anisotropy::default(),
            0.0,
        )?;
        let strict_changed = before
            .as_chunks::<4>()
            .0
            .iter()
            .zip(strict.as_chunks::<4>().0)
            .filter(|(a, b)| a != b)
            .count();
        let (lit_before, lit_after) = (lit(&before), lit(&after));
        let lit_strict = lit(&strict);
        println!(
            "  yaw {yaw:.2}: lit {lit_before} -> {lit_after} ({:+}), {changed} pixel(s) differ",
            lit_after as i64 - lit_before as i64
        );
        println!(
            "            0x7f bucket alone: lit {lit_strict} ({:+}), {strict_changed} pixel(s) differ",
            lit_strict as i64 - lit_before as i64
        );
        if let Some(prefix) = &png_prefix {
            for (label, pixels) in [("before", &before), ("after", &after)] {
                let path = format!("{prefix}-{label}-yaw{yaw:.2}.png");
                std::fs::write(&path, oag_texture::png::encode_rgba(SIZE, SIZE, pixels))?;
                println!("    wrote {path}");
            }
        }
        total_before += lit_before;
        total_after += lit_after;
        total_changed += changed;
    }
    println!(
        "  total over {} frames: lit {total_before} -> {total_after} ({:+}), {total_changed} pixel(s) differ",
        YAWS.len(),
        total_after as i64 - total_before as i64
    );
    Ok(())
}
