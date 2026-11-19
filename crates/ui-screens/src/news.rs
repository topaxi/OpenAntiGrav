//! The footer ticker's message source: `Data\Plugins\news\Definition.xml`'s
//! `PI_News` items, the weighted draw among them and each item's message
//! (`docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`, "The footer
//! ticker's message choice").
//!
//! The items are the disc's own: 124 rows on Pulse PSP, 121 on PS2, each a
//! string id, a class (`GenStats`, `Profile`, `TeamStats`, `TrackStats`,
//! `Unlockable`, or none, which the original reads as `GenStats` member 0), a
//! member (the statistic's slot, or the unlock flag), a format and a
//! `NewsFrequency`. Nothing here transcribes them.
//!
//! **The draw.** An item's weight is `NewsFrequency + 1`; an `Unlockable` item
//! has none (the original shows those first, from unlock flags, never by
//! draw). One draw is `rand() % total`, the item whose range holds it, again
//! while it is the item drawn last, and a draw whose item has no message (a
//! statistic of 0) is followed by another. The generator is ours
//! ([`oag_core::Rng`]), since the original's is libc's `rand()` on the wall
//! clock.
//!
//! **What is not modelled:** the unlock announcements (`TKR_FYI*` from
//! `Unlockable` items), the `TeamStats` and `TrackStats` messages (no per-team
//! or per-circuit lap and time record is kept), and the profile favourites
//! this build has no counter for. Each answers "no message" and the draw
//! moves on, which is what the original does for an empty statistic.

use oag_core::Rng;
use oag_tables::fexml;
use oag_ui::language::StringTable;

/// How the statistic is printed (`NewsFormat`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    /// `%d` of the integer slot.
    #[default]
    Int,
    /// `%.2f` of the slot read as a float.
    Float,
    /// Seconds in an integer slot, printed as `H hours M mins` or `M mins S seconds`.
    Time,
    /// The same, off a float slot.
    FloatTime,
    /// The slot over `NewsMax / 100`.
    Percent,
    /// A string slot; the `GenStats` builder draws nothing for it.
    String,
}

/// Where the statistic lives (`NewsClass`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Class {
    /// The profile's lifetime statistics table; also what an item naming no class is.
    #[default]
    GenStats,
    /// A favourite (team, circuit, speed class, music track, event, weapon), as a string.
    Profile,
    /// One team's laps, time and loyalty.
    TeamStats,
    /// One circuit's laps and time.
    TrackStats,
    /// An unlock announcement; never drawn.
    Unlockable,
}

/// One `PI_News` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewsItem {
    /// The string-table id of the message (`NewsID`).
    pub id: String,
    pub format: Format,
    pub class: Class,
    /// The statistic's slot, or the unlock flag (`NewsMember`).
    pub member: u32,
    /// `NewsFrequency`: the draw weight less one.
    pub frequency: i32,
    /// `NewsMax`, the divisor of a [`Format::Percent`] item.
    pub max: i32,
}

impl NewsItem {
    /// The number of draw values the item owns: none for an unlock announcement.
    #[must_use]
    pub fn weight(&self) -> u32 {
        if self.class == Class::Unlockable {
            0
        } else {
            u32::try_from(self.frequency + 1).unwrap_or(0)
        }
    }
}

/// A title's items and the name its statistics are filed under.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Feed {
    /// `oag_title::Title::name`: whose lifetime table the items quote.
    pub title: String,
    pub items: Vec<NewsItem>,
}

/// Reads every `PI_News` row of a `Definition.xml`, in file order.
#[must_use]
pub fn parse(xml: &str) -> Vec<NewsItem> {
    let root = fexml::parse(xml);
    let mut items = Vec::new();
    collect(&root, &mut items);
    items
}

