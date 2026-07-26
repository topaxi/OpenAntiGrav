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
    /// Recognised as an ISO 9660 volume, but not as a console we handle.
    Unknown,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Psp => "PSP",
            Self::Ps2 => "PS2",
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
fn normalise_serial(raw: &str) -> String {
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
    }
}
