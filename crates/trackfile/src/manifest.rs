//! The TOML manifest: names, version, integer structure and the counts the glb
//! is checked against. Holds no `f32` the simulation reads (a decimal round trip
//! is not guaranteed bit-exact); a float that is not an integer lives in the glb.

use std::collections::BTreeMap;

use oag_vex::track::Junction;
use serde::{Deserialize, Serialize};

use crate::art::ArtTable;
use crate::{Error, Track};

/// The schema kind a manifest declares in its `[toml-schema]` header.
pub const SCHEMA_KIND: &str = "track";
/// The one schema major this reader reads. A file of another major is refused
/// by name. There is no minor: a compatible addition goes into the same `v1`
/// schema, and a reader of this major reports a key it does not know as unread.
pub const FORMAT_MAJOR: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Manifest {
    #[serde(rename = "toml-schema")]
    pub toml_schema: Option<oag_schema::Header>,
    pub track: TrackTable,
    pub spline: Spline,
    pub counts: Counts,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub art: Option<ArtTable>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TrackTable {
    pub id: String,
    pub title: String,
    pub host: String,
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
    #[serde(deserialize_with = "pair")]
    pub prev: [i64; 2],
    #[serde(deserialize_with = "pair")]
    pub next: [i64; 2],
}

/// Exactly two integers: a fixed-size array alone would read the first two of a
/// longer one and drop the rest unseen.
fn pair<'de, D: serde::Deserializer<'de>>(d: D) -> Result<[i64; 2], D::Error> {
    let given: Vec<i64> = Deserialize::deserialize(d)?;
    <[i64; 2]>::try_from(given)
        .map_err(|v| serde::de::Error::invalid_length(v.len(), &"an array of exactly two integers"))
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Counts {
    pub collision_nodes: usize,
    pub speedup_pads: usize,
    pub weapon_pads: usize,
    /// Written only when the track has art, so a file without it is what 1.0 wrote.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub art_meshes: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// What a manifest read produced for the glb half to finish.
#[derive(Debug)]
pub(crate) struct Parsed {
    pub glb: String,
    pub id: String,
    pub title: String,
    pub host: String,
    pub notes: Vec<String>,
    pub point_version: u32,
    pub paths: Vec<PathRow>,
    pub junctions: Vec<Junction>,
    pub counts: Counts,
    pub art: Option<ArtTable>,
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
    let host = match track.host.as_deref() {
        Some(h) if !h.trim().is_empty() => h.to_string(),
        _ => {
            return Err(Error::Unwritable(
                "the track names no host title (track.host, e.g. \"wipeout-pulse\")".into(),
            ));
        }
    };
    let manifest = Manifest {
        toml_schema: Some(oag_schema::Header {
            location: oag_schema::location(SCHEMA_KIND, FORMAT_MAJOR),
            version: oag_schema::LANGUAGE_VERSION.to_owned(),
        }),
        track: TrackTable {
            id: track.id.clone(),
            title: track.title.clone(),
            host,
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
            art_meshes: track.art.as_ref().map_or(0, |a| a.meshes.len()),
        },
        art: track.art.as_ref().map(ArtTable::of),
        extra: BTreeMap::new(),
    };
    toml::to_string(&manifest).map_err(|e| Error::Unwritable(e.to_string()))
}

pub(crate) fn read(text: &str) -> Result<Parsed, Error> {
    let m: Manifest = toml::from_str(text).map_err(|e| Error::Manifest(e.to_string()))?;
    oag_schema::check(m.toml_schema.as_ref(), SCHEMA_KIND, FORMAT_MAJOR)?;
    if m.track.id.is_empty() {
        return Err(Error::Manifest("track.id is empty".into()));
    }
    if m.track.host.trim().is_empty() {
        return Err(Error::Manifest(
            "track.host is empty; it names the title the track runs on, e.g. \"wipeout-pulse\""
                .into(),
        ));
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
    let mut art = m.art;
    if let Some(table) = &mut art {
        unread.extend(
            std::mem::take(&mut table.extra)
                .into_keys()
                .map(|k| format!("manifest key art.{k}")),
        );
    }
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
        id: m.track.id,
        title: m.track.title,
        host: m.track.host,
        notes: m.track.notes,
        point_version: m.spline.point_version,
        paths: m.spline.path,
        junctions,
        counts: m.counts,
        art,
        unread,
    })
}
