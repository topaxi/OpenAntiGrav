//! Where a circuit's collision geometry comes from.
//!
//! Two answers, and which one applies is a property of the circuit rather than
//! of the title: every title before Wipeout 2048 keeps collision inside the
//! track's own `.vex`, as nodes of five named classes, and 2048 ships a
//! `track_col.col` beside it. A 2048 circuit's `.vex` authors no collision node
//! or, on `sol`, a `Reset` node alone. See `docs/formats/2048-collision.md`.
//!
//! Its own module because `super` is at this project's 1,000-line ceiling.

use oag_assets::Archives;
use oag_vex::collision::{self, CollisionNode, SurfaceKind};
use oag_vex::kdcol;

/// Every collidable surface a circuit authors, from wherever it authors them.
///
/// The sibling container is asked **only when the `.vex` authors no ground**
/// (no `Floor` or `MagFloor` node), and its nodes follow the `.vex`'s own. The
/// original reads `track_col.col` after every `track.vex`
/// (`docs/formats/2048-collision.md`, "Which circuits read the container");
/// the re-shipped HD circuits author their ground in both, and reading both
/// would double it. A `.vex` that fails to decode is still an error rather than
/// a silent fall-through to the sibling.
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
    if nodes
        .iter()
        .any(|node| matches!(node.kind, SurfaceKind::Floor | SurfaceKind::MagFloor))
    {
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
            if !nodes.is_empty() {
                report.push(format!(
                    "{sibling}: read beside {} `.vex` collision node(s) that author no ground",
                    nodes.len()
                ));
            }
            let mut nodes = nodes;
            nodes.extend(from_kdtree);
            Ok(nodes)
        }
        // Loud, because a circuit whose collision container will not decode is a
        // circuit with no ground, and a craft falling through the world is a
        // symptom nobody should have to trace back to here.
        Err(error) => {
            report.push(format!(
                "{sibling}: does not decode ({error}) - this circuit has no ground"
            ));
            Ok(nodes)
        }
    }
}
