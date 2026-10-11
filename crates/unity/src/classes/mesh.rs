use super::{field, int, text, uint};
use crate::error::{Error, Result};
use crate::value::Value;

/// How one vertex channel's components are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexFormat {
    /// 32-bit float.
    Float32,
    /// 16-bit float.
    Float16,
    /// Unsigned 8-bit, normalised to 0..1.
    UNorm8,
    /// Signed 8-bit, normalised to -1..1.
    SNorm8,
    /// Unsigned 16-bit, normalised to 0..1.
    UNorm16,
    /// Signed 16-bit, normalised to -1..1.
    SNorm16,
    /// Unsigned 8-bit integer.
    UInt8,
    /// Signed 8-bit integer.
    SInt8,
    /// Unsigned 16-bit integer.
    UInt16,
    /// Signed 16-bit integer.
    SInt16,
    /// Unsigned 32-bit integer.
    UInt32,
    /// Signed 32-bit integer.
    SInt32,
}

impl VertexFormat {
    /// Maps a channel format id. The ids were renumbered in 2019: before it,
    /// Unity had five (`Float`, `Float16`, `Color`, `Byte`, `UInt32`).
    pub fn from_id(id: u32, engine_major: u32) -> Result<Self> {
        if engine_major < 2019 {
            return Ok(match id {
                0 => Self::Float32,
                1 => Self::Float16,
                2 => Self::UNorm8,
                3 => Self::UInt8,
                4 => Self::UInt32,
                _ => return Err(Error::Unsupported("vertex channel format")),
            });
        }
        Ok(match id {
            0 => Self::Float32,
            1 => Self::Float16,
            2 => Self::UNorm8,
            3 => Self::SNorm8,
            4 => Self::UNorm16,
            5 => Self::SNorm16,
            6 => Self::UInt8,
            7 => Self::SInt8,
            8 => Self::UInt16,
            9 => Self::SInt16,
            10 => Self::UInt32,
            11 => Self::SInt32,
            _ => return Err(Error::Unsupported("vertex channel format")),
        })
    }

    /// Bytes of one component.
    #[must_use]
    pub fn size(self) -> usize {
        match self {
            Self::UNorm8 | Self::SNorm8 | Self::UInt8 | Self::SInt8 => 1,
            Self::Float16 | Self::UNorm16 | Self::SNorm16 | Self::UInt16 | Self::SInt16 => 2,
            Self::Float32 | Self::UInt32 | Self::SInt32 => 4,
        }
    }

    fn decode(self, b: &[u8]) -> f32 {
        match self {
            Self::Float32 => f32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            Self::Float16 => f16_to_f32(u16::from_le_bytes([b[0], b[1]])),
            Self::UNorm8 => f32::from(b[0]) / 255.0,
            Self::SNorm8 => (f32::from(b[0] as i8) / 127.0).max(-1.0),
            Self::UNorm16 => f32::from(u16::from_le_bytes([b[0], b[1]])) / 65535.0,
            Self::SNorm16 => (f32::from(i16::from_le_bytes([b[0], b[1]])) / 32767.0).max(-1.0),
            Self::UInt8 => f32::from(b[0]),
            Self::SInt8 => f32::from(b[0] as i8),
            Self::UInt16 => f32::from(u16::from_le_bytes([b[0], b[1]])),
            Self::SInt16 => f32::from(i16::from_le_bytes([b[0], b[1]])),
            Self::UInt32 => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32,
            Self::SInt32 => i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32,
        }
    }
}

fn f16_to_f32(h: u16) -> f32 {
    let sign = u32::from(h >> 15) << 31;
    let exp = u32::from(h >> 10 & 0x1f);
    let man = u32::from(h & 0x3ff);
    let bits = match (exp, man) {
        (0, 0) => sign,
        (0, m) => {
            let shift = m.leading_zeros() - 21;
            sign | (127 - 15 - shift + 1) << 23 | ((m << (shift + 1)) & 0x3ff) << 13
        }
        (31, 0) => sign | 0x7f80_0000,
        (31, m) => sign | 0x7f80_0000 | m << 13,
        (e, m) => sign | (e + 127 - 15) << 23 | m << 13,
    };
    f32::from_bits(bits)
}

