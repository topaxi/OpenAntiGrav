//! A Detonator hull of this project's own drawn on the craft's own hull
//! ([`oag_title::DetonatorCraft::own_hull`], `docs/formats/own-hull.md`).

use oag_core::math::Vec3;
use oag_mesh::mesh::{self, Model, own_hull::OwnHull};
use oag_race::Mode;

use crate::SparkAnchor;

/// The own hull files a race in `mode` draws on `ships`' hull
/// ([`oag_title::DetonatorCraft::own_hull`]); empty for none.
#[must_use]
pub fn files(ships: oag_title::race::ShipPaths, mode: Mode) -> &'static [&'static str] {
    (mode == Mode::Detonator)
        .then_some(ships.detonator)
        .flatten()
        .map_or(&[][..], |craft| craft.own_hull)
}

/// The first of the title's own hull files for `mode` that fits the host `.vex` `blob`
/// ([`mesh::own_hull::fits`]), `Ok(None)` where the mode flies no own hull.
///
/// # Errors
///
/// No file reads and fits: each file's outcome, joined.
pub fn find(
    archives: &mut oag_assets::Archives,
    ships: oag_title::race::ShipPaths,
    mode: Mode,
    blob: &[u8],
) -> Result<Option<OwnHull>, String> {
    first_fitting(archives, files(ships, mode), blob)
}

/// The first of `files` that reads and fits the host `.vex` `blob`, `Ok(None)` for no files.
///
/// # Errors
///
/// No file reads and fits: each file's outcome, joined.
pub fn first_fitting(
    archives: &mut oag_assets::Archives,
    files: &[&str],
    blob: &[u8],
) -> Result<Option<OwnHull>, String> {
    if files.is_empty() {
        return Ok(None);
    }
    let mut outcomes = Vec::new();
    for file in files {
        let hull = match archives.read_name(file) {
            Ok(bytes) => mesh::own_hull::parse(&bytes),
            Err(_) => {
                outcomes.push(format!("{file}: not found"));
                continue;
            }
        };
        match hull {
            Ok(hull) => {
                let misfits = mesh::own_hull::misfits(blob, &hull);
                if misfits.is_empty() {
                    return Ok(Some(hull));
                }
                outcomes.push(format!("{file}: {}", misfits.join(", ")));
            }
            Err(error) => outcomes.push(format!("{file}: {error}")),
        }
    }
    Err(outcomes.join("; "))
}

/// [`find`], with a `warn` naming each file's outcome where none fits.
pub fn for_host(
    archives: &mut oag_assets::Archives,
    ships: oag_title::race::ShipPaths,
    mode: Mode,
    hull_name: &str,
    blob: &[u8],
) -> Option<OwnHull> {
    find(archives, ships, mode, blob).unwrap_or_else(|outcomes| {
        log::warn!(
            "{hull_name}: no own Detonator hull fits this model ({outcomes}) - the disc's hull draws"
        );
        None
    })
}

/// The anchors the hull's own file places, over the host's.
#[derive(Debug)]
pub struct Anchors {
    /// `engine_flare`'s position and Z axis.
    pub flare: Option<(Vec3, Vec3)>,
    /// Every `ship_collision_fx*`, empty where the file places none.
    pub collision_fx: Vec<SparkAnchor>,
}

/// Reads `hull`'s anchors, as `oag_livery`'s locator reads take them off a `.vex` (row 3 the
/// position, row 2 the flare's axis, row 1 a spark's up).
#[must_use]
pub fn anchors(hull: &OwnHull) -> Anchors {
    let row = |m: &[f32; 16], r: usize| Vec3::new(m[r * 4], m[r * 4 + 1], m[r * 4 + 2]);
    Anchors {
        flare: hull
            .locator("engine_flare")
            .map(|l| (row(&l.transform, 3), row(&l.transform, 2))),
        collision_fx: hull
            .locators_starting("ship_collision_fx")
            .map(|l| SparkAnchor {
                position: row(&l.transform, 3),
                up: row(&l.transform, 1),
            })
            .collect(),
    }
}

/// Replaces `hull`'s geometry with `own`'s ([`mesh::own_hull::substitute`]), leaving it as it
/// is where nothing resolves; the report names each part, surface and anchor.
pub fn apply(own: &OwnHull, model: &mut Model, blob: &[u8], report: &mut Vec<String>) -> bool {
    let from = report.len();
    match mesh::own_hull::substitute(model, blob, own, report) {
        Some(substituted) => {
            for line in report[from..]
                .iter()
                .filter(|line| line.ends_with("not drawn"))
            {
                log::warn!("{line}");
            }
            *model = substituted;
            true
        }
        None => {
            log::warn!(
                "{}: the own Detonator hull resolved nothing on this model - the disc's hull draws",
                model.label
            );
            false
        }
    }
}
