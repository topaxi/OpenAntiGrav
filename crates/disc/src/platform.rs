//! Identifies which console a disc image is for, and which title.
//!
//! This exists so nothing downstream has to infer the platform from a filename
//! or a file size. Both are guesses; the disc says so itself.
//!
//! - **PSP** UMDs carry `UMD_DATA.BIN` in the root, whose first field is the
//!   disc ID (for example `ULUS-10358`), and a `PSP_GAME` directory.
//! - **PS2** DVDs carry `SYSTEM.CNF`, an INI-ish file whose `BOOT2` line names
//!   the boot ELF, from which the serial is derived (for example
//!   `cdrom0:\SLES_557.12;1` gives `SLES-55712`).
//! - **PS3** BD-ROMs carry `PS3_DISC.SFB`, a small keyed table whose `TITLE_ID`
//!   field holds the serial already hyphenated (for example `BCES-00664`).
//!
//! Identification reads only the plain part of a PS3 disc, so an **encrypted**
//! image identifies exactly as well as a decrypted one: `PS3_DISC.SFB` is never
//! inside an encrypted region. `docs/formats/ps3-disc.md` covers what reading
//! the rest of such a disc needs, which is a key this crate never sees.

use crate::error::Result;
use crate::iso9660::Entry;
use crate::source::SectorSource;

/// Which console an image targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Sony PlayStation Portable (UMD).
    Psp,
    /// Sony PlayStation 2 (DVD).
    Ps2,
    /// Sony PlayStation 3 (BD-ROM).
    ///
    /// Identified, but nothing downstream can use one: no PS3 release ships a
    /// Wipeout Pulse archive or soundtrack, and a PS3 disc's assets are behind
    /// [encryption](https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/ps3-disc.md)
    /// this crate deliberately knows nothing about. Code that matches on a
    /// platform therefore groups `Ps3` with [`Platform::Unknown`] rather than
    /// with `Psp` or `Ps2` - it is a disc we can name, not one we can read.
    Ps3,
    /// Sony PlayStation Vita.
    ///
    /// **Never identified from a disc**, unlike the three above: a Vita title
    /// ships as a `.pkg`, and what this project reads is the decrypted package
    /// extracted to a directory. Nothing in that tree is an ISO 9660 volume, so
    /// [`identify`](crate::identify) never returns this - it reaches
    /// `oag_assets::Layout` through the archive candidate that matched, which
    /// is the same path an extracted `USRDIR` with no `UMD_DATA.BIN` already
    /// took. See `oag_2048`.
    Vita,
    /// Sony PlayStation 4.
    ///
    /// **Never identified from a disc, on the same terms as [`Self::Vita`]**:
    /// Wipeout: Omega Collection ships as a `.pkg` pair (a base package plus a
    /// mandatory patch), decrypted and extracted to two sibling directories.
    /// Neither is an ISO 9660 volume, so [`identify`](crate::identify) never
    /// returns this either - it reaches `oag_assets::Layout` through the
    /// archive candidate that matched. See `oag_omega`.
    Ps4,
    /// Recognised as an ISO 9660 volume, but not as a console we handle.
    Unknown,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Psp => "PSP",
            Self::Ps2 => "PS2",
            Self::Ps3 => "PS3",
            Self::Vita => "Vita",
            Self::Ps4 => "PS4",
            Self::Unknown => "unknown",
        };
        f.write_str(s)
    }
}

/// What could be determined about the title on a disc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleInfo {
    /// The console the disc targets.
    pub platform: Platform,
    /// Disc serial, normalised to `AAAA-NNNNN`, if it could be read.
    pub serial: Option<String>,
    /// The boot executable path, if the disc names one.
    pub boot_path: Option<String>,
    /// The raw first line of the identifying file, kept for the record.
    pub raw: Option<String>,
}