/// Well-known channel slots.
pub mod channel {
    /// Position.
    pub const POSITION: usize = 0;
    /// Normal.
    pub const NORMAL: usize = 1;
    /// Tangent.
    pub const TANGENT: usize = 2;
    /// Vertex colour.
    pub const COLOR: usize = 3;
    /// First UV set; sets 0..8 follow.
    pub const UV0: usize = 4;
}

/// One vertex attribute's place in the vertex buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Channel {
    /// Which stream holds it.
    pub stream: u8,
    /// Byte offset within one vertex of that stream.
    pub offset: u8,
    /// Component format.
    pub format: VertexFormat,
    /// Components per vertex; 0 means the channel is unused.
    pub dimension: u8,
}

/// Index width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexFormat {
    /// 16-bit.
    U16,
    /// 32-bit.
    U32,
}

/// One drawable range of the index buffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubMesh {
    /// Byte offset into the index buffer.
    pub first_byte: u32,
    /// Number of indices.
    pub index_count: u32,
    /// Unity topology id (0 triangles, 1 triangle strip, 2 quads, 3 lines, 4 line strip, 5 points).
    pub topology: u32,
    /// Added to every index.
    pub base_vertex: u32,
    /// First vertex this submesh uses.
    pub first_vertex: u32,
    /// Vertices it uses.
    pub vertex_count: u32,
}

/// A `Mesh`: vertex channels over a stream-packed buffer, plus submeshes.
#[derive(Debug, Clone)]
pub struct Mesh {
    /// Its name.
    pub name: String,
    /// Vertex count.
    pub vertex_count: usize,
    /// The channel table (14 entries in 2018).
    pub channels: Vec<Channel>,
    /// The index width.
    pub index_format: IndexFormat,
    /// The submeshes.
    pub submeshes: Vec<SubMesh>,
    vertex_data: Vec<u8>,
    index_data: Vec<u8>,
    stream_offsets: [usize; 4],
    stream_strides: [usize; 4],
}

fn align16(n: usize) -> usize {
    (n + 15) & !15
}

impl Mesh {
    /// Reads a `Mesh` value written by an engine of major version `engine_major`
    /// (the channel format ids depend on it). Refuses a compressed mesh, an
    /// external vertex stream, and any buffer whose size the channel table does
    /// not account for exactly.
    pub fn from_value(v: &Value, engine_major: u32) -> Result<Self> {
        const C: &str = "Mesh";
        let bad = |field| Error::Field { class: C, field };
        if int(v, C, "m_MeshCompression")? != 0 {
            return Err(Error::Unsupported("compressed mesh"));
        }
        if let Some(s) = v.get("m_StreamData")
            && uint(s, C, "size")? != 0
        {
            return Err(Error::Unsupported("mesh data in an external stream"));
        }
        let index_format = match v.get("m_IndexFormat").and_then(Value::as_i64) {
            Some(0) => IndexFormat::U16,
            Some(1) => IndexFormat::U32,
            Some(_) => return Err(Error::Unsupported("index format")),
            None => match v.get("m_Use16BitIndices").and_then(Value::as_bool) {
                Some(true) => IndexFormat::U16,
                Some(false) => IndexFormat::U32,
                None => return Err(bad("m_IndexFormat")),
            },
        };
        let index_data = field(v, C, "m_IndexBuffer")?
            .as_bytes()
            .ok_or_else(|| bad("m_IndexBuffer"))?
            .to_vec();

        let mut submeshes = Vec::new();
        for s in field(v, C, "m_SubMeshes")?
            .as_array()
            .ok_or_else(|| bad("m_SubMeshes"))?
        {
            let u = |n: &'static str| -> Result<u32> {
                u32::try_from(uint(s, C, n)?).map_err(|_| Error::OutOfRange("submesh field"))
            };
            submeshes.push(SubMesh {
                first_byte: u("firstByte")?,
                index_count: u("indexCount")?,
                topology: u32::try_from(int(s, C, "topology")?)
                    .map_err(|_| Error::OutOfRange("topology"))?,
                base_vertex: u("baseVertex")?,
                first_vertex: u("firstVertex")?,
                vertex_count: u("vertexCount")?,
            });
        }

