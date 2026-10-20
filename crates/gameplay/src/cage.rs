//! The collision cage's track half: which control point a craft is over, and the sibling
//! path's at a fork. The geometry and the push are `oag_physics::cage`.
//!
//! `FUN_08831aa4` locates the box collider's centre with `AiTrack_UpdateCursor(500.0, ..)`
//! and builds the located **control point's** cross-section, not an interpolated sample.
//! The cursor is a three-nearest scan refined by a walk to each candidate's best point
//! (`docs/formats/track.md`, "Locating a ship on the track"), so the nearest control point
//! over every path is what it settles on; this scans for it directly, no cursor kept.
//! Ties go to the first in file order.
//!
//! **Built only where the original runs it**: the header flag `FUN_0887d36c` reads
//! (`oag_vex::track::cage_flag`), on a spline version that carries the eight cage bytes.
//! See `docs/ghidra/functions/psp-pulse-usa/collision-cage.md`.

use oag_core::math::Vec3;
use oag_physics::cage::{Located, Section};
use oag_vex::track::{AiTrack, CAGE_VERSION, SHORT_POINT_VERSION, SplinePoint};

/// One path's control points as the cage reads them.
#[derive(Debug, Clone, PartialEq)]
struct CagePath {
    /// Lifted positions, what the locate measures to.
    points: Vec<Vec3>,
    /// Each point's cross-section.
    sections: Vec<Section>,
    /// `AiTrack_LocateOnSiblingPath` (`0x0887d4d4`): the other successor of this path's
    /// entry junction, else another predecessor of its exit junction.
    sibling: Option<usize>,
}

/// A track's cage, ready to locate against.
#[derive(Debug, Clone, PartialEq)]
pub struct CageTrack {
    paths: Vec<CagePath>,
}

impl CageTrack {
    /// The cage of `ai`, or `None` when the original would not run one on it: the header
    /// flag clear (`flag`, `oag_vex::track::cage_flag` of the same payload), or a spline
    /// version without the cage bytes (before [`CAGE_VERSION`], or the short points of
    /// [`SHORT_POINT_VERSION`] onwards).
    #[must_use]
    pub fn new(ai: &AiTrack, flag: bool) -> Option<Self> {
        let authored = (CAGE_VERSION..SHORT_POINT_VERSION).contains(&ai.version);
        if !flag || !authored {
            return None;
        }
        let paths = ai
            .paths
            .iter()
            .enumerate()
            .map(|(index, path)| CagePath {
                points: path.points.iter().map(lifted).collect(),
                sections: path.points.iter().map(section).collect(),
                sibling: sibling(ai, index),
            })
            .collect();
        Some(Self { paths })
    }

    /// How many control points the cage covers.
    #[must_use]
    pub fn point_count(&self) -> usize {
        self.paths.iter().map(|p| p.points.len()).sum()
    }

    /// Where the cage looks for a box centred at `centre`.
    #[must_use]
    pub fn locate(&self, centre: Vec3) -> Option<Located> {
        let (path, index) = nearest(self.paths.iter().enumerate(), centre)?;
        let at = &self.paths[path];
        let point = at.points[index];
        // `AiTrack_UpdateCursor(500.0, .., only = sibling)` from the located point's own
        // position, not the craft's.
        let sibling = at.sibling.and_then(|other| {
            let (_, index) = nearest(std::iter::once((other, &self.paths[other])), point)?;
            Some(self.paths[other].sections[index])
        });
        Some(Located {
            point,
            section: at.sections[index],
            sibling,
        })
    }
}

/// The cage's pass on one craft, right after its `oag_physics::step`: where the original
/// runs it, in the ship entity update after `Body_StepWorld` (`FUN_088418e0`, `0x08842384`),
/// for the human (`entity+0x368 == 0`) and every AI craft (`2`) alike.
///
/// Corners and locate from the pose the hull was tested at, impulses on the body as the
/// hull left it (`oag_physics::cage::resolve`). A track with no cage does nothing.
///
/// **Not ported here:** the reset the response can ask for (`CageResponse::reset`,
/// `Ship_SetState(3)`), and the original's skip of a craft whose `entity+0x8c` reads `6`.
pub fn pass(
    cage: Option<&CageTrack>,
    state: &mut oag_physics::ShipState,
    handling: &oag_physics::Handling,
    wall: &oag_physics::WallResponse,
) -> oag_physics::cage::CageResponse {
    let Some(cage) = cage else {
        return oag_physics::cage::CageResponse::default();
    };
    let located = cage.locate(wall.synced_position);
    oag_physics::cage::resolve(
        state,
        handling,
        wall.synced_position,
        wall.synced_orientation,
        located.as_ref(),
    )
}

fn nearest<'a>(
    paths: impl Iterator<Item = (usize, &'a CagePath)>,
    to: Vec3,
) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize, f32)> = None;
    for (path, at) in paths {
        for (index, point) in at.points.iter().enumerate() {
            let d = (*point - to).length_squared();
            if best.is_none_or(|(_, _, b)| d < b) {
                best = Some((path, index, d));
            }
        }
    }
    best.map(|(path, index, _)| (path, index))
}

fn lifted(point: &SplinePoint) -> Vec3 {
    Vec3::from_array(point.lifted_pos())
}

fn section(point: &SplinePoint) -> Section {
    Section::new(
        lifted(point),
        Vec3::from_array(point.tangent),
        Vec3::from_array(point.down),
        Vec3::from_array(point.lateral),
        point.cage,
    )
}

/// `AiTrack_LocateOnSiblingPath`'s pick, read at `0x0887d4d4`: the entry junction's other
/// successor (`+0x08`, else `+0x0c` when `+0x08` is this path), else the exit junction's
/// first predecessor that is not this path (`+0x00`, then `+0x04`).
fn sibling(ai: &AiTrack, path: usize) -> Option<usize> {
    let this = &ai.paths[path];
    let from_entry = this.entry.and_then(|j| {
        let next = ai.junctions[j].next;
        if next[0] == Some(path) {
            next[1]
        } else {
            next[0]
        }
    });
    from_entry.or_else(|| {
        let prev = ai.junctions[this.exit?].prev;
        prev.into_iter().flatten().find(|&p| p != path)
    })
}

#[cfg(test)]
mod tests;
