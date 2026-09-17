//! HD's start gantry digit board: what its own `.rcsmodel` authors for the
//! animated `uvOffset` curve four of its five materials carry.
//!
//! `#[ignore]`d because it needs `data/images/hdfury-ps3-eu-dec.iso`; run with
//! `just test-data`. See `docs/formats/edge-animation.md` for the full byte
//! layout this reproduces, and
//! `docs/ghidra/functions/ps3-hdfury-eu/billboards.md`'s 2026-09-17 sections
//! for the RE trail that located it.
//!
//! # What this is for
//!
//! `oag_rcs::edgeanim` decodes Sony's own Edge Animation Tools clip format,
//! and `oag_rcs::rcsmodel::material::curve` reads a material's own pointer to
//! one. Both were written from a byte-level model built against this exact
//! file, so this reproduces the numbers that model was checked against
//! rather than asserting something new: a decoder that silently regressed
//! would fail here before it failed anywhere a picture is judged.
//!
//! **This does not assert the curve drives the countdown's digit selection.**
//! It does not - see the last two tests. The curve is real, decoded and
//! sampled correctly; what it authors is a smooth reveal ramp per material,
//! staggered across the four materials, not four discrete glyph states.

mod rcsmodel_common;

use oag_rcs::rcsmodel;
use rcsmodel_common::image;

/// The raw name hash of one parameter-table entry, read directly off the
/// file rather than through `Material::parameters` - that list drops sampler
/// entries and so does not line up with a [`oag_rcs::rcsmodel::material::curve::Channel::target_index`],
/// which indexes the raw table. `material_at` is the material record's own
/// file offset; `+0x34` is the table `oag_rcs::rcsmodel::material::parameters`
/// module documents.
fn raw_parameter_hash(data: &[u8], material_at: usize, target_index: u32) -> u32 {
    let be32 = |at: usize| u32::from_be_bytes(data[at..at + 4].try_into().unwrap());
    let table = be32(material_at + 0x34) as usize;
    be32(table + target_index as usize * 0x20)
}

/// The digit board's own model, holding five materials: one plain
/// `simpletexture` and four `uvoffsetscale` variants.
const GANTRY_MODEL: &str = "/data/billboards/hd_adverts/321go/321go_startfinish.rcsmodel";

fn model() -> Option<Vec<u8>> {
    let path = image()?;
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", path.display());
    let mut archive = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
    Some(
        archive
            .read_path(GANTRY_MODEL)
            .expect("the digit board's model reads"),
    )
}

/// **Four of five materials carry a curve, and they are exactly the four
/// whose own name says `uvoffsetscale`.**
///
/// The correlation with the material name, not merely a nonzero pointer, is
/// what makes `Material::curve`'s `+0x20`-then-`+0xc` reading a decode rather
/// than a guess at a plausible offset - see
/// `docs/ghidra/functions/ps3-hdfury-eu/billboards.md`'s confidence-90 note.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn four_of_five_materials_carry_a_curve() {
    let Some(data) = model() else { return };
    let parsed = rcsmodel::Model::parse(&data).expect("the model parses");
    assert_eq!(
        parsed.materials.len(),
        5,
        "the digit board's own material count"
    );

    for material in &parsed.materials {
        let carries_curve = material.name.contains("uvoffsetscale");
        assert_eq!(
            material.curve.is_some(),
            carries_curve,
            "{:?}: curve presence should follow the material name",
            material.name
        );
    }
}

