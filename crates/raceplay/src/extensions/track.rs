//! The track as simulation modules read it: the region `oag_ext_abi::track` lays out,
//! built once at load from the track's pads, its triggers, its spline paths, its routes
//! and their author-defined properties, and the trigger boxes the race tests crafts
//! against.
//!
//! A disc circuit carries its pads with no properties; a track file adds what its
//! author set (`oag_trackfile::props`). The engine gives no property a meaning.

use oag_trackfile::props::{Props, Trigger};
use oag_vex::pads::PadVolume;

/// What a track carries for a mod beyond its pads: empty for a disc circuit.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrackProps {
    /// The track's own properties.
    pub track: Props,
    /// One list per speed pad, or none.
    pub speedup_pads: Vec<Props>,
    /// One list per weapon pad, or none.
    pub weapon_pads: Vec<Props>,
    /// The trigger volumes.
    pub triggers: Vec<Trigger>,
    /// One list per spline path, or none.
    pub paths: Vec<Props>,
}

/// The track a race's modules read: its properties and its pads.
#[derive(Debug, Clone, Copy)]
pub struct Track<'a> {
    /// The track's name for the load report (its circuit or `foreign:<id>`).
    pub name: &'a str,
    /// The properties and triggers.
    pub props: &'a TrackProps,
    /// The speed pads, in the race's order.
    pub speedup_pads: &'a [PadVolume],
    /// The weapon pads.
    pub weapon_pads: &'a [PadVolume],
    /// The spline graph's paths, in the track's order.
    pub paths: &'a [oag_vex::track::Path],
    /// The ways from a fork back to the lap, in the course's order: the numbering a
    /// `prefer-route` names.
    pub routes: &'a [oag_race::course::Route],
}

/// The encoded region and what the race tests against, shared by every snapshot.
#[cfg(feature = "extensions")]
#[derive(Debug, Clone, Default)]
pub(super) struct Region {
    /// The bytes the host writes into a guest's `oag_track` buffer.
    pub bytes: std::sync::Arc<[u8]>,
    /// The track's speed pads: the bound of a race-wide `scale-pad`.
    pub speed_pads: usize,
    /// The track's routes: the bound of a `prefer-route`.
    pub routes: usize,
    /// The trigger boxes, as pad volumes (armed, unexpanded: the author's box).
    pub triggers: std::sync::Arc<[PadVolume]>,
}

#[cfg(feature = "extensions")]
fn keyed(props: &Props) -> Vec<(u32, f32)> {
    props
        .iter()
        .map(|(name, value)| (oag_ext_abi::track::prop_key(name), *value))
        .collect()
}

/// The region of `track`.
///
/// # Errors
///
/// A cap of `oag_ext_abi::track` passed, or two names of one entity hashing alike.
#[cfg(feature = "extensions")]
pub(super) fn encode(track: Track<'_>) -> Result<Region, oag_ext_abi::track::TooBig> {
    use oag_ext_abi::track::{Entity, TRACK_LEN, write};
    let none = Props::new();
    let lists = |pads: &[PadVolume], props: &[Props]| -> Vec<Vec<(u32, f32)>> {
        (0..pads.len())
            .map(|i| keyed(props.get(i).unwrap_or(&none)))
            .collect()
    };
    let speed = lists(track.speedup_pads, &track.props.speedup_pads);
    let weapon = lists(track.weapon_pads, &track.props.weapon_pads);
    let triggers: Vec<Vec<(u32, f32)>> = track
        .props
        .triggers
        .iter()
        .map(|t| keyed(&t.props))
        .collect();
    fn entities<'a>(pads: &[PadVolume], props: &'a [Vec<(u32, f32)>]) -> Vec<Entity<'a>> {
        pads.iter()
            .zip(props)
            .map(|(pad, props)| Entity {
                to_world: pad.to_world,
                min: pad.min,
                max: pad.max,
                props,
            })
            .collect()
    }
    let boxes: Vec<PadVolume> = track
        .props
        .triggers
        .iter()
        .map(|t| PadVolume {
            to_world: t.to_world,
            min: t.min,
            max: t.max,
            disabled: 0.0,
        })
        .collect();
    let path_props: Vec<Vec<(u32, f32)>> = (0..track.paths.len())
        .map(|i| keyed(track.props.paths.get(i).unwrap_or(&none)))
        .collect();
    // A route carries the properties of the paths it drives, in order; where two set
    // one name the first path's value is kept.
    let route_props: Vec<Vec<(u32, f32)>> = track
        .routes
        .iter()
        .map(|route| {
            let mut merged: Vec<(u32, f32)> = Vec::new();
            for &path in &route.paths {
                for &(key, value) in path_props.get(usize::from(path)).into_iter().flatten() {
                    if merged.iter().all(|(k, _)| *k != key) {
                        merged.push((key, value));
                    }
                }
            }
            merged
        })
        .collect();
    let paths: Vec<Entity<'_>> = track
        .paths
        .iter()
        .zip(&path_props)
        .map(|(path, props)| bounded(path.points.iter().map(|p| p.pos), props))
        .collect();
    let routes: Vec<Entity<'_>> = track
        .routes
        .iter()
        .zip(&route_props)
        .map(|(route, props)| bounded(route.positions.iter().map(|p| p.to_array()), props))
        .collect();
    let mut bytes = vec![0; TRACK_LEN];
    let used = write(
        &mut bytes,
        &keyed(&track.props.track),
        &entities(track.speedup_pads, &speed),
        &entities(track.weapon_pads, &weapon),
        &entities(&boxes, &triggers),
        &paths,
        &routes,
    )?;
    bytes.truncate(used);
    Ok(Region {
        bytes: bytes.into(),
        speed_pads: track.speedup_pads.len(),
        routes: track.routes.len(),
        triggers: boxes.into(),
    })
}

/// An entity placed at the identity, its box the bounds of `points` in world space.
#[cfg(feature = "extensions")]
fn bounded<'a>(
    points: impl Iterator<Item = [f32; 3]>,
    props: &'a [(u32, f32)],
) -> oag_ext_abi::track::Entity<'a> {
    let (mut min, mut max) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
    for p in points {
        for axis in 0..3 {
            min[axis] = min[axis].min(p[axis]);
            max[axis] = max[axis].max(p[axis]);
        }
    }
    if min[0] > max[0] {
        (min, max) = ([0.0; 3], [0.0; 3]);
    }
    oag_ext_abi::track::Entity {
        to_world: oag_ext_abi::track::IDENTITY,
        min,
        max,
        props,
    }
}
