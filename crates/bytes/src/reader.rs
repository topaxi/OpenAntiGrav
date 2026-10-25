use std::marker::PhantomData;

/// A byte order as a value, for a format that names its own at run time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrder {
    Little,
    Big,
}

pub trait Endian {
    const ORDER: ByteOrder;

    fn read_u16(bytes: [u8; 2]) -> u16;
    fn read_u32(bytes: [u8; 4]) -> u32;
    fn read_u64(bytes: [u8; 8]) -> u64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Be;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Le;

impl Endian for Be {
    const ORDER: ByteOrder = ByteOrder::Big;

    fn read_u16(bytes: [u8; 2]) -> u16 {
        u16::from_be_bytes(bytes)
    }

    fn read_u32(bytes: [u8; 4]) -> u32 {
        u32::from_be_bytes(bytes)
    }

    fn read_u64(bytes: [u8; 8]) -> u64 {
        u64::from_be_bytes(bytes)
    }
}

impl Endian for Le {
    const ORDER: ByteOrder = ByteOrder::Little;

    fn read_u16(bytes: [u8; 2]) -> u16 {
        u16::from_le_bytes(bytes)
    }

    fn read_u32(bytes: [u8; 4]) -> u32 {
        u32::from_le_bytes(bytes)
    }

    fn read_u64(bytes: [u8; 8]) -> u64 {
        u64::from_le_bytes(bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("needed {wanted} bytes at offset {at}, {available} left")]
pub struct Truncated {
    /// The offset the read started at.
    pub at: usize,
    pub wanted: usize,
    /// Bytes left from `at` to the end.
    pub available: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VarintError {
    #[error(transparent)]
    Truncated(#[from] Truncated),
    /// Longer than ten bytes, or a tenth byte that overflows a `u64`.
    #[error("varint at offset {at} overflows a u64")]
    Overlong { at: usize },
}

impl VarintError {
    /// The offset the varint started at.
    #[must_use]
    pub fn at(&self) -> usize {
        match self {
            Self::Truncated(err) => err.at,
            Self::Overlong { at } => *at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Reader<'a, E> {
    data: &'a [u8],
    pos: usize,
    _endian: PhantomData<E>,
}

impl<'a> Reader<'a, Le> {
    #[must_use]
    pub fn le(data: &'a [u8]) -> Self {
        Self::new(data)
    }
}

impl<'a> Reader<'a, Be> {
    #[must_use]
    pub fn be(data: &'a [u8]) -> Self {
        Self::new(data)
    }
}

impl<'a, E: Endian> Reader<'a, E> {
    #[must_use]
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            _endian: PhantomData,
        }
    }

    /// The same input at the same position, read in another byte order.
    #[must_use]
    pub fn into_endian<F: Endian>(self) -> Reader<'a, F> {
        Reader {
            data: self.data,
            pos: self.pos,
            _endian: PhantomData,
        }
    }

    /// Whether every byte has been read.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    #[must_use]
    pub fn position(&self) -> usize {
        self.pos
    }

    #[must_use]
    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    /// The unread bytes, without consuming them.
    #[must_use]
    pub fn rest(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }

    fn truncated(&self, wanted: usize) -> Truncated {
        Truncated {
            at: self.pos,
            wanted,
            available: self.remaining(),
        }
    }

    /// Moves to an absolute offset, at most the length of the input.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when `pos` is past the end.
    pub fn seek(&mut self, pos: usize) -> Result<(), Truncated> {
        if pos > self.data.len() {
            return Err(self.truncated(pos - self.pos));
        }

        self.pos = pos;
        Ok(())
    }

    /// Ends the input at absolute offset `end`, so a read past it fails as at
    /// the end of the input.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when `end` is past the input, or before the position.
    pub fn limit(&mut self, end: usize) -> Result<(), Truncated> {
        if end > self.data.len() || end < self.pos {
            return Err(self.truncated(end.saturating_sub(self.pos)));
        }

        self.data = &self.data[..end];
        Ok(())
    }

    /// Skips `n` bytes.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when fewer than `n` remain.
    pub fn skip(&mut self, n: usize) -> Result<(), Truncated> {
        self.take(n).map(|_| ())
    }

    /// Skips to the next multiple of `n` from the start of the input. An `n`
    /// of zero, as a corrupt padding field may hold, skips nothing.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when the padding runs past the end.
    pub fn align(&mut self, n: usize) -> Result<(), Truncated> {
        match self.pos.checked_rem(n) {
            None | Some(0) => Ok(()),
            Some(rem) => self.skip(n - rem),
        }
    }

    /// The next `n` bytes.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when fewer than `n` remain.
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], Truncated> {
        if n > self.remaining() {
            return Err(self.truncated(n));
        }

        let out = &self.data[self.pos..self.pos + n];
        self.pos += n;

        Ok(out)
    }

    /// The next `N` bytes as an array.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when fewer than `N` remain.
    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], Truncated> {
        let mut out = [0; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    /// The bytes up to the next NUL, consuming the NUL and not returning it.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when no NUL remains.
    pub fn cstr(&mut self) -> Result<&'a [u8], Truncated> {
        let rest = self.rest();
        let Some(end) = rest.iter().position(|&b| b == 0) else {
            return Err(self.truncated(rest.len() + 1));
        };

        self.pos += end + 1;
        Ok(&rest[..end])
    }

    /// An unsigned LEB128 varint of at most ten bytes.
    ///
    /// # Errors
    ///
    /// [`VarintError::Truncated`] when the input ends inside it, measured from
    /// its first byte, or [`VarintError::Overlong`]. The position is left at
    /// the start of the varint either way.
    pub fn varint(&mut self) -> Result<u64, VarintError> {
        let start = self.pos;
        let mut value = 0u64;

        for shift in (0..70).step_by(7) {
            let Ok(byte) = self.u8() else {
                let wanted = self.pos - start + 1;
                self.pos = start;
                return Err(VarintError::Truncated(self.truncated(wanted)));
            };

            let bits = u64::from(byte & 0x7f);

            if shift == 63 && bits > 1 {
                break;
            }

            value |= bits << shift;

            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }

        self.pos = start;
        Err(VarintError::Overlong { at: start })
    }

    /// One byte.
    ///
    /// # Errors
    ///
    /// [`Truncated`] at the end of the input.
    pub fn u8(&mut self) -> Result<u8, Truncated> {
        Ok(self.take(1)?[0])
    }

    /// A `u16` in the reader's byte order.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when fewer than two bytes remain.
    pub fn u16(&mut self) -> Result<u16, Truncated> {
        Ok(E::read_u16(self.array()?))
    }

    /// An `i16` in the reader's byte order.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when fewer than two bytes remain.
    pub fn i16(&mut self) -> Result<i16, Truncated> {
        Ok(self.u16()?.cast_signed())
    }

    /// A `u32` in the reader's byte order.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when fewer than four bytes remain.
    pub fn u32(&mut self) -> Result<u32, Truncated> {
        Ok(E::read_u32(self.array()?))
    }

    /// An `i32` in the reader's byte order.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when fewer than four bytes remain.
    pub fn i32(&mut self) -> Result<i32, Truncated> {
        Ok(self.u32()?.cast_signed())
    }

    /// A `u64` in the reader's byte order.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when fewer than eight bytes remain.
    pub fn u64(&mut self) -> Result<u64, Truncated> {
        Ok(E::read_u64(self.array()?))
    }

    /// An `i64` in the reader's byte order.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when fewer than eight bytes remain.
    pub fn i64(&mut self) -> Result<i64, Truncated> {
        Ok(self.u64()?.cast_signed())
    }

    /// An `f32` in the reader's byte order.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when fewer than four bytes remain.
    pub fn f32(&mut self) -> Result<f32, Truncated> {
        Ok(f32::from_bits(self.u32()?))
    }

    /// An `f64` in the reader's byte order.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when fewer than eight bytes remain.
    pub fn f64(&mut self) -> Result<f64, Truncated> {
        Ok(f64::from_bits(self.u64()?))
    }
}

#[cfg(test)]
mod tests;
