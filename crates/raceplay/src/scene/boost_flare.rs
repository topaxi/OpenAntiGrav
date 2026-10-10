//! One boost plume and one engine flare per grid slot, built from each
//! slot's own livery - the additive-pipeline pair `Scene::new` builds beside
//! the shield shell.
//!
//! Split out of `scene.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; the loop and its comments moved here
//! unchanged - a move, with no behaviour change.

use super::*;

// The boost plume, drawn additively. It is real geometry (a `.vex`
// mesh, not a camera-facing quad), so it needs depth testing and a
// model matrix, which is what `Drawable` already gives every other
// mesh here rather than a third thing beside `exhaust::Pipeline`.
// An additive blend in place of `TRANSPARENT_BLEND` is the one thing
// that has to differ - the model's texture is named `_ADD`, and
// drawing it with the ordinary lerp blend looks plausible and is
// wrong.
//
// **Which additive blend changed on 2026-08-08, from the ribbon's
// `TRAIL_BLEND` to the flare's `exhaust::BLEND`, and it is a bug fix
// rather than a retuning.** The two differ only in the source factor -
// `One` against `SrcAlpha` - and the plume is the one model here whose
// authored vertex alpha is not constant: it is bimodal, `0` on the
// orange rim and `255` in the white core (pinned by
// `boost_plume_ground_truth.rs`). Under `TRAIL_BLEND` alpha cannot
// reach the picture at all, so the load site compensated by
// premultiplying it into the RGB *per vertex* - and that is simply the
// wrong arithmetic. Premultiplying at a vertex and then interpolating
// is not interpolating and then multiplying: it maps the rim to
// **black**, so what interpolates across the fin is black-to-white and
// the authored orange never exists anywhere on it. `SrcAlpha` does the
// same multiply *per fragment*, after the rasteriser has interpolated
// colour and alpha separately, which is what real hardware does and
// what leaves a dimmed orange fringe fading out along the fin.
//
// Measured against the first matched-pose pad capture (boost age
// `0.367` s, the original's own recorded camera and its measured
// entry intensity), over the magenta signature `r > 110, b > 110,
// r - g > 25, b - g > 20` in a crop around the craft:
//
// | build | pixels | mean colour | `b - r` |
// | --- | ---: | --- | ---: |
// | the original | 9,217 | `(223, 164, 224)` | +1 |
// | premultiplied, `TRAIL_BLEND` | 493 | `(180, 141, 228)` | +48 |
// | raw, `BLEND` (this) | 1,993 | `(159, 118, 175)` | +16 |
//
// Extent 4x better and the red/blue balance three times closer to the
// original's neutral. The premultiplied build was blue-dominant; raw
// alpha alone recovers the hue but draws the fins as hard-edged solid
// wedges, which is the 2026-08-07 and 2026-08-08 result reproduced
// exactly, and `SrcAlpha` is what supplies the falloff those wedges are
// missing. Still 22% of the original's extent - but that comparison is
// against the original's frame, so it is one of the readings the
// 2026-08-09 projection finding invalidates: our whole image is zoomed
// 1.12-1.26x depending on speed, because the original widens its fov
// with speed and we do not. Re-take it with `--camera-fov` before
// leaning on the number. The missing bright-pass is unaffected and
// still real; "a craft drawn too large" was the wrong half and is
// withdrawn - see `docs/rendering/projection-vs-the-original.md`.
//
// **This is an empirical approximation and the real mechanism is
// something else.** The GE state for the transparent mesh pass is now
// read: `GU_ALPHA_TEST` is *disabled*, `GU_BLEND` is *enabled* with
// `GU_FIX` white on both sides, and nothing in the pass scales fragment
// RGB but vertex colour x texture x the blend - so no source-alpha
// weight exists on the hardware at all. What supplies the original's
// falloff is the *texture*: the pass sets `TEXMAPMODE` uvgen 2,
// environment mapping from the vertex normal and lights 0/1, and
// `pulse_boost2_ADD`'s own bright-to-dark gradient varies across the
// fin. Every plume batch declares normals (`vtype = 0x013d` on all 32
// across 8 teams - `cargo run -p oag-assets --example
// boost_vertex_type`), so that generation has something to vary with.
//
// **Environment-mapped UV generation was implemented here 2026-08-09
// and replaced 2026-08-10**: the plume's compiled list is replayed
// under `TEXMAPMODE` 0 (settled by reading the recorded frame stream
// in GE order - mesh-draw.md, "The plume is replayed under
// `TEXMAPMODE` 0"), so the original samples the *authored* UVs
// through the keyframed `TEXOFFSET` u-scroll authored in the file
// itself. `Drawable::apply_uv_transform` now does exactly that;
// `oag_render::texgen` remains correct for batches genuinely inside
// the transparent-pass bracket, which the plume is not.
//
// **`SrcAlpha` stays, and it is no longer a stand-in - it is a
// measured choice that beat the recovered alternative.** The argument
// for going back to `TRAIL_BLEND` was strong on paper: the recovered
// GE state really is `GU_FIX` white on both sides, `SrcAlpha` was only
// ever introduced as a substitute for the missing texture falloff, and
// that falloff now exists. So it was tried, at the original's own pose
// (`data/traces/pad0-boost.csv` tick 62), against the original's own
// frame (the original's frame at that tick), over the capture's own
// plume mask (`min(r, b) - g > 25` and `luma > 60`):
//
// | build | plume px | mean | `b - r` | orange px |
// | --- | ---: | --- | ---: | ---: |
// | the original | 9,435 | `(220, 165, 237)` | +17.6 | 230 |
// | ours, authored UVs (what shipped before) | 4,480 | `(184, 143, 203)` | +19.0 | 2,679 |
// | **ours, texgen + `SrcAlpha` (this)** | **5,996** | `(197, 150, 210)` | +13.3 | **2,041** |
// | ours, texgen + `TRAIL_BLEND` (the recovered blend) | 1,958 | `(243, 181, 227)` | -15.6 | 7,030 |
//
// **The measurements live in
// `docs/ghidra/functions/psp-pulse-usa/exhaust.md`**, not here. This
// table was duplicated across three files and re-measured four times -
// wrong pose age, a boost accumulator charging 100x too slowly,
// ADR-0020 moving background luminance 54 %, and finally a mask that
// gated on absolute brightness - and every duplicate drifted. One home.
//
// What survives all four, and is why this line reads `exhaust::BLEND`:
// the recovered `One`/`One` blend is the worst row on every column,
// restoring the authored `(255, 98, 5)` rim at full strength where the
// original's plume and ribbon are one violet family with nothing near
// that orange. Generated coordinates beat authored ones on every
// column too.
//
// **So something in the recovered blend chain is still incomplete**,
// and that is worth stating rather than papering over: the GE state
// was read exhaustively - all five setters to their command byte,
// `Gu_TexFunc` confirmed `MODULATE`/`TCC_RGBA` - and it does not
// reproduce the picture, while an unrecovered source-alpha weight does.
// `SrcAlpha` is kept because it measures better, not because it is
// understood.
//
// **Do not read any column stated against the original as calibrated.**
// This comment used to blame a craft that "renders about 1.4x too
// large"; that is refuted - the mesh is correct to 0.15 % and the whole
// frame is zoomed, because the original widens its fov with speed and
// we do not. At this capture's 148.9 units/s that is roughly 1.25x, and
// it shifts which of the original's pixels fall inside the mask. The
// ours-versus-ours results above are unaffected: all three builds
// render at the same pose, so they share one zoom and a ratio between
// them cancels it. See `docs/rendering/projection-vs-the-original.md`,
// `docs/ghidra/functions/psp-pulse-usa/exhaust.md` and `mesh-draw.md`.
//
// One per grid slot, cloned the way the hulls above are: every craft can
// be on a speed pad at once, and eight plumes need eight model matrices
// and eight sampled UV transforms a frame.
//
// **Each slot's own team's plume, sampled through its own team's
// keyframes.** Eight per-team plumes played through one team's UV
// track is the kind of wrong that renders plausibly, so the transform
// is carried per slot beside the model. A slot whose team ships no
// plume simply has none, which is why this is a `Vec` of `Option`
// rather than a shorter `Vec` - the draw loop indexes by slot.
/// [`build`]'s own return: the boost plume's drawables, each slot's sampled
/// UV transform, and the engine flare's drawables, in that order.
type BoostAndFlares = (
    Vec<Option<Drawable>>,
    Vec<Option<oag_vex::vex::TexTransform>>,
    Vec<Option<Drawable>>,
);

