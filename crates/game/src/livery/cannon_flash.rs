//! Wipeout HD's two `cannon_flash` locators - where its Cannon's muzzle flash
//! is drawn.
//!
//! HD does not flash at the round: `CannonBullet`'s update (`0x00131478`)
//! re-places `hd_muzzleflash` every tick of the flash window at the world
//! matrix of one of the firing craft's two `cannon_flash` nodes, which
//! `FUN_002d8ec0` sorts into a left and a right by whether the node's name
//! contains `left`. They live in the team's `Locators.vex`, read from the same
//! place and on the same terms as [`super::absorb`]'s set - see
//! `docs/ghidra/functions/ps3-hdfury-eu/cannon.md`.

use oag_core::math::Mat4;
use oag_vex::vex;

use super::{LOCATORS_ENTRY, sibling_entry};

/// The `cannon_flash` locators in the hull's own model space, `[left, right]`,
/// as `glam` matrices (the row-major payload read as columns - the same
/// transpose every other locator here takes). Both `None` on a hull that
/// authors neither, which the report says; a hull with only one keeps the
/// other `None` rather than mirroring it.
pub(super) fn locators(
    archives: &mut oag_assets::Archives,
    hull_name: &str,
    blob: &[u8],
    report: &mut Vec<String>,
) -> [Option<Mat4>; 2] {
    let mut found = pair(blob);
    let mut source = hull_name.to_string();
    if found == [None, None]
        && let Some(name) = sibling_entry(hull_name, LOCATORS_ENTRY)
        && let Ok(sibling) = archives.read_name(&name)
    {
        found = pair(&sibling);
        source = name;
    }
    report.push(match found {
        [None, None] => {
            format!("{hull_name}: no cannon_flash locators - a Cannon shot draws no muzzle flash")
        }
        [left, right] => {
            let at = |m: Option<Mat4>| m.map(|m| (m.w_axis.truncate(), m.z_axis.truncate()));
            format!(
                "{source}: cannon_flash locators (position, Z axis), left {:?}, right {:?}",
                at(left),
                at(right),
            )
        }
    });
    found
}

/// Sorts one file's `cannon_flash` nodes the way `FUN_002d8ec0` does: a
/// lowercased name containing `left` is the left muzzle, any other name the
/// right. A second node on the same side replaces the first, as the
/// original's store does.
fn pair(blob: &[u8]) -> [Option<Mat4>; 2] {
    let mut out = [None, None];
    let Ok(nodes) = vex::nodes(blob) else {
        return out;
    };
    for (name, matrix) in vex::named_class_world_transforms(blob, &nodes, vex::CLASS_CANNON_FLASH) {
        let left = name.is_some_and(|name| name.to_ascii_lowercase().contains("left"));
        out[usize::from(!left)] = Some(Mat4::from_cols_array(&matrix));
    }
    out
}
