//! The track as simulation modules read it: the region `oag_ext_abi::track` lays out,
//! built once at load from the track's pads, its triggers and their author-defined
//! properties, and the trigger boxes the race tests crafts against.
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
}

/// The encoded region and what the race tests against, shared by every snapshot.
#[cfg(feature = "extensions")]
#[derive(Debug, Clone, Default)]
pub(super) struct Region {
    /// The bytes the host writes into a guest's `oag_track` buffer.
    pub bytes: std::sync::Arc<[u8]>,
    /// The track's speed pads: the bound of a race-wide `scale-pad`.
    pub speed_pads: usize,
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
    let mut bytes = vec![0; TRACK_LEN];
    let used = write(
        &mut bytes,
        &keyed(&track.props.track),
        &entities(track.speedup_pads, &speed),
        &entities(track.weapon_pads, &weapon),
        &entities(&boxes, &triggers),
    )?;
    bytes.truncate(used);
    Ok(Region {
        bytes: bytes.into(),
        speed_pads: track.speedup_pads.len(),
        triggers: boxes.into(),
    })
}
