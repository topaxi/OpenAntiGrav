//! The pointer drawn over a title's screens, from `assets/cursors/*.svg`.
//!
//! **Ours, every one.** No title in this lineage was authored for a mouse,
//! so there is no cursor on any disc to recover; each SVG is drawn in its
//! title's own menu palette and says so in its own comment - see
//! `assets/cursors/README.md` for the table. [`oag_title::Title::cursor`]
//! names which one a title gets, and [`LAUNCHER`] is this build's own for
//! the one screen that runs before a title is known.
//!
//! # Drawn by the frame, not handed to the window system
//!
//! The cursor is a [`Draw::Sprite`] the frame loop paints last, at the
//! pointer's position, over whatever stage is up - not a
//! `winit::window::CustomCursor`. Two reasons. A window-system cursor is
//! whatever the compositor decides to show, which over a borderless
//! fullscreen game on a handheld's compositor is nothing at all, and a
//! player with no visible pointer has no way to find out the menus answer
//! one. And drawing it ourselves is what lets it vanish exactly when it
//! means nothing - during a race, or once a finger has lifted - rather than
//! whenever the platform feels like hiding it. The window's own cursor is
//! hidden over the game for the same reason a second one would be worse
//! than none: see `main/gpu.rs`.
//!
//! # One geometry, five palettes
//!
//! Every SVG is a 24x32 box with the hotspot at its top-left corner, drawn
//! [`WIDTH`] by [`HEIGHT`] units of the PSP grid the overlay renderer works
//! in - the same grid the performance overlay is authored in, so the cursor
//! is the same size on screen whatever title is up, and about the size of a
//! menu row's capital. Rasterised once per title at [`RASTER_HEIGHT`], which
//! is more pixels than any window shorter than 4K will draw it with.

use resvg::{tiny_skia, usvg};

use oag_ui::frontend::Draw;

use oag_hud::sprite::{DecodedImage, Sheet};

/// The disc chooser's cursor: this build's own, in the window icon's colours.
pub const LAUNCHER: &str = include_str!("../../../assets/cursors/launcher.svg");

/// The sheet entry the cursor is looked up by.
const SRC: &str = "cursor";

/// How wide the cursor is on screen, in PSP grid units.
pub const WIDTH: f32 = 12.0;
/// How tall, in the same units: the SVGs are 24x32, drawn at half.
pub const HEIGHT: f32 = 16.0;
/// The rasterised height in pixels; the width follows the SVG's own aspect.
pub const RASTER_HEIGHT: u32 = 64;

/// A sprite sheet holding nothing but `svg`, rasterised, keyed by [`SRC`].
///
/// Built through the same [`Sheet::build_with`] every disc image goes
/// through, so the renderer treats the cursor exactly as it treats a
/// front-end image. Straight alpha, which is what the sheet stores:
/// `tiny_skia` renders premultiplied, and handing that over as-is would
/// darken every anti-aliased edge by its own coverage.
///
/// # Panics
///
/// If `svg` fails to parse. The five sources are committed and tested, so
/// this is a programmer error rather than a runtime one.
#[must_use]
pub fn sheet(svg: &str) -> Sheet {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())
        .expect("a committed cursor SVG failed to parse");
    let source = tree.size();
    let scale = RASTER_HEIGHT as f32 / source.height();
    let width = (source.width() * scale).ceil().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, RASTER_HEIGHT).expect("a positive raster size");
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let straight = pixel.demultiply();
            [
                straight.red(),
                straight.green(),
                straight.blue(),
                straight.alpha(),
            ]
        })
        .collect();
    // The report is the sheet's own inventory line ("image cursor: 48x64,
    // 32bpp"), which a boot prints for a disc's images and nobody needs for
    // one committed SVG.
    let mut report = Vec::new();
    // The on-screen controls' generated shapes ride in the same sheet: the
    // overlay renderer draws sprites from this one and no other.
    let mut images = vec![DecodedImage {
        src: SRC.to_string(),
        width,
        height: RASTER_HEIGHT,
        rgba,
        quad_extent: None,
        blend: None,
    }];
    images.extend(crate::touch_controls::art::images());
    Sheet::build_with(&[], images, &mut report)
}

/// The cursor at `at`, its hotspot on the point, for a renderer whose
/// sheet is [`sheet`]'s.
///
/// `None` when the sheet has no cursor in it, which is only a sheet built
/// some other way - so a renderer that was never given one draws nothing
/// rather than a texel of whatever it does hold.
#[must_use]
pub fn draw(sheet: &Sheet, at: (f32, f32)) -> Option<Draw> {
    let placed = sheet.get(SRC)?;
    Some(Draw::Sprite {
        rect: [at.0, at.1, WIDTH, HEIGHT],
        uv: [
            placed.x as f32,
            placed.y as f32,
            placed.width as f32,
            placed.height as f32,
        ],
        color: [1.0, 1.0, 1.0, 1.0],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every committed cursor parses, rasterises at the shared geometry,
    /// and has something opaque at its hotspot corner - a tip that is
    /// transparent would be a cursor pointing at nothing.
    #[test]
    fn every_title_cursor_rasterises_with_an_opaque_tip() {
        let sources = [
            ("launcher", LAUNCHER),
            ("pulse", oag_pulse::TITLE.cursor),
            ("pure", oag_pure::TITLE.cursor),
            ("hd", oag_hd::TITLE.cursor),
            ("2048", oag_2048::TITLE.cursor),
        ];
        for (name, svg) in sources {
            let sheet = sheet(svg);
            let placed = sheet.get(SRC).unwrap_or_else(|| panic!("{name}: placed"));
            assert_eq!(placed.height, RASTER_HEIGHT, "{name}");
            assert_eq!(
                placed.width,
                RASTER_HEIGHT * 3 / 4,
                "{name}: 24x32 at 64 high"
            );
            // Two pixels in from the corner, inside the stroke.
            let alpha = sheet
                .alpha_at(
                    placed,
                    2.5 / placed.width as f32,
                    2.5 / placed.height as f32,
                )
                .unwrap_or_else(|| panic!("{name}: alpha"));
            assert!(alpha > 0.5, "{name}: the tip is see-through ({alpha})");
            // And the far corner is clear: the arrow does not fill its box.
            let alpha = sheet.alpha_at(placed, 0.97, 0.97).unwrap();
            assert!(alpha < 0.05, "{name}: the far corner is painted ({alpha})");
        }
    }

    #[test]
    fn the_sprite_puts_its_hotspot_on_the_point() {
        let sheet = sheet(LAUNCHER);
        let Some(Draw::Sprite { rect, .. }) = draw(&sheet, (100.0, 50.0)) else {
            panic!("a sprite");
        };
        assert_eq!(rect, [100.0, 50.0, WIDTH, HEIGHT]);
        assert!(draw(&Sheet::default(), (0.0, 0.0)).is_none());
    }
}
