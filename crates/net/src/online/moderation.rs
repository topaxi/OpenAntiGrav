//! The lobby's moderation file for players who host: a text file the operator edits and the
//! lobby appends to, read again when it changes
//! (`docs/architecture/adr/0092-a-player-hosted-race-is-a-room-whose-owner-runs-the-race-host.md`,
//! decisions 6 and 7; `docs/networking/online-lobby.md`, part 12).
//!
//! One entry a line; blank lines and lines starting with `#` are skipped. Every date is a UTC
//! stamp as the log writes one (`2026-11-18T12:34:56.789Z`, `oag_log::stamp`).
//!
//! - `key <64 hex digits> <since> [until <stamp>] [reason]`: the key may not host. Without
//!   `until` the ban is permanent (the form an operator writes by hand); with it, the ban ends
//!   then (the form the lobby writes).
//! - `addr <ip address> <since> [until <stamp>] [reason]`: the same for an address.
//! - `strike <host key> <when> race <16 hex digits> by <reporter key> [reason]`: one case the
//!   lobby proved against a host, in the race whose params hash starts with those digits,
//!   reported by that player.
//!
//! A proven case is a strike, not a ban: a key is banned once [`STRIKES_TO_BAN`] of its strikes
//! within [`STRIKE_WINDOW`] come from distinct races, each reported by a different player, and
//! the strikes a ban was made on do not count again. The n-th ban the lobby makes on a key lasts
//! [`FIRST_BAN`] doubled n - 1 times, at most [`LONGEST_BAN`], so the history is kept: an
//! expired ban stops refusing but still counts. A line that does not parse, a missing date
//! included, is reported by line number and skipped, never a ban and never a reason to stop.

use std::fmt;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use web_time::{Duration, Instant};

/// How often the file is looked at for a change. Chosen.
const RELOAD_EVERY: Duration = Duration::from_secs(2);

/// The most lines a moderation file is read for; the rest is reported and skipped. Chosen: a
/// proven case writes one line and a ban one more, so this is years of a busy lobby.
const MAX_LINES: usize = 100_000;

/// The reason shown when a ban line gives none.
const NO_REASON: &str = "banned by the operator";

/// The reason kept when a strike line gives none.
const NO_STRIKE_REASON: &str = "proven by the lobby";

/// Strikes that ban a key, counted over distinct races and distinct reporters. Chosen: two
/// players in two races agreeing is the least that rules out one player's grudge and one odd
/// race; one constant so it can be tightened.
pub const STRIKES_TO_BAN: usize = 2;

/// How far back a strike counts. Chosen: a week holds the races of a player who hosts a few
/// times a week, so a cheater who keeps cheating is reported twice, while two honest races a
/// month apart that some unknown fault made disagree do not add up to a ban.
pub const STRIKE_WINDOW: Duration = Duration::from_secs(7 * 24 * 3600);

/// How long a key's first ban lasts; each later one doubles it (ADR-0092, decision 6).
pub const FIRST_BAN: Duration = Duration::from_secs(3600);

/// The longest ban the lobby makes itself. Chosen: reached at the eleventh ban (1 h doubled
/// nine times is 21 days, ten times 42); past a month the key is as good as gone, and an
/// operator who wants it gone for good writes a line without `until`.
pub const LONGEST_BAN: Duration = Duration::from_secs(30 * 24 * 3600);

/// One ban: why, since when, and until when (`None`: permanent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ban {
    /// Why, as the player is told.
    pub reason: String,
    /// When the ban was made.
    pub since: SystemTime,
    /// When it ends; `None` for a ban that never does.
    pub until: Option<SystemTime>,
}

impl Ban {
    /// Whether the ban still refuses at `now`.
    #[must_use]
    pub fn active(&self, now: SystemTime) -> bool {
        self.until.is_none_or(|until| now < until)
    }

    /// The ban's line after its subject: the date, the end if it has one, then the reason.
    fn tail(&self) -> String {
        let until = self
            .until
            .map(|until| format!(" until {}", oag_log::stamp::format(until)))
            .unwrap_or_default();
        format!(
            "{}{until} {}",
            oag_log::stamp::format(self.since),
            one_line(&self.reason)
        )
    }
}

/// One case the lobby proved against a host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strike {
    /// The host's key.
    pub key: [u8; 32],
    /// When it was proven.
    pub at: SystemTime,
    /// The race, as [`race_id`] of its params hash.
    pub race: u64,
    /// The key of the player whose report proved it (all zeroes: a player with no key, and all
    /// of those count as one reporter).
    pub by: [u8; 32],
    /// What was proven.
    pub reason: String,
}

