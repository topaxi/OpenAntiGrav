//! Wipeout HD's `absorb` locators - where its weapon-absorb burst plays.
//!
//! HD does not play the burst at the `Ship Collision Fx` set the way Pulse
//! does: `FUN_000d2f58` collects up to six nodes of the `absorb` class
//! (`0x3ee`, [`vex::CLASS_ABSORB`]) and `FUN_000d9398` fires at those. They
//! live beside the collision-fx set in the team's `Locators.vex`, so they are
//! read from the same place, the hull first and then the sibling - see
//! `docs/ghidra/functions/ps3-hdfury-eu/absorb-feedback.md`.

use oag_core::math::Vec3;
use oag_vex::vex;

use super::{LOCATORS_ENTRY, sibling_entry};

/// Every `absorb` locator's position in the hull's own model space, in file
/// order - the depth-first order the original's collector walks. Empty on a
/// source whose format version names no such class (every PSP and PS2 file)
/// and on a hull that authors none, which the report says.
pub(super) fn locators(
    archives: &mut oag_assets::Archives,
    hull_name: &str,
    blob: &[u8],
    report: &mut Vec<String>,
) -> Vec<Vec3> {
    let Ok(Some(_)) = vex::classes_of(blob).map(|classes| classes.absorb) else {
        return Vec::new();
    };
    let mut found = positions(blob);
    let mut source = hull_name.to_string();
    if found.is_empty()
        && let Some(name) = sibling_entry(hull_name, LOCATORS_ENTRY)
        && let Ok(sibling) = archives.read_name(&name)
    {
        found = positions(&sibling);
        source = name;
    }
    report.push(if found.is_empty() {
        format!("{hull_name}: no absorb locators - an absorb plays no burst on this craft")
    } else {
        format!(
            "{source}: {} absorb locator(s) for the absorb burst",
            found.len()
        )
    });
    found
}

fn positions(blob: &[u8]) -> Vec<Vec3> {
    let Ok(Some(class)) = vex::classes_of(blob).map(|classes| classes.absorb) else {
        return Vec::new();
    };
    let Ok(nodes) = vex::nodes(blob) else {
        return Vec::new();
    };
    vex::class_world_transforms(blob, &nodes, class)
        .into_iter()
        .map(|m| Vec3::new(m[12], m[13], m[14]))
        .collect()
}
