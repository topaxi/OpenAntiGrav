//! The `arc_anchor_point` locator: where a craft's magstrip arc wake is
//! anchored.
//!
//! `MagstripWake_Construct` resolves a node of that name against the ship and
//! warns `Ship has no arc_anchor_point locator` when it is missing, so the
//! wake belongs to the hull. Exactly one per hull, on the centreline, in
//! `Locators.vex`, identical on HD and Omega - see
//! `crates/game/tests/magstrip_anchor_ground_truth.rs` and
//! `docs/ghidra/functions/ps4-omega-eu/ships-effects.md`.

use oag_core::math::Mat4;
use oag_vex::vex;

use super::{LOCATORS_ENTRY, sibling_entry};

/// The locator's name, as `MagstripWake_Construct` looks it up.
pub const NAME: &str = "arc_anchor_point";

/// The anchor in the hull's own model space as a `glam` matrix (the row-major
/// payload read as columns, the transpose every locator here takes), or `None`
/// on a hull that authors none - which the report says, and which draws no
/// wake for that craft.
pub(super) fn locator(
    archives: &mut oag_assets::Archives,
    hull_name: &str,
    blob: &[u8],
    report: &mut Vec<String>,
) -> Option<Mat4> {
    let mut found = find(blob);
    let mut source = hull_name.to_string();
    if found.is_none()
        && let Some(name) = sibling_entry(hull_name, LOCATORS_ENTRY)
        && let Ok(sibling) = archives.read_name(&name)
    {
        found = find(&sibling);
        source = name;
    }
    match found {
        Some(matrix) => report.push(format!(
            "{source}: {NAME} at {:?}",
            matrix.w_axis.truncate().to_array()
        )),
        None => report.push(format!(
            "{hull_name}: no {NAME} locator - the magstrip arc wake draws nothing on this craft"
        )),
    }
    found
}

fn find(blob: &[u8]) -> Option<Mat4> {
    let nodes = vex::nodes(blob).ok()?;
    let class = nodes
        .iter()
        .find(|node| node.name.as_deref() == Some(NAME))?
        .class_id;
    vex::matrix::named_class_world_transforms(blob, &nodes, class)
        .into_iter()
        .find(|(name, _)| *name == Some(NAME))
        .map(|(_, matrix)| Mat4::from_cols_array(&matrix))
}
