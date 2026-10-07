//! Where an inline stride-18 or stride-22 chunk keeps its `Uv1` when its last
//! four bytes are a colour - split out of [`super`] under the 1,000-line rule, as
//! [`super::stride`] is: one `impl Mesh` block about one question.

use super::{Error, Mesh, NORMAL_OFFSET, Result, SubMesh, TexcoordFormat};

/// The inline strides whose tail may be a colour rather than a coordinate
/// - see [`Mesh::texcoords`]. Stride 22 joined 18 on 2026-10-07: a stride-22
/// vertex that ends in **two** colours (`ff 9f 00 4c` twice on
/// `hd_bomb_shockwaves`) keeps `Uv1` at `+0x0a` all the same, which the
/// material's vertex program confirms (`o[TC3].xy = v[2].xy`, `v[3]` and
/// `v[4]` the two colours).
pub(super) const STRIDES: [usize; 2] = [18, 22];

/// The stride [`Mesh::inline_colours`] reads, which only the stride-18 flyer
/// needs.
const STRIDE: usize = 18;

/// Where `Uv1` sits when it does: right after the packed normal.
const OFFSET: usize = NORMAL_OFFSET + 4;

impl Mesh {
    /// An inline stride-18 (or, since 2026-10-07, stride-22) chunk whose last
    /// four bytes are **not** two halves
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
    ///
    /// **Stride 22, same census**: 4 inline chunks read non-finite in the tail
    /// and finite at `+0x0a` (`hd_bomb_shockwaves`, both of `hd_plasma_ring`'s
    /// and `hd_missile_explosion`'s), 1 more is finite in both and 10 are
    /// finite in neither - those 11 keep the tail read.
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

impl Mesh {
    /// The four colour bytes an inline stride-18 vertex ends in, `R G B A`, one
    /// per vertex.
    ///
    /// **Read only for the chunk that says what it carries there.** `00_flyer`'s
    /// two surfaces are the worked case: `card_reflectShape`'s tail is `ff ff
    /// ff 4c` at the card's bottom edge, `ff ff ff 26` a third of the way down
    /// and `ff ff ff 00` at the bottom - an alpha ramp, the reflection's fade -
    /// and `cardShape`'s is `ff ff ff ff` on its front face and `70 70 70 ff`
    /// on some of its edge geometry. Nothing here says the last four bytes are
    /// a colour on *every* inline chunk (see [`Self::inline_uv_before_colour`]:
    /// they are two halves on most), so this answers the bytes and leaves the
    /// meaning to a caller that knows the chunk.
    ///
    /// # Errors
    ///
    /// [`Error::OutOfBounds`] when the buffer leaves the file, and
    /// [`Error::UnknownChunkLayout`] for a described chunk, whose colours a
    /// declaration places and this does not.
    pub fn inline_colours(&self, data: &[u8], submesh: &SubMesh) -> Result<Vec<[u8; 4]>> {
        if self.decl.is_some() {
            return Err(Error::UnknownChunkLayout { got: 0x05, at: 0 });
        }
        let end = submesh.vertex_offset + STRIDE * submesh.vertex_count;
        if end > data.len() {
            return Err(Error::OutOfBounds {
                what: "a vertex buffer",
                end,
                len: data.len(),
            });
        }
        Ok((0..submesh.vertex_count)
            .map(|k| {
                let at = submesh.vertex_offset + k * STRIDE + STRIDE - 4;
                [data[at], data[at + 1], data[at + 2], data[at + 3]]
            })
            .collect())
    }
}

impl Mesh {
    /// The two colour fields an inline stride-22 vertex ends in, folded the way
    /// `hd_bombfire_shockwaves_glow`'s vertex program reads them: the first
    /// field's `R G B` (`o[TC0].xyz = v[3].xyz`) and the second field's `A`
    /// (`o[TC3].z = v[4].w`), each over 255, as `[r, g, b, a]`.
    ///
    /// The fields sit at `+0x0e` and `+0x12`, after the normal and the `Uv1`
    /// [`Self::inline_uv_before_colour`] finds at `+0x0a`. **Only for a chunk
    /// whose tail cannot be a coordinate** - the same test that moves its
    /// `Uv1` - so a stride-22 chunk carrying a real tail keeps its meaning;
    /// that is `hd_bomb_shockwaves` and nothing else a caller routes here.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownChunkLayout`] for a described chunk or another stride,
    /// [`Error::NoTexcoord`] when the tail is a coordinate after all, and
    /// [`Error::OutOfBounds`] when the buffer leaves the file.
    pub fn inline_two_colours(
        &self,
        data: &[u8],
        submesh: &SubMesh,
        stride: usize,
    ) -> Result<Vec<[f32; 4]>> {
        if self.decl.is_some() || stride != 22 {
            return Err(Error::UnknownChunkLayout { got: 0x05, at: 0 });
        }
        let end = submesh.vertex_offset + stride * submesh.vertex_count;
        if end > data.len() {
            return Err(Error::OutOfBounds {
                what: "a vertex buffer",
                end,
                len: data.len(),
            });
        }
        let tail = self.coords_at(data, submesh, stride, stride - 4, TexcoordFormat::Half)?;
        if tail.iter().all(|c| c[0].is_finite() && c[1].is_finite()) {
            return Err(Error::NoTexcoord);
        }
        Ok((0..submesh.vertex_count)
            .map(|k| {
                let at = submesh.vertex_offset + k * stride;
                let byte = |o: usize| f32::from(data[at + o]) / 255.0;
                [byte(14), byte(15), byte(16), byte(21)]
            })
            .collect())
    }
}
