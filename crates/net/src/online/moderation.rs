//! The lobby's ban list for players who host: a text file the operator edits, read again
//! when it changes
//! (`docs/architecture/adr/0092-a-player-hosted-race-is-a-room-whose-owner-runs-the-race-host.md`,
//! decision 6).
//!
//! One ban a line, `key <64 hex digits> [reason]` or `addr <ip address> [reason]`; blank lines
//! and lines starting with `#` are skipped. A line that does not parse is reported and skipped,
//! never a reason to stop serving.

use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use web_time::{Duration, Instant};

/// How often the file is looked at for a change. Chosen.
const RELOAD_EVERY: Duration = Duration::from_secs(2);

/// The most lines a ban file is read for. Chosen: far past any list kept by hand.
const MAX_BANS: usize = 10_000;

/// The reason shown when a ban line gives none.
const NO_REASON: &str = "banned by the operator";

/// The parsed list.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Bans {
    keys: Vec<([u8; 32], String)>,
    addrs: Vec<(IpAddr, String)>,
}

impl Bans {
    /// Parses a ban file's text. Returns the list and one line per entry that was skipped.
    #[must_use]
    pub fn parse(text: &str) -> (Self, Vec<String>) {
        let mut bans = Self::default();
        let mut skipped = Vec::new();
        for (number, line) in text.lines().enumerate().take(MAX_BANS) {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut words = line.splitn(3, char::is_whitespace);
            let (kind, subject) = (words.next().unwrap_or(""), words.next().unwrap_or(""));
            let reason = words
                .next()
                .map(str::trim)
                .filter(|reason| !reason.is_empty())
                .unwrap_or(NO_REASON)
                .to_string();
            match kind {
                "key" => match crate::wt::parse_hash(subject) {
                    Ok(key) => bans.keys.push((key, reason)),
                    Err(_) => skipped.push(format!("line {}: not a 64-digit key", number + 1)),
                },
                "addr" => match subject.parse::<IpAddr>() {
                    Ok(addr) => bans.addrs.push((canonical(addr), reason)),
                    Err(_) => skipped.push(format!("line {}: not an address", number + 1)),
                },
                _ => skipped.push(format!("line {}: expected `key` or `addr`", number + 1)),
            }
        }
        (bans, skipped)
    }

    /// Why this key or address may not host, if it may not.
    #[must_use]
    pub fn banned(&self, key: &[u8; 32], addr: IpAddr) -> Option<&str> {
        let addr = canonical(addr);
        self.keys
            .iter()
            .find(|(banned, _)| banned == key)
            .map(|(_, reason)| reason.as_str())
            .or_else(|| {
                self.addrs
                    .iter()
                    .find(|(banned, _)| *banned == addr)
                    .map(|(_, reason)| reason.as_str())
            })
    }

    /// How many bans the list holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.keys.len() + self.addrs.len()
    }

    /// Whether the list is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An IPv4 address a dual-stack socket reports as `::ffff:a.b.c.d` is that IPv4 address.
pub(crate) fn canonical(addr: IpAddr) -> IpAddr {
    match addr {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(addr, IpAddr::V4),
        IpAddr::V4(_) => addr,
    }
}

/// A ban file on disk, read again when its modification time moves.
#[derive(Debug)]
pub struct BanFile {
    path: Option<PathBuf>,
    bans: Bans,
    /// The file's modification time and length when it was last read.
    seen: Option<(SystemTime, u64)>,
    looked: Option<Instant>,
}

impl BanFile {
    /// The list at `path`, or an empty one that never changes for `None`. A file that does not
    /// exist yet is an empty list, and is read once it appears.
    #[must_use]
    pub fn new(path: Option<PathBuf>) -> Self {
        let mut file = Self {
            path,
            bans: Bans::default(),
            seen: None,
            looked: None,
        };
        file.refresh();
        file
    }

    /// The list as last read.
    #[must_use]
    pub fn bans(&self) -> &Bans {
        &self.bans
    }

    /// Looks at the file if it is due; whether the list changed.
    pub fn refresh(&mut self) -> bool {
        let Some(path) = self.path.clone() else {
            return false;
        };
        if self.looked.is_some_and(|at| at.elapsed() < RELOAD_EVERY) {
            return false;
        }
        self.looked = Some(Instant::now());
        let modified = std::fs::metadata(&path)
            .and_then(|meta| Ok((meta.modified()?, meta.len())))
            .ok();
        if modified == self.seen {
            return false;
        }
        self.seen = modified;
        let bans = read(&path);
        let changed = bans != self.bans;
        self.bans = bans;
        if changed {
            log::info!(
                "lobby: {} bans read from {}",
                self.bans.len(),
                path.display()
            );
        }
        changed
    }
}

fn read(path: &Path) -> Bans {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Bans::default();
    };
    let (bans, skipped) = Bans::parse(&text);
    for line in skipped.iter().take(8) {
        log::warn!("lobby: {}: {line}", path.display());
    }
    bans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ban_file_names_keys_and_addresses_with_their_reasons_and_skips_what_it_cannot_read() {
        let key = "ab".repeat(32);
        let text = format!(
            "# the list\n\nkey {key} altered the race\naddr 192.0.2.7\naddr ::ffff:192.0.2.9 v4 in v6\n\
             key short\nnonsense here\n"
        );
        let (bans, skipped) = Bans::parse(&text);
        assert_eq!(bans.len(), 3);
        assert_eq!(skipped.len(), 2, "{skipped:?}");
        let free: IpAddr = "198.51.100.1".parse().expect("addr");
        assert_eq!(bans.banned(&[0xAB; 32], free), Some("altered the race"));
        assert_eq!(
            bans.banned(&[1; 32], "192.0.2.7".parse().expect("addr")),
            Some(NO_REASON)
        );
        assert_eq!(
            bans.banned(&[1; 32], "192.0.2.9".parse().expect("addr")),
            Some("v4 in v6")
        );
        assert_eq!(bans.banned(&[1; 32], free), None);
    }
}
