//! The TOML manifest: names, version, integer structure and the counts the glb
//! is checked against. Holds no `f32` the simulation reads (a decimal round trip
//! is not guaranteed bit-exact); a float that is not an integer lives in the glb.

use std::collections::BTreeMap;

use oag_vex::track::Junction;
use serde::{Deserialize, Serialize};

use crate::{Error, Track};

/// The format's name in `track.format`.
pub const FORMAT_NAME: &str = "oag-track";
/// The major this reader reads. A file of another major is refused by name.
pub const FORMAT_MAJOR: u32 = 1;
/// The minor this writer writes. Newer minors add optional keys, which a reader
/// of this major reports as unread rather than refusing.
pub const FORMAT_MINOR: u32 = 0;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Manifest {
    pub track: TrackTable,
    pub spline: Spline,
    pub counts: Counts,
    #[serde(flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TrackTable {
    pub format: String,
    pub version: String,
    pub id: String,
    pub title: String,
    pub glb: String,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Spline {
    pub point_version: u32,
    #[serde(default)]
    pub path: Vec<PathRow>,
    #[serde(default)]
    pub junction: Vec<JunctionRow>,
}

/// `entry` and `exit` are junction indices, `-1` for none (TOML has no null).
/// `max_spacing_bits` is the `f32`'s bit pattern: an integer, so exact.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct PathRow {
    pub points: usize,
    pub entry: i64,
    pub exit: i64,
    pub max_spacing_bits: u32,
}

/// Path indices, `-1` for none: primary then alternate.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct JunctionRow {
    pub prev: [i64; 2],
    pub next: [i64; 2],
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Counts {
    pub collision_nodes: usize,
    pub speedup_pads: usize,
    pub weapon_pads: usize,
}

/// What a manifest read produced for the glb half to finish.
#[derive(Debug)]
pub(crate) struct Parsed {
    pub glb: String,
    pub version: (u32, u32),
    pub id: String,
    pub title: String,
    pub notes: Vec<String>,
    pub point_version: u32,
    pub paths: Vec<PathRow>,
    pub junctions: Vec<Junction>,
    pub counts: Counts,
    pub unread: Vec<String>,
}

pub(crate) fn opt(v: i64) -> Option<usize> {
    usize::try_from(v).ok()
}

fn code(v: Option<usize>) -> i64 {
    v.map_or(-1, |i| i64::try_from(i).unwrap_or(i64::MAX))
}

pub(crate) fn write(track: &Track, glb_name: &str) -> Result<String, Error> {
    if track.id.is_empty() {
        return Err(Error::Unwritable("the track has no id".into()));
    }
    let manifest = Manifest {
        track: TrackTable {
            format: FORMAT_NAME.to_string(),
            version: format!("{FORMAT_MAJOR}.{FORMAT_MINOR}"),
            id: track.id.clone(),
            title: track.title.clone(),
            glb: glb_name.to_string(),
            notes: track.notes.clone(),
            extra: BTreeMap::new(),
        },
        spline: Spline {
            point_version: track.ai.version,
            path: track
                .ai
                .paths
                .iter()
                .map(|p| PathRow {
                    points: p.points.len(),
                    entry: code(p.entry),
                    exit: code(p.exit),
                    max_spacing_bits: p.max_spacing.to_bits(),
                })
                .collect(),
            junction: track
                .ai
                .junctions
                .iter()
                .map(|j| JunctionRow {
                    prev: j.prev.map(code),
                    next: j.next.map(code),
                })
                .collect(),
        },
        counts: Counts {
            collision_nodes: track.nodes.len(),
            speedup_pads: track.speedup_pads.len(),
            weapon_pads: track.weapon_pads.len(),
        },
        extra: BTreeMap::new(),
    };
    toml::to_string(&manifest).map_err(|e| Error::Unwritable(e.to_string()))
}

pub(crate) fn read(text: &str) -> Result<Parsed, Error> {
    let m: Manifest = toml::from_str(text).map_err(|e| Error::Manifest(e.to_string()))?;
    if m.track.format != FORMAT_NAME {
        return Err(Error::NotThisFormat(m.track.format));
    }
    let version = parse_version(&m.track.version)?;
    if version.0 != FORMAT_MAJOR {
        return Err(Error::UnknownMajor {
            found: m.track.version,
            major: version.0,
            supported: FORMAT_MAJOR,
        });
    }
    if m.track.id.is_empty() {
        return Err(Error::Manifest("track.id is empty".into()));
    }
    let mut unread: Vec<String> = m
        .extra
        .keys()
        .map(|k| format!("manifest key [{k}]"))
        .collect();
    unread.extend(
        m.track
            .extra
            .keys()
            .map(|k| format!("manifest key track.{k}")),
    );
    let n = m.spline.path.len();
    let range = |what: &str, v: i64, limit: usize| -> Result<Option<usize>, Error> {
        if v == -1 {
            return Ok(None);
        }
        match usize::try_from(v) {
            Ok(i) if i < limit => Ok(Some(i)),
            _ => Err(Error::Range {
                what: what.to_string(),
                why: format!("{v} is not -1 or an index below {limit}"),
            }),
        }
    };
    for (i, p) in m.spline.path.iter().enumerate() {
        range(
            &format!("spline.path[{i}].entry"),
            p.entry,
            m.spline.junction.len(),
        )?;
        range(
            &format!("spline.path[{i}].exit"),
            p.exit,
            m.spline.junction.len(),
        )?;
    }
    let junctions = m
        .spline
        .junction
        .iter()
        .enumerate()
        .map(|(i, j)| {
            let one = |name: &str, v: [i64; 2]| -> Result<[Option<usize>; 2], Error> {
                Ok([
                    range(&format!("spline.junction[{i}].{name}[0]"), v[0], n)?,
                    range(&format!("spline.junction[{i}].{name}[1]"), v[1], n)?,
                ])
            };
            Ok(Junction {
                prev: one("prev", j.prev)?,
                next: one("next", j.next)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(Parsed {
        glb: m.track.glb,
        version,
        id: m.track.id,
        title: m.track.title,
        notes: m.track.notes,
        point_version: m.spline.point_version,
        paths: m.spline.path,
        junctions,
        counts: m.counts,
        unread,
    })
}

fn parse_version(s: &str) -> Result<(u32, u32), Error> {
    let bad = || Error::BadVersion(s.to_string());
    let (a, b) = s.split_once('.').ok_or_else(bad)?;
    Ok((a.parse().map_err(|_| bad())?, b.parse().map_err(|_| bad())?))
}
