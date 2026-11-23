//! A Zone craft's end: the death shell drawn over the hull and the debris model it breaks
//! into, `zoneship_dead_whiteshell.vex` and `zoneship_dead.vex`.
//!
//! The Zone ship directory ships both on HD/Fury, 2048 and Omega; `Ship_ReloadModelForSkin`
//! loads the shell into `+0x6afc` and `DebrisManager_Construct` the 112 pieces
//! (`docs/ghidra/functions/ps3-hdfury-eu/zone-destroy.md`, "The model slots", row Z11).

use super::*;

/// The two models of a Zone end.
#[derive(Debug, Clone)]
pub struct ZoneEndModels {
    /// The white shell, animated by its own `Anim Transform` over `0` to `1`.
    pub shell: Model,
    /// The debris, every piece in the one model's buffers.
    pub debris: Model,
    /// The vertices of each piece of [`Self::debris`], by the `ShipBit_N` node it hangs under.
    pub pieces: Vec<Vec<u32>>,
}

/// How many pieces `DebrisManager_Construct` collects for a Zone craft (`0x70`).
pub const PIECES: usize = 112;

/// The archive entry name of one of the Zone end's models: `stem` under the Zone hull's
/// own directory.
fn entry_name(ships: oag_title::race::ShipPaths, stem: &str) -> Option<String> {
    let craft = ships.own_craft(true, false)?;
    Some(oag_title::race::entry_name_in(
        craft.root(ships.dir),
        craft.directory(""),
        stem,
    ))
}

/// One of the end's models. A Vita or PS4 `.rcsmodel` takes its node motion from the `.vex` it
/// was exported beside (`mesh::rcs::psp2::build_with_vex`): the shell's `Anim Transform` is
/// there and not in a skeleton, which neither title ships for it.
fn model(
    archives: &mut oag_assets::Archives,
    name: &str,
    store: oag_gpu::formats::SceneStore,
    textures: &mut zone_livery::Reads,
) -> Result<(Model, String), String> {
    let blob = archives
        .read_name(name)
        .map_err(|_| format!("{name}: not in the archive set"))?;
    let geometry = oag_vex::vex::geometry_is_external(&blob)
        .then(|| mesh::rcs::sibling_name(name))
        .flatten()
        .and_then(|sibling| archives.read_name(&sibling).ok())
        .filter(|geometry| mesh::rcs::psp2::is_psp2(geometry));
    let Some(geometry) = geometry else {
        return shield::shield_model(archives, name, store, textures)
            .map(|(model, note, _)| (model, note));
    };
    mesh::rcs::psp2::build_with_vex(name, &geometry, &blob, &mut |path| {
        textures.read(archives, path)
    })
    .map(|(model, built)| (model, built.describe()))
    .map_err(|error| format!("{name}: its .rcsmodel does not build ({error:#})"))
}

/// Loads the shell and the debris of a Zone race, or says why it draws nothing.
///
/// `None` outside a Zone race and on a title whose Zone craft has no directory of its own.
pub fn load(
    archives: &mut oag_assets::Archives,
    ships: oag_title::race::ShipPaths,
    mode: Mode,
    store: oag_gpu::formats::SceneStore,
    end: oag_title::destroyed::ZoneEnd,
    report: &mut Vec<String>,
) -> Option<ZoneEndModels> {
    if mode != Mode::Zone {
        return None;
    }
    let shell_name = entry_name(ships, end.shell)?;
    let debris_name = entry_name(ships, end.debris)?;
    let mut textures = zone_livery::Reads::new(None).named("Zone");
    let shell = match model(archives, &shell_name, store, &mut textures) {
        Ok((model, note)) => {
            report.push(format!("{shell_name}: {note}"));
            model
        }
        Err(line) => {
            report.push(format!("{line} - the Zone end draws no death shell"));
            return None;
        }
    };
    let debris = match model(archives, &debris_name, store, &mut textures) {
        Ok((model, note)) => {
            report.push(format!("{debris_name}: {note}"));
            model
        }
        Err(line) => {
            report.push(format!("{line} - the Zone end throws no debris"));
            return None;
        }
    };
    let pieces = match pieces(archives, &debris_name, &debris) {
        Ok(pieces) => pieces,
        Err(line) => {
            report.push(format!(
                "{debris_name}: {line} - the Zone end throws no debris"
            ));
            return None;
        }
    };
    report.push(format!(
        "{debris_name}: {} piece(s) of {} expected",
        pieces.len(),
        PIECES
    ));
    Some(ZoneEndModels {
        shell,
        debris,
        pieces,
    })
}

/// Splits the debris model into its `ShipBit_N` pieces and lists each one's vertices.
fn pieces(
    archives: &mut oag_assets::Archives,
    name: &str,
    debris: &Model,
) -> Result<Vec<Vec<u32>>, String> {
    let blob = archives
        .read_name(name)
        .map_err(|_| "not in the archive set".to_string())?;
    let nodes =
        vex::nodes(&blob).map_err(|error| format!("its node tree will not walk ({error})"))?;
    let groups: Vec<String> = nodes
        .iter()
        .filter_map(|node| node.name.clone())
        .filter(|name| name.starts_with("ShipBit_") && !name.ends_with("Shape"))
        .collect();
    let names: Vec<&str> = groups.iter().map(String::as_str).collect();
    let parts = if oag_vex::vex::geometry_is_external(&blob) {
        let geometry = mesh::rcs::sibling_name(name)
            .and_then(|sibling| archives.read_name(&sibling).ok())
            .ok_or_else(|| {
                "a .vex with external geometry and no .rcsmodel beside it".to_string()
            })?;
        if mesh::rcs::psp2::is_psp2(&geometry) {
            mesh::groups::split_psp2(debris, &geometry, &blob, &names)
        } else {
            mesh::groups::split(debris, &blob, &names)
        }
    } else {
        mesh::groups::split(debris, &blob, &names)
    }
    .map_err(|error| format!("{error:#}"))?;
    Ok(parts
        .into_iter()
        .map(|part| {
            let mut vertices: Vec<u32> = [
                &part.model.draws,
                &part.model.alpha_tested_draws,
                &part.model.transparent_draws,
            ]
            .into_iter()
            .flatten()
            .flat_map(|draw| {
                debris.indices[draw.range.start as usize..draw.range.end as usize]
                    .iter()
                    .copied()
            })
            .collect();
            vertices.sort_unstable();
            vertices.dedup();
            vertices
        })
        .filter(|vertices| !vertices.is_empty())
        .collect())
}
