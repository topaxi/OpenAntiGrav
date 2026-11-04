//! Controller rumble patterns: one [`Pattern`] per `Data/XML/Rumble/<name>.xml`.
//!
//! HD/Fury, 2048 and Omega ship 69 of them. A pattern is two byte tracks, one
//! byte per pad update, plus a few flags. The layout, the census and the traps
//! are on `docs/formats/rumble-xml.md`; what plays them is
//! `docs/gameplay/rumble.md`.
//!
//! This module is the XML front end and the in-memory form. The TOML front end,
//! which fills the same [`Pattern`], lives in `oag-mod` (this crate has no
//! dependencies).
//!
//! # A reader decodes or refuses by name
//!
//! Every shipped file parses. A file this reader does not understand is refused
//! with an [`Error`] naming the thing, not guessed at: an unknown element, an
//! unknown word for `Type`, `Class`, `Priority` or `Behaviour`, an `FPS` other
//! than 60, a track token that is not a byte, `FallOff*` on a `Global` pattern,
//! `LoopStart` or `LoopEnd` on a `OneShot`. The original's parser is more
//! forgiving (an unknown `Priority` word reads as High); none of that occurs on
//! a disc, so refusing costs nothing and shows drift.
//!
//! Tracks are tokenised on white space, so the trailing space every shipped
//! track ends in is not needed and a track without it keeps its last value.

use std::fmt;

use crate::fexml::{attributes, tag_end};

/// Frames per second every shipped pattern declares. Not read by the original;
/// a different value is refused.
pub const FPS: u32 = 60;

/// `Type`: whether a held voice wraps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Plays once from the first frame to the end of the longer track.
    OneShot,
    /// A held voice wraps from `loop_end` back to `loop_start`.
    Looping,
}

/// `Class`: which mask bit a voice is gated by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Class {
    /// In-race feel.
    Game,
    /// Menu and front-end feel.
    Ui,
}

/// `Priority`: what a full voice ring drops first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Priority {
    /// 0.
    Low,
    /// 1.
    Medium,
    /// 2.
    High,
}

/// The `Positional` behaviour's distances, in the world's unit, as the file
/// spells them. `inner` can exceed `outer`; the gain law is
/// `docs/gameplay/rumble.md`'s.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FallOff {
    /// `FallOffInner`.
    pub inner: f32,
    /// `FallOffOuter`.
    pub outer: f32,
}

/// One rumble pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    /// `Type`.
    pub kind: Kind,
    /// `LoopStart`; 0 when absent. Meaningful on [`Kind::Looping`] only.
    pub loop_start: u32,
    /// `LoopEnd`; 0 when absent. Meaningful on [`Kind::Looping`] only.
    pub loop_end: u32,
    /// `Class`, `None` where the file has none; see [`Pattern::effective_class`].
    pub class: Option<Class>,
    /// `Priority`, `None` where the file has none; see
    /// [`Pattern::effective_priority`].
    pub priority: Option<Priority>,
    /// `Behaviour`: `Some` is `Positional`, `None` is `Global`.
    pub positional: Option<FallOff>,
    /// `LargeActuator`: the large motor, one byte per frame.
    pub large: Vec<u8>,
    /// `SmallActuator`: the small motor, one byte per frame (only 0 and 1 ship).
    pub small: Vec<u8>,
}

impl Pattern {
    /// `Class` with the original's default for an absent element.
    #[must_use]
    pub fn effective_class(&self) -> Class {
        self.class.unwrap_or(Class::Game)
    }

    /// `Priority` with the original's default for an absent element.
    #[must_use]
    pub fn effective_priority(&self) -> Priority {
        self.priority.unwrap_or(Priority::Low)
    }

    /// Frames until the voice ends: the longer of the two tracks. A frame past
    /// the shorter track reads 0.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.large.len().max(self.small.len())
    }

    /// Checks the combinations the file format allows, shared by both front
    /// ends so they refuse the same things.
    ///
    /// # Errors
    ///
    /// [`Error::LoopOnOneShot`] for a loop range on a one-shot.
    pub fn validate(&self) -> Result<(), Error> {
        if self.kind == Kind::OneShot && (self.loop_start != 0 || self.loop_end != 0) {
            return Err(Error::LoopOnOneShot);
        }
        Ok(())
    }
}

