//! Building a texture's mip chain on the CPU.
//!
//! Split out of `mesh_render.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

/// Downsamples `rgba` into a full mip chain by repeated 2x2 box filtering,
/// down to a 1x1 level.
///
/// Track and ship textures are seen at every distance and grazing angle a
/// chase camera produces; without mips, minification aliases into shimmer
/// that a single sample can't fix. Filtering happens on `Rgba8UnormSrgb`
/// sample data, matching what the sampler itself blends between levels.
pub(super) fn mip_chain(width: u32, height: u32, rgba: &[u8]) -> Vec<(u32, u32, Vec<u8>)> {
    let mut levels: Vec<(u32, u32, Vec<u8>)> = vec![(width, height, rgba.to_vec())];
    loop {
        let (w, h, data) = levels.last().expect("levels is never empty");
        let (w, h) = (*w, *h);
        if w == 1 && h == 1 {
            break;
        }
        let next_width = (w / 2).max(1);
        let next_height = (h / 2).max(1);
        let mut next = vec![0u8; (next_width * next_height * 4) as usize];
        for y in 0..next_height {
            let y0 = (y * 2).min(h - 1);
            let y1 = (y * 2 + 1).min(h - 1);
            for x in 0..next_width {
                let x0 = (x * 2).min(w - 1);
                let x1 = (x * 2 + 1).min(w - 1);
                for c in 0..4usize {
                    let texel = |sx: u32, sy: u32| data[((sy * w + sx) * 4) as usize + c] as u32;
                    let sum = texel(x0, y0) + texel(x1, y0) + texel(x0, y1) + texel(x1, y1);
                    next[((y * next_width + x) * 4) as usize + c] = ((sum + 2) / 4) as u8;
                }
            }
        }
        levels.push((next_width, next_height, next));
    }
    levels
}
