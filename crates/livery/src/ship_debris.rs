//! The pieces an ordinary craft breaks into: `Ship_Damage\Part_N.vex`, one model per piece
//! (`DebrisManager_Construct`, `docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`, "The
//! debris throw").

use super::*;

/// How many throws can be in the air at once; the oldest is reused. The pieces of every throw are
/// one model's buffers re-posed each frame, so this many copies are built.
pub const COPIES: usize = 4;

/// Every piece's model, [`COPIES`] times over in one model.
#[derive(Debug, Clone)]
pub struct ShipDebrisModels {
    /// All copies of all pieces, in piece order within a copy.
    pub debris: Model,
    /// Each piece's vertices in the first copy; copy `n` is `n * stride` further on.
    pub pieces: Vec<Vec<u32>>,
    /// Vertices in one copy.
    pub stride: u32,
}

/// Loads the pieces `spec` names from the title's ships directory, or says why it draws nothing.
pub fn load(
    archives: &mut oag_assets::Archives,
    ships: oag_title::race::ShipPaths,
    store: oag_gpu::formats::SceneStore,
    spec: oag_title::destroyed::ShipDebris,
    report: &mut Vec<String>,
) -> Option<ShipDebrisModels> {
    let mut textures = zone_livery::Reads::new(None).named("Debris");
    let mut parts = Vec::with_capacity(spec.pieces);
    for number in 1..=spec.pieces {
        let stem = format!("{}{number}", spec.model_stem);
        let name = oag_title::race::entry_name_in(ships.dir, spec.directory, &stem);
        match zone_end::model(archives, &name, store, &mut textures) {
            Ok((model, _)) if !model.vertices.is_empty() => parts.push(model),
            Ok(_) => {
                report.push(format!("{name}: no geometry - the craft throws no debris"));
                return None;
            }
            Err(line) => {
                report.push(format!("{line} - the craft throws no debris"));
                return None;
            }
        }
    }
    let mut pieces = Vec::with_capacity(parts.len());
    let mut stride = 0u32;
    for part in &parts {
        let count = part.vertices.len() as u32;
        pieces.push((stride..stride + count).collect());
        stride += count;
    }
    let all: Vec<Model> = (0..COPIES).flat_map(|_| parts.iter().cloned()).collect();
    report.push(format!(
        "{}\\{}: {} piece(s), {stride} vertices each, {COPIES} throw(s) at once",
        spec.directory,
        spec.model_stem,
        pieces.len()
    ));
    Some(ShipDebrisModels {
        debris: mesh::merge("ship debris", all),
        pieces,
        stride,
    })
}
