//! The lobby's ban list for players who host: a text file the operator edits, read again
//! when it changes
//! (`docs/architecture/adr/0092-a-player-hosted-race-is-a-room-whose-owner-runs-the-race-host.md`,
//! decision 6).
//!
//! One ban a line, `key <64 hex digits> [date] [reason]` or `addr <ip address> [date]
//! [reason]`; blank lines and lines starting with `#` are skipped. The date is when the ban was
//! made, a UTC stamp as the log writes one (`2026-11-18T12:34:56.789Z`, `oag_log::stamp`), so a
//! list can be pruned or bans given a lifetime later; a line without one is a ban of unknown
//! date. The lobby writes the date on every ban it makes itself. A line that does not parse is
//! reported and skipped, never a reason to stop serving.

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

/// One ban: why, and since when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ban {
    /// Why, as the player is told.
    pub reason: String,
    /// When the ban was made (`oag_log::stamp`), `None` for a line that gives no date.
    pub since: Option<String>,
}

impl Ban {
    /// A ban made now, for `reason`.
    #[must_use]
    pub fn now(reason: &str) -> Self {
        Self {
            reason: reason.replace(['\n', '\r'], " "),
            since: Some(oag_log::stamp::format(SystemTime::now())),
        }
    }

    /// The ban's line after its subject: the date if known, then the reason.
    fn tail(&self) -> String {
        match &self.since {
            Some(since) => format!("{since} {}", self.reason),
            None => self.reason.clone(),
        }
    }
}

/// The parsed list.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Bans {
    keys: Vec<([u8; 32], Ban)>,
    addrs: Vec<(IpAddr, Ban)>,
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
            let rest = words.next().map_or("", str::trim);
            let (since, rest) = match rest.split_once(char::is_whitespace) {
                Some((first, more)) if is_stamp(first) => (Some(first.to_string()), more.trim()),
                None if is_stamp(rest) => (Some(rest.to_string()), ""),
                _ => (None, rest),
            };
            let reason = Some(rest)
                .filter(|reason| !reason.is_empty())
                .unwrap_or(NO_REASON)
                .to_string();
            let reason = Ban { reason, since };
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
            .map(|(_, ban)| ban.reason.as_str())
            .or_else(|| {
                self.addrs
                    .iter()
                    .find(|(banned, _)| *banned == addr)
                    .map(|(_, ban)| ban.reason.as_str())
            })
    }

    /// The ban on `key`, if there is one: its reason and date.
    #[must_use]
    pub fn key_ban(&self, key: &[u8; 32]) -> Option<&Ban> {
        self.keys
            .iter()
            .find(|(banned, _)| banned == key)
            .map(|(_, ban)| ban)
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

/// Whether `word` is a ban's date: a log stamp and nothing more.
fn is_stamp(word: &str) -> bool {
    word.len() == oag_log::stamp::WIDTH && oag_log::stamp::leads(word)
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
    /// Keys the lobby banned itself, on a proof: kept whatever the file says.
    proven: Vec<([u8; 32], Ban)>,
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
            proven: Vec::new(),
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
        let mut bans = read(&path);
        for (key, ban) in &self.proven {
            if !bans.keys.iter().any(|(k, _)| k == key) {
                bans.keys.push((*key, ban.clone()));
            }
        }
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

impl BanFile {
    /// Bans `key` from hosting for `reason` from now, and after a restart: the line, dated, is
    /// appended to the file when there is one (the operator may delete it to lift the ban).
    pub fn ban(&mut self, key: [u8; 32], reason: &str) {
        let ban = Ban::now(reason);
        if !self.bans.keys.iter().any(|(k, _)| *k == key) {
            self.bans.keys.push((key, ban.clone()));
        }
        let line = format!("key {} {}\n", crate::wt::hex(&key), ban.tail());
        self.proven.push((key, ban));
        let Some(path) = &self.path else {
            return;
        };
        let written = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()));
        if let Err(e) = written {
            log::warn!("lobby: could not add the ban to {}: {e}", path.display());
        }
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
        assert_eq!(
            bans.key_ban(&[0xAB; 32]).and_then(|b| b.since.clone()),
            None
        );
    }

    #[test]
    fn a_ban_names_its_date_and_the_lobby_writes_it_on_its_own_bans() {
        let key = "cd".repeat(32);
        let text = format!(
            "key {key} 2026-11-18T09:15:00.000Z forged an input\naddr 192.0.2.7 \
             2026-11-17T00:00:00.000Z\n"
        );
        let (bans, skipped) = Bans::parse(&text);
        assert!(skipped.is_empty(), "{skipped:?}");
        let ban = bans.key_ban(&[0xCD; 32]).expect("the key is banned");
        assert_eq!(ban.reason, "forged an input");
        assert_eq!(ban.since.as_deref(), Some("2026-11-18T09:15:00.000Z"));
        assert_eq!(
            bans.banned(&[1; 32], "192.0.2.7".parse().expect("addr")),
            Some(NO_REASON),
            "a date with no reason"
        );
        let dir = std::env::temp_dir().join(format!("oag-ban-date-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("bans.txt");
        let _ = std::fs::remove_file(&path);
        let mut file = BanFile::new(Some(path.clone()));
        file.ban([7; 32], "altered the race");
        let written = std::fs::read_to_string(&path).expect("the ban was written");
        let (again, skipped) = Bans::parse(&written);
        assert!(skipped.is_empty(), "{written}");
        let ban = again.key_ban(&[7; 32]).expect("read back");
        assert_eq!(ban.reason, "altered the race");
        assert!(ban.since.as_deref().is_some_and(is_stamp), "{written}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