#[allow(clippy::too_many_arguments)]
pub(super) fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    liveries: &[oag_livery::Livery],
    format: wgpu::TextureFormat,
    anisotropy: Anisotropy,
    sample_count: u32,
    scene_depth: mesh_render::Depth,
    zone_art: &mesh_render::zone::StageArt,
    shadow_maps: mesh_render::ShadowMaps<'_>,
) -> Result<BoostAndFlares> {
    let mut boost = Vec::new();
    let mut boost_uv_transforms = Vec::new();
    let mut flares = Vec::new();
    for slot in 0..GRID_SLOTS as usize {
        let livery = &liveries[slot.min(liveries.len().saturating_sub(1))];
        boost_uv_transforms.push(livery.boost_uv.clone());
        // **The material's own equation reaches the pipeline by itself.**
        // Every draw of an HD flare carries the factor pair its
        // `.rcsmaterial` authors (`SrcAlpha`/`One`), and
        // `mesh_render::TransparentPipelines::select` prefers that over any
        // class - so this argument is the fallback for a model that
        // authored none, and it is the same equation either way.
        //
        // **`GlowMask::Protected`, unlike the plume's.** The plume's
        // `Written` is a *recovered* override - the original's draw path
        // opens the alpha channel there. Nothing has been read out of HD's
        // executable about its flare, so it takes the default every
        // authored batch gets. It still reaches HD's bloom through that
        // chain's luminance term, which is not a substitute for the
        // reading and is not being treated as one.
        flares.push(
            match livery
                .flare
                .as_ref()
                .filter(|model| !model.indices.is_empty())
            {
                Some(model) => Some(Drawable::new_attached(
                    device,
                    queue,
                    model.clone(),
                    format,
                    anisotropy,
                    sample_count,
                    scene_depth,
                    mesh_render::ADDITIVE_BLEND,
                    mesh_render::GlowMask::Protected,
                    zone_art,
                    shadow_maps,
                    mesh_render::ShadowReceiver::Mapped,
                )?),
                None => None,
            },
        );
        let Some(model) = livery
            .boost
            .as_ref()
            .filter(|model| !model.indices.is_empty())
        else {
            boost.push(None);
            continue;
        };
        {
            boost.push(Some(Drawable::new_attached(
                device,
                queue,
                model.clone(),
                format,
                anisotropy,
                sample_count,
                scene_depth,
                exhaust::BLEND,
                // **`Written` reaches the PS2 plume alone**, whose `0xc0`
                // batches stamp the stencil. A Pulse PSP plume stamps as
                // measured instead (`Model::stamps_glow` overrides this):
                // transparent and without `0xc0`, it writes no mask at all.
                // See `mesh_render::GlowMask` and docs/rendering/glow-mask.md.
                mesh_render::GlowMask::Written,
                zone_art,
                shadow_maps,
                mesh_render::ShadowReceiver::Mapped,
            )?));
        }
    }
    Ok((boost, boost_uv_transforms, flares))
}
