//! The `.vex` scene format: models, tracks, everything 3D.
//!
//! A Maya scene export. The file is a 16-byte header followed by a depth-first
//! pre-order node tree, with embedded textures appended after it.
//!
//! ```text
//! file header, 16 bytes:
//!   +0x00  u32   version, 6 in Pulse
//!   +0x04  u32   size of the node tree
//!   +0x08  u32   size of the embedded texture block
//!   +0x0c  char  "VEXX"
//!
//! node:
//!   +0x00  u32   class_id
//!   +0x04  u16   header_size
//!   +0x08  u32   data_size
//!   +0x0c  u32   child_count
//!   +0x10  char  name, NUL-terminated, when header_size >= 0x20
//!
//! the next node begins at offset + header_size + data_size
//! ```
//!
//! See `docs/formats/vex.md` for the class-ID table and the evidence.
//!
//! # Geometry is pre-batched GE draw calls
//!
//! Meshes are not portable vertex and index buffers. They are batches of PSP
//! Graphics Engine draw calls, **never indexed**, with vertices stored inline
//! after each batch header. This module decodes them into ordinary vertex
//! arrays.
//!
//! Positions are **always** three `s16`, scaled by a per-batch `f32`:
//!
//! ```text
//! position = s16 / 32768.0 * scale
//! ```
//!
//! Missing that scale is the classic failure here: every model comes out a
//! uniform wrong size, which looks like a units problem rather than a decoding
//! bug.

use std::fmt;

/// Bytes of file header before the node tree.
pub const FILE_HEADER_LEN: usize = 16;

/// File magic, at `+0x0c` of the header.
pub const MAGIC: &[u8; 4] = b"VEXX";

/// Class ID of a `Mesh` node.
pub const CLASS_MESH: u32 = 0x125;

/// Class ID of a `Texture` node.
pub const CLASS_TEXTURE: u32 = 0x3c1;

/// Divisor for `s16` positions and the `f32` scale.
///
/// The game's own offline direction uses 32767; 32768 matches the hardware
/// exactly and the 0.003% difference is irrelevant.
const POSITION_DIVISOR: f32 = 32768.0;

/// Something wrong with a `.vex` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the file header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// A structure points outside the file.
    OutOfBounds {
        /// What was being read.
        what: &'static str,
        /// Where it would end.
        end: usize,
        /// Size of the file.
        len: usize,
    },
    /// A vertex type this build does not handle.
    ///
    /// The reachable set is small and fully enumerated; anything else means
    /// either an unexplored asset class or a decoding error, and both are worth
    /// hearing about loudly.
    UnsupportedVertexType {
        /// The value found.
        vertex_type: u16,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { got } => {
                write!(f, "need at least {FILE_HEADER_LEN} bytes, got {got}")
            }
            Self::OutOfBounds { what, end, len } => {
                write!(f, "{what} ends at {end} but the file is {len} bytes")
            }
            Self::UnsupportedVertexType { vertex_type } => {
                write!(f, "unsupported GU vertex type {vertex_type:#06x}")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// How each vertex component is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexLayout {
    /// Bytes per vertex.
    pub stride: usize,
    /// Offset of the three `s16` position components.
    pub position: usize,
    /// Offset of the two texture coordinates, and their format.
    pub texcoord: Option<(usize, TexcoordFormat)>,
    /// Offset of the three `s8` normal components.
    pub normal: Option<usize>,
    /// Offset of the ABGR8888 colour.
    pub colour: Option<usize>,
}

/// How texture coordinates are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TexcoordFormat {
    /// Two `u8`, divided by 128.
    U8,
    /// Two `f32`.
    F32,
}

