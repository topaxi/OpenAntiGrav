//! Where Pure's centred text starts.

use super::{Atlas, Cell};

/// Where the pen starts for `text` centred on `x` on Pure's snapped pen: `x` less half the string's
/// measure, less the first glyph's bearing, **not** rounded (the first glyph's own step rounds it).
///
/// The measure is the original's width function, not [`super::SnappedRun::end`]: it starts the pen
/// a bearing left of the origin, steps every glyph up to a whole unit as the draw does, and ends on
/// the last glyph's ink (`bearing + authored width`) rather than its advance
/// (`docs/ghidra/functions/psp-pure-usa/menu-text-centring.md`). The arithmetic keeps the
/// original's order, term by term in `f32`, because the `.5` ties of an odd measure at scale 1
/// decide the first column. No kerning: the face's pair lists are empty. Glyphs the atlas lacks
/// are left out, as the draw leaves them.
#[must_use]
#[expect(
    clippy::suboptimal_flops,
    reason = "the original multiplies and adds in separate steps; a fused add moves a tie"
)]
pub fn snapped_centre_pen(atlas: &Atlas, text: &str, scale: f32, x: f32) -> f32 {
    let scale = scale * atlas.texel_scale;
    let cells: Vec<Cell> = text.chars().filter_map(|c| atlas.cell(c)).collect();
    let (Some(first), Some(last)) = (cells.first(), cells.last()) else {
        return x;
    };
    let mut pen = 0.0 - first.bearing * scale;
    for cell in &cells[..cells.len() - 1] {
        pen = (pen + cell.bearing * scale).ceil();
        pen = (pen - cell.bearing * scale) + cell.advance * scale;
    }
    let measure = pen + (last.bearing + last.ink) * scale;
    (x - measure * 0.5) - first.bearing * scale
}
