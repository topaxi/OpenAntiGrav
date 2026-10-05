//! Where Wipeout HD hangs a craft's SPU vertex light: `EngineLightData.xml`
//! beside the hull, and the `Engine Flare` locator's own Z axis.
//!
//! `EngineFlare_SubmitSpuLight` (`0x0029ff28` in `ps3-hdfury-eu`, confidence
//! 85 - `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "The captured
//! buffer's producer is found") anchors the light at
//!
//! ```text
//! flare_node.position - flare_node.z_axis * Distance
//! ```
//!
//! where the flare node is the craft's `Engine Flare` locator carried through
//! the craft's world matrix, and `Distance`/`Radius` are `Ship_LoadEngineLightData`'s
//! two reads (`0x000d25e0`). This module resolves both the file and the axis
//! per team at load; `crate::race::engine_light` places the light per frame.

use super::*;

/// One team's engine-light authoring, in the hull's own model space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineLight {
    /// `<Distance>` and `<Radius>`, as the ship directory authors them.
    pub data: oag_tables::enginelight::EngineLightData,
    /// The `Engine Flare` locator's Z axis, model space - row 2 of the same
    /// 4x4 [`super::engine_flare`] reads the position off. The original
    /// multiplies `Distance` by the node's *world* row, which carries the
    /// craft's `0.75` scale; composing this with the craft's model matrix as
    /// a vector reproduces that exactly.
    pub axis: Vec3,
}

/// Reads a team's `EngineLightData.xml`, on a title whose flare is a model -
/// the only titles that ship the file - and pairs it with the locator axis.
///
/// `None`, and a report line, for a team with no file (HD's mode ships:
/// `detonator`, `zone battle`) or no locator; nothing at all is reported on
/// a sprite-flare title, where the absence is the format's rather than the
/// team's.
pub(super) fn load(
    archives: &mut oag_assets::Archives,
    team: &str,
    ship_dir: &str,
    flare: &oag_title::flare::Flare,
    axis: Option<Vec3>,
    report: &mut Vec<String>,
) -> Option<EngineLight> {
    let oag_title::flare::Flare::PerTeam(authored) = flare else {
        return None;
    };
    if !authored.engine_light {
        // Said once per craft: a title that ships the file but whose light is
        // not read is an absence, not a silent skip.
        report.push(format!(
            "{team}: this title ships EngineLightData.xml but its SPU vertex light is the \
             PS3's and unread on this platform - no engine light (chosen, not measured)"
        ));
        return None;
    }
    let name = oag_tables::enginelight::entry_name_in(ship_dir, team);
    let Ok(blob) = archives.read_name(&name) else {
        report.push(format!(
            "{name}: not in the archive set - this craft's engine casts no SPU vertex light"
        ));
        return None;
    };
    let text = match String::from_utf8(blob) {
        Ok(text) => text,
        Err(_) => {
            report.push(format!(
                "{name}: not text - this craft's engine casts no SPU vertex light"
            ));
            return None;
        }
    };
    let data = match oag_tables::enginelight::parse(&text) {
        Ok(data) => data,
        Err(error) => {
            report.push(format!(
                "{name}: {error} - this craft's engine casts no SPU vertex light"
            ));
            return None;
        }
    };
    let Some(axis) = axis else {
        report.push(format!(
            "{name}: Distance {} / Radius {} read, but the hull authors no Engine Flare \
             locator to hang the light off - no SPU vertex light",
            data.distance, data.radius
        ));
        return None;
    };
    report.push(format!(
        "{name}: engine light Distance {} behind the flare node along its Z axis {:?}, \
         Radius {} (EngineFlare_SubmitSpuLight, renderer.md, confidence 85)",
        data.distance, axis, data.radius
    ));
    Some(EngineLight { data, axis })
}
