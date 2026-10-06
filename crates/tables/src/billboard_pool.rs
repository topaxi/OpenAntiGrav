//! The advert a colour fill picks: `PI004`'s catalogue, shuffled and drawn from.
//!
//! A `<Billboard num="3" type="portrait" color="grey"/>` names no model. Pulse's
//! `Billboard_CreateFromColour` (`0x08901004` on the PSP) draws one from a
//! per-race pool: the first entry whose type matches and whose colour mask shares
//! a bit with the slot's, which then goes to the back of the pool. **The pool is
//! `Data\Plugins\PI004\Definition.xml`'s 35 entries in file order, shuffled once
//! by `FUN_08900e90`'s fixed swap loop** - no random number is involved, so every
//! race of a circuit hands the same slots the same adverts.
//!
//! **Confidence 90 for the order, 80 for the draws over several slots.** A live
//! PPSSPP read of the pool in a Talon's Junction race (2026-10-06) matched this
//! module's shuffle entry for entry (35 of 35, with the one draw that race made
//! already rotated to the back), and the circuit's grey portrait slot was bound
//! to the entry the model predicts, `TRIAKIS_PORTRAIT_01`. The rotation over more
//! than one draw is read from the code, not yet seen live on a circuit with
//! several colour slots. See `docs/ghidra/functions/psp-pulse-usa/billboards.md`.
//!
//! The colour mask bits are read off the live entries' `+0xa4` words (`grey` 1,
//! `red` 2, `green` 4, `blue` 8, `orange` 0x10, `purple` 0x20, `yellow` 0x40,
//! `white` 0x80), every one of them agreeing with the catalogue's own colour
//! lists.

use crate::fexml;
use crate::trackstartup::{Fill, TrackStartup, descendants, entry_name};

/// One catalogue entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The entry's own name, `Portrait2` or `Landscape14`.
    pub name: String,
    /// `portrait` or `landscape`, lower case: the type a slot asks for.
    pub kind: String,
    /// The advert's archive entry, in the spelling [`TrackStartup`] uses.
    pub location: String,
    /// The colours it is tagged with, as `colour_mask` bits.
    pub colours: u8,
}

/// The bit a colour name sets in an entry's mask, or 0 for a name the engine has
/// no bit for.
#[must_use]
pub fn colour_mask(names: &str) -> u8 {
    names
        .split_whitespace()
        .map(|name| match name.to_ascii_lowercase().as_str() {
            "grey" => 0x01,
            "red" => 0x02,
            "green" => 0x04,
            "blue" => 0x08,
            "orange" => 0x10,
            "purple" => 0x20,
            "yellow" => 0x40,
            "white" => 0x80,
            _ => 0,
        })
        .fold(0, |all, bit| all | bit)
}

/// `Definition.xml`'s entries, in file order. Empty for a file that is not one.
#[must_use]
pub fn parse(xml: &[u8]) -> Vec<Entry> {
    let text = String::from_utf8_lossy(xml);
    let expanded = fexml::text(xml);
    let document = fexml::parse(expanded.as_deref().unwrap_or(&text));
    descendants(&document)
        .into_iter()
        .filter(|node| node.name.eq_ignore_ascii_case("PI_Billboard"))
        .filter_map(|node| {
            Some(Entry {
                name: node.attr("name")?.to_string(),
                kind: node.value("type")?.to_ascii_lowercase(),
                location: entry_name(node.value("location")?),
                colours: colour_mask(node.value("color").unwrap_or_default()),
            })
        })
        .collect()
}

/// The race's pool of adverts a colour can draw from.
#[derive(Debug, Clone)]
pub struct Pool {
    entries: Vec<Entry>,
}

impl Pool {
    /// The pool `FUN_08900e90` builds from the catalogue in file order.
    ///
    /// Its counter starts at 0 and steps once per swap, for twice the entry
    /// count: each step swaps the entry at `(counter & 0xff) % n` with the one at
    /// `(counter >> 8) % n`.
    #[must_use]
    pub fn new(mut entries: Vec<Entry>) -> Self {
        let n = entries.len();
        for counter in 1..=n * 2 {
            entries.swap((counter & 0xff) % n, (counter >> 8) % n);
        }
        Self { entries }
    }

    /// The entries in their current order.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Draws for a slot of `kind` filled with `colour`: the first entry of that
    /// type sharing a colour bit, moved to the back of the pool. `None` when
    /// nothing matches.
    pub fn draw(&mut self, kind: &str, colour: &str) -> Option<&Entry> {
        let want = colour_mask(colour);
        let at = self
            .entries
            .iter()
            .position(|e| e.kind.eq_ignore_ascii_case(kind) && e.colours & want != 0)?;
        let last = self.entries.len() - 1;
        self.entries.swap(at, last);
        self.entries.last()
    }
}

/// The advert every colour slot of `manifest` draws, as `(num, entry)`, in the
/// order the engine makes the draws: manifest order, every colour slot
/// including those whose circuit authors no quad, since each one takes its entry
/// out of the pool. A slot an earlier entry of the file already filled is
/// skipped, as the engine skips it.
#[must_use]
pub fn colour_fills(manifest: &TrackStartup, catalogue: Vec<Entry>) -> Vec<(u32, Option<Entry>)> {
    if catalogue.is_empty() {
        return Vec::new();
    }
    let mut pool = Pool::new(catalogue);
    let mut filled = Vec::new();
    let mut out = Vec::new();
    for billboard in &manifest.billboards {
        if filled.contains(&billboard.num) {
            continue;
        }
        filled.push(billboard.num);
        if let Fill::Colour(colour) = &billboard.fill {
            out.push((billboard.num, pool.draw(&billboard.kind, colour).cloned()));
        }
    }
    out
}

#[cfg(test)]
mod tests;
