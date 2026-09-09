//! The application icon, rasterised from `assets/icons/64x64.svg`.
//!
//! One source, not a per-consumer redraw: `main/gpu.rs` builds a
//! [`winit::window::Icon`] from it for the titlebar/taskbar on X11 and
//! Windows, and `--write-icon` (see `main.rs`) writes it as a PNG for the
//! desktop entry `just install-desktop-file` installs and the one
//! `scripts/build-appimage.sh` packages. Before this module the AppImage's
//! own icon was a hand-drawn chevron with no connection to `assets/icons/`
//! at all, so a packaged build showed one design in the window and another
//! in the taskbar; see `docs/tools/packaging.md`.
//!
//! No winit type appears here on purpose: rasterising is pure data in, data
//! out, so it is testable without a window or a GPU, the same reason
//! `crate::capture` exists. `main/window.rs`'s `window_icon` is the only
//! place that touches [`winit::window::Icon`]; `main/gpu.rs` only calls it.

use resvg::{tiny_skia, usvg};

/// The vector source. 64x64 rather than the also-committed 16x16: that
/// smaller icon is a separately hand-tuned design for legibility at one
/// exact size (see its own file), not a scaled-down version of this one, and
/// winit's [`winit::window::Icon`] takes a single buffer - there is nowhere
/// to hand it a second size even if the two agreed.
const SOURCE: &[u8] = include_bytes!("../../../assets/icons/64x64.svg");

/// A rasterised RGBA8 icon, straight ARGB-free and premultiplied-free -
/// `winit::window::Icon::from_rgba` and `oag_texture::png::encode_rgba` both
/// want the same thing: `width * height * 4` bytes, row-major, unassociated
/// alpha.
#[derive(Debug)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Renders [`SOURCE`] at `size` pixels square.
///
/// The source is a vector, so any `size` re-renders cleanly - unlike
/// upscaling a raster, there is no interpolation to soften.
///
/// # Panics
///
/// If the committed SVG fails to parse, or `size` is zero. The first is a
/// programmer error. The second **is** reachable from the command line, via
/// `--icon-size`, which is why the caller in `main` validates it and reports
/// it as a CLI error - this claimed no player input reaches it, and one did
/// (finding G2 of the 2026-08-18 review).
#[must_use]
pub fn rasterize(size: u32) -> Rgba {
    let tree = usvg::Tree::from_data(SOURCE, &usvg::Options::default())
        .expect("assets/icons/64x64.svg failed to parse");

    let mut pixmap =
        tiny_skia::Pixmap::new(size, size).expect("--write-icon/--icon-size must be non-zero");

    let source = tree.size();
    let scale = size as f32 / source.width().max(source.height());
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Rgba {
        width: size,
        height: size,
        pixels: pixmap.take(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterizes_to_the_requested_size() {
        let icon = rasterize(64);
        assert_eq!(icon.width, 64);
        assert_eq!(icon.height, 64);
        assert_eq!(icon.pixels.len(), 64 * 64 * 4);
    }

    #[test]
    fn the_icon_is_not_blank() {
        // A parse or transform mistake that renders nothing produces an
        // all-zero (fully transparent) buffer, which would otherwise still
        // pass "right length" - so the regression this test actually guards
        // is the ring's own pixels reaching the buffer, not just its shape.
        let icon = rasterize(64);
        let opaque = icon.pixels.as_chunks::<4>().0.iter().any(|p| p[3] > 0);
        assert!(opaque, "rasterized icon has no opaque pixel");
    }

    #[test]
    fn a_different_size_scales_cleanly() {
        let icon = rasterize(256);
        assert_eq!(icon.width, 256);
        assert_eq!(icon.pixels.len(), 256 * 256 * 4);
    }
}
