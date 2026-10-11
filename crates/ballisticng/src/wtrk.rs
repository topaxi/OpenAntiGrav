//! The `.wtrk` wrapper and the bare `.csf` bundle.
//!
//! Layout (measured on all 32 Workshop tracks):
//!
//! | What | Bytes |
//! | --- | --- |
//! | magic | `u8` length (26) then `"Workshop Track (Version 1)"` |
//! | table | `u32` entry count, each `u8` name length, name, `u64` absolute offset |
//! | value | at its offset: `u32` byte length, then the payload |
//!
//! Entries seen: `WorkshopID` (a `u64`), `WorkshopTitle` and
//! `WorkshopDescription` (a .NET length-prefixed string), `WorkshopVisibility`
//! (a `u64`), `WorkshopTags` (a `u32` count then that many strings),
//! `WorkshopPreview` (a string) and `TrackFile` (a `u32` bundle length, then the
//! `UnityFS` bundle). The bundle's length plus 4 is the entry's length and the
//! entry runs to the end of the file.

use crate::error::{Error, Result};

const MAGIC: &[u8] = b"Workshop Track (Version 1)";
const BUNDLE_SIGNATURE: &[u8] = b"UnityFS\0";

/// A Workshop track item: its metadata and where its bundle is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wtrk {
    /// The Steam Workshop id.
    pub workshop_id: u64,
    /// The author's title.
    pub title: String,
    /// The author's description.
    pub description: String,
    /// The visibility word, uninterpreted.
    pub visibility: u64,
    /// The tags, in file order (renderer names such as `DirectX` included).
    pub tags: Vec<String>,
    /// The author's local preview path (the image itself is not in the file).
    pub preview: String,
    /// Where the `UnityFS` bundle starts in the file.
    pub bundle_start: usize,
    /// How many bytes it runs.
    pub bundle_len: usize,
}

impl Wtrk {
    /// Whether the author tagged the item `tag` (compared exactly).
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// The bundle's bytes within `file`, the bytes this was parsed from.
    #[must_use]
    pub fn bundle<'a>(&self, file: &'a [u8]) -> &'a [u8] {
        &file[self.bundle_start..self.bundle_start + self.bundle_len]
    }
}

struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.data.len());
        let end = end.ok_or(Error::Wrapper("a field runs past the end of the file"))?;
        let s = &self.data[self.at..end];
        self.at = end;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("4 bytes"),
        ))
    }

    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("8 bytes"),
        ))
    }

    /// A .NET `BinaryWriter` string: a 7-bit-encoded length, then UTF-8.
    fn string(&mut self) -> Result<String> {
        let mut len = 0usize;
        for shift in (0..35).step_by(7) {
            let b = self.u8()?;
            len |= usize::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                let bytes = self.take(len)?;
                return String::from_utf8(bytes.to_vec())
                    .map_err(|_| Error::Wrapper("a string is not UTF-8"));
            }
        }
        Err(Error::Wrapper("a string length does not end"))
    }
}

impl Wtrk {
    /// Parses a `.wtrk`. Refuses by name anything but the one layout above:
    /// an unknown or repeated entry, a length that disagrees with the entry's,
    /// a bundle that does not start with the `UnityFS` signature.
    pub fn parse(file: &[u8]) -> Result<Self> {
        let mut c = Cursor { data: file, at: 0 };
        let magic_len = usize::from(c.u8()?);
        if c.take(magic_len)? != MAGIC {
            return Err(Error::Wrapper("not a Workshop Track (Version 1)"));
        }
        let count = c.u32()?;
        if count > 64 {
            return Err(Error::Wrapper("an implausible entry count"));
        }
        let mut entries: Vec<(String, usize)> = Vec::new();
        for _ in 0..count {
            let n = usize::from(c.u8()?);
            let name = String::from_utf8(c.take(n)?.to_vec())
                .map_err(|_| Error::Wrapper("an entry name is not UTF-8"))?;
            let offset = usize::try_from(c.u64()?)
                .map_err(|_| Error::Wrapper("an entry offset does not fit"))?;
            if entries.iter().any(|(e, _)| *e == name) {
                return Err(Error::Wrapper("an entry name repeats"));
            }
            entries.push((name, offset));
        }
        let table_end = c.at;

        let mut out = Self {
            workshop_id: 0,
            title: String::new(),
            description: String::new(),
            visibility: 0,
            tags: Vec::new(),
            preview: String::new(),
            bundle_start: 0,
            bundle_len: 0,
        };
        let mut seen = 0u32;
        for (i, (name, offset)) in entries.iter().enumerate() {
            if *offset < table_end {
                return Err(Error::Wrapper("an entry starts inside the table"));
            }
            let mut v = Cursor {
                data: file,
                at: *offset,
            };
            let len = v.u32()? as usize;
            let end =
                v.at.checked_add(len)
                    .filter(|&e| e <= file.len())
                    .ok_or(Error::Wrapper("an entry runs past the end of the file"))?;
            let next = entries.get(i + 1).map_or(file.len(), |(_, o)| *o);
            if end != next {
                return Err(Error::Wrapper(
                    "an entry does not end where the next starts",
                ));
            }
            let mut body = Cursor {
                data: &file[..end],
                at: v.at,
            };
            match name.as_str() {
                "WorkshopID" if len == 8 => out.workshop_id = body.u64()?,
                "WorkshopVisibility" if len == 8 => out.visibility = body.u64()?,
                "WorkshopTitle" => out.title = body.string()?,
                "WorkshopDescription" => out.description = body.string()?,
                "WorkshopPreview" => out.preview = body.string()?,
                "WorkshopTags" => {
                    let n = body.u32()?;
                    for _ in 0..n {
                        out.tags.push(body.string()?);
                    }
                }
                "TrackFile" => {
                    let inner = body.u32()? as usize;
                    if inner + 4 != len {
                        return Err(Error::Wrapper("the bundle length disagrees with its entry"));
                    }
                    if !file[body.at..end].starts_with(BUNDLE_SIGNATURE) {
                        return Err(Error::Wrapper("the track file is not a UnityFS bundle"));
                    }
                    out.bundle_start = body.at;
                    out.bundle_len = inner;
                }
                _ => return Err(Error::Wrapper("an entry this reader has no layout for")),
            }
            if name != "TrackFile" && body.at != end {
                return Err(Error::Wrapper("an entry has bytes left after its fields"));
            }
            seen += 1;
        }
        if seen != 7 || out.bundle_len == 0 {
            return Err(Error::Wrapper("an expected entry is missing"));
        }
        Ok(out)
    }
}

/// The bundle of a `.csf` ship: the whole file, once it is checked to start with
/// the `UnityFS` signature.
pub fn csf_bundle(file: &[u8]) -> Result<&[u8]> {
    if file.starts_with(BUNDLE_SIGNATURE) {
        Ok(file)
    } else {
        Err(Error::Wrapper("a .csf is not a bare UnityFS bundle"))
    }
}
