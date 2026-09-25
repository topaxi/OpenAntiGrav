//! Where an inline stride-18 chunk keeps its `Uv1` when its last four bytes
//! are a colour - split out of [`super`] under the 1,000-line rule, as
//! [`super::stride`] is: one `impl Mesh` block about one question.

use super::{Mesh, NORMAL_OFFSET, SubMesh, TexcoordFormat};

/// The one inline stride whose tail may be a colour rather than a coordinate
/// - see [`Mesh::texcoords`].
pub(super) const STRIDE: usize = 18;

/// Where `Uv1` sits when it does: right after the packed normal.
const OFFSET: usize = NORMAL_OFFSET + 4;

impl Mesh {
    /// An inline stride-18 chunk whose last four bytes are **not** two halves
    /// (`ff ff ff cc` on the LeachBall's sphere, which is `NaN` twice) reads
    /// its `Uv1` from `+0x0a` instead, right after the normal.
    ///
    /// **Why `+0x0a`, and why only then.** Declared stride-18 chunks carry
    /// `Uv1` and a four-byte `VertexColour1` in **both** orders (834 put
    /// `Uv1` at `+0x0a`, 554 at `+0x0e`; neither the material's own
    /// attribute slots nor anything else in the chunk predicts which), so an
    /// inline chunk's layout cannot be looked up. What the data can settle is
    /// which four bytes are *not* a coordinate: a vertex whose tail reads
    /// non-finite is carrying a colour there, and the one other four-byte
    /// field of an 18-byte vertex is the coordinate. Measured disc-wide with
    /// `crates/render/examples/hd_unlit_probe.rs --undeclared`: 62 inline
    /// stride-18 chunks read a non-finite tail on some vertex *and* a finite
    /// `+0x0a` on every vertex (the Plasma ball's and halo's shells, the
    /// LeachBall's sphere, `frontendscene_hd_atg`'s 43 chunks among them);
    /// 12 more read non-finite in both places and keep the tail read, and
    /// the 118 whose tail is finite everywhere are untouched. See
    /// `docs/rendering/hd-unlit-programs.md`.
    pub(super) fn inline_uv_before_colour(
        &self,
        data: &[u8],
        submesh: &SubMesh,
        stride: usize,
        tail: Vec<[f32; 2]>,
    ) -> Vec<[f32; 2]> {
        let finite = |c: &[f32; 2]| c[0].is_finite() && c[1].is_finite();
        if tail.iter().all(finite) {
            return tail;
        }
        match self.coords_at(data, submesh, stride, OFFSET, TexcoordFormat::Half) {
            Ok(early) if early.iter().all(finite) => early,
            _ => tail,
        }
    }
}
