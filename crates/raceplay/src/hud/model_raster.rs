//! A flat `<Mode3D>` model, rasterised on the CPU at the HUD's own 1 unit = 1 pixel, for the
//! sheet.
//!
//! Pure's speed and shield gauge is three polygon models, not sprites (`oag_title::BarModels`),
//! and the HUD's draw list is rectangles of a sheet. The original's own pass is an orthographic
//! one at 480x272 with no depth test, so a model there is exactly what this produces: every
//! triangle filled at pixel centres, the vertex colour multiplying the texel, the batch blended
//! over what is under it. Done once at load, the result is one more sheet image, and everything
//! downstream (placement, split views, the safe area, a bar's crop) is the sprite path as it was.
//!
//! **Orientation.** The block is orthographic with `y` counted up from the bottom of the
//! screen, so a model vertex `(x, y)` lands at `(x, -y)` from the widget's origin in the
//! y-down screen. That is why the outline's texture, stored cyan box above yellow box, reads
//! yellow above cyan in a frame of the original.
//!
//! **Chosen, not measured:** nearest texel sampling and clamped addressing. The clamp is read
//! off the data (a fill's texture is opaque on its left half and clear on its right, and slides
//! left by 5.0 texture widths over the model's own offset track, which only makes a fill if
//! the edge texel repeats), the nearest filter is not.

use oag_mesh::mesh::{DrawCall, Model};

/// A model drawn flat.
#[derive(Debug, Clone)]
pub(super) struct Raster {
    /// Canvas width in pixels.
    pub width: u32,
    /// Canvas height in pixels.
    pub height: u32,
    /// RGBA8888, straight alpha, row-major from the top left.
    pub rgba: Vec<u8>,
    /// Where the canvas's middle sits from the model's origin, y down.
    pub centre: [f32; 2],
}

/// The model's own animation clock at the end of its longest texture track: the state the
/// original's bar is in at a fraction of `1.0`.
pub(super) fn end_of_tracks(model: &Model) -> f32 {
    model
        .anim_tracks
        .iter()
        .filter_map(|track| match track {
            oag_mesh::mesh::AnimTrack::Psp(t) => {
                let unit = if t.seconds_per_key > 0.0 {
                    t.seconds_per_key
                } else {
                    1.0 / 60.0
                };
                let last = t.offset.times.iter().chain(&t.scale.times).copied().max()?;
                Some(f32::from(last) * unit)
            }
            _ => None,
        })
        .fold(0.0, f32::max)
}

/// The model's own animation clock at the key where its texture offset is furthest from the
/// start: for a Pure weapon icon, whose quad windows ten texels of a 128-wide strip, the state
/// that shows the glyph. The track runs `0 -> 217/256 -> 217/256 -> 0` over a second; the
/// original's frames of every held weapon show the `217/256` state, and the model at time 0
/// draws only the strip's empty leading texels.
pub(super) fn held_at(model: &Model) -> f32 {
    let mut best = (0.0f32, 0.0f32);
    for track in &model.anim_tracks {
        let oag_mesh::mesh::AnimTrack::Psp(t) = track else {
            continue;
        };
        let unit = if t.seconds_per_key > 0.0 {
            t.seconds_per_key
        } else {
            1.0 / 60.0
        };
        for (&time, value) in t.offset.times.iter().zip(&t.offset.values) {
            let far = f32::from(value.0).abs().max(f32::from(value.1).abs());
            if far > best.0 {
                best = (far, f32::from(time) * unit);
            }
        }
    }
    best.1
}

/// Draws `model` with its texture tracks sampled at `seconds`. `None` for a model with no
/// triangles, no decoded texture, or no extent.
pub(super) fn raster(model: &Model, seconds: f32) -> Option<Raster> {
    let texture = model.textures.iter().flatten().next()?;
    let texels = texture.rgba()?;
    let (tw, th) = (texture.width as usize, texture.height as usize);
    if tw == 0 || th == 0 || model.indices.len() < 3 {
        return None;
    }
    let tracks = oag_mesh::mesh_render::TexAnims::sample(model, seconds);

    let mut min = [f32::MAX; 2];
    let mut max = [f32::MIN; 2];
    for &index in &model.indices {
        let p = model.vertices.get(index as usize)?.position;
        let at = [p[0], -p[1]];
        for axis in 0..2 {
            min[axis] = min[axis].min(at[axis]);
            max[axis] = max[axis].max(at[axis]);
        }
    }
    let origin = [min[0].round() as i32, min[1].round() as i32];
    let end = [max[0].round() as i32, max[1].round() as i32];
    let (width, height) = ((end[0] - origin[0]) as usize, (end[1] - origin[1]) as usize);
    if width == 0 || height == 0 || width > 2048 || height > 2048 {
        return None;
    }
    let mut canvas = vec![0u8; width * height * 4];

    let lists = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ];
    for draw in lists.into_iter().flatten() {
        fill_draw(
            model,
            draw,
            &tracks,
            texels,
            (tw, th),
            origin,
            (width, height),
            &mut canvas,
        );
    }
    Some(Raster {
        width: width as u32,
        height: height as u32,
        rgba: canvas,
        centre: [
            origin[0] as f32 + width as f32 * 0.5,
            origin[1] as f32 + height as f32 * 0.5,
        ],
    })
}

