//! The `TITLE_ID` out of a `PARAM.SFO`, the key-value file every PS3, Vita and
//! PS4 package carries beside its game data.
//!
//! A disc image names its release through its own volume (`UMD_DATA.BIN`,
//! `SYSTEM.CNF`, `PS3_DISC.SFB`); an extracted package has none of those, but it
//! keeps its `param.sfo`, which is what the console itself reads the release
//! from. Only the one field is read: this is an identifier, not an SFO parser.

use crate::platform::normalise_serial;

const MAGIC: &[u8; 4] = b"\0PSF";
const HEADER: usize = 20;
const INDEX_ENTRY: usize = 16;

fn le16(bytes: &[u8], at: usize) -> Option<usize> {
    let raw: [u8; 2] = bytes.get(at..at + 2)?.try_into().ok()?;
    Some(usize::from(u16::from_le_bytes(raw)))
}

fn le32(bytes: &[u8], at: usize) -> Option<usize> {
    let raw: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
    usize::try_from(u32::from_le_bytes(raw)).ok()
}

/// The raw string value of one key of a `PARAM.SFO`, or `None` for bytes that
/// are not an SFO, a missing key, or a value that is not UTF-8.
#[must_use]
pub fn text_field(sfo: &[u8], wanted: &str) -> Option<String> {
    if sfo.get(..4)? != MAGIC {
        return None;
    }
    let keys = le32(sfo, 8)?;
    let data = le32(sfo, 12)?;
    let count = le32(sfo, 16)?;
    for index in 0..count.min(256) {
        let entry = HEADER + index * INDEX_ENTRY;
        let key_start = keys.checked_add(le16(sfo, entry)?)?;
        let key = sfo.get(key_start..)?;
        let key = &key[..key.iter().position(|&b| b == 0)?];
        if key != wanted.as_bytes() {
            continue;
        }
        let length = le32(sfo, entry + 4)?;
        let value_start = data.checked_add(le32(sfo, entry + 12)?)?;
        let value = sfo.get(value_start..value_start.checked_add(length)?)?;
        let value = &value[..value.iter().position(|&b| b == 0).unwrap_or(value.len())];
        return std::str::from_utf8(value).ok().map(str::to_string);
    }
    None
}

/// The release a `PARAM.SFO` names, normalised to `AAAA-NNNNN`
/// (`PCSF00007` gives `PCSF-00007`), or `None` for bytes that are not an SFO or
/// carry no `TITLE_ID`.
#[must_use]
pub fn title_id(sfo: &[u8]) -> Option<String> {
    text_field(sfo, "TITLE_ID")
        .filter(|text| !text.is_empty())
        .map(|text| normalise_serial(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sfo(pairs: &[(&str, &str)]) -> Vec<u8> {
        let mut keys = Vec::new();
        let mut values = Vec::new();
        let mut index = Vec::new();
        for (key, value) in pairs {
            index.extend_from_slice(&u16::try_from(keys.len()).unwrap().to_le_bytes());
            index.extend_from_slice(&0x0204_u16.to_le_bytes());
            index.extend_from_slice(&u32::try_from(value.len() + 1).unwrap().to_le_bytes());
            index.extend_from_slice(&16_u32.to_le_bytes());
            index.extend_from_slice(&u32::try_from(values.len()).unwrap().to_le_bytes());
            keys.extend_from_slice(key.as_bytes());
            keys.push(0);
            values.extend_from_slice(value.as_bytes());
            values.push(0);
        }
        let key_table = HEADER + index.len();
        let data_table = key_table + keys.len();
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&0x101_u32.to_le_bytes());
        out.extend_from_slice(&u32::try_from(key_table).unwrap().to_le_bytes());
        out.extend_from_slice(&u32::try_from(data_table).unwrap().to_le_bytes());
        out.extend_from_slice(&u32::try_from(pairs.len()).unwrap().to_le_bytes());
        out.extend_from_slice(&index);
        out.extend_from_slice(&keys);
        out.extend_from_slice(&values);
        out
    }

    #[test]
    fn the_title_id_is_found_among_other_keys_and_normalised() {
        let bytes = sfo(&[("APP_VER", "01.04"), ("TITLE_ID", "PCSF00007")]);
        assert_eq!(title_id(&bytes).as_deref(), Some("PCSF-00007"));
    }

    #[test]
    fn something_that_is_not_an_sfo_or_has_no_title_id_names_nothing() {
        assert_eq!(title_id(b"not an sfo at all"), None);
        assert_eq!(title_id(&sfo(&[("APP_VER", "01.00")])), None);
        assert_eq!(title_id(&[]), None);
    }
}