/// Works out the platform and serial from the disc contents.
pub fn identify(source: &mut dyn SectorSource, entries: &[Entry]) -> Result<TitleInfo> {
    if let Some(info) = identify_psp(source, entries)? {
        return Ok(info);
    }
    if let Some(info) = identify_ps2(source, entries)? {
        return Ok(info);
    }
    if let Some(info) = identify_ps3(source, entries)? {
        return Ok(info);
    }
    Ok(TitleInfo {
        platform: Platform::Unknown,
        serial: None,
        boot_path: None,
        raw: None,
    })
}

fn find<'a>(entries: &'a [Entry], name: &str) -> Option<&'a Entry> {
    entries
        .iter()
        .find(|e| !e.is_directory && e.path.eq_ignore_ascii_case(name))
}

fn identify_psp(source: &mut dyn SectorSource, entries: &[Entry]) -> Result<Option<TitleInfo>> {
    let Some(umd_data) = find(entries, "UMD_DATA.BIN") else {
        return Ok(None);
    };

    // The file is small; cap the read anyway so a bogus size cannot ask for a
    // gigabyte.
    let data = source.read_range(umd_data.lba, umd_data.size.min(4096))?;
    let text = String::from_utf8_lossy(&data);
    let first_line = text.lines().next().unwrap_or_default().trim().to_string();

    // Format is `ULUS-10358|0123456789ABCDEF|0001|G`.
    let serial = first_line
        .split('|')
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(normalise_serial);

    let boot_path = ["PSP_GAME/SYSDIR/EBOOT.BIN", "PSP_GAME/SYSDIR/BOOT.BIN"]
        .into_iter()
        .find(|p| find(entries, p).is_some())
        .map(str::to_string);

    Ok(Some(TitleInfo {
        platform: Platform::Psp,
        serial,
        boot_path,
        raw: Some(first_line),
    }))
}

fn identify_ps2(source: &mut dyn SectorSource, entries: &[Entry]) -> Result<Option<TitleInfo>> {
    let Some(system_cnf) = find(entries, "SYSTEM.CNF") else {
        return Ok(None);
    };

    let data = source.read_range(system_cnf.lba, system_cnf.size.min(4096))?;
    let text = String::from_utf8_lossy(&data);

    // `BOOT2 = cdrom0:\SLES_557.12;1`
    let boot_line = text
        .lines()
        .map(str::trim)
        .find(|l| l.to_ascii_uppercase().starts_with("BOOT2"));

    let boot_value = boot_line
        .and_then(|l| l.split_once('='))
        .map(|(_, v)| v.trim().to_string());

    let serial = boot_value.as_deref().and_then(serial_from_boot_path);

    Ok(Some(TitleInfo {
        platform: Platform::Ps2,
        serial,
        boot_path: boot_value,
        raw: boot_line.map(str::to_string),
    }))
}

fn identify_ps3(source: &mut dyn SectorSource, entries: &[Entry]) -> Result<Option<TitleInfo>> {
    let Some(sfb) = find(entries, "PS3_DISC.SFB") else {
        return Ok(None);
    };

    let data = source.read_range(sfb.lba, sfb.size.min(4096))?;
    let raw = sfb_field(&data, "TITLE_ID");
    let serial = raw.as_deref().map(normalise_serial);

    let boot_path =
        find(entries, "PS3_GAME/USRDIR/EBOOT.BIN").map(|_| "PS3_GAME/USRDIR/EBOOT.BIN".to_string());

    Ok(Some(TitleInfo {
        platform: Platform::Ps3,
        serial,
        boot_path,
        raw,
    }))
}