impl VertexLayout {
    /// Derives the layout from a GU vertex type.
    ///
    /// Mirrors the game's own stride calculator rather than the general PSP
    /// rule. For every reachable combination the two agree, but reproducing the
    /// game's version means a disagreement would show up as an error here
    /// rather than as silently shifted vertices.
    pub fn from_vertex_type(vertex_type: u16) -> Result<Self> {
        // Position is always three s16: the stride calculation hard-codes a
        // `+ 6`, so bits 7-8 are always 2.
        if vertex_type & 0x0180 != 0x0100 {
            return Err(Error::UnsupportedVertexType { vertex_type });
        }
        // Weights, indices, morphs and the transform-2D bit are never handled.
        if vertex_type & 0xFE00 != 0 {
            return Err(Error::UnsupportedVertexType { vertex_type });
        }

        let mut offset = 0usize;
        let mut align = 2usize;

        let texcoord = match vertex_type & 3 {
            0 => None,
            1 => {
                let at = offset;
                offset += 2;
                Some((at, TexcoordFormat::U8))
            }
            3 => {
                let at = offset;
                offset += 8;
                align = 4;
                Some((at, TexcoordFormat::F32))
            }
            // u16 texcoords fall through the game's own branch without
            // advancing the offset, which is either a pruned case or a latent
            // bug. Either way, refuse rather than guess.
            _ => return Err(Error::UnsupportedVertexType { vertex_type }),
        };

        let colour = if vertex_type & 0x1c == 0x1c {
            offset = offset.next_multiple_of(4);
            align = 4;
            let at = offset;
            offset += 4;
            Some(at)
        } else if vertex_type & 0x1c != 0 {
            return Err(Error::UnsupportedVertexType { vertex_type });
        } else {
            None
        };

        let normal = if vertex_type & 0x60 == 0x20 {
            offset = offset.next_multiple_of(2);
            let at = offset;
            offset += 3;
            Some(at)
        } else if vertex_type & 0x60 != 0 {
            return Err(Error::UnsupportedVertexType { vertex_type });
        } else {
            None
        };

        offset = offset.next_multiple_of(2);
        let position = offset;
        let stride = (offset + 6).next_multiple_of(align);

        Ok(Self {
            stride,
            position,
            texcoord,
            normal,
            colour,
        })
    }
}

/// One decoded vertex, in model units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// Position in model units, scale already applied.
    pub position: [f32; 3],
    /// Unit normal, if the format carries one.
    pub normal: Option<[f32; 3]>,
    /// Texture coordinates, if the format carries them.
    pub texcoord: Option<[f32; 2]>,
    /// Vertex colour as RGBA, if the format carries one.
    pub colour: Option<[u8; 4]>,
}

/// One batch of draw calls.
#[derive(Debug, Clone)]
pub struct Batch {
    /// Render-pass selection bits.
    pub pass_mask: u16,
    /// Index into the mesh's material array.
    pub material_index: u8,
    /// GU primitive type. 3 is triangles, 4 is a triangle strip.
    pub primitive_type: u8,
    /// The raw GU vertex type.
    pub vertex_type: u16,
    /// Per-batch position scale.
    pub scale: f32,
    /// Decoded vertices.
    pub vertices: Vec<Vertex>,
}

/// GU primitive type for a triangle list.
pub const PRIM_TRIANGLES: u8 = 3;

/// GU primitive type for a triangle strip.
pub const PRIM_TRIANGLE_STRIP: u8 = 4;

