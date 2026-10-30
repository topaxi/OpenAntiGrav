//! Author-defined properties and trigger volumes: what a track file carries for a
//! simulation mod to read, and the engine gives no meaning.
//!
//! A property is a name and an `f32`. The Blender add-on exports every custom
//! property prefixed `mod_` on a pad or trigger empty with the prefix stripped;
//! a bool is `1.0` or `0.0`, a string is refused. The track's own properties are
//! `[props]` in the manifest, a pad's or a trigger's are its node's `oag_props`
//! extras. A mod reads one by the FNV-1a 32 hash of its name
//! (`oag_ext_abi::track::prop_key`), so two names of one entity hashing alike are
//! refused, as is a track past the ABI's caps.

use std::collections::BTreeMap;

use oag_ext_abi::track::{MAX_PROPS, MAX_TRIGGERS, prop_key};
use serde_json::Value;

use crate::{Error, Track};

/// One entity's properties, by name.
pub type Props = BTreeMap<String, f32>;

/// A box a craft enters and leaves: a mod hears both (`trigger-enter`,
/// `trigger-exit`). Placed like a pad, without the pad's vertical expansion: the
/// box is the author's.
#[derive(Debug, Clone, PartialEq)]
pub struct Trigger {
    /// The author's name for it (the Blender empty's `oag/triggers/<name>`).
    pub name: String,
    /// Local to world: row-major, translation in row 3, like a pad's.
    pub to_world: [f32; 16],
    /// The box minimum, local space.
    pub min: [f32; 3],
    /// The box maximum, local space.
    pub max: [f32; 3],
    /// Its properties.
    pub props: Props,
}

fn refuse(what: &str, why: impl Into<String>) -> Error {
    Error::Props {
        what: what.to_string(),
        why: why.into(),
    }
}

/// A JSON object of properties: numbers and bools.
pub(crate) fn from_json(what: &str, value: &Value) -> Result<Props, Error> {
    let object = value
        .as_object()
        .ok_or_else(|| refuse(what, "oag_props entry is not an object"))?;
    object
        .iter()
        .map(|(key, v)| {
            let value = match v {
                Value::Number(n) => n
                    .as_f64()
                    .ok_or_else(|| refuse(what, format!("property {key:?} is not a number")))?
                    as f32,
                Value::Bool(b) => f32::from(u8::from(*b)),
                other => {
                    return Err(refuse(
                        what,
                        format!("property {key:?} is {other}; properties are numbers and bools"),
                    ));
                }
            };
            Ok((key.clone(), value))
        })
        .collect()
}

/// A JSON object of properties, each `f32` widened exactly.
pub(crate) fn to_json(props: &Props) -> Value {
    Value::Object(
        props
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::json!(f64::from(*v))))
            .collect(),
    )
}

/// The manifest's `[props]`: numbers and bools.
pub(crate) fn from_toml(table: BTreeMap<String, toml::Value>) -> Result<Props, Error> {
    table
        .into_iter()
        .map(|(key, v)| {
            let value = match v {
                toml::Value::Float(f) => f as f32,
                toml::Value::Integer(i) => i as f32,
                toml::Value::Boolean(b) => f32::from(u8::from(b)),
                other => {
                    return Err(refuse(
                        "props",
                        format!(
                            "{key:?} is a {}; properties are numbers and bools",
                            other.type_str()
                        ),
                    ));
                }
            };
            Ok((key, value))
        })
        .collect()
}

/// The manifest's `[props]`, each `f32` widened exactly.
pub(crate) fn to_toml(props: &Props) -> BTreeMap<String, toml::Value> {
    props
        .iter()
        .map(|(k, v)| (k.clone(), toml::Value::Float(f64::from(*v))))
        .collect()
}

fn check_one(what: &str, props: &Props) -> Result<(), Error> {
    let mut seen: Vec<(u32, &str)> = Vec::with_capacity(props.len());
    for (key, value) in props {
        if key.is_empty() {
            return Err(refuse(what, "a property has an empty name"));
        }
        if !value.is_finite() {
            return Err(refuse(what, format!("property {key:?} is not finite")));
        }
        let hash = prop_key(key);
        if let Some((_, other)) = seen.iter().find(|(h, _)| *h == hash) {
            return Err(refuse(
                what,
                format!("properties {other:?} and {key:?} hash alike ({hash:#010x}); rename one"),
            ));
        }
        seen.push((hash, key));
    }
    Ok(())
}

/// Everything a mod's read of the track's properties relies on: one property list
/// per pad or none, names that are not empty and do not hash alike, finite values,
/// finite trigger boxes, and the ABI's trigger and property caps.
pub(crate) fn check(track: &Track) -> Result<(), Error> {
    for (what, props, pads) in [
        (
            "speed pads",
            &track.speedup_pad_props,
            track.speedup_pads.len(),
        ),
        (
            "weapon pads",
            &track.weapon_pad_props,
            track.weapon_pads.len(),
        ),
    ] {
        if !props.is_empty() && props.len() != pads {
            return Err(refuse(
                what,
                format!("{} property lists for {pads} pads", props.len()),
            ));
        }
        for (i, p) in props.iter().enumerate() {
            check_one(&format!("{what} {i}"), p)?;
        }
    }
    if !track.path_props.is_empty() && track.path_props.len() != track.ai.paths.len() {
        return Err(refuse(
            "spline paths",
            format!(
                "{} property lists for {} paths",
                track.path_props.len(),
                track.ai.paths.len()
            ),
        ));
    }
    for (i, p) in track.path_props.iter().enumerate() {
        check_one(&format!("spline path {i}"), p)?;
    }
    check_one("props", &track.props)?;
    if track.triggers.len() > MAX_TRIGGERS {
        return Err(refuse(
            "triggers",
            format!(
                "{} triggers, a mod reads at most {MAX_TRIGGERS}",
                track.triggers.len()
            ),
        ));
    }
    for (i, t) in track.triggers.iter().enumerate() {
        let what = format!("trigger {i} ({})", t.name);
        check_one(&what, &t.props)?;
        if !t
            .to_world
            .iter()
            .chain(&t.min)
            .chain(&t.max)
            .all(|v| v.is_finite())
        {
            return Err(Error::NonFinite {
                record: what,
                field: "box".into(),
            });
        }
    }
    let total = track.props.len()
        + track
            .speedup_pad_props
            .iter()
            .chain(&track.weapon_pad_props)
            .chain(&track.path_props)
            .map(BTreeMap::len)
            .sum::<usize>()
        + track.triggers.iter().map(|t| t.props.len()).sum::<usize>();
    if total > MAX_PROPS {
        return Err(refuse(
            "props",
            format!("{total} properties, a mod reads at most {MAX_PROPS}"),
        ));
    }
    Ok(())
}