/// **Every curve drives `uvOffset.x` and `uvOffset.y`, confirmed by hash, not
/// by position.**
///
/// `oag_rcs::rcsmaterial::name_hash("uvOffset")` against the material's own
/// parameter-table entry at each channel's `target_index` - the same
/// confirmation `billboards.md` already ran, reproduced here so a parser
/// regression fails a test instead of only a report.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn every_channel_is_uv_offset_x_and_y() {
    let Some(data) = model() else { return };
    let parsed = rcsmodel::Model::parse(&data).expect("the model parses");
    let uv_offset_hash = oag_rcs::rcsmaterial::name_hash("uvOffset");

    // The material offset table `oag_rcs::rcsmodel::material` documents -
    // needed here because a channel's `target_index` is into the material's
    // own *raw* `+0x34` parameter table, which `Material::parameters` does
    // not preserve positions for (see `raw_parameter_hash`).
    let be32 = |at: usize| u32::from_be_bytes(data[at..at + 4].try_into().unwrap());
    let material_table = be32(0x30) as usize;

    let mut checked = 0;
    for (index, material) in parsed.materials.iter().enumerate() {
        let Some(curve) = &material.curve else {
            continue;
        };
        let material_at = be32(material_table + index * 4) as usize;
        let channels = curve.channels();
        assert_eq!(
            channels.len(),
            2,
            "{:?}: two channels, x and y",
            material.name
        );
        for (channel, expect_component) in channels.iter().zip([0u8, 1u8]) {
            assert_eq!(channel.component, expect_component);
            let hash = raw_parameter_hash(&data, material_at, channel.target_index);
            assert_eq!(
                hash, uv_offset_hash,
                "{:?} channel {:?}: target is uvOffset by hash",
                material.name, channel
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 4, "all four uvoffsetscale materials checked");
}

/// **The clip's own header, reproduced exactly.**
///
/// `duration` and `sample_frequency` are read straight off the bytes;
/// `sample_frequency` differs per material (this is what makes the four
/// curves ramp at different real-time windows despite a shared 13.333 s
/// `duration` - see the staggered-reveal test below) while `duration` does
/// not.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_clip_header_matches_the_disc() {
    let Some(data) = model() else { return };
    let parsed = rcsmodel::Model::parse(&data).expect("the model parses");
    let curved: Vec<_> = parsed
        .materials
        .iter()
        .filter_map(|m| m.curve.as_ref())
        .collect();
    assert_eq!(curved.len(), 4);

    for curve in &curved {
        assert!(
            (curve.clip().duration - 13.333_333).abs() < 1e-3,
            "all four clips share one loop duration"
        );
    }

    // Material 1's own clip (`docs/formats/edge-animation.md`'s worked
    // example): 49 frame-sets, ~6182 frames at ~463.56 Hz.
    let first = curved[0];
    assert_eq!(first.clip().num_anim_user_channels(), 2);
    assert!((first.clip().sample_frequency - 463.561_5).abs() < 1e-2);
}

