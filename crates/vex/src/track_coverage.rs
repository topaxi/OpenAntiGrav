//! Byte coverage of the three geometry-adjacent payload classes: `WO Track`,
//! `section` and the five collision classes. See
//! `crates/formats/src/coverage.rs` for what a claim and a gap mean.
//!
//! **`WO Track` is expected to close with no gap at all** -
//! [`track::AiTrack::encoded_len`]'s own doc already claims "equal to the
//! payload length on every shipped track" - so [`wo_track_coverage`] exists
//! to make that claim checkable from outside the parser rather than to find
//! anything new in it.
//!
//! **`section` is the opposite case, and only `pad[6]` is a real gap here.**
//! `docs/formats/track.md`'s own struct comment names two fields nothing
//! reads - `pad[6]` and the trailing name string. [`section_coverage`]
//! leaves `pad[6]` unclaimed on purpose, but the name-and-its-padding tail
//! is claimed whole rather than only up to the NUL: the payload is already
//! the node's own aligned length, so "the rest of it" is a real span to
//! claim, not a width to guess. `pad[6]` is what the sweep in
//! `crates/vex/tests/payload_coverage_ground_truth.rs` reports as the gap.
//!
//! **Collision is closed by construction.** [`collision::from_vex`] already
//! refuses a node whose chunk walk does not land on the payload's own
//! alignment boundary, so every node it returns claims completely; see
//! [`collision_coverage`].

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
/// Deliberately leaves one gap a real disc always has: `pad[6]` at `+0x02`,
/// never read and never initialised. Everything from the name onward is
/// claimed as one span reaching the payload's own end, padding included -
/// a claim about reach rather than an assumption about content, since the
/// payload is already the node's own aligned length.
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

    // The name is NUL-terminated and then padded to the node's own 16-byte
    // alignment - `docs/formats/track.md`'s own struct comment says so - so
    // the claim runs to the payload's end rather than stopping at the NUL.
    // That is a claim about *reach*, not about content: this project has
    // already found one field that turned out to be exporter residue rather
    // than padding (`ship_alt.dat`'s header, see `docs/formats/wad.md`), and
    // whether this tail is genuinely zero on every section is a question for
    // the corpus sweep to answer, not for this function to assume.
    seen.claim(
        at,
        payload.len() - at,
        "the section name and its alignment padding",
    );
    seen
}

/// Coverage of one collision node's payload.
///
/// [`collision::from_vex`] already refuses a node whose chunk walk does not
/// reach the payload's own 16-byte alignment boundary, so this always claims
/// completely for a node the decoder accepted at all - the point is to make
/// that closure checkable from the coverage sweep, not to find a gap in it.
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
