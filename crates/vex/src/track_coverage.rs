//! Byte coverage of the three geometry-adjacent payload classes: `WO Track`,
//! `section` and the five collision classes. See
//! `crates/formats/src/coverage.rs` for what a claim and a gap mean.
//!
//! **`WO Track` is expected to close with no gap at all**:
//! [`track::AiTrack::encoded_len`] claims equality with the payload length, and
//! [`wo_track_coverage`] makes that checkable from outside the parser.
//!
//! **`section` is the opposite case: only `pad[6]` is a real gap.**
//! `docs/formats/track.md` names two fields nothing reads, `pad[6]` and the
//! trailing name string. [`section_coverage`] leaves `pad[6]` unclaimed (what
//! `crates/vex/tests/payload_coverage_ground_truth.rs` reports) but claims the
//! name and its padding whole: the payload is the node's aligned length, so "the
//! rest" is a real span, not a width to guess.
//!
//! **Collision is closed by construction**: [`collision::from_vex`] refuses a node
//! whose chunk walk misses the payload's alignment boundary, so every node it
//! returns claims completely; see [`collision_coverage`].

use oag_formats::ByteOrder;
use oag_formats::coverage::Coverage;

use crate::{collision, track};

/// Coverage of one `WO Track` payload.
#[must_use]
pub fn wo_track_coverage(payload: &[u8]) -> Coverage {
    let mut seen = Coverage::new(payload.len());
    let Ok(ai_track) = track::parse(payload) else {
        return seen;
    };
    seen.claim(0, track::HEADER_LEN, "the WO Track header");
    let reserved = track::reserved_len(ai_track.version);
    if reserved > 0 {
        seen.claim(track::HEADER_LEN, reserved, "the reserved block");
    }
    let paths_at = track::HEADER_LEN + reserved;
    seen.claim(
        paths_at,
        ai_track.paths.len() * track::PATH_LEN,
        "the path array",
    );
    let junctions_at = paths_at + ai_track.paths.len() * track::PATH_LEN;
    seen.claim(
        junctions_at,
        ai_track.junctions.len() * track::JUNCTION_LEN,
        "the junction array",
    );
    let point_len = track::point_len(ai_track.version);
    let points_at = junctions_at + ai_track.junctions.len() * track::JUNCTION_LEN;
    let point_count: usize = ai_track.paths.iter().map(|p| p.points.len()).sum();
    seen.claim(points_at, point_count * point_len, "the control points");
    seen
}

/// Bytes of a `section` payload before the optional bounding-box block:
/// `index`, `has_bounds`, `pad[6]` and the 8-byte PVS mask.
const SECTION_FIXED_LEN: usize = 0x10;

/// Bytes of the optional bounding-box block.
const SECTION_BOUNDS_LEN: usize = 0x20;

/// Coverage of one `section` payload.
///
/// Deliberately leaves one gap a real disc always has: `pad[6]` at `+0x02`, never
/// read or initialised. Everything from the name onward is claimed as one span to
/// the payload's end, padding included: a claim about reach, not content.
#[must_use]
pub fn section_coverage(payload: &[u8]) -> Coverage {
    let mut seen = Coverage::new(payload.len());
    if payload.len() < SECTION_FIXED_LEN {
        return seen;
    }
    seen.claim(0, 2, "the section index and has_bounds flag");
    // `+0x02..+0x08` is `pad[6]`, deliberately unclaimed: never read, never
    // initialised, see `docs/formats/track.md#section-visibility-not-geometry`.
    seen.claim(8, 8, "the PVS mask");

    let has_bounds = payload[1] != 0;
    let mut at = SECTION_FIXED_LEN;
    if has_bounds && at + SECTION_BOUNDS_LEN <= payload.len() {
        seen.claim(at, SECTION_BOUNDS_LEN, "the section bounding box");
        at += SECTION_BOUNDS_LEN;
    }

    // The name is NUL-terminated then padded to the node's 16-byte alignment
    // (`docs/formats/track.md`), so the claim runs to the payload's end. That is
    // reach, not content: one field here already turned out to be exporter
    // residue rather than padding (`ship_alt.dat`'s header, `docs/formats/wad.md`),
    // and whether this tail is zero on every section is for the corpus sweep.
    seen.claim(
        at,
        payload.len() - at,
        "the section name and its alignment padding",
    );
    seen
}

/// Coverage of one collision node's payload.
///
/// [`collision::from_vex`] refuses a node whose chunk walk misses the payload's
/// 16-byte alignment boundary, so this claims completely for any accepted node;
/// the point is making that closure checkable from the sweep, not finding a gap.
#[must_use]
pub fn collision_coverage(payload: &[u8], order: ByteOrder) -> Coverage {
    let mut seen = Coverage::new(payload.len());
    let Ok(geometry) = collision::parse_chunks(payload, order) else {
        return seen;
    };
    seen.claim(0, geometry.encoded_len(), "collision chunks");
    let padded = geometry.padded_len();
    if padded > geometry.encoded_len() {
        seen.claim(
            geometry.encoded_len(),
            padded - geometry.encoded_len(),
            "alignment padding",
        );
    }
    seen
}

#[cfg(test)]
mod tests;
