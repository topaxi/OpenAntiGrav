//! The craft's wreck: `shipwreck.vex`, the model it becomes once it has blown up.
//!
//! `Ship_LoadModel` (`0x08843258`) reads it into `entity+0x8b8` beside the hull
//! and `Ship_SetState`'s case 5 makes it the live model - see
//! `docs/ghidra/functions/psp-pulse-usa/ship-wreck-model.md`. A craft whose
//! source ships none keeps its hull, which is what `Ship_SelectWreckModel`
//! does for a null `+0x8b8`, so an absence here is reported and not fatal.
//!
//! **No extra pass.** The hull's meshes carry `0x2000` in their flag word and
//! the wreck's do not (`0x1821` on Assegai's, against the hull's `0x3001`), so
//! `oag_render::shine::build` finds no batch on any of the eight teams and
//! nothing here builds one.

use super::*;

/// One team's wreck: the model, and the `Ship Collision Fx` locators the
/// destruction effects ride.
#[derive(Debug, Clone)]
pub struct Wreck {
    /// `Data\Ships\<Team>\shipwreck.vex`, or `zonewreck.vex` in Zone mode.
    pub model: Model,
    /// The wreck's own `Ship Collision Fx` locators in its model space - the
    /// nodes `Ship_GatherCollisionFxNodes` collects once the wreck is live, and
    /// where `FUN_0883e064` spawns the explosion and the death sparks.
    pub fx: Vec<SparkAnchor>,
}

/// The wreck's archive entry name: [`oag_pulse::race::ships::ZONE_WRECK`] in a
/// Zone race, [`oag_pulse::race::ships::WRECK`] otherwise.
#[must_use]
pub(crate) fn entry_name(dir: &str, team: &str, mode: Mode) -> String {
    use oag_pulse::race::ships;
    let stem = if mode == Mode::Zone {
        ships::ZONE_WRECK
    } else {
        ships::WRECK
    };
    ships::entry_name_in(dir, team, stem)
}

/// Loads a team's wreck, or says why there is none.
pub(super) fn load(
    archives: &mut oag_assets::Archives,
    team: &str,
    dir: &str,
    mode: Mode,
    report: &mut Vec<String>,
) -> Option<Wreck> {
    let name = entry_name(dir, team, mode);
    let blob = match archives.read_name(&name) {
        Ok(blob) => blob,
        Err(_) => {
            report.push(format!(
                "{name}: not in the archive set - the craft keeps its hull when it is wrecked"
            ));
            return None;
        }
    };
    let wreck = match mesh::build_with_textures(&name, &blob, None) {
        Ok(wreck) => wreck,
        Err(error) => {
            report.push(format!(
                "{name}: {} bytes, does not parse ({error}) - the craft keeps its hull when it is wrecked",
                blob.len()
            ));
            return None;
        }
    };
    let fx = collision_fx_locators(&blob);
    report.push(format!(
        "{name}: {} triangle(s), {} Ship Collision Fx locator(s), {} 0x2000 batch(es) - the \
         wreck has no extra pass",
        wreck.indices.len() / 3,
        fx.len(),
        wreck.shine_draws.len()
    ));
    Some(Wreck { model: wreck, fx })
}