/// Whether sector 0 of a PS3 image declares an encrypted region.
///
/// The table lists the **plain** spans as `first, last` pairs; every gap
/// between one pair and the next is encrypted (`docs/formats/ps3-disc.md`). A
/// decrypted image keeps the table unchanged, so this says "the disc declares
/// encrypted regions", not "this copy is still encrypted" - ask it only after
/// a read of the archives has already failed.
#[must_use]
pub fn ps3_declares_encrypted_regions(sector0: &[u8], sector_count: u32) -> bool {
    let Some(head) = sector0.get(..8) else {
        return false;
    };
    let plain = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as usize;
    if plain == 0 || plain > 64 {
        return false;
    }
    let Some(last) = sector0.get(8 + 4 * (plain * 2 - 1)..8 + 4 * plain * 2) else {
        return false;
    };
    let last = u32::from_be_bytes([last[0], last[1], last[2], last[3]]);
    plain > 1 || last.saturating_add(1) < sector_count
}

/// Reads one field out of a `PS3_DISC.SFB` table.
///
/// The file is magic `.SFB`, a version word, then 32-byte entries from `0x20`:
/// a 16-byte NUL-padded key, then the value's big-endian offset and length,
/// then padding. Values live past the entry table, at those offsets.
///
/// A reader this small lives here rather than in `oag-formats` because that
/// crate depends on this one - identification cannot reach forwards to it
/// without a cycle. It handles exactly the one field identification needs.
fn sfb_field(data: &[u8], key: &str) -> Option<String> {
    const HEADER: usize = 0x20;
    const ENTRY: usize = 0x20;
    const KEY_LEN: usize = 16;

    if data.len() < HEADER || &data[..4] != b".SFB" {
        return None;
    }

    let mut at = HEADER;
    while at + KEY_LEN + 8 <= data.len() {
        let name = &data[at..at + KEY_LEN];
        // The table ends at the first empty slot rather than at a count.
        if name[0] == 0 {
            return None;
        }
        if trim_nul(name) == key {
            let offset = u32::from_be_bytes(data[at + KEY_LEN..at + KEY_LEN + 4].try_into().ok()?);
            let len = u32::from_be_bytes(data[at + KEY_LEN + 4..at + KEY_LEN + 8].try_into().ok()?);
            let start = offset as usize;
            let end = start.checked_add(len as usize)?;
            let value = data.get(start..end.min(data.len()))?;
            let value = trim_nul(value);
            return (!value.is_empty()).then_some(value);
        }
        at += ENTRY;
    }
    None
}