fn collect(node: &fexml::Node, items: &mut Vec<NewsItem>) {
    if node.name.eq_ignore_ascii_case("PI_News") {
        let Some(id) = node.value("NewsID") else {
            return;
        };
        let number = |name: &str| node.value(name).and_then(|v| v.trim().parse::<i32>().ok());
        items.push(NewsItem {
            id: id.to_string(),
            format: match node
                .value("NewsFormat")
                .map(str::to_ascii_lowercase)
                .as_deref()
            {
                Some("float") => Format::Float,
                Some("time") => Format::Time,
                Some("floattime") => Format::FloatTime,
                Some("percent") => Format::Percent,
                Some("string") => Format::String,
                _ => Format::Int,
            },
            class: match node
                .value("NewsClass")
                .map(str::to_ascii_lowercase)
                .as_deref()
            {
                Some("profile") => Class::Profile,
                Some("teamstats") => Class::TeamStats,
                Some("trackstats") => Class::TrackStats,
                Some("unlockable") => Class::Unlockable,
                _ => Class::GenStats,
            },
            member: number("NewsMember")
                .and_then(|m| u32::try_from(m).ok())
                .unwrap_or(0),
            frequency: number("NewsFrequency").unwrap_or(0),
            max: number("NewsMax").unwrap_or(0),
        });
        return;
    }
    for child in &node.children {
        collect(child, items);
    }
}

/// What the profile has to say: the lifetime statistics the messages quote.
pub trait Statistics {
    /// A `GenStats` slot; `0` for one nothing counts.
    fn general(&self, member: u32) -> f32;
    /// A `Profile` favourite, `None` for the original's "none".
    fn favourite(&self, member: u32) -> Option<String>;
}

/// A value a message template takes.
enum Arg {
    Int(i64),
    Float(f64),
    Text(String),
}

/// `printf` for the three conversions the ticker's templates use: `%d`, `%s`
/// and `%.2f`, plus `%%`. A conversion with no argument left prints nothing.
fn format_message(template: &str, args: &[Arg]) -> String {
    let mut out = String::new();
    let mut args = args.iter();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let mut spec = String::new();
        while let Some(&next) = chars.peek() {
            spec.push(next);
            chars.next();
            if next.is_ascii_alphabetic() || next == '%' {
                break;
            }
        }
        match spec.as_str() {
            "%" => out.push('%'),
            "d" => {
                if let Some(Arg::Int(v)) = args.next() {
                    out.push_str(&v.to_string());
                }
            }
            "s" => {
                if let Some(Arg::Text(v)) = args.next() {
                    out.push_str(v);
                }
            }
            ".2f" => {
                if let Some(Arg::Float(v)) = args.next() {
                    out.push_str(&format!("{v:.2}"));
                }
            }
            other => {
                out.push('%');
                out.push_str(other);
            }
        }
    }
    out
}

/// `3725` as `1 hour 2 mins` or, under an hour, `2 mins 5 seconds`: the
/// original's `%d %s %d %s` over the `MSC_SEC`, `MSC_MIN` and `MSC_HOUR`
/// strings, singular exactly when the count is 1.
fn duration(seconds: i64, strings: &StringTable) -> String {
    let seconds = if seconds == 0xffff { 0 } else { seconds };
    let minutes = (seconds / 60) % 60;
    let hours = seconds / 60 / 60;
    let word = |one: bool, singular: &str, plural: &str| {
        strings
            .get(if one { singular } else { plural })
            .unwrap_or_default()
            .to_string()
    };
    let sec_word = word(seconds % 60 == 1, "MSC_SEC", "MSC_SECONDS");
    let min_word = word(minutes == 1, "MSC_MIN", "MSC_MINS");
    let hour_word = word(hours == 1, "MSC_HOUR", "MSC_HOURS");
    if hours < 1 {
        format!("{minutes} {min_word} {} {sec_word}", seconds % 60)
    } else {
        format!("{hours} {hour_word} {minutes} {min_word}")
    }
}

