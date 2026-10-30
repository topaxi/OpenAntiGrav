//! A fixed line of text a guest formats a number or a time into, with no allocator:
//! `write!(line, ...)` through `core::fmt`, and the two formats a HUD wants.

use core::fmt;

use oag_ext_abi::MAX_HUD_TEXT;

/// Up to [`MAX_HUD_TEXT`] bytes of text, built with `write!`. A write that does not
/// fit is refused whole (`fmt::Error`) and the line keeps what it had.
#[derive(Debug, Clone, Copy)]
pub struct Line {
    bytes: [u8; MAX_HUD_TEXT],
    len: usize,
}

impl Default for Line {
    fn default() -> Self {
        Self::new()
    }
}

impl Line {
    /// An empty line.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: [0; MAX_HUD_TEXT],
            len: 0,
        }
    }

    /// The text so far.
    #[must_use]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }

    /// Appends `m:ss.t` for `ticks` at 60 Hz: minutes, seconds, tenths.
    ///
    /// # Errors
    ///
    /// `fmt::Error` when it does not fit.
    pub fn time(&mut self, ticks: u32) -> fmt::Result {
        let tenths = ticks * 10 / 60;
        fmt::Write::write_fmt(
            self,
            format_args!("{}:{:02}.{}", tenths / 600, tenths / 10 % 60, tenths % 10),
        )
    }

    /// Appends `value` in decimal.
    ///
    /// # Errors
    ///
    /// `fmt::Error` when it does not fit.
    pub fn number(&mut self, value: u32) -> fmt::Result {
        fmt::Write::write_fmt(self, format_args!("{value}"))
    }
}

impl fmt::Write for Line {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self.len + text.len();
        let Some(room) = self.bytes.get_mut(self.len..end) else {
            return Err(fmt::Error);
        };
        room.copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}