#[allow(clippy::too_many_arguments)]
fn fill_draw(
    model: &Model,
    draw: &DrawCall,
    tracks: &oag_mesh::mesh_render::TexAnims,
    texels: &[u8],
    (tw, th): (usize, usize),
    origin: [i32; 2],
    (width, height): (usize, usize),
    canvas: &mut [u8],
) {
    let indices = &model.indices[draw.range.start as usize..draw.range.end as usize];
    for triangle in indices.as_chunks::<3>().0 {
        let v = [
            &model.vertices[triangle[0] as usize],
            &model.vertices[triangle[1] as usize],
            &model.vertices[triangle[2] as usize],
        ];
        let pos = v.map(|v| {
            [
                v.position[0] - origin[0] as f32,
                -v.position[1] - origin[1] as f32,
            ]
        });
        let area = (pos[1][0] - pos[0][0]) * (pos[2][1] - pos[0][1])
            - (pos[2][0] - pos[0][0]) * (pos[1][1] - pos[0][1]);
        if area == 0.0 {
            continue;
        }
        // The batch's own culling: a single-sided one drops the triangles facing away. A
        // clockwise or counter-clockwise front is not recovered for the HUD pass, so only the
        // winding the strips were authored with draws; see `Model` `culled`.
        if draw.culled && area > 0.0 {
            continue;
        }
        let lo_x = pos
            .iter()
            .map(|p| p[0])
            .fold(f32::MAX, f32::min)
            .floor()
            .max(0.0) as usize;
        let hi_x = (pos.iter().map(|p| p[0]).fold(f32::MIN, f32::max).ceil() as usize).min(width);
        let lo_y = pos
            .iter()
            .map(|p| p[1])
            .fold(f32::MAX, f32::min)
            .floor()
            .max(0.0) as usize;
        let hi_y = (pos.iter().map(|p| p[1]).fold(f32::MIN, f32::max).ceil() as usize).min(height);
        let uv = v.map(|v| {
            let [su, sv, ou, ov] =
                tracks.transform[(v.anim as usize).min(tracks.transform.len() - 1)];
            [v.texcoord[0] * su + ou, v.texcoord[1] * sv + ov]
        });
        for y in lo_y..hi_y {
            for x in lo_x..hi_x {
                // A hair off the pixel centre, so a pixel on the diagonal two triangles of a quad share
                // belongs to exactly one of them. On the exact centre both took it, which an opaque
                // gauge hides and a translucent backdrop shows as a dark seam.
                let p = [x as f32 + 0.5007, y as f32 + 0.5011];
                let w0 = ((pos[1][0] - p[0]) * (pos[2][1] - p[1])
                    - (pos[2][0] - p[0]) * (pos[1][1] - p[1]))
                    / area;
                let w1 = ((pos[2][0] - p[0]) * (pos[0][1] - p[1])
                    - (pos[0][0] - p[0]) * (pos[2][1] - p[1]))
                    / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let blend = |f: &dyn Fn(usize) -> f32| w0 * f(0) + w1 * f(1) + w2 * f(2);
                let u = blend(&|i| uv[i][0]);
                let tv = blend(&|i| uv[i][1]);
                let tx = ((u * tw as f32).floor() as i64).clamp(0, tw as i64 - 1) as usize;
                let ty = ((tv * th as f32).floor() as i64).clamp(0, th as i64 - 1) as usize;
                let texel = &texels[(ty * tw + tx) * 4..][..4];
                let colour: [f32; 4] = std::array::from_fn(|c| blend(&|i| v[i].colour[c]));
                let src: [f32; 4] =
                    std::array::from_fn(|c| f32::from(texel[c]) / 255.0 * colour[c]);
                over(&mut canvas[(y * width + x) * 4..][..4], src);
            }
        }
    }
}

/// `src` (straight RGBA, 0 to 1) over the pixel `dst`, straight alpha.
fn over(dst: &mut [u8], src: [f32; 4]) {
    let da = f32::from(dst[3]) / 255.0;
    let out_a = src[3] + da * (1.0 - src[3]);
    if out_a <= 0.0 {
        return;
    }
    for c in 0..3 {
        let d = f32::from(dst[c]) / 255.0;
        let value = (src[c] * src[3] + d * da * (1.0 - src[3])) / out_a;
        dst[c] = (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    }
    dst[3] = (out_a.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
}
