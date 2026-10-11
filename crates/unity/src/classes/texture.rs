use super::{field, int, text, uint};
use crate::error::{Error, Result};
use crate::value::Value;

/// A `Texture2D` pixel format, by Unity's `TextureFormat` id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    /// 8-bit alpha only.
    Alpha8,
    /// 16-bit ARGB, 4 bits each.
    Argb4444,
    /// 8-bit R, G, B.
    Rgb24,
    /// 8-bit R, G, B, A.
    Rgba32,
    /// 8-bit A, R, G, B.
    Argb32,
    /// 5-6-5 RGB.
    Rgb565,
    /// 16-bit single channel.
    R16,
    /// BC1, opaque or 1-bit alpha.
    Dxt1,
    /// BC2 (obsolete in Unity, still found in old content).
    Dxt3,
    /// BC3.
    Dxt5,
    /// 16-bit RGBA, 4 bits each.
    Rgba4444,
    /// 8-bit B, G, R, A.
    Bgra32,
    /// A format id this reader has no size rule for.
    Other(i32),
}

impl TextureFormat {
    /// Maps Unity's id.
    #[must_use]
    pub fn from_id(id: i32) -> Self {
        match id {
            1 => Self::Alpha8,
            2 => Self::Argb4444,
            3 => Self::Rgb24,
            4 => Self::Rgba32,
            5 => Self::Argb32,
            7 => Self::Rgb565,
            9 => Self::R16,
            10 => Self::Dxt1,
            11 => Self::Dxt3,
            12 => Self::Dxt5,
            13 => Self::Rgba4444,
            14 => Self::Bgra32,
            other => Self::Other(other),
        }
    }

    /// Bytes of one mip level, or `None` for a format with no size rule.
    #[must_use]
    pub fn level_len(self, width: usize, height: usize) -> Option<usize> {
        let px = width.checked_mul(height)?;
        let block = |bytes: usize| {
            width
                .div_ceil(4)
                .checked_mul(height.div_ceil(4))?
                .checked_mul(bytes)
        };
        match self {
            Self::Alpha8 => Some(px),
            Self::Argb4444 | Self::Rgb565 | Self::R16 | Self::Rgba4444 => px.checked_mul(2),
            Self::Rgb24 => px.checked_mul(3),
            Self::Rgba32 | Self::Argb32 | Self::Bgra32 => px.checked_mul(4),
            Self::Dxt1 => block(8),
            Self::Dxt3 | Self::Dxt5 => block(16),
            Self::Other(_) => None,
        }
    }
}

/// A `Texture2D`: format, size and the raw mip chain, undecoded.
#[derive(Debug, Clone)]
pub struct Texture2d {
    /// Its name (empty for generated textures).
    pub name: String,
    /// Width in texels.
    pub width: u32,
    /// Height in texels.
    pub height: u32,
    /// The pixel format.
    pub format: TextureFormat,
    /// Mip levels stored.
    pub mip_count: u32,
    /// All mip levels, largest first, bottom row first, as Unity stores them.
    /// Empty while [`Self::stream`] is set and not yet filled.
    pub data: Vec<u8>,
    /// Where the pixels live when they are not inline (a `.resS` node of the
    /// same bundle). [`crate::assets::Assets::texture`] fills `data` from it.
    pub stream: Option<StreamRef>,
}

/// A byte range of a bundle node holding pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamRef {
    /// The stream path, such as `archive:/CAB-xxxx/CAB-xxxx.resS`.
    pub path: String,
    /// Offset into the node.
    pub offset: u64,
    /// Length.
    pub size: u64,
}

/// The most texels per side accepted.
const MAX_SIDE: u32 = 1 << 15;

