//! Byte coverage of a `.pob` particle system.
//!
//! Everything [`ParticleSystem`](oag_pob::ParticleSystem) itself claims
//! (the container header, the slot table, the resource name, the emitter
//! tree, its modifier lists, and - PSP only, see `crate::texture` -
//! each emitter's own positionally-addressed embedded texture) is
//! straightforward to re-derive here the same way `oag_rcs::rcsmodel::coverage`
//! does. Landing the texture claim alone moved the PSP corpus from 25.62% to
//! 56.76% (`crates/pob/tests/pob_coverage_ground_truth.rs`, 2026-09-17) -
//! more than half of what was an "unlocated" gap was sitting on disc the
//! whole time, just not addressed through the slot table. **The slot-resolved records
//! are the deliberate exception**, per `docs/formats/pob.md`'s own "Not
//! determined": two passes at their internals came back a dead end, so this
//! module measures what a slot resolves to (a fixup site, always 4 bytes,
//! always claimed) and claims its *target* only when the target is itself
//! readable as a NUL-terminated string, exactly the "43% of targets are
//! developer strings" the format page already measured. A target that is not
//! a string stays an honest, reported gap: this is not a decode, and does not
//! pretend to be one.

use oag_formats::coverage::Coverage;

use crate::{EMITTER_LEN, HEADER_LEN, NAME_LEN, ParticleSystem, SLOT_LEN};

/// Bytes of one modifier list node, up to and including its `next` pointer.
/// Matches `pob.rs`'s own private `NODE_LEN`.
const MODIFIER_NODE_LEN: usize = 0x34;

/// Coverage of one `.pob` resource.
#[must_use]
pub fn coverage(data: &[u8]) -> Coverage {
    let mut seen = Coverage::new(data.len());
    let Ok(system) = ParticleSystem::parse(data) else {
        return seen;
    };
    let order = system.order;

    seen.claim(0, HEADER_LEN, "the SYSP header");
    let base = system.resource_base();
    seen.claim(HEADER_LEN, system.slots.len() * SLOT_LEN, "the slot table");
    seen.claim(base, NAME_LEN, "the resource name");

    for (index, slot) in system.slots.iter().enumerate() {
        // The fixup *site* - always four bytes, always at a known offset,
        // whatever the slot resolves to.
        let Some(site) = slot else { continue };
        seen.claim(base + *site as usize, SLOT_LEN, "a slot fixup site");

        // The *target* - claimed only when it is a NUL-terminated string,
        // which is the one shape this module can bound without guessing a
        // record layout nobody has decoded. `resolve_slot` returns an offset
        // into `system.payload`, i.e. relative to just past the name field,
        // so it is shifted back to an absolute offset into `data` here.
        // See the module doc for why the rest is left as a gap rather than a
        // guess.
        if let Ok(Some(relative)) = system.resolve_slot(data, index) {
            let target = base + NAME_LEN + relative;
            if let Some(nul) = data[target..].iter().position(|&b| b == 0)
                && nul > 0
                && data[target..target + nul]
                    .iter()
                    .all(|&b| b.is_ascii_graphic() || b == b' ')
            {
                seen.claim(target, nul + 1, "a slot target string");
            }
        }
    }

    if let Ok(emitters) = system.emitters(data) {
        for emitter in &emitters {
            let start = base + emitter.offset;
            seen.claim(start, EMITTER_LEN, "an emitter record");
            claim_modifiers(
                &mut seen,
                data,
                order,
                base,
                &emitter.modifiers,
                emitter.offset,
            );
            // See `crate::texture` for why this is positional (right
            // after the record) rather than resolved through a slot -
            // PSP only; never present on a PS2 or HD file.
            if let Some(texture) = system.embedded_texture(data, emitter) {
                seen.claim(
                    start + EMITTER_LEN,
                    crate::texture::TEXTURE_HEADER_LEN,
                    "an embedded texture header",
                );
                seen.claim(
                    texture.palette_offset,
                    texture.palette.len(),
                    "an embedded texture palette",
                );
                seen.claim(
                    texture.pixel_offset,
                    texture.indices.len(),
                    "an embedded texture's level 0 pixels",
                );
            }
        }
    }

    seen
}

/// Modifier list nodes are not returned with their own file offset - only
/// [`crate::Emitter::offset`] is public - so this re-walks the same
/// `+0x9b4` chain `pob::parse_modifiers` does, purely to claim each node's
/// span; nothing here re-derives a field `pob` does not already expose.
fn claim_modifiers(
    seen: &mut Coverage,
    data: &[u8],
    order: oag_formats::ByteOrder,
    base: usize,
    modifiers: &[crate::Modifier],
    emitter_offset: usize,
) {
    if modifiers.is_empty() {
        return;
    }
    let record_start = base + emitter_offset;
    if data.len() < record_start + 0x9b8 {
        return;
    }
    let mut next = order.u32(data, record_start + 0x9b4) as usize;
    let mut claimed = 0usize;
    while next != 0 && claimed < modifiers.len() {
        let start = base + next;
        if data.len() < start + MODIFIER_NODE_LEN {
            break;
        }
        seen.claim(start, MODIFIER_NODE_LEN, "a modifier node");
        next = order.u32(data, start + 0x30) as usize;
        claimed += 1;
    }
}

#[cfg(test)]
mod tests;
