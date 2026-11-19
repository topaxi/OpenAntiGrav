//! The `Dynamic HUD` warp on a renderer's quads: [`Warp`] says which draws bend and which
//! bars are cut into columns first; the law is `oag_hud::concave`'s.

use super::quad::{MODE_TRIANGLE, Quad};

/// One bent draw list, spent by the next [`super::Renderer::overlay`] the way the scissors are.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Warp {
    /// The rectangles of the sprites drawn as [`oag_hud::concave::COLUMNS`] columns
    /// ([`oag_hud::Frame::columns`]); every other sprite bends at its four corners.
    pub columns: Vec<[f32; 4]>,
    /// The draws from this index on are drawn flat: the lock-on reticle, which the original
    /// draws as a model and its warp never reaches.
    pub until: usize,
    /// How far the middle column's edges move, in grid units (`oag_hud::concave::amplitude_for`).
    pub amplitude: f32,
}

impl Warp {
    /// Whether a sprite at `rect` is drawn as columns.
    pub(super) fn columned(&self, rect: [f32; 4]) -> bool {
        self.columns.contains(&rect)
    }
}

/// `quad` cut into the warp's columns, each a share of its sheet window.
pub(super) fn columns(quad: Quad) -> impl Iterator<Item = Quad> {
    oag_hud::concave::columns(quad.rect, quad.uv)
        .into_iter()
        .map(move |(rect, uv)| Quad { rect, uv, ..quad })
}

/// Sets every quad's corner offsets on a grid of `size`. A triangle packs other data into its
/// fields and a turned quad's corners are not its rect's, so both stay flat; the HUD draws
/// neither but the reticle.
pub(super) fn bend(quads: &mut [Quad], size: (f32, f32), amplitude: f32) {
    for quad in quads {
        if quad.mode < MODE_TRIANGLE - 0.5 && quad.rotation == 0.0 {
            quad.warp = oag_hud::concave::corners(quad.rect, size, amplitude);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::quad::MODE_SPRITE;

    fn sprite(rect: [f32; 4]) -> Quad {
        Quad {
            rect,
            uv: [0.0, 0.0, 64.0, 8.0],
            color: [1.0; 4],
            border: [1.0; 4],
            mode: MODE_SPRITE,
            rotation: 0.0,
            chamfer: [0.0; 2],
            tile: [0.0; 2],
            warp: [0.0; 4],
        }
    }

    #[test]
    fn a_bent_quad_carries_the_law_at_its_corners_and_a_turned_one_stays_flat() {
        let size = (480.0, 272.0);
        let mut quads = [sprite([300.0, 240.0, 140.0, 8.0]), {
            let mut turned = sprite([200.0, 100.0, 8.0, 8.0]);
            turned.rotation = 0.5;
            turned
        }];
        bend(&mut quads, size, 35.0);
        assert_eq!(
            quads[0].warp,
            oag_hud::concave::corners([300.0, 240.0, 140.0, 8.0], size, 35.0)
        );
        assert!(quads[0].warp[0] < -10.0, "the bottom half rises");
        assert_eq!(quads[1].warp, [0.0; 4]);
    }

    #[test]
    fn a_columned_sprite_is_four_quads_sharing_its_colour() {
        let cut: Vec<Quad> = columns(sprite([300.0, 240.0, 140.0, 8.0])).collect();
        assert_eq!(cut.len(), oag_hud::concave::COLUMNS);
        assert_eq!(cut[0].rect[0], 300.0);
        assert_eq!(cut[1].uv[0], 16.0);
        assert!(cut.iter().all(|q| q.color == [1.0; 4] && q.rect[2] == 35.0));
    }
}
