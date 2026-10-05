//! Where a circuit's collision geometry comes from.
//!
//! Two answers, and which one applies is a property of the circuit rather than
//! of the title: every title before Wipeout 2048 keeps collision inside the
//! track's own `.vex`, as nodes of five named classes, and 2048 ships a
//! `track_col.col` beside it and authors no collision node at all. See
//! `docs/formats/2048-collision.md`.
//!
//! Its own module because `super` is at this project's 1,000-line ceiling.

use oag_assets::Archives;
use oag_vex::collision::{self, CollisionNode};
use oag_vex::kdcol;

/// Every collidable surface a circuit authors, from wherever it authors them.
///
/// The sibling container is asked **only when the `.vex` answered with no
/// collision at all**, so no other title's load changes shape - and a `.vex`
/// that fails to decode is still an error rather than a silent fall-through to
/// the sibling.
///
/// # Errors
///
/// Propagates a `.vex` collision decode failure. A *sibling* that will not
/// decode is reported instead: it leaves the circuit with no ground, which the
/// report says in those words, but it is not a reason to refuse to load a race
/// that would otherwise run.
pub(super) fn of_track(
    archives: &mut Archives,
    track: &str,
    track_blob: &[u8],
    report: &mut Vec<String>,
) -> anyhow::Result<Vec<CollisionNode>> {
    let nodes = collision::from_vex(track_blob).map_err(|e| anyhow::anyhow!("{}: {e}", track))?;
    if !nodes.is_empty() {
        return Ok(nodes);
    }
    let Some(sibling) = kdcol::sibling_name(track) else {
        return Ok(nodes);
    };
    let Ok(blob) = archives.read_name(&sibling) else {
        return Ok(nodes);
    };
    match kdcol::parse(&blob) {
        Ok(decoded) => {
            let (from_kdtree, unplaced) = kdcol::collision_nodes(&decoded);
            report.push(format!(
                "{sibling}: {} vertices, {} triangles, {} k-d node(s)",
                decoded.mesh.vertices.len(),
                decoded.mesh.triangles.len(),
                decoded.nodes.len()
            ));
            for (byte, count) in unplaced {
                report.push(format!(
                    "{sibling}: surface byte {byte} is not placed - {count} triangle(s) are \
                     not collidable; see oag_vex::kdcol::class_of"
                ));
            }
            Ok(from_kdtree)
        }
        // Loud, because a circuit whose collision container will not decode is a
        // circuit with no ground, and a craft falling through the world is a
        // symptom nobody should have to trace back to here.
        Err(error) => {
            report.push(format!(
                "{sibling}: does not decode ({error}) - this circuit has no collision at all"
            ));
            Ok(nodes)
        }
    }
}
