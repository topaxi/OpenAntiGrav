//! [`node_attributes`]: the named `f32` attributes a node header carries.
//!
//! Split out of `vex.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::{Node, byte_order, cstr_at};

/// The named `f32` attributes a node's header carries, in authored order.
///
/// Maya's own per-node extra attributes, exported into the header after the
/// name. The engine looks one up by name with a `strcmp` and reads an `f32` at
/// a per-attribute offset; `AnimTransform_Bind` (`0x088fe5a4`) does it three
/// times, for `LoopEnd`, `AnimEnd` and `FixedFrames`:
///
/// ```text
/// header +0x06 u16   offset of the first attribute, from the header
/// header +0x0e u16   the list's length in bytes; 0 means there is no list
///
/// attribute +0x01 u8    offset of the f32 value, from the attribute
/// attribute +0x02 u16   stride to the next attribute; 0 terminates
/// attribute +0x04 ...   NUL-terminated name
/// ```
///
/// This is what the `+0x0e` field is - the header short whose meaning was
/// recorded as unestablished, with its "small and round (24, 36, 48, 56, 68,
/// 368)" values noted. They are byte counts, of exactly this list.
///
/// **The lookup is case sensitive in the original, so this reproduces the
/// names verbatim** and does not fold them: three nodes over Pulse's twelve
/// circuits author `Loopend`, which the engine's `strcmp` does not match, so
/// those three do not loop. Matching them here would be a departure.
///
/// **Not every consumer of this list uses `strcmp`.** `CloudGroup_Init`
/// (`0x08933048`, `docs/ghidra/functions/psp-pulse-usa/clouds.md`) reads the
/// same list shape - `Overlap`, `SpriteRadius`, the colour-ramp names - with
/// `strcasecmp` instead, a case-*insensitive* compare. Both readings are off
/// the consumer's own disassembly, so this is a genuine per-caller divergence
/// rather than a mistake in either page. It makes no difference to any
/// shipped file (every cloud attribute name already matches this function's
/// case exactly), which is why [`crate::cloud`] reuses this function
/// unchanged instead of adding a second, case-folding walker for a
/// difference no shipped byte exercises - but a future consumer whose data
/// *does* differ by case should check which comparison its own handler uses
/// before assuming this function's case-sensitive answer is theirs too.
///
/// Returns an empty vector for a node with no list, a header this file does not
/// contain, or a list that does not walk.
#[must_use]
pub fn node_attributes(data: &[u8], node: &Node) -> Vec<(String, f32)> {
    let mut out = Vec::new();
    if node.unk_0x0e == 0 || node.header_size < 0x10 {
        return out;
    }
    let Some(header) = data.get(node.offset..node.offset + node.header_size) else {
        return out;
    };
    let order = byte_order(data);
    let mut at = usize::from(order.u16(header, 6));
    // The list is inside the header and each entry declares its own stride, so
    // a corrupt stride is bounded by the header rather than by a guess. The
    // count guard is for a stride that points backwards into a cycle.
    for _ in 0..64 {
        if at + 5 > header.len() {
            break;
        }
        let stride = usize::from(order.u16(header, at + 2));
        if stride == 0 {
            break;
        }
        let value_at = at + usize::from(header[at + 1]);
        let Some(name) = cstr_at(header, at + 4) else {
            break;
        };
        if value_at + 4 > header.len() {
            break;
        }
        out.push((name, order.f32(header, value_at)));
        at += stride;
    }
    out
}
