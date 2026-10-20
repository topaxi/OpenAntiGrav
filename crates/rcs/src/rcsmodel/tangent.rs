//! A chunk's authored tangent stream.

use super::{Error, Mesh, Result, SubMesh};

impl Mesh {
    /// The `tangent` attribute's four raw bytes per vertex, or `None` where the
    /// chunk declares none.
    ///
    /// Raw, because the vertex program decodes them itself: `xyz` as `2b - 1`
    /// (a unit vector, perpendicular to the normal) and `w` as the factor on
    /// the binormal `cross(N, T) * w`. `docs/rendering/hd-ship-materials.md`,
    /// "Hull normal maps".
    ///
    /// # Errors
    ///
    /// [`Error::OutOfBounds`] as [`Self::positions`].
    pub fn tangents(
        &self,
        data: &[u8],
        submesh: &SubMesh,
        stride: usize,
    ) -> Result<Option<Vec<[u8; 4]>>> {
        let Some(attribute) = self.decl.as_ref().and_then(|decl| decl.tangent()) else {
            return Ok(None);
        };
        let offset = usize::from(attribute.offset);
        let end =
            submesh.vertex_offset + stride * submesh.vertex_count.saturating_sub(1) + offset + 4;
        if submesh.vertex_count > 0 && end > data.len() {
            return Err(Error::OutOfBounds {
                what: "a vertex buffer's tangents",
                end,
                len: data.len(),
            });
        }
        Ok(Some(
            (0..submesh.vertex_count)
                .map(|k| {
                    let at = submesh.vertex_offset + k * stride + offset;
                    [data[at], data[at + 1], data[at + 2], data[at + 3]]
                })
                .collect(),
        ))
    }
}
