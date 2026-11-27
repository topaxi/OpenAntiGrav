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
    /// `Data\Ships\<Team>\shipwreck.vex`, or `zonewreck.vex` in a Pulse Zone race.
    pub model: Model,
    /// The wreck's own `Ship Collision Fx` locators in its model space - the
    /// nodes `Ship_GatherCollisionFxNodes` collects once the wreck is live, and
    /// where `FUN_0883e064` spawns the explosion and the death sparks.
    pub fx: Vec<SparkAnchor>,
}

/// The wreck's archive entry name.
///
/// A title whose Zone craft is one hull of its own ([`oag_title::race::ZoneCraft::OwnShipAt`]:
/// HD/Fury, 2048, Omega) keeps that craft's `shipwreck` in its directory beside the hull
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-hull.md`, loader step 6, `%s\shipwreck.vex`).
/// Pulse keeps a per-team [`oag_pulse::race::ships::ZONE_WRECK`] wherever the Zone hull is
/// flown (a Zone race, and its Detonator), [`oag_pulse::race::ships::WRECK`] otherwise.
#[must_use]
pub(crate) fn entry_name(
    archives: &mut oag_assets::Archives,
    team: &str,
    ships: oag_title::race::ShipPaths,
    mode: Mode,
    report: &mut Vec<String>,
) -> String {
    use oag_pulse::race::ships as stems;
    let craft = entry::own_craft(ships, mode);
    if craft.is_some_and(|craft| craft.livery_key().is_some()) {
        let (dir, owner) = entry::model_directory_with(
            archives,
            ships,
            team,
            craft,
            |dir, team| entry::stem_entry_name(dir, team, stems::WRECK),
            report,
        );
        return entry::stem_entry_name(dir, owner, stems::WRECK);
    }
    let stem = if craft.is_some() {
        stems::ZONE_WRECK
    } else {
        stems::WRECK
    };
    oag_title::race::entry_name_in(ships.dir, team, stem)
}

/// Loads a team's wreck, or says why there is none. `zone` is the Zone livery folder the
/// race's hull wears ([`zone_livery::swap`]), `None` outside a Zone race: the original's
/// substitution table is global, so the wreck reads through it too
/// (`docs/ghidra/functions/ps3-hdfury-eu/zone-craft.md`, "Open").
pub fn load(
    archives: &mut oag_assets::Archives,
    team: &str,
    ships: oag_title::race::ShipPaths,
    mode: Mode,
    store: oag_gpu::formats::SceneStore,
    zone: Option<LiveryFolder>,
    report: &mut Vec<String>,
) -> Option<Wreck> {
    let name = entry_name(archives, team, ships, mode, report);
    let blob = match archives.read_name(&name) {
        Ok(blob) => blob,
        Err(_) => {
            report.push(format!(
                "{name}: not in the archive set - the craft keeps its hull when it is wrecked"
            ));
            return None;
        }
    };
    let mut textures = zone_livery::Reads::new(zone).named(entry::craft_label(mode));
    // A PS3 wreck is a `.vex` header over a `.rcsmodel`, as the hull and the shield shell are,
    // so it takes the layout branch they share ([`shield::shield_model`]).
    let built = if oag_vex::vex::geometry_is_external(&blob) {
        shield::shield_model(archives, &name, store, &mut textures).map(|(model, _, _)| model)
    } else {
        mesh::build_with_textures(&name, &blob, None).map_err(|error| error.to_string())
    };
    if let Some(line) = textures.describe(team) {
        report.push(format!("{name}: {line}"));
    }
    let mut wreck = match built {
        Ok(wreck) => wreck,
        Err(error) => {
            report.push(format!(
                "{name}: {} bytes, does not parse ({error}) - the craft keeps its hull when it is wrecked",
                blob.len()
            ));
            return None;
        }
    };
    // A PS2 model's texture block is empty by design; the hull's own gate, on
    // the same signature, so a PSP wreck never reaches the directory rule.
    if !wreck.textures.is_empty() && wreck.textures.iter().all(Option::is_none) {
        ps2_skin(archives, &name, &blob, &mut wreck, report);
    }
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
