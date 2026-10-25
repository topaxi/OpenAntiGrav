//! Read cstring for Unity formats with [`oag_bytes::Reader`].

use oag_bytes::{Endian, Reader};

use crate::error::{Error, Result};

impl From<oag_bytes::Truncated> for Error {
    fn from(err: oag_bytes::Truncated) -> Self {
        Self::Truncated(err)
    }
}

pub trait ReaderExt<'a> {
    /// A NUL-terminated string (UTF-8 required).
    fn cstring(&mut self) -> Result<&'a str>;
}

impl<'a, E: Endian> ReaderExt<'a> for Reader<'a, E> {
    fn cstring(&mut self) -> Result<&'a str> {
        let at = self.position();
        let bytes = self.cstr()?;
        std::str::from_utf8(bytes).map_err(|_| {
            // Leave the position where the string started, as a failed read does.
            self.seek(at).expect("a position already read");
            Error::InvalidUtf8
        })
    }
}