/// The tip a time item with nothing counted falls back to: the six per-mode
/// time items each name the mode the player has not tried.
fn untried_tip(id: &str) -> Option<&'static str> {
    const TIPS: [(&str, &str); 6] = [
        ("TKR_SRTIM", "TKR_NOSR"),
        ("TKR_SLTIM", "TKR_NOSL"),
        ("TKR_TTTIM", "TKR_NOTT"),
        ("TKR_TOURTIM", "TKR_NOTOURN"),
        ("TKR_ELIMTIM", "TKR_NOELIM"),
        ("TKR_ZTIM", "TKR_NOZONE"),
    ];
    TIPS.iter()
        .find(|(item, _)| item.eq_ignore_ascii_case(id))
        .map(|(_, tip)| *tip)
}

/// The message `item` shows, or `None` when it has nothing to say (the
/// original's builder returns 0 and the draw goes again).
#[must_use]
pub fn message(item: &NewsItem, strings: &StringTable, stats: &dyn Statistics) -> Option<String> {
    let template = |id: &str| strings.get(id).map(str::to_string);
    let one = |arg: Arg| Some(format_message(&template(&item.id)?, &[arg]));
    match item.class {
        Class::GenStats => {
            let slot = stats.general(item.member);
            let count = slot as i64;
            match item.format {
                Format::String => None,
                Format::Float => (slot > 0.0)
                    .then(|| one(Arg::Float(f64::from(slot))))
                    .flatten(),
                Format::Time | Format::FloatTime if count < 1 => {
                    untried_tip(&item.id).and_then(template)
                }
                Format::Time | Format::FloatTime => one(Arg::Text(duration(count, strings))),
                Format::Percent => {
                    let divisor = (item.max / 100) as f32;
                    (count >= 1)
                        .then(|| one(Arg::Int((count as f32 / divisor) as i64)))
                        .flatten()
                }
                Format::Int => (count >= 1).then(|| one(Arg::Int(count))).flatten(),
            }
        }
        Class::Profile => one(Arg::Text(stats.favourite(item.member)?)),
        Class::TeamStats | Class::TrackStats | Class::Unlockable => None,
    }
}

/// The message of the item named `id`, for a capture that pins one.
#[must_use]
pub fn message_for(
    items: &[NewsItem],
    id: &str,
    strings: &StringTable,
    stats: &dyn Statistics,
) -> Option<String> {
    let item = items.iter().find(|item| item.id.eq_ignore_ascii_case(id))?;
    message(item, strings, stats)
}

/// `rand() % total` over the items' weight ranges, again while it lands on
/// `last`: the index of the next item, and the draw's own total.
fn draw(items: &[NewsItem], last: usize, rng: &mut Rng) -> usize {
    let total: u32 = items.iter().map(NewsItem::weight).sum();
    let weighted = items.iter().filter(|item| item.weight() > 0).count();
    if weighted < 2 {
        return items
            .iter()
            .position(|item| item.weight() > 0)
            .unwrap_or(last);
    }
    loop {
        let mut value = rng.below(total);
        let mut picked = usize::try_from(value).unwrap_or(0);
        for (index, item) in items.iter().enumerate() {
            let weight = item.weight();
            if value < weight {
                picked = index;
                break;
            }
            value -= weight;
        }
        if picked != last {
            return picked;
        }
    }
}

/// `count` messages in the order the ticker plays them: each a draw
/// ([`NewsItem::weight`]) that is not the one before and that has something to
/// say. Empty when no item ever does.
#[must_use]
pub fn sequence(
    items: &[NewsItem],
    strings: &StringTable,
    stats: &dyn Statistics,
    rng: &mut Rng,
    count: usize,
) -> Vec<String> {
    let mut out = Vec::new();
    if !items
        .iter()
        .any(|item| item.weight() > 0 && message(item, strings, stats).is_some())
    {
        return out;
    }
    let mut last = 0;
    while out.len() < count {
        last = draw(items, last, rng);
        if let Some(text) = message(&items[last], strings, stats) {
            out.push(text);
        }
    }
    out
}

#[cfg(test)]
mod tests;
