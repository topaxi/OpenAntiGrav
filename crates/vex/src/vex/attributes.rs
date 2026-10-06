//! [`node_attributes`]: the named `f32` attributes a node header carries.
//!
//! Split out of `vex.rs` under the 1,000-line rule; no behaviour change.

use super::{Node, byte_order, cstr_at};

/// The named `f32` attributes a node's header carries, in authored order.
///
/// Maya's per-node extra attributes, exported into the header after the name. The
/// engine looks one up by name with a `strcmp` and reads an `f32` at a
/// per-attribute offset; `AnimTransform_Bind` (`0x088fe5a4`) does so for
/// `LoopEnd`, `AnimEnd` and `FixedFrames`:
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
/// This is the `+0x0e` header short once recorded as unestablished ("small and
/// round (24, 36, 48, 56, 68, 368)"): the byte count of exactly this list.
///
/// **The lookup is case sensitive in the original, so names are reproduced
/// verbatim, not folded**: three nodes over Pulse's twelve circuits author
/// `Loopend`, which the engine's `strcmp` does not match, so they do not loop.
///
/// **Not every consumer uses `strcmp`.** `CloudGroup_Init` (`0x08933048`,
/// `docs/ghidra/functions/psp-pulse-usa/clouds.md`) reads the same list shape
/// with `strcasecmp`, a per-caller divergence read off each disassembly. It
/// changes nothing on shipped files (every cloud attribute name matches this
/// function's case), so [`crate::cloud`] reuses this function rather than add a
/// case-folding walker; a future consumer whose data differs by case should
/// check which comparison its own handler uses.
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
    // The list is inside the header and entries declare their own stride, so a
    // corrupt stride is bounded by the header; the count guard stops a stride
    // that points backwards into a cycle.
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

/// The string a node's header carries under `name`, compared without case.
///
/// The same list [`node_attributes`] walks, read the other way: the value is a
/// NUL-terminated string, not an `f32`. `PsysNode_Init` (`0x089156a0`) looks its
/// `Name` up this way with `strcasecmp`, which is why this one folds case where
/// [`node_attributes`] does not. See
/// `docs/ghidra/functions/psp-pulse-usa/placed-particle-systems.md`.
#[must_use]
pub fn node_string_attribute(data: &[u8], node: &Node, name: &str) -> Option<String> {
    if node.unk_0x0e == 0 || node.header_size < 0x10 {
        return None;
    }
    let header = data.get(node.offset..node.offset + node.header_size)?;
    let order = byte_order(data);
    let mut at = usize::from(order.u16(header, 6));
    for _ in 0..64 {
        if at + 5 > header.len() {
            return None;
        }
        let stride = usize::from(order.u16(header, at + 2));
        if stride == 0 {
            return None;
        }
        if cstr_at(header, at + 4)?.eq_ignore_ascii_case(name) {
            return cstr_at(header, at + usize::from(header[at + 1]));
        }
        at += stride;
    }
    None
}