/// Why a pattern was not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Not valid UTF-8.
    NotText,
    /// No `<RumbleApp>` root.
    NoRoot,
    /// No `<RumbleData>` element.
    NoData,
    /// An element inside `RumbleData` this reader does not know.
    UnknownElement(String),
    /// An element that occurs twice.
    Duplicate(&'static str),
    /// An element or attribute with text that is not in the format's set.
    UnknownValue {
        /// The element or attribute.
        field: &'static str,
        /// What the file says.
        value: String,
    },
    /// An element that holds other elements or is never closed.
    Malformed(&'static str),
    /// `FPS` other than 60.
    Fps(String),
    /// A track token that is not a whole number from 0 to 255.
    TrackValue {
        /// `LargeActuator` or `SmallActuator`.
        track: &'static str,
        /// The token.
        token: String,
    },
    /// `FallOffInner` or `FallOffOuter` on a `Global` pattern.
    FallOffOnGlobal,
    /// `LoopStart` or `LoopEnd` on a `OneShot`.
    LoopOnOneShot,
    /// A TOML front end's refusal, as that front end words it.
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotText => f.write_str("not valid UTF-8"),
            Self::NoRoot => f.write_str("no <RumbleApp> element"),
            Self::NoData => f.write_str("no <RumbleData> element"),
            Self::UnknownElement(name) => write!(f, "unknown element <{name}> in <RumbleData>"),
            Self::Duplicate(name) => write!(f, "<{name}> occurs twice"),
            Self::UnknownValue { field, value } => write!(f, "unknown {field} {value:?}"),
            Self::Malformed(name) => write!(f, "<{name}> is not a plain text element"),
            Self::Fps(value) => write!(f, "FPS {value:?}, only {FPS} is read"),
            Self::TrackValue { track, token } => {
                write!(f, "{track} holds {token:?}, not a byte")
            }
            Self::FallOffOnGlobal => f.write_str("FallOff attributes on a Global pattern"),
            Self::LoopOnOneShot => f.write_str("a loop range on a OneShot pattern"),
            Self::Other(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for Error {}

/// Reads one pattern file.
///
/// # Errors
///
/// An [`Error`] naming what is missing or not understood.
pub fn parse(data: &[u8]) -> Result<Pattern, Error> {
    let text = std::str::from_utf8(data).map_err(|_| Error::NotText)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let root = text.find("<RumbleApp").ok_or(Error::NoRoot)?;
    let body = &text[root..];
    let start = body.find("<RumbleData").ok_or(Error::NoData)?;
    let data = &body[start..];
    let open_end = tag_end(data).ok_or(Error::Malformed("RumbleData"))?;
    let inner_end = data
        .find("</RumbleData")
        .ok_or(Error::Malformed("RumbleData"))?;
    let mut rest = data
        .get(open_end + 1..inner_end)
        .ok_or(Error::Malformed("RumbleData"))?;

    let mut kind: Option<Kind> = None;
    let mut loop_range: Option<(u32, u32)> = None;
    let mut class = None;
    let mut priority = None;
    let mut behaviour: Option<bool> = None;
    let mut fps_seen = false;
    let mut large: Option<(Vec<u8>, Option<FallOff>)> = None;
    let mut small: Option<Vec<u8>> = None;

    while let Some(open) = rest.find('<') {
        rest = &rest[open..];
        let close = tag_end(rest).ok_or(Error::Malformed("RumbleData"))?;
        let tag = &rest[1..close];
        let self_closed = tag.ends_with('/');
        let tag = tag.trim_end_matches('/');
        let (name, attr_text) = tag.split_once(char::is_whitespace).unwrap_or((tag, ""));
        let attrs = attributes(attr_text);
        let after = &rest[close + 1..];
        let (value, next) = if self_closed {
            ("", after)
        } else {
            let end_tag = format!("</{name}");
            let end = after.find(&end_tag).ok_or(Error::Malformed("RumbleData"))?;
            let value = &after[..end];
            let tail = &after[end..];
            let tail_close = tail.find('>').ok_or(Error::Malformed("RumbleData"))?;
            (value, &tail[tail_close + 1..])
        };
        rest = next;
        let text_value = value.trim();

        match name {
            "FPS" => {
                if fps_seen {
                    return Err(Error::Duplicate("FPS"));
                }
                fps_seen = true;
                if text_value.parse::<u32>() != Ok(FPS) {
                    return Err(Error::Fps(text_value.to_owned()));
                }
            }
            "Type" => {
                if kind.is_some() {
                    return Err(Error::Duplicate("Type"));
                }
                kind = Some(match text_value {
                    "OneShot" => Kind::OneShot,
                    "Looping" => Kind::Looping,
                    other => return Err(unknown("Type", other)),
                });
                let get = |key: &'static str| -> Result<Option<u32>, Error> {
                    attrs
                        .iter()
                        .find(|(k, _)| k == key)
                        .map(|(_, v)| v.trim().parse().map_err(|_| unknown(key, v)))
                        .transpose()
                };
                let (start, end) = (get("LoopStart")?, get("LoopEnd")?);
                if start.is_some() || end.is_some() {
                    loop_range = Some((start.unwrap_or(0), end.unwrap_or(0)));
                }
                if let Some((key, _)) = attrs
                    .iter()
                    .find(|(k, _)| k != "LoopStart" && k != "LoopEnd")
                {
                    return Err(Error::UnknownElement(format!("Type {key}=")));
                }
            }
            "Class" => {
                if class.is_some() {
                    return Err(Error::Duplicate("Class"));
                }
                class = Some(match text_value {
                    "Game" => Class::Game,
                    "UI" => Class::Ui,
                    other => return Err(unknown("Class", other)),
                });
            }
            "Priority" => {
                if priority.is_some() {
                    return Err(Error::Duplicate("Priority"));
                }
                priority = Some(match text_value {
                    "Low" => Priority::Low,
                    "Medium" => Priority::Medium,
                    "High" => Priority::High,
                    other => return Err(unknown("Priority", other)),
                });
            }
            "Behaviour" => {
                if behaviour.is_some() {
                    return Err(Error::Duplicate("Behaviour"));
                }
                behaviour = Some(match text_value {
                    "Global" => false,
                    "Positional" => true,
                    other => return Err(unknown("Behaviour", other)),
                });
            }
            "LargeActuator" => {
                if large.is_some() {
                    return Err(Error::Duplicate("LargeActuator"));
                }
                let fall = |key: &'static str| -> Result<Option<f32>, Error> {
                    attrs
                        .iter()
                        .find(|(k, _)| k == key)
                        .map(|(_, v)| v.trim().parse().map_err(|_| unknown(key, v)))
                        .transpose()
                };
                let (inner, outer) = (fall("FallOffInner")?, fall("FallOffOuter")?);
                if let Some((key, _)) = attrs
                    .iter()
                    .find(|(k, _)| k != "FallOffInner" && k != "FallOffOuter")
                {
                    return Err(Error::UnknownElement(format!("LargeActuator {key}=")));
                }
                let fall_off = (inner.is_some() || outer.is_some()).then(|| FallOff {
                    inner: inner.unwrap_or(0.0),
                    outer: outer.unwrap_or(0.0),
                });
                large = Some((track("LargeActuator", value)?, fall_off));
            }
            "SmallActuator" => {
                if small.is_some() {
                    return Err(Error::Duplicate("SmallActuator"));
                }
                small = Some(track("SmallActuator", value)?);
            }
            other => return Err(Error::UnknownElement(other.to_owned())),
        }
    }

    let (large, fall_off) = large.unwrap_or_default();
    let positional = match (behaviour.unwrap_or(false), fall_off) {
        (true, fall) => Some(fall.unwrap_or_default()),
        (false, None) => None,
        (false, Some(_)) => return Err(Error::FallOffOnGlobal),
    };
    let kind = kind.unwrap_or(Kind::OneShot);
    let (loop_start, loop_end) = loop_range.unwrap_or((0, 0));
    let pattern = Pattern {
        kind,
        loop_start,
        loop_end,
        class,
        priority,
        positional,
        large,
        small: small.unwrap_or_default(),
    };
    pattern.validate()?;
    Ok(pattern)
}

fn unknown(field: &'static str, value: &str) -> Error {
    Error::UnknownValue {
        field,
        value: value.to_owned(),
    }
}

fn track(name: &'static str, text: &str) -> Result<Vec<u8>, Error> {
    text.split_ascii_whitespace()
        .map(|token| {
            token.parse::<u8>().map_err(|_| Error::TrackValue {
                track: name,
                token: token.to_owned(),
            })
        })
        .collect()
}

/// Patterns by name, in the order they were added. A `Vec`, not a fixed array:
/// a mod can add names and a stub that nothing loads still parses.
///
/// Names compare ASCII case-insensitively: the three titles spell the
/// directory differently (`Rumble`, `rumble`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Library {
    entries: Vec<(String, Pattern)>,
}

