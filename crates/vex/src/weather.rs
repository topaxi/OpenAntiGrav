//! The weather anchors a circuit authors: `weatherPos` (`0x3da`) nodes.
//!
//! **A `weatherPos` does not name an effect: it places the rain or snow while
//! the camera is in a covered section.** `WeatherPos_Init` (`0x0892c528`, the
//! class's `+0x7c` slot) walks up to the nearest ancestor that parents a
//! `section`, takes its index, collects the section node and stores the
//! `weatherPos` at the section's `+0x64` with its world matrix at `+0x60`.
//! `Vex_LoadModel` sets one bit in a 64-bit mask per section whose `+0x64` is
//! filled, and `Weather_Update` reads that mask as "covered": the weather's own
//! instance rides the camera in an open section and sits at the section's anchor
//! in a covered one. See `docs/ghidra/functions/psp-pulse-usa/weather.md`.
//!
//! Several nodes may resolve to one section; the original keeps the last it
//! initialised, in node order, and so does [`anchors`].

use crate::vex::{self, Node};

/// Class id of a `weatherPos` node, version 6 (Pulse).
pub const CLASS_WEATHER_POS: u32 = 0x3da;

/// A covered section's weather anchor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    /// The section it belongs to - the section's own index, also its bit in
    /// the covered mask.
    pub section: u8,
    /// The `weatherPos` node's world matrix, row-major with the
    /// translation in row 3.
    pub world: [f32; 16],
}

/// Every covered section's anchor, in section order.
///
/// Version-6 files only: another version returns nothing, not a guess. A
/// `weatherPos` with no ancestor parenting a `section` is left out, as the
/// original's walk reaches the root and returns.
#[must_use]
pub fn anchors(data: &[u8], nodes: &[Node]) -> Vec<Anchor> {
    let Ok(classes) = vex::classes_of(data) else {
        return Vec::new();
    };
    let (Some(section_class), 6) = (classes.section, classes.version) else {
        return Vec::new();
    };
    let world = vex::world_transforms(data, nodes);
    let mut found: [Option<[f32; 16]>; 64] = [None; 64];
    for (at, node) in nodes.iter().enumerate() {
        if node.class_id != CLASS_WEATHER_POS {
            continue;
        }
        let mut up = node.parent;
        while let Some(ancestor) = up {
            let section = nodes
                .iter()
                .enumerate()
                .rfind(|(_, n)| n.parent == Some(ancestor) && n.class_id == section_class)
                .and_then(|(_, n)| data.get(n.payload()).and_then(|p| p.first().copied()));
            if let Some(index) = section {
                if let (Some(slot), Some(matrix)) =
                    (found.get_mut(usize::from(index)), world.get(at))
                {
                    *slot = Some(*matrix);
                }
                break;
            }
            up = nodes[ancestor].parent;
        }
    }
    found
        .iter()
        .enumerate()
        .filter_map(|(index, matrix)| {
            matrix.map(|world| Anchor {
                section: index as u8,
                world,
            })
        })
        .collect()
}

/// The covered mask: bit `i` is set where section `i` has an anchor.
///
/// What `Vex_LoadModel` builds in `DAT_08b3bfd0`/`d4`; [`covered`] tests it.
#[must_use]
pub fn covered_mask(anchors: &[Anchor]) -> u64 {
    anchors.iter().fold(0, |mask, a| {
        mask | 1u64.checked_shl(u32::from(a.section)).unwrap_or(0)
    })
}

/// `FUN_0887866c`: whether `section` is covered. **A section below zero,
/// the value before the camera has published one, counts as covered**; a
/// section past the 64-bit mask does not.
#[must_use]
pub fn covered(mask: u64, section: i32) -> bool {
    if section < 0 {
        return true;
    }
    mask.checked_shr(section as u32).is_some_and(|m| m & 1 != 0)
}