impl Strike {
    fn line(&self) -> String {
        format!(
            "strike {} {} race {:016x} by {} {}\n",
            crate::wt::hex(&self.key),
            oag_log::stamp::format(self.at),
            self.race,
            crate::wt::hex(&self.by),
            one_line(&self.reason)
        )
    }
}

/// A race's id on a strike line: the first eight bytes of its params hash, which carries the
/// seed the lobby drew, so two races differ even in the same room or across lobby restarts.
#[must_use]
pub fn race_id(params_hash: &[u8; 32]) -> u64 {
    let mut first = [0; 8];
    first.copy_from_slice(&params_hash[..8]);
    u64::from_be_bytes(first)
}

/// Why a key or address may not host, and for how long.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refused<'a> {
    /// The ban's reason.
    pub reason: &'a str,
    /// The time left; `None` for a permanent ban.
    pub left: Option<Duration>,
}

impl fmt::Display for Refused<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.left {
            Some(left) => write!(f, "{} ({} left)", self.reason, span(left)),
            None => f.write_str(self.reason),
        }
    }
}

/// A duration as a player reads it, rounded up to the minute: `59 min`, `2 h`, `3 d 4 h`.
#[must_use]
pub fn span(length: Duration) -> String {
    let minutes = length.as_secs().div_ceil(60).max(1);
    if minutes >= 24 * 60 {
        let hours = minutes.div_ceil(60);
        match hours % 24 {
            0 => format!("{} d", hours / 24),
            h => format!("{} d {h} h", hours / 24),
        }
    } else if minutes >= 60 {
        match minutes % 60 {
            0 => format!("{} h", minutes / 60),
            m => format!("{} h {m} min", minutes / 60),
        }
    } else {
        format!("{minutes} min")
    }
}

/// Where a host's key stands after its strikes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Standing {
    /// Strikes that count toward a ban: in the window, since the last ban, over distinct races
    /// and distinct reporters.
    pub counted: usize,
    /// The ban due now and its length, if one is.
    pub ban: Option<Duration>,
}

/// How long the lobby's `nth` ban on a key lasts (counted from 1).
#[must_use]
pub fn ban_length(nth: usize) -> Duration {
    let doublings = u32::try_from(nth.saturating_sub(1))
        .unwrap_or(u32::MAX)
        .min(20);
    FIRST_BAN.saturating_mul(1 << doublings).min(LONGEST_BAN)
}

/// The parsed file.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Bans {
    keys: Vec<([u8; 32], Ban)>,
    addrs: Vec<(IpAddr, Ban)>,
    strikes: Vec<Strike>,
}

enum Entry {
    Key([u8; 32], Ban),
    Addr(IpAddr, Ban),
    Strike(Strike),
}

impl Bans {
    /// Parses a moderation file's text. Returns the list and one line per entry that was
    /// skipped, naming the line and why.
    #[must_use]
    pub fn parse(text: &str) -> (Self, Vec<String>) {
        let mut bans = Self::default();
        let mut skipped = Vec::new();
        for (number, line) in text.lines().enumerate() {
            if number >= MAX_LINES {
                skipped.push(format!(
                    "line {}: past the {MAX_LINES} lines read, the rest is not read",
                    number + 1
                ));
                break;
            }
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            match parse_line(line) {
                Ok(Entry::Key(key, ban)) => bans.keys.push((key, ban)),
                Ok(Entry::Addr(addr, ban)) => bans.addrs.push((addr, ban)),
                Ok(Entry::Strike(strike)) => bans.strikes.push(strike),
                Err(why) => skipped.push(format!("line {}: {why}", number + 1)),
            }
        }
        (bans, skipped)
    }

    fn extend(&mut self, other: Self) {
        self.keys.extend(other.keys);
        self.addrs.extend(other.addrs);
        self.strikes.extend(other.strikes);
    }