impl Library {
    /// An empty library.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `pattern` under `name`, replacing and returning a pattern of the
    /// same name (an override by name).
    pub fn insert(&mut self, name: &str, pattern: Pattern) -> Option<Pattern> {
        let name = name.to_ascii_lowercase();
        if let Some(slot) = self.entries.iter_mut().find(|(n, _)| *n == name) {
            return Some(std::mem::replace(&mut slot.1, pattern));
        }
        self.entries.push((name, pattern));
        None
    }

    /// The pattern called `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Pattern> {
        self.entries
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, p)| p)
    }

    /// Every pattern with its lower-cased name, in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Pattern)> {
        self.entries.iter().map(|(n, p)| (n.as_str(), p))
    }

    /// How many patterns.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The pattern name an archive path carries: the file stem of a path under a
/// `rumble` directory ending in `.xml` or `.toml`, in either case and with
/// either separator. `None` for anything else.
#[must_use]
pub fn pattern_name(path: &str) -> Option<(&str, &str)> {
    let folded = path.to_ascii_lowercase();
    let (dir, file) = folded.rsplit_once(['/', '\\'])?;
    if !(dir == "rumble" || dir.ends_with("/rumble") || dir.ends_with("\\rumble")) {
        return None;
    }
    let stem_len = file.rsplit_once('.')?.0.len();
    let ext = &file[stem_len + 1..];
    if ext != "xml" && ext != "toml" {
        return None;
    }
    let at = path.len() - file.len();
    Some((&path[at..at + stem_len], &path[at + stem_len + 1..]))
}

#[cfg(test)]
mod tests;