/// **The digit board's four glyph-material curves read as continuously
/// varying wipes, not four discrete glyph states.**
///
/// Sampled 40 times evenly across the shared 13.333 s loop, each curve's own
/// `uvOffset.y` moves gradually - several real intermediate steps, never one
/// step that accounts for nearly the whole range. A glyph selector choosing
/// between four UV cells - the way Pulse's own gantry does
/// (`crates/vex/tests/start_gantry_ground_truth.rs`) - would show the reverse
/// signature: {0, 0.25, 0.5, 0.75}-sized jumps between long flat plateaus.
/// What these curves show instead (see the printed ranges) is each material
/// resting near one value for part of the loop and moving through a wide
/// span of intermediate values elsewhere - continuous motion, not a
/// four-state switch. This does not by itself explain how the digit board
/// reads `3`, `2`, `1`, `GO` to a player; see `docs/formats/edge-animation.md`
/// for what is and is not settled about that.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn each_curve_varies_continuously_not_in_glyph_sized_steps() {
    let Some(data) = model() else { return };
    let parsed = rcsmodel::Model::parse(&data).expect("the model parses");
    let curved: Vec<_> = parsed
        .materials
        .iter()
        .filter_map(|m| m.curve.as_ref())
        .collect();
    assert_eq!(curved.len(), 4);

    for (index, curve) in curved.iter().enumerate() {
        let duration = curve.clip().duration;
        // Strictly inside `[0, duration)`: a sample at or past `duration`
        // wraps back to (near) frame 0 via `Curve::sample`'s own
        // `rem_euclid`, which would read as a spurious drop right at the loop
        // seam rather than anything the curve itself does.
        let samples: Vec<f32> = (0..40)
            .map(|i| {
                let t = duration * (i as f32) / 40.0;
                curve
                    .sample(&data, t)
                    .into_iter()
                    .find(|(ch, _)| ch.component == 1)
                    .map_or(0.0, |(_, v)| v)
            })
            .collect();

        let min = samples.iter().copied().fold(f32::MAX, f32::min);
        let max = samples.iter().copied().fold(f32::MIN, f32::max);
        let span = max - min;
        println!("material {index}: y ranges {min:.4}..{max:.4} across the loop");
        assert!(
            span > 0.05,
            "material {index}: the y channel actually moves"
        );

        // Not one instantaneous snap from resting value to resting value: a
        // glyph selector swaps in one frame (no adjacent-sample gradient at
        // all, 40-sample resolution or not), a wipe spreads the change over
        // several. So at least one step is a real fraction of the whole
        // range without being nearly all of it.
        let steps: Vec<f32> = samples
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .collect();
        let max_step = steps.iter().copied().fold(0.0f32, f32::max);
        assert!(
            max_step < span * 0.9,
            "material {index}: one step of {max_step:.4} accounts for nearly the whole range \
             {span:.4} - that is what a discrete glyph jump would look like"
        );
        assert!(
            steps
                .iter()
                .filter(|&&s| s > span * 0.02 && s < span * 0.9)
                .count()
                >= 2,
            "material {index}: expected at least two real intermediate steps, evidence of a \
             gradual wipe rather than a snap"
        );
    }
}

/// **The one ramp this page's own evaluator was cross-checked against by
/// hand**: material 1's `uvOffset.y`, sampled across its own frame-set 44's
/// intra keyframe with no jump at either boundary.
///
/// The values here are the ones `docs/formats/edge-animation.md` cites as the
/// worked example. A future change to `edgeanim::Clip`'s arithmetic that
/// still parses without error but samples the wrong bytes will move these.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn material_one_s_y_channel_ramps_smoothly_across_its_frameset_boundary() {
    let Some(data) = model() else { return };
    let parsed = rcsmodel::Model::parse(&data).expect("the model parses");
    let curve = parsed
        .materials
        .iter()
        .find_map(|m| m.curve.as_ref())
        .expect("material 1 carries a curve");

    // `Clip::sample_user_channel` directly, not `Curve::sample` - the latter
    // wraps `time` against `duration` first, and a frame this close to the
    // clip's own last (of 6182) lands within a rounding error of that wrap
    // boundary, which would read as a spurious drop back to frame 0 rather
    // than anything the decode itself does.
    let sf = curve.clip().sample_frequency;
    let sample_y = |frame: f32| -> f32 { curve.clip().sample_user_channel(&data, 1, frame / sf) };

    // Frame-set boundaries - `docs/formats/edge-animation.md`'s table.
    assert!((sample_y(5805.0) - 0.1755).abs() < 1e-3);
    assert!((sample_y(5934.0) - 0.4585).abs() < 1e-3);
    assert!((sample_y(6063.0) - 0.7415).abs() < 1e-3);
    assert!((sample_y(6181.0) - 1.0).abs() < 1e-3);

    // No discontinuity either side of the boundary: a wrong "final" address
    // (a stale offset, or R/T/S bleeding into U) would show up as a jump
    // rather than a straight line.
    let before = sample_y(5804.5);
    let after = sample_y(5805.5);
    assert!(
        (after - before).abs() < 0.01,
        "no jump across the frameset boundary: {before:.4} then {after:.4}"
    );
}
