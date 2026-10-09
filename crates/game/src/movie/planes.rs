//! Turning what a browser's decoder hands out into the I420 planes
//! `video.wesl` reads. Used by `webcodecs.rs`; plain functions so they are
//! tested natively.

use oag_video::av1::Geometry;

/// Splits an `NV12` frame's interleaved chroma (`U V U V ...`) into the two
/// planes I420 has, appended to `out` after the luma.
pub(super) fn split_nv12(uv: &[u8], out: &mut Vec<u8>) {
    out.extend(uv.iter().step_by(2));
    out.extend(uv.iter().skip(1).step_by(2));
}

/// The YCbCr matrix a browser converted a frame to RGB with, read off the
/// frame's own `colorSpace.matrix`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Matrix {
    /// BT.601 (`smpte170m`, `bt470bg`), what the PSP's encoder produced and
    /// `video.wesl` draws with.
    Bt601,
    /// BT.709, which Firefox 155 converts the PSP's movies with (they signal no
    /// colour description of their own).
    Bt709,
}

impl Matrix {
    /// `colorSpace.matrix` as WebCodecs spells it; anything else is BT.601.
    pub(super) fn named(name: &str) -> Self {
        if name == "bt709" {
            Self::Bt709
        } else {
            Self::Bt601
        }
    }

    fn weights(self) -> (f32, f32) {
        match self {
            Self::Bt601 => (0.299, 0.114),
            Self::Bt709 => (0.2126, 0.0722),
        }
    }
}

/// Packed RGB (4 bytes a pixel, tightly packed rows) back into limited-range
/// I420 through the inverse of `matrix`, the one the browser converted with,
/// for a browser that hands out only RGB frames (Firefox 155 gives `BGRX`,
/// software decode or not). That recovers the stream's own samples, which
/// `video.wesl` then draws as BT.601 as natively. Close to the native frame,
/// not equal to it: the browser rounded and clipped to RGB, and chroma here is
/// the 2x2 average of what it upsampled.
pub(super) fn rgb_to_i420(
    packed: &[u8],
    g: Geometry,
    red_first: bool,
    matrix: Matrix,
    out: &mut Vec<u8>,
) {
    let (w, h) = (g.width as usize, g.height as usize);
    let (kr, kb) = matrix.weights();
    let kg = 1.0 - kr - kb;
    let rgb = |x: usize, y: usize| {
        let p = &packed[(y.min(h - 1) * w + x.min(w - 1)) * 4..][..3];
        let (r, b) = if red_first {
            (p[0], p[2])
        } else {
            (p[2], p[0])
        };
        (f32::from(r), f32::from(p[1]), f32::from(b))
    };
    let luma = |(r, gr, b): (f32, f32, f32)| kr * r + kg * gr + kb * b;
    let byte = |v: f32| v.round().clamp(0.0, 255.0) as u8;
    for y in 0..h {
        for x in 0..w {
            out.push(byte(16.0 + 219.0 / 255.0 * luma(rgb(x, y))));
        }
    }
    let mut cb = Vec::with_capacity(g.chroma_len());
    let mut cr = Vec::with_capacity(g.chroma_len());
    for cy in 0..g.chroma_height as usize {
        for cx in 0..g.chroma_width as usize {
            let (mut r, mut gr, mut b) = (0.0, 0.0, 0.0);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (pr, pg, pb) = rgb(cx * 2 + dx, cy * 2 + dy);
                (r, gr, b) = (r + pr / 4.0, gr + pg / 4.0, b + pb / 4.0);
            }
            let y = luma((r, gr, b));
            cb.push(byte(128.0 + 224.0 / 255.0 * (b - y) / (2.0 * (1.0 - kb))));
            cr.push(byte(128.0 + 224.0 / 255.0 * (r - y) / (2.0 * (1.0 - kr))));
        }
    }
    out.extend(cb);
    out.extend(cr);
}

/// An I420 frame's FNV-1a over the luma plane and the mean of each plane, for
/// comparing a browser's frame with the native decode's ("Movies" in
/// `docs/tools/web.md`).
pub(super) fn summary(i420: &[u8], g: Geometry) -> (u64, [f32; 3]) {
    let (luma, chroma) = (g.luma_len(), g.chroma_len());
    let planes = [
        &i420[..luma],
        &i420[luma..luma + chroma],
        &i420[luma + chroma..luma + 2 * chroma],
    ];
    let hash = planes[0].iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    let mean = |plane: &[u8]| {
        plane.iter().map(|&b| u64::from(b)).sum::<u64>() as f32 / plane.len().max(1) as f32
    };
    (hash, planes.map(mean))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nv12_chroma_splits_into_u_then_v() {
        let mut out = vec![9];
        split_nv12(&[1, 2, 3, 4, 5, 6], &mut out);
        assert_eq!(out, [9, 1, 3, 5, 2, 4, 6]);
    }

    #[test]
    fn rgb_black_and_white_come_back_at_limited_range() {
        let g = Geometry::new(2, 2);
        let mut white_black = Vec::new();
        for value in [255u8, 0, 255, 0] {
            white_black.extend_from_slice(&[value, value, value, 0]);
        }
        let mut out = Vec::new();
        rgb_to_i420(&white_black, g, false, Matrix::Bt709, &mut out);
        assert_eq!(
            out,
            [235, 16, 235, 16, 128, 128],
            "luma, then neutral chroma"
        );
    }

    #[test]
    fn rgb_chroma_is_the_two_by_two_average_and_honours_channel_order() {
        // One pure-red pixel and three black ones: chroma sees a quarter red.
        let g = Geometry::new(2, 2);
        let mut bgrx = vec![0u8; 16];
        bgrx[2] = 255;
        let mut rgbx = vec![0u8; 16];
        rgbx[0] = 255;
        let (mut from_bgrx, mut from_rgbx) = (Vec::new(), Vec::new());
        rgb_to_i420(&bgrx, g, false, Matrix::Bt601, &mut from_bgrx);
        rgb_to_i420(&rgbx, g, true, Matrix::Bt601, &mut from_rgbx);
        assert_eq!(from_bgrx, from_rgbx);
        assert_eq!(from_rgbx[0], 81, "red's luma, 16 + 0.256788 * 255");
        // 128 - 0.148223 * 63.75 and 128 + 0.439216 * 63.75.
        assert_eq!(&from_rgbx[4..], [119, 156]);
    }

    #[test]
    fn the_matrix_the_browser_named_is_the_one_inverted() {
        let g = Geometry::new(2, 2);
        let red = [255u8, 0, 0, 0].repeat(4);
        let mut out = Vec::new();
        rgb_to_i420(&red, g, true, Matrix::named("bt709"), &mut out);
        // 16 + 219/255 * 0.2126 * 255, and red's Cr is the top of the range.
        assert_eq!(out[0], 63);
        assert_eq!(out[5], 240);
        assert_eq!(Matrix::named("smpte170m"), Matrix::Bt601);
    }

    #[test]
    fn a_summary_hashes_the_luma_and_averages_each_plane() {
        let g = Geometry::new(2, 2);
        let (hash, means) = summary(&[10, 20, 30, 40, 128, 64], g);
        assert_eq!(means, [25.0, 128.0, 64.0]);
        let (other, _) = summary(&[10, 20, 30, 41, 128, 64], g);
        assert_ne!(hash, other);
    }
}
