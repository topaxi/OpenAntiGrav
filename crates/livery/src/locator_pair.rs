//! A left/right pair of same-class locators in a hull's `Locators.vex`: Wipeout HD's two
//! `cannon_flash` nodes (where its Cannon's muzzle flash is drawn) and its two `wingtip` nodes
//! (where the wingtip contrails are laid).
//!
//! HD does not flash at the round: `CannonBullet`'s update (`0x00131478`)
//! re-places `hd_muzzleflash` every tick of the flash window at the world
//! matrix of one of the firing craft's two `cannon_flash` nodes, which
//! `FUN_002d8ec0` sorts into a left and a right by whether the node's name
//! contains `left`. The wingtip nodes drive themselves
//! (`ShipWingtipVortices_Update`, `0x002db130`) and carry the side in the importer's
//! file order, which the names mirror. Both live in the team's `Locators.vex`, read from
//! the same place and on the same terms as [`super::absorb`]'s set - see
//! `docs/ghidra/functions/ps3-hdfury-eu/cannon.md` and `wingtip-vortices.md`.

use oag_core::math::Mat4;
use oag_vex::vex;

use super::{LOCATORS_ENTRY, sibling_entry};

/// Which pair to read, and what its absence costs.
#[derive(Debug, Clone, Copy)]
pub(super) struct Pair {
    /// The class every node of the pair carries.
    pub class: u32,
    /// The locators' common name, for the report.
    pub name: &'static str,
    /// What a hull without them loses, for the report.
    pub absent: &'static str,
}

/// The `cannon_flash` locators: a Cannon shot's muzzle flash.
pub(super) const CANNON_FLASH: Pair = Pair {
    class: vex::classes::CLASS_CANNON_FLASH,
    name: "cannon_flash",
    absent: "a Cannon shot draws no muzzle flash",
};

/// The `wingtip` locators: the contrails after a speed pad.
pub(super) const WINGTIP: Pair = Pair {
    class: vex::classes::CLASS_WINGTIP,
    name: "wingtip",
    absent: "the craft lays no wingtip contrail",
};

/// The pair's locators in the hull's own model space, `[left, right]`,
/// as `glam` matrices (the row-major payload read as columns - the same
/// transpose every other locator here takes). Both `None` on a hull that
/// authors neither, which the report says; a hull with only one keeps the
/// other `None` rather than mirroring it.
pub(super) fn locators(
    pair: Pair,
    archives: &mut oag_assets::Archives,
    hull_name: &str,
    blob: &[u8],
    report: &mut Vec<String>,
) -> [Option<Mat4>; 2] {
    let mut found = sort(pair.class, blob);
    let mut source = hull_name.to_string();
    if found == [None, None]
        && let Some(name) = sibling_entry(hull_name, LOCATORS_ENTRY)
        && let Ok(sibling) = archives.read_name(&name)
    {
        found = sort(pair.class, &sibling);
        source = name;
    }
    report.push(match found {
        [None, None] => format!("{hull_name}: no {} locators - {}", pair.name, pair.absent),
        [left, right] => {
            let at = |m: Option<Mat4>| m.map(|m| (m.w_axis.truncate(), m.z_axis.truncate()));
            format!(
                "{source}: {} locators (position, Z axis), left {:?}, right {:?}",
                pair.name,
                at(left),
                at(right),
            )
        }
    });
    found
}

/// Sorts one file's nodes of `class` the way `FUN_002d8ec0` does: a
/// lowercased name containing `left` is the left one, any other name the
/// right. A second node on the same side replaces the first, as the
/// original's store does.
fn sort(class: u32, blob: &[u8]) -> [Option<Mat4>; 2] {
    let mut out = [None, None];
    let Ok(nodes) = vex::nodes(blob) else {
        return out;
    };
    for (name, matrix) in vex::matrix::named_class_world_transforms(blob, &nodes, class) {
        let left = name.is_some_and(|name| name.to_ascii_lowercase().contains("left"));
        out[usize::from(!left)] = Some(Mat4::from_cols_array(&matrix));
    }
    out
}