/// Trims trailing NUL padding and surrounding whitespace from a fixed field.
fn trim_nul(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

/// Extracts `SLES-55712` from something like `cdrom0:\SLES_557.12;1`.
fn serial_from_boot_path(boot: &str) -> Option<String> {
    let file = boot
        .rsplit(['\\', '/', ':'])
        .next()?
        .split(';')
        .next()?
        .trim();
    if file.is_empty() {
        return None;
    }
    Some(normalise_serial(file))
}

/// Normalises a serial to the conventional `AAAA-NNNNN` form.
///
/// PS2 boot paths write it as `SLES_557.12`; UMD_DATA already uses `ULUS-10358`.
/// Normalising here means callers can compare serials without caring which
/// disc they came from.
pub(crate) fn normalise_serial(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();

    let split = cleaned
        .char_indices()
        .find(|(_, c)| c.is_ascii_digit())
        .map(|(i, _)| i);

    match split {
        Some(i) if i > 0 => format!("{}-{}", &cleaned[..i], &cleaned[i..]),
        _ => cleaned,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sector 0 with `bounds` as the plain-region table.
    fn region_sector(bounds: &[u32]) -> Vec<u8> {
        let mut sector = vec![0u8; 2048];
        sector[0..4].copy_from_slice(&((bounds.len() / 2) as u32).to_be_bytes());
        for (i, bound) in bounds.iter().enumerate() {
            sector[8 + 4 * i..12 + 4 * i].copy_from_slice(&bound.to_be_bytes());
        }
        sector
    }

    #[test]
    fn a_gap_in_the_plain_regions_is_an_encrypted_region() {
        // Three plain spans with gaps between them, as the shipped HD disc has.
        let table = region_sector(&[0, 0x75f, 0xecac0, 0xecb00, 0xed040, 0xed4df]);
        assert!(ps3_declares_encrypted_regions(&table, 0x113000));
        // One plain span covering the whole disc declares nothing encrypted.
        let plain = region_sector(&[0, 99]);
        assert!(!ps3_declares_encrypted_regions(&plain, 100));
        assert!(!ps3_declares_encrypted_regions(&[], 100));
    }

    #[test]
    fn normalises_a_ps2_boot_path() {
        assert_eq!(
            serial_from_boot_path(r"cdrom0:\SLES_557.12;1").as_deref(),
            Some("SLES-55712")
        );
        assert_eq!(
            serial_from_boot_path(r"cdrom0:\SCUS_971.13;1").as_deref(),
            Some("SCUS-97113")
        );
    }

    #[test]
    fn leaves_an_already_normalised_serial_alone() {
        assert_eq!(normalise_serial("ULUS-10358"), "ULUS-10358");
        assert_eq!(normalise_serial("ULES00929"), "ULES-00929");
    }

    #[test]
    fn handles_a_boot_path_with_no_serial() {
        assert_eq!(serial_from_boot_path("cdrom0:\\"), None);
    }

    #[test]
    fn platform_displays_readably() {
        assert_eq!(Platform::Psp.to_string(), "PSP");
        assert_eq!(Platform::Ps2.to_string(), "PS2");
        assert_eq!(Platform::Ps3.to_string(), "PS3");
    }

    /// Builds a `PS3_DISC.SFB` from nothing, laid out the way a real one is:
    /// magic, version, a 32-byte entry per field, then the values after the
    /// table. No disc content is involved.
    fn sfb(fields: &[(&str, &str)]) -> Vec<u8> {
        let mut out = vec![0u8; 0x200];
        out[..4].copy_from_slice(b".SFB");
        out[4..8].copy_from_slice(&0x0001_0000u32.to_be_bytes());

        for (i, (key, value)) in fields.iter().enumerate() {
            let at = 0x20 + i * 0x20;
            out[at..at + key.len()].copy_from_slice(key.as_bytes());
            let offset = u32::try_from(out.len()).expect("fixture fits");
            out[at + 16..at + 20].copy_from_slice(&offset.to_be_bytes());
            out[at + 20..at + 24].copy_from_slice(&0x20u32.to_be_bytes());

            let mut padded = vec![0u8; 0x20];
            padded[..value.len()].copy_from_slice(value.as_bytes());
            out.extend_from_slice(&padded);
        }
        out
    }

    #[test]
    fn reads_a_title_id_out_of_an_sfb() {
        let data = sfb(&[("HYBRID_FLAG", "gu"), ("TITLE_ID", "BCES-00664")]);
        assert_eq!(sfb_field(&data, "TITLE_ID").as_deref(), Some("BCES-00664"));
        assert_eq!(sfb_field(&data, "HYBRID_FLAG").as_deref(), Some("gu"));
        assert_eq!(sfb_field(&data, "NOT_A_FIELD"), None);
    }

    #[test]
    fn a_ps3_serial_survives_normalising() {
        // The SFB writes it hyphenated already, unlike a PS2 boot path.
        assert_eq!(normalise_serial("BCES-00664"), "BCES-00664");
    }

    #[test]
    fn refuses_anything_that_is_not_an_sfb() {
        assert_eq!(sfb_field(b"", "TITLE_ID"), None);
        assert_eq!(sfb_field(&[0u8; 0x400], "TITLE_ID"), None);
        assert_eq!(sfb_field(b".SFB not long enough", "TITLE_ID"), None);
    }

    #[test]
    fn a_field_pointing_past_the_end_yields_nothing_rather_than_panicking() {
        let mut data = sfb(&[("TITLE_ID", "BCES-00664")]);
        // Point the value at an offset far beyond the file.
        data[0x20 + 16..0x20 + 20].copy_from_slice(&0xffff_0000u32.to_be_bytes());
        assert_eq!(sfb_field(&data, "TITLE_ID"), None);
    }
}