    /// Why this key or address may not host at `now`, if it may not.
    #[must_use]
    pub fn banned(&self, key: &[u8; 32], addr: IpAddr, now: SystemTime) -> Option<Refused<'_>> {
        let addr = canonical(addr);
        let on_key = self.key_bans(key);
        let on_addr = self
            .addrs
            .iter()
            .filter(move |(a, _)| *a == addr)
            .map(|(_, b)| b);
        longest(on_key.chain(on_addr), now).map(|ban| Refused {
            reason: &ban.reason,
            left: ban
                .until
                .map(|until| until.duration_since(now).unwrap_or_default()),
        })
    }

    /// The ban on `key` that refuses at `now`, if there is one (a permanent one first, else the
    /// one that ends last).
    #[must_use]
    pub fn key_ban(&self, key: &[u8; 32], now: SystemTime) -> Option<&Ban> {
        longest(self.key_bans(key), now)
    }

    /// Every ban line on `key`, active or not, oldest first.
    pub fn key_bans<'a>(&'a self, key: &[u8; 32]) -> impl Iterator<Item = &'a Ban> + use<'a> {
        let key = *key;
        self.keys
            .iter()
            .filter(move |(k, _)| *k == key)
            .map(|(_, b)| b)
    }

    /// Every strike on `key`, oldest first.
    pub fn strikes<'a>(&'a self, key: &[u8; 32]) -> impl Iterator<Item = &'a Strike> + use<'a> {
        let key = *key;
        self.strikes.iter().filter(move |s| s.key == key)
    }

    /// Where `key` stands at `now`: the strikes that count, and the ban due if they reach
    /// [`STRIKES_TO_BAN`] and no ban refuses it already.
    #[must_use]
    pub fn standing(&self, key: &[u8; 32], now: SystemTime) -> Standing {
        let timed: Vec<&Ban> = self.key_bans(key).filter(|b| b.until.is_some()).collect();
        let last = timed.iter().map(|b| b.since).max();
        let from = now.checked_sub(STRIKE_WINDOW).unwrap_or(UNIX_EPOCH);
        let counting: Vec<(u64, [u8; 32])> = self
            .strikes(key)
            .filter(|s| s.at >= from && s.at <= now && last.is_none_or(|last| s.at > last))
            .map(|s| (s.race, s.by))
            .collect();
        let counted = distinct_pairs(&counting);
        let ban = (counted >= STRIKES_TO_BAN && self.key_ban(key, now).is_none())
            .then(|| ban_length(timed.len() + 1));
        Standing { counted, ban }
    }

    /// How many entries the file holds: bans and strikes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.keys.len() + self.addrs.len() + self.strikes.len()
    }

    /// Whether the file holds nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The ban among `bans` that refuses at `now` the longest.
fn longest<'a>(bans: impl Iterator<Item = &'a Ban>, now: SystemTime) -> Option<&'a Ban> {
    bans.filter(|ban| ban.active(now))
        .max_by_key(|ban| (ban.until.is_none(), ban.until))
}

/// The most strikes that pair a distinct race with a distinct reporter: a maximum matching
/// between races and reporters (Kuhn's augmenting paths), so the order the strikes came in does
/// not decide it.
fn distinct_pairs(strikes: &[(u64, [u8; 32])]) -> usize {
    let mut races: Vec<u64> = strikes.iter().map(|(race, _)| *race).collect();
    races.sort_unstable();
    races.dedup();
    let mut owner: Vec<([u8; 32], usize)> = Vec::new();
    let mut matched = 0;
    for race in 0..races.len() {
        let mut seen = vec![false; races.len()];
        if augment(race, &races, strikes, &mut owner, &mut seen) {
            matched += 1;
        }
    }
    matched
}

fn augment(
    race: usize,
    races: &[u64],
    strikes: &[(u64, [u8; 32])],
    owner: &mut Vec<([u8; 32], usize)>,
    seen: &mut [bool],
) -> bool {
    if seen[race] {
        return false;
    }
    seen[race] = true;
    for (_, by) in strikes.iter().filter(|(r, _)| *r == races[race]) {
        match owner.iter().position(|(reporter, _)| reporter == by) {
            None => {
                owner.push((*by, race));
                return true;
            }
            Some(at) => {
                let other = owner[at].1;
                if augment(other, races, strikes, owner, seen) {
                    owner[at].1 = race;
                    return true;
                }
            }
        }
    }
    false
}

/// The next whitespace-separated word of `text` and what follows it.
fn word(text: &str) -> (&str, &str) {
    let text = text.trim_start();
    text.split_at(text.find(char::is_whitespace).unwrap_or(text.len()))
}

fn reason_or(rest: &str, none: &str) -> String {
    Some(rest.trim())
        .filter(|reason| !reason.is_empty())
        .unwrap_or(none)
        .to_string()
}