        let vd = field(v, C, "m_VertexData")?;
        let vertex_count = usize::try_from(uint(vd, C, "m_VertexCount")?)
            .map_err(|_| Error::OutOfRange("vertex count"))?;
        let vertex_data = field(vd, C, "m_DataSize")?
            .as_bytes()
            .ok_or_else(|| bad("m_DataSize"))?
            .to_vec();
        let mut channels = Vec::new();
        for c in field(vd, C, "m_Channels")?
            .as_array()
            .ok_or_else(|| bad("m_Channels"))?
        {
            let u = |n: &'static str| -> Result<u32> {
                u32::try_from(uint(c, C, n)?).map_err(|_| Error::OutOfRange("channel field"))
            };
            let dimension = (u("dimension")? & 0xf) as u8;
            let format = if dimension == 0 {
                VertexFormat::Float32
            } else {
                VertexFormat::from_id(u("format")?, engine_major)?
            };
            let (stream, offset) = (u("stream")?, u("offset")?);
            if stream > 3 || offset > 255 || dimension > 4 {
                return Err(Error::OutOfRange("channel stream, offset or dimension"));
            }
            channels.push(Channel {
                stream: stream as u8,
                offset: offset as u8,
                format,
                dimension,
            });
        }

        let mut stream_strides = [0usize; 4];
        for c in channels.iter().filter(|c| c.dimension > 0) {
            let end = usize::from(c.offset) + usize::from(c.dimension) * c.format.size();
            let s = &mut stream_strides[usize::from(c.stream)];
            *s = (*s).max(end);
        }
        let mut stream_offsets = [0usize; 4];
        let mut at = 0usize;
        let mut end = 0usize;
        for s in 0..4 {
            stream_offsets[s] = at;
            if stream_strides[s] > 0 {
                end = vertex_count
                    .checked_mul(stream_strides[s])
                    .and_then(|n| n.checked_add(at))
                    .ok_or(Error::OutOfRange("vertex buffer size"))?;
                at = align16(end);
            }
        }
        if vertex_count > 0 && end != vertex_data.len() {
            return Err(Error::Malformed(
                "vertex buffer size differs from the channel layout",
            ));
        }
        if vertex_count == 0 && !vertex_data.is_empty() {
            return Err(Error::Malformed("vertex data with no vertices"));
        }
        Ok(Self {
            name: text(v, C, "m_Name")?,
            vertex_count,
            channels,
            index_format,
            submeshes,
            vertex_data,
            index_data,
            stream_offsets,
            stream_strides,
        })
    }

    /// A channel as flat `f32`s, `dimension` per vertex, or `None` when unused.
    pub fn channel_f32(&self, slot: usize) -> Result<Option<(usize, Vec<f32>)>> {
        let Some(c) = self.channels.get(slot).filter(|c| c.dimension > 0) else {
            return Ok(None);
        };
        let (dim, fsize) = (usize::from(c.dimension), c.format.size());
        let stride = self.stream_strides[usize::from(c.stream)];
        let base = self.stream_offsets[usize::from(c.stream)] + usize::from(c.offset);
        let mut out = Vec::with_capacity(self.vertex_count * dim);
        for i in 0..self.vertex_count {
            for d in 0..dim {
                let at = base + i * stride + d * fsize;
                let b = self
                    .vertex_data
                    .get(at..at + fsize)
                    .ok_or(Error::Malformed("vertex read past the buffer"))?;
                out.push(c.format.decode(b));
            }
        }
        Ok(Some((dim, out)))
    }

    fn vec_channel<const N: usize>(&self, slot: usize) -> Result<Option<Vec<[f32; N]>>> {
        let Some((dim, flat)) = self.channel_f32(slot)? else {
            return Ok(None);
        };
        if dim < N {
            return Err(Error::Malformed(
                "channel has fewer components than its meaning needs",
            ));
        }
        Ok(Some(
            flat.chunks_exact(dim)
                .map(|c| std::array::from_fn(|i| c[i]))
                .collect(),
        ))
    }

    /// Positions, if the mesh has them.
    pub fn positions(&self) -> Result<Option<Vec<[f32; 3]>>> {
        self.vec_channel(channel::POSITION)
    }

    /// Normals.
    pub fn normals(&self) -> Result<Option<Vec<[f32; 3]>>> {
        self.vec_channel(channel::NORMAL)
    }

    /// Tangents (xyz and handedness w).
    pub fn tangents(&self) -> Result<Option<Vec<[f32; 4]>>> {
        self.vec_channel(channel::TANGENT)
    }

    /// Vertex colours.
    pub fn colors(&self) -> Result<Option<Vec<[f32; 4]>>> {
        self.vec_channel(channel::COLOR)
    }

    /// UV set `set` (0..8).
    pub fn uvs(&self, set: usize) -> Result<Option<Vec<[f32; 2]>>> {
        if set >= 8 {
            return Err(Error::OutOfRange("uv set"));
        }
        self.vec_channel(channel::UV0 + set)
    }

    /// The raw indices of a submesh, with `baseVertex` added, in file order.
    pub fn indices(&self, submesh: usize) -> Result<Vec<u32>> {
        let s = self
            .submeshes
            .get(submesh)
            .ok_or(Error::OutOfRange("submesh index"))?;
        let width = match self.index_format {
            IndexFormat::U16 => 2,
            IndexFormat::U32 => 4,
        };
        let start = s.first_byte as usize;
        let len = (s.index_count as usize)
            .checked_mul(width)
            .ok_or(Error::OutOfRange("index count"))?;
        let bytes = self
            .index_data
            .get(
                start
                    ..start
                        .checked_add(len)
                        .ok_or(Error::OutOfRange("index range"))?,
            )
            .ok_or(Error::Malformed("submesh indices outside the index buffer"))?;
        let mut out = Vec::with_capacity(s.index_count as usize);
        for c in bytes.chunks_exact(width) {
            let raw = match self.index_format {
                IndexFormat::U16 => u32::from(u16::from_le_bytes([c[0], c[1]])),
                IndexFormat::U32 => u32::from_le_bytes([c[0], c[1], c[2], c[3]]),
            };
            let i = raw
                .checked_add(s.base_vertex)
                .ok_or(Error::OutOfRange("index"))?;
            if i as usize >= self.vertex_count {
                return Err(Error::Malformed("index past the vertex count"));
            }
            out.push(i);
        }
        Ok(out)
    }

    /// A submesh as triangles. Lists and strips are read; quads, lines and
    /// points are [`Error::Unsupported`].
    pub fn triangles(&self, submesh: usize) -> Result<Vec<[u32; 3]>> {
        let topology = self
            .submeshes
            .get(submesh)
            .ok_or(Error::OutOfRange("submesh index"))?
            .topology;
        let idx = self.indices(submesh)?;
        match topology {
            0 => {
                if idx.len() % 3 != 0 {
                    return Err(Error::Malformed("triangle list length"));
                }
                Ok(idx.as_chunks::<3>().0.to_vec())
            }
            1 => {
                let mut out = Vec::new();
                for (i, w) in idx.windows(3).enumerate() {
                    if w[0] == w[1] || w[1] == w[2] || w[0] == w[2] {
                        continue;
                    }
                    out.push(if i % 2 == 0 {
                        [w[0], w[1], w[2]]
                    } else {
                        [w[1], w[0], w[2]]
                    });
                }
                Ok(out)
            }
            _ => Err(Error::Unsupported(
                "mesh topology other than triangles or strips",
            )),
        }
    }
}