impl Batch {
    /// Expands the batch into triangles, as index triples into
    /// [`vertices`](Self::vertices).
    ///
    /// Most batches are strips, so a renderer that only understands triangle
    /// lists needs this. Strips alternate winding every triangle, and getting
    /// that wrong makes every other face point inwards, which with back-face
    /// culling on produces a model full of holes.
    ///
    /// Returns an empty list for primitive types other than triangles and
    /// strips; points, lines and sprites are not geometry a mesh viewer can
    /// use, and none appear in the models examined.
    #[must_use]
    pub fn triangles(&self) -> Vec<[u32; 3]> {
        let count = self.vertices.len();
        match self.primitive_type {
            PRIM_TRIANGLES => (0..count / 3)
                .map(|t| {
                    let i = (t * 3) as u32;
                    [i, i + 1, i + 2]
                })
                .collect(),
            PRIM_TRIANGLE_STRIP => (0..count.saturating_sub(2))
                .map(|i| {
                    let i = i as u32;
                    // Flip winding on odd triangles so all faces agree.
                    if i % 2 == 0 {
                        [i, i + 1, i + 2]
                    } else {
                        [i + 1, i, i + 2]
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Whether this batch is transparent and must be drawn after opaque ones.
    #[must_use]
    pub fn is_transparent(&self) -> bool {
        self.pass_mask & 0x0700 != 0
    }

    /// Whether this batch is alpha-tested rather than blended.
    #[must_use]
    pub fn is_alpha_tested(&self) -> bool {
        self.pass_mask & 0x0800 != 0
    }

    /// Whether back-face culling is enabled. Clear means two-sided.
    #[must_use]
    pub fn is_culled(&self) -> bool {
        self.pass_mask & 0x0020 != 0
    }
}

/// A node in the scene tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Class ID; see the table in `docs/formats/vex.md`.
    pub class_id: u32,
    /// Offset of the node header from the start of the file.
    pub offset: usize,
    /// Bytes of node header.
    pub header_size: usize,
    /// Bytes of node payload.
    pub data_size: usize,
    /// Number of immediate children.
    pub child_count: usize,
    /// Node name from the header, when it carries one.
    pub name: Option<String>,
    /// Depth in the tree, zero for the root.
    pub depth: usize,
}

impl Node {
    /// Range of the node's payload within the file.
    #[must_use]
    pub fn payload(&self) -> std::ops::Range<usize> {
        let start = self.offset + self.header_size;
        start..start + self.data_size
    }
}

fn u16_at(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([data[at], data[at + 1]])
}

fn u32_at(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

fn f32_at(data: &[u8], at: usize) -> f32 {
    f32::from_bits(u32_at(data, at))
}

/// Format version, from the file header.
pub fn version(data: &[u8]) -> Result<u32> {
    if data.len() < FILE_HEADER_LEN {
        return Err(Error::TooShort { got: data.len() });
    }
    Ok(u32_at(data, 0))
}

/// Whether the file carries the `VEXX` magic.
///
/// The magic sits at `+0x0c`, not at the start, so a naive signature check
/// misses it.
#[must_use]
pub fn has_magic(data: &[u8]) -> bool {
    data.len() >= FILE_HEADER_LEN && &data[12..16] == MAGIC
}

/// Byte length of the node tree, from the file header.
pub fn tree_len(data: &[u8]) -> Result<usize> {
    if data.len() < FILE_HEADER_LEN {
        return Err(Error::TooShort { got: data.len() });
    }
    Ok(u32_at(data, 4) as usize)
}

/// Byte length of the embedded texture block, from the file header.
pub fn texture_len(data: &[u8]) -> Result<usize> {
    if data.len() < FILE_HEADER_LEN {
        return Err(Error::TooShort { got: data.len() });
    }
    Ok(u32_at(data, 8) as usize)
}

/// Reads a NUL-terminated name out of a node header.
fn name_at(data: &[u8], at: usize, header_size: usize) -> Option<String> {
    // Names live after the fixed fields; a minimal header has none.
    if header_size < 0x20 {
        return None;
    }
    let start = at + 0x10;
    let end = (at + header_size).min(data.len());
    let bytes = data.get(start..end)?;
    let text = bytes.split(|&b| b == 0).next()?;
    if text.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(text).into_owned())
}

/// Walks the node tree, depth-first.
///
/// Stops at the first structurally impossible node rather than erroring, since
/// the tree is followed by embedded texture data with no explicit terminator.
pub fn nodes(data: &[u8]) -> Result<Vec<Node>> {
    if data.len() < FILE_HEADER_LEN {
        return Err(Error::TooShort { got: data.len() });
    }

    let mut out = Vec::new();
    // Children follow their parent, so a stack of remaining counts tracks depth.
    let mut remaining: Vec<usize> = Vec::new();
    let mut at = FILE_HEADER_LEN;

    // The tree is followed by embedded textures, so stop at its declared end
    // rather than trying to detect where node data stops looking like nodes.
    let end = (FILE_HEADER_LEN + tree_len(data)?).min(data.len());

    while at + 16 <= end {
        // Retire finished subtrees *before* reading the depth, not after. A
        // parent whose children are all consumed is no longer an ancestor, and
        // deferring the pop reports the first node after a completed subtree at
        // the depth of that subtree rather than its own.
        while remaining.last() == Some(&0) {
            remaining.pop();
        }

        let header_size = u16_at(data, at + 4) as usize;
        let node = Node {
            class_id: u32_at(data, at),
            offset: at,
            header_size,
            data_size: u32_at(data, at + 8) as usize,
            child_count: u32_at(data, at + 12) as usize,
            name: name_at(data, at, header_size),
            depth: remaining.len(),
        };

        // A header smaller than the fields already read, or a node running past
        // the tree, means the structure is not what we think it is.
        if node.header_size < 16 || node.payload().end > end {
            break;
        }

        let children = node.child_count;
        let next = node.payload().end;
        out.push(node);

        if let Some(last) = remaining.last_mut() {
            *last -= 1;
        }
        if children > 0 {
            remaining.push(children);
        }

        // Guard against a zero-length node, which would loop forever.
        if next <= at {
            break;
        }
        at = next;
    }

    Ok(out)
}

/// A texture embedded in the file.
#[derive(Debug, Clone)]
pub struct EmbeddedTexture {
    /// Original asset path, from the node name.
    pub name: Option<String>,
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// 4 or 8.
    pub bits_per_pixel: u8,
    /// Number of mip levels present. Only the base level is decoded.
    pub mip_count: u8,
    /// Palette, RGBA8888.
    pub palette: Vec<[u8; 4]>,
    /// Base-level pixel indices, one per pixel.
    pub indices: Vec<u8>,
}

impl EmbeddedTexture {
    /// Expands the base level to RGBA8888.
    #[must_use]
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.indices.len() * 4);
        for &i in &self.indices {
            let c = self
                .palette
                .get(i as usize)
                .copied()
                .unwrap_or([255, 0, 255, 255]);
            out.extend_from_slice(&c);
        }
        out
    }
}

/// Extracts the textures appended after the node tree.
///
/// Their data is not pointed at from anywhere: the header's pointer fields are
/// zero at rest and patched at load. Instead each texture's palette and texels
/// are packed back to back in node order, starting immediately after the tree.
///
/// That the sizes add up exactly to the header's declared texture length is
/// what confirms the packing, and this function checks it.
pub fn textures(data: &[u8]) -> Result<Vec<EmbeddedTexture>> {
    let block = FILE_HEADER_LEN + tree_len(data)?;
    let mut at = block;
    let mut out = Vec::new();

    for node in nodes(data)?
        .into_iter()
        .filter(|n| n.class_id == CLASS_TEXTURE)
    {
        let p = &data[node.payload()];
        if p.len() < 0x10 {
            continue;
        }

        let width = u16_at(p, 0);
        let height = u16_at(p, 2);
        let bits_per_pixel = p[4];
        let mip_count = p[5];
        let clut_size = u32_at(p, 8) as usize;
        let texel_size = u32_at(p, 12) as usize;

        let end = at + clut_size + texel_size;
        if end > data.len() {
            return Err(Error::OutOfBounds {
                what: "embedded texture",
                end,
                len: data.len(),
            });
        }
        if !matches!(bits_per_pixel, 4 | 8) {
            at = end;
            continue;
        }

        let palette: Vec<[u8; 4]> = data[at..at + clut_size]
            .chunks_exact(4)
            .map(|c| [c[0], c[1], c[2], c[3]])
            .collect();

        // Only the base level; mips follow it and are not needed for viewing.
        let pixels = usize::from(width) * usize::from(height);
        let base_bytes = pixels * usize::from(bits_per_pixel) / 8;
        let texels = &data[at + clut_size..end];

        let indices = if bits_per_pixel == 8 {
            texels.get(..base_bytes).unwrap_or(texels).to_vec()
        } else {
            let mut v = Vec::with_capacity(pixels);
            for &b in texels.get(..base_bytes).unwrap_or(texels) {
                v.push(b & 0x0f);
                v.push(b >> 4);
            }
            v
        };

        out.push(EmbeddedTexture {
            name: node.name,
            width,
            height,
            bits_per_pixel,
            mip_count,
            palette,
            indices,
        });
        at = end;
    }

    Ok(out)
}

/// Materials of one mesh payload.
///
/// Stride 0x14, starting at `+0x30`. The `u32` at `+0x04` indexes the model's
/// texture array.
#[must_use]
pub fn mesh_materials(payload: &[u8]) -> Vec<u32> {
    if payload.len() < 0x30 {
        return Vec::new();
    }
    let count = usize::from(u16_at(payload, 2));
    (0..count)
        .filter_map(|i| {
            let at = 0x30 + i * 0x14;
            (at + 8 <= payload.len()).then(|| u32_at(payload, at + 4))
        })
        .collect()
}

/// Decodes the batches of one mesh payload.
///
/// `payload` is the mesh node's data. `batch_list` selects list A (`0`) or list
/// B (`1`); a batch belongs to a list while the corresponding `pass_mask` bit
/// is set.
pub fn mesh_batches(payload: &[u8], batch_list: u8) -> Result<Vec<Batch>> {
    if payload.len() < 0x30 {
        return Err(Error::TooShort { got: payload.len() });
    }

    // The loader relocates these into pointers; in the file they are offsets
    // from the start of the mesh payload.
    let list_offset = u32_at(payload, if batch_list == 0 { 4 } else { 8 }) as usize;
    let terminator = if batch_list == 0 { 1u16 } else { 2 };

    let mut out = Vec::new();
    let mut at = list_offset;

    while at + 0x40 <= payload.len() {
        let pass_mask = u16_at(payload, at);
        if pass_mask & terminator == 0 {
            break;
        }

        let flags = payload[at + 3];
        let header_size = if flags & 0x40 != 0 { 0x80 } else { 0x40 };

        let use_alternate = u16_at(payload, at + 6) != 0;
        let vertex_count = usize::from(u16_at(payload, at + if use_alternate { 6 } else { 4 }));
        let primitive_type = payload[at + if use_alternate { 9 } else { 8 }];
        let vertex_type = u16_at(payload, at + 0x0a);
        let payload_size = usize::from(u16_at(payload, at + 0x0c));
        let alternate_offset = usize::from(u16_at(payload, at + 0x0e));
        let scale = f32_at(payload, at + 0x10);

        let layout = VertexLayout::from_vertex_type(vertex_type)?;

        let base = at + header_size + if use_alternate { alternate_offset } else { 0 };
        let end = base + vertex_count * layout.stride;
        if end > payload.len() {
            return Err(Error::OutOfBounds {
                what: "batch vertices",
                end,
                len: payload.len(),
            });
        }

        let mut vertices = Vec::with_capacity(vertex_count);
        for i in 0..vertex_count {
            vertices.push(decode_vertex(
                payload,
                base + i * layout.stride,
                &layout,
                scale,
            ));
        }

        out.push(Batch {
            pass_mask,
            material_index: payload[at + 2],
            primitive_type,
            vertex_type,
            scale,
            vertices,
        });

        // `payload_size` covers the vertex data only, so a batch is its header
        // plus that.
        let step = header_size + payload_size;
        if step == 0 {
            break;
        }
        at += step;
    }

    Ok(out)
}

fn decode_vertex(data: &[u8], at: usize, layout: &VertexLayout, scale: f32) -> Vertex {
    let p = at + layout.position;
    let component = |o: usize| f32::from(i16::from_le_bytes([data[o], data[o + 1]]));

    let position = [
        component(p) / POSITION_DIVISOR * scale,
        component(p + 2) / POSITION_DIVISOR * scale,
        component(p + 4) / POSITION_DIVISOR * scale,
    ];

    let normal = layout.normal.map(|o| {
        let n = |i: usize| f32::from(data[o + at + i] as i8) / 128.0;
        [n(0), n(1), n(2)]
    });

    let texcoord = layout.texcoord.map(|(o, format)| match format {
        TexcoordFormat::U8 => [
            f32::from(data[at + o]) / 128.0,
            f32::from(data[at + o + 1]) / 128.0,
        ],
        TexcoordFormat::F32 => [f32_at(data, at + o), f32_at(data, at + o + 4)],
    });

    let colour = layout.colour.map(|o| {
        [
            data[at + o],
            data[at + o + 1],
            data[at + o + 2],
            data[at + o + 3],
        ]
    });

    Vertex {
        position,
        normal,
        texcoord,
        colour,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a `.vex` file from `(class_id, child_count, payload_len)` nodes,
    /// in the depth-first pre-order the format stores them in.
    fn build_tree(nodes: &[(u32, usize, usize)]) -> Vec<u8> {
        let mut tree = Vec::new();
        for &(class_id, children, payload_len) in nodes {
            tree.extend(class_id.to_le_bytes());
            tree.extend(0x20u16.to_le_bytes()); // header_size
            tree.extend(0u16.to_le_bytes()); // padding to +0x08
            tree.extend((payload_len as u32).to_le_bytes());
            tree.extend((children as u32).to_le_bytes());
            tree.extend([0u8; 0x10]); // the name area, left empty
            tree.extend(std::iter::repeat_n(0u8, payload_len));
        }

        let mut out = Vec::new();
        out.extend(6u32.to_le_bytes());
        out.extend((tree.len() as u32).to_le_bytes());
        out.extend(0u32.to_le_bytes());
        out.extend(MAGIC);
        out.extend(tree);
        out
    }

    #[test]
    fn walks_a_flat_tree() {
        let data = build_tree(&[(1, 0, 0), (2, 0, 16), (3, 0, 0)]);
        let nodes = nodes(&data).expect("walk");
        assert_eq!(nodes.len(), 3);
        assert_eq!(
            nodes.iter().map(|n| n.class_id).collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert!(nodes.iter().all(|n| n.depth == 0));
        assert_eq!(nodes[1].data_size, 16);
        assert_eq!(nodes[1].payload().len(), 16);
    }

    /// A parent whose subtree is finished must stop counting as an ancestor.
    ///
    /// `root -> a -> b`, then a sibling of `root`. The sibling is at depth 0, and
    /// reporting it at depth 2 is what an implementation does if it retires
    /// finished subtrees after reading the depth instead of before. Nothing reads
    /// `depth` yet, which is the only reason this was survivable.
    #[test]
    fn depth_returns_to_zero_after_a_completed_subtree() {
        let data = build_tree(&[(1, 1, 0), (2, 1, 0), (3, 0, 0), (4, 0, 0)]);
        let depths: Vec<usize> = nodes(&data)
            .expect("walk")
            .iter()
            .map(|n| n.depth)
            .collect();
        assert_eq!(depths, [0, 1, 2, 0]);
    }

    #[test]
    fn depth_tracks_siblings_at_every_level() {
        // root
        //   a
        //     a1
        //     a2
        //   b
        let data = build_tree(&[(1, 2, 0), (2, 2, 0), (3, 0, 0), (4, 0, 0), (5, 0, 0)]);
        let depths: Vec<usize> = nodes(&data)
            .expect("walk")
            .iter()
            .map(|n| n.depth)
            .collect();
        assert_eq!(depths, [0, 1, 2, 2, 1]);
    }

    #[test]
    fn stops_at_a_node_that_runs_past_the_tree() {
        let mut data = build_tree(&[(1, 0, 0), (2, 0, 0)]);
        // Claim a payload far larger than the file for the second node.
        let second = FILE_HEADER_LEN + 0x20;
        data[second + 8..second + 12].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
        let nodes = nodes(&data).expect("walk");
        assert_eq!(nodes.len(), 1, "the impossible node must not be reported");
    }

    #[test]
    fn stops_at_a_header_too_small_to_be_one() {
        let mut data = build_tree(&[(1, 0, 0), (2, 0, 0)]);
        let second = FILE_HEADER_LEN + 0x20;
        data[second + 4..second + 6].copy_from_slice(&8u16.to_le_bytes());
        assert_eq!(nodes(&data).expect("walk").len(), 1);
    }

    #[test]
    fn a_truncated_file_is_refused_rather_than_walked() {
        assert!(matches!(nodes(&[]), Err(Error::TooShort { .. })));
        assert!(matches!(nodes(&[0u8; 8]), Err(Error::TooShort { .. })));
    }

    /// A tree length larger than the file must not read past the end.
    #[test]
    fn a_lying_tree_length_is_clamped_to_the_file() {
        let mut data = build_tree(&[(1, 0, 0)]);
        data[4..8].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
        let nodes = nodes(&data).expect("walk");
        assert_eq!(nodes.len(), 1);
    }

    /// Every combination the game's stride calculator can reach.
    /// Values from the recovered layout table; see `docs/formats/vex.md`.
    #[test]
    fn derives_every_reachable_layout() {
        let cases: &[(u16, usize, usize)] = &[
            (0x100, 6, 0),
            (0x101, 8, 2),
            (0x103, 16, 8),
            (0x11c, 12, 4),
            (0x11d, 16, 8),
            (0x11f, 20, 12),
            (0x120, 10, 4),
            (0x121, 12, 6),
            (0x123, 20, 12),
            (0x13c, 16, 8),
            (0x13d, 20, 12),
            (0x13f, 24, 16),
        ];

        for &(vertex_type, stride, position) in cases {
            let layout = VertexLayout::from_vertex_type(vertex_type)
                .unwrap_or_else(|e| panic!("{vertex_type:#06x}: {e}"));
            assert_eq!(layout.stride, stride, "stride for {vertex_type:#06x}");
            assert_eq!(layout.position, position, "position for {vertex_type:#06x}");
        }
    }

    #[test]
    fn identifies_present_components() {
        let full = VertexLayout::from_vertex_type(0x13f).unwrap();
        assert_eq!(full.texcoord, Some((0, TexcoordFormat::F32)));
        assert_eq!(full.colour, Some(8));
        assert_eq!(full.normal, Some(12));
        assert_eq!(full.position, 16);

        let bare = VertexLayout::from_vertex_type(0x100).unwrap();
        assert_eq!(bare.texcoord, None);
        assert_eq!(bare.colour, None);
        assert_eq!(bare.normal, None);
    }

    #[test]
    fn rejects_u16_texcoords() {
        // The game's own calculator falls through this case without advancing
        // the offset. Guessing would shift every following field.
        assert!(matches!(
            VertexLayout::from_vertex_type(0x102),
            Err(Error::UnsupportedVertexType { .. })
        ));
    }

    #[test]
    fn rejects_non_s16_positions() {
        // The `+ 6` in the stride formula only holds for s16 positions.
        for vertex_type in [0x000, 0x080, 0x180] {
            assert!(
                matches!(
                    VertexLayout::from_vertex_type(vertex_type),
                    Err(Error::UnsupportedVertexType { .. })
                ),
                "{vertex_type:#06x} should be refused"
            );
        }
    }

    #[test]
    fn rejects_weights_and_morphs() {
        assert!(VertexLayout::from_vertex_type(0x0500).is_err());
    }

    #[test]
    fn scales_positions_into_model_units() {
        let layout = VertexLayout::from_vertex_type(0x100).unwrap();
        let mut data = vec![0u8; 16];
        // Full-scale positive, full-scale negative, zero.
        data[0..2].copy_from_slice(&i16::MAX.to_le_bytes());
        data[2..4].copy_from_slice(&(-32768i16).to_le_bytes());
        data[4..6].copy_from_slice(&0i16.to_le_bytes());

        let v = decode_vertex(&data, 0, &layout, 100.0);
        assert!((v.position[0] - 99.997).abs() < 0.01, "{:?}", v.position);
        assert!((v.position[1] + 100.0).abs() < 0.001, "{:?}", v.position);
        assert_eq!(v.position[2], 0.0);
    }

    #[test]
    fn a_missing_scale_would_be_obvious() {
        // Guards the mistake this format invites: forgetting the scale leaves
        // every model in the unit cube.
        let layout = VertexLayout::from_vertex_type(0x100).unwrap();
        let mut data = vec![0u8; 16];
        data[0..2].copy_from_slice(&16384i16.to_le_bytes());

        let scaled = decode_vertex(&data, 0, &layout, 250.0);
        assert!((scaled.position[0] - 125.0).abs() < 0.001);
    }

    #[test]
    fn rejects_a_short_file() {
        assert_eq!(version(&[0u8; 4]), Err(Error::TooShort { got: 4 }));
        assert_eq!(nodes(&[0u8; 4]), Err(Error::TooShort { got: 4 }));
    }

    fn batch_with(primitive_type: u8, vertex_count: usize) -> Batch {
        Batch {
            pass_mask: 1,
            material_index: 0,
            primitive_type,
            vertex_type: 0x100,
            scale: 1.0,
            vertices: vec![
                Vertex {
                    position: [0.0; 3],
                    normal: None,
                    texcoord: None,
                    colour: None,
                };
                vertex_count
            ],
        }
    }

    #[test]
    fn expands_a_triangle_list() {
        let t = batch_with(PRIM_TRIANGLES, 9).triangles();
        assert_eq!(t, vec![[0, 1, 2], [3, 4, 5], [6, 7, 8]]);
    }

    #[test]
    fn a_triangle_list_ignores_a_trailing_partial_triangle() {
        assert_eq!(batch_with(PRIM_TRIANGLES, 8).triangles().len(), 2);
    }

    #[test]
    fn expands_a_strip_with_alternating_winding() {
        // Getting the flip wrong makes every other face point inwards, which
        // with culling on riddles the model with holes.
        let t = batch_with(PRIM_TRIANGLE_STRIP, 5).triangles();
        assert_eq!(t, vec![[0, 1, 2], [2, 1, 3], [2, 3, 4]]);
    }

    #[test]
    fn a_strip_shorter_than_a_triangle_yields_nothing() {
        assert!(batch_with(PRIM_TRIANGLE_STRIP, 2).triangles().is_empty());
        assert!(batch_with(PRIM_TRIANGLE_STRIP, 0).triangles().is_empty());
    }

    #[test]
    fn unsupported_primitives_yield_nothing_rather_than_garbage() {
        for prim in [0u8, 1, 2, 5, 6] {
            assert!(batch_with(prim, 12).triangles().is_empty(), "prim {prim}");
        }
    }

    #[test]
    fn classifies_pass_masks() {
        let batch = |pass_mask| Batch {
            pass_mask,
            material_index: 0,
            primitive_type: 3,
            vertex_type: 0x100,
            scale: 1.0,
            vertices: Vec::new(),
        };
        assert!(batch(0x0101).is_transparent());
        assert!(!batch(0x0021).is_transparent());
        assert!(batch(0x0801).is_alpha_tested());
        assert!(batch(0x0021).is_culled());
        assert!(!batch(0x0001).is_culled());
    }
}