impl Texture2d {
    /// Whether it holds no pixels (Unity's dynamic-font atlas is 0 by 0).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Reads a `Texture2D` value. Refuses an external `.resS` stream, a format
    /// with no size rule, and any image whose byte length is not exactly its
    /// mip chain.
    pub fn from_value(v: &Value) -> Result<Self> {
        const C: &str = "Texture2D";
        let to_u32 = |x: i64| u32::try_from(x).map_err(|_| Error::OutOfRange("texture size"));
        let width = to_u32(int(v, C, "m_Width")?)?;
        let height = to_u32(int(v, C, "m_Height")?)?;
        let format_id = i32::try_from(int(v, C, "m_TextureFormat")?)
            .map_err(|_| Error::OutOfRange("texture format"))?;
        let mip_count = to_u32(int(v, C, "m_MipCount")?)?;
        let data = field(v, C, "image data")?
            .as_bytes()
            .ok_or(Error::Field {
                class: C,
                field: "image data",
            })?
            .to_vec();
        let sd = field(v, C, "m_StreamData")?;
        let stream = if uint(sd, C, "size")? != 0 {
            if !data.is_empty() {
                return Err(Error::Malformed(
                    "texture has both inline and streamed pixels",
                ));
            }
            Some(StreamRef {
                path: text(sd, C, "path")?,
                offset: uint(sd, C, "offset")?,
                size: uint(sd, C, "size")?,
            })
        } else {
            None
        };
        let name = text(v, C, "m_Name")?;
        let t = Self {
            name,
            width,
            height,
            format: TextureFormat::from_id(format_id),
            mip_count,
            data,
            stream,
        };
        if t.stream.is_some() {
            t.check_shape()?;
            return Ok(t);
        }
        t.validate()?;
        Ok(t)
    }

    /// Supplies the streamed pixels and checks them against the mip chain.
    pub fn fill_from_stream(&mut self, bytes: &[u8]) -> Result<()> {
        if self.stream.is_none() {
            return Err(Error::Malformed("texture has no stream to fill from"));
        }
        self.data = bytes.to_vec();
        self.validate()
    }

    fn check_shape(&self) -> Result<()> {
        if self.is_empty() {
            return Err(Error::Malformed("an empty texture has a stream"));
        }
        if self.width > MAX_SIDE
            || self.height > MAX_SIDE
            || self.mip_count == 0
            || self.mip_count > 32
        {
            return Err(Error::OutOfRange("texture size or mip count"));
        }
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        let t = self;
        let (width, height, mip_count) = (t.width, t.height, t.mip_count);
        if t.is_empty() {
            if !t.data.is_empty() {
                return Err(Error::Malformed("an empty texture carries pixels"));
            }
            return Ok(());
        }
        self.check_shape()?;
        let mut total = 0usize;
        for level in 0..mip_count {
            let (w, h) = (
                (width >> level).max(1) as usize,
                (height >> level).max(1) as usize,
            );
            total = t
                .format
                .level_len(w, h)
                .and_then(|n| total.checked_add(n))
                .ok_or(match t.format {
                    TextureFormat::Other(id) => Error::UnsupportedTextureFormat(id),
                    _ => Error::OutOfRange("texture mip chain size"),
                })?;
        }
        if total != t.data.len() {
            return Err(Error::Malformed(
                "texture bytes differ from the mip chain length",
            ));
        }
        Ok(())
    }

    /// Decodes an uncompressed format to RGBA8, top row first. `None` for a
    /// block-compressed format, which a caller decodes itself.
    #[must_use]
    pub fn base_level_rgba8(&self) -> Option<Vec<u8>> {
        if self.is_empty() {
            return Some(Vec::new());
        }
        let (w, h) = (self.width as usize, self.height as usize);
        let n = w * h;
        let src = self.data.get(..self.format.level_len(w, h)?)?;
        let mut out = Vec::with_capacity(n * 4);
        for row in (0..h).rev() {
            let r = &src[row * (src.len() / h.max(1))..(row + 1) * (src.len() / h.max(1))];
            match self.format {
                TextureFormat::Rgba32 => out.extend_from_slice(r),
                TextureFormat::Argb32 => r
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .for_each(|p| out.extend_from_slice(&[p[1], p[2], p[3], p[0]])),
                TextureFormat::Bgra32 => r
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .for_each(|p| out.extend_from_slice(&[p[2], p[1], p[0], p[3]])),
                TextureFormat::Rgb24 => r
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .for_each(|p| out.extend_from_slice(&[p[0], p[1], p[2], 255])),
                TextureFormat::Alpha8 => r
                    .iter()
                    .for_each(|&a| out.extend_from_slice(&[255, 255, 255, a])),
                _ => return None,
            }
        }
        Some(out)
    }
}