fn parse_line(line: &str) -> Result<Entry, String> {
    let (kind, rest) = word(line);
    let (subject, rest) = word(rest);
    let (since, rest) = word(rest);
    let Some(since) = oag_log::stamp::parse(since) else {
        return Err(format!(
            "no date after the {kind} (a UTC stamp like 2026-11-18T12:34:56.000Z)"
        ));
    };
    let key = |text: &str, what: &str| {
        crate::wt::parse_hash(text).map_err(|_| format!("{what} is not a 64-digit key"))
    };
    match kind {
        "key" | "addr" => {
            let (until, rest) = match word(rest) {
                ("until", after) => {
                    let (stamp, after) = word(after);
                    let until = oag_log::stamp::parse(stamp)
                        .ok_or_else(|| "`until` is not followed by a UTC stamp".to_string())?;
                    (Some(until), after)
                }
                _ => (None, rest),
            };
            let ban = Ban {
                reason: reason_or(rest, NO_REASON),
                since,
                until,
            };
            if kind == "key" {
                Ok(Entry::Key(key(subject, "the key")?, ban))
            } else {
                let addr = subject
                    .parse::<IpAddr>()
                    .map_err(|_| "not an address".to_string())?;
                Ok(Entry::Addr(canonical(addr), ban))
            }
        }
        "strike" => {
            let host = key(subject, "the host's key")?;
            let (word_race, rest) = word(rest);
            let (race, rest) = word(rest);
            let race = (word_race == "race" && race.len() == 16)
                .then(|| u64::from_str_radix(race, 16).ok())
                .flatten()
                .ok_or_else(|| "a strike needs `race <16 hex digits>`".to_string())?;
            let (word_by, rest) = word(rest);
            let (by, rest) = word(rest);
            if word_by != "by" {
                return Err("a strike needs `by <reporter's key>` after its race".into());
            }
            Ok(Entry::Strike(Strike {
                key: host,
                at: since,
                race,
                by: key(by, "the reporter's key")?,
                reason: reason_or(rest, NO_STRIKE_REASON),
            }))
        }
        _ => Err("expected `key`, `addr` or `strike`".into()),
    }
}

fn one_line(text: &str) -> String {
    text.replace(['\n', '\r'], " ")
}

/// An IPv4 address a dual-stack socket reports as `::ffff:a.b.c.d` is that IPv4 address.
pub(crate) fn canonical(addr: IpAddr) -> IpAddr {
    match addr {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(addr, IpAddr::V4),
        IpAddr::V4(_) => addr,
    }
}

/// A moderation file on disk, read again when its modification time moves.
#[derive(Debug)]
pub struct BanFile {
    path: Option<PathBuf>,
    bans: Bans,
    /// The file's modification time and length when it was last read.
    seen: Option<(SystemTime, u64)>,
    looked: Option<Instant>,
    /// Lines the lobby made that are in no file (no path, or the write failed): read with the
    /// file every time, so they hold until a restart.
    unwritten: String,
}

impl BanFile {
    /// The file at `path`, or for `None` a list only the lobby's own entries change. A file
    /// that does not exist yet is an empty list, and is read once it appears.
    #[must_use]
    pub fn new(path: Option<PathBuf>) -> Self {
        let mut file = Self {
            path,
            bans: Bans::default(),
            seen: None,
            looked: None,
            unwritten: String::new(),
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
        bans.extend(Bans::parse(&self.unwritten).0);
        let changed = bans != self.bans;
        self.bans = bans;
        if changed {
            log::info!(
                "lobby: {} bans and strikes read from {}",
                self.bans.len(),
                path.display()
            );
        }
        changed
    }

    /// Records a proven case against a host and, if its strikes now reach [`STRIKES_TO_BAN`],
    /// bans its key for the next ban's length. Both lines are appended to the file when there is
    /// one, so they hold after a restart. Returns where the key stands.
    pub fn strike(&mut self, strike: &Strike, now: SystemTime) -> Standing {
        self.add(&strike.line());
        let standing = self.bans.standing(&strike.key, now);
        if let Some(length) = standing.ban {
            let ban = Ban {
                reason: format!(
                    "caught cheating in {} races, each reported by another player (the last: {})",
                    standing.counted, strike.reason
                ),
                since: now,
                until: Some(now + length),
            };
            self.add(&format!(
                "key {} {}\n",
                crate::wt::hex(&strike.key),
                ban.tail()
            ));
        }
        standing
    }

    /// One line the lobby made: into the list now, and onto the file.
    fn add(&mut self, line: &str) {
        self.bans.extend(Bans::parse(line).0);
        let Some(path) = &self.path else {
            self.unwritten.push_str(line);
            return;
        };
        let written = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()));
        if let Err(e) = written {
            log::warn!("lobby: could not add to {}: {e}", path.display());
            self.unwritten.push_str(line);
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
mod tests;
