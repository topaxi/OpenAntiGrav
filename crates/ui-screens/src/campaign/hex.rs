//! Where a hex widget sits on `Grid Selection`/`Cell Selection`.

use oag_ui::frontend::Placed;
use oag_ui::screen::{Image, Screen};

/// The resolved screen rect of `Outline_{x}_{y}` (or `Medal_{x}_{y}`,
/// wherever a slot has no base outline of its own) - position and size, the
/// same rect [`pointer::hit`] tests against. **`Outline_` first, not
/// `Medal_`**: Pulse authors no explicit width or height for a hex either
/// way, since both `hex_filled.mip`/`hex_outline.mip` are the same
/// single-hex 32x32 size, so the two prefixes were interchangeable there.
/// HD's own `Medal_{x}_{y}` breaks that assumption - it sources
/// `Hexmedal_HD.mip`, a shared multi-colour atlas (1024x256, not one hex),
/// authors no width/height of its own either, and every grid slot in HD's
/// template carries a `Medal_{x}_{y}` widget regardless of whether that
/// cell actually has a medal. Trying `Medal_` first therefore returned the
/// whole atlas's size as "the hex's rect" for every occupied HD cell, which
/// sent `Selector`'s own centred position (`centred_selector_draw`) far
/// from the hex it was meant to mark - the stray floating outline
/// `docs/ui/campaign-screens.md`'s "An open cell-grid artifact" section
/// found. `Outline_` is always present and always single-hex-sized on both
/// titles, so it is the reliable source of a hex's own size; `Medal_` stays
/// as a fallback for the same reason it was tried at all.
pub(crate) fn hex_rect(
    screen: &Screen,
    x: usize,
    y: usize,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Option<[f32; 4]> {
    hex_image(screen, x, y, sprites).map(|(_, rect, _)| rect)
}

/// [`hex_rect`]'s widget and its sprite's placement alongside the rect, for a
/// caller that needs to know which image the rect came from.
pub(crate) fn hex_image<'a>(
    screen: &'a Screen,
    x: usize,
    y: usize,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Option<(&'a Image, [f32; 4], Placed)> {
    for prefix in ["Outline_", "Medal_"] {
        let name = format!("{prefix}{x}_{y}");
        if let Some(image) = screen
            .images
            .iter()
            .find(|image| image.name.as_deref() == Some(name.as_str()))
            && let Some(placed) = sprites(&image.src)
        {
            let width = image.width.unwrap_or(placed.width as f32);
            let height = image.height.unwrap_or(placed.height as f32);
            return Some((image, [image.x, image.y, width, height], placed));
        }
    }
    None
}
