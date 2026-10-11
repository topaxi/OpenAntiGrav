//! A bounds-checked cursor. Every read returns [`Error::Truncated`] rather than
//! indexing past the end.

use crate::error::{Error, Result};

/// Byte order of the integers a [`Reader`] returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    /// Most significant byte first (the UnityFS and SerializedFile headers).
    Big,
    /// Least significant byte first (most object data).
    Little,
}

/// A cursor over borrowed bytes.
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    endian: Endian,
}

macro_rules! num {
    ($name:ident, $ty:ty, $n:literal) => {
        #[doc = concat!("Reads a `", stringify!($ty), "`.")]
        pub fn $name(&mut self) -> Result<$ty> {
            let b = self.array::<$n>(stringify!($ty))?;
            Ok(match self.endian {
                Endian::Big => <$ty>::from_be_bytes(b),
                Endian::Little => <$ty>::from_le_bytes(b),
            })
        }
    };
}

impl<'a> Reader<'a> {
    /// A cursor at the start of `data`.
    #[must_use]
    pub fn new(data: &'a [u8], endian: Endian) -> Self {
        Self {
            data,
            pos: 0,
            endian,
        }
    }

    /// The current offset.
    #[must_use]
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Bytes left.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    /// The current byte order.
    #[must_use]
    pub fn endian(&self) -> Endian {
        self.endian
    }

    /// Switches byte order.
    pub fn set_endian(&mut self, endian: Endian) {
        self.endian = endian;
    }

    /// Moves to an absolute offset (at most the length).
    pub fn seek(&mut self, pos: usize) -> Result<()> {
        if pos > self.data.len() {
            return Err(Error::Truncated {
                what: "seek",
                at: self.pos,
                need: pos,
                have: self.data.len(),
            });
        }
        self.pos = pos;
        Ok(())
    }

    /// Takes the next `n` bytes.
    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        self.take(n, "bytes")
    }

    fn take(&mut self, n: usize, what: &'static str) -> Result<&'a [u8]> {
        if n > self.remaining() {
            return Err(Error::Truncated {
                what,
                at: self.pos,
                need: n,
                have: self.remaining(),
            });
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn array<const N: usize>(&mut self, what: &'static str) -> Result<[u8; N]> {
        let s = self.take(N, what)?;
        let mut a = [0u8; N];
        a.copy_from_slice(s);
        Ok(a)
    }

    /// Reads a `u8`.
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1, "u8")?[0])
    }

    num!(u16, u16, 2);
    num!(i16, i16, 2);
    num!(u32, u32, 4);
    num!(i32, i32, 4);
    num!(u64, u64, 8);
    num!(i64, i64, 8);
    num!(f32, f32, 4);
    num!(f64, f64, 8);

    /// A NUL-terminated string (UTF-8 required).
    pub fn cstr(&mut self) -> Result<&'a str> {
        let rest = &self.data[self.pos..];
        let end = rest.iter().position(|&b| b == 0).ok_or(Error::Truncated {
            what: "cstring",
            at: self.pos,
            need: rest.len() + 1,
            have: rest.len(),
        })?;
        let s = std::str::from_utf8(&rest[..end]).map_err(|_| Error::InvalidUtf8)?;
        self.pos += end + 1;
        Ok(s)
    }

    /// Skips to the next multiple of `n` (relative to the start of the data).
    pub fn align(&mut self, n: usize) -> Result<()> {
        let rem = self.pos % n;
        if rem != 0 {
            self.take(n - rem, "alignment")?;
        }
        Ok(())
    }
}
