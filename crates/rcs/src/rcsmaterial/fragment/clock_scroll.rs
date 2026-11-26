//! The rate a fragment program scrolls a texture's `u` at, off its own literal.

use super::{Instruction, Program, Source};

/// A register lane: ordinal, half-precision, lane `0..=3`.
type Lane = (u8, bool, u8);

impl Program {
    /// The literal `A` of a texture tap at `u + A * clock`, where `clock` is the
    /// parameter named `clock_hash`.
    ///
    /// Followed by data flow, not by matching a number: the parameter is moved
    /// into a register, a `MAD` multiplies it by a literal the file authors (a
    /// slot no parameter patches) and adds a coordinate, and a later texture
    /// instruction reads that lane as its `u`. HD's shield shell
    /// (`docs/ghidra/functions/ps3-hdfury-eu/shield.md`, "The lattice scrolls").
    ///
    /// `None` for a program with no such tap, or with two taps at different rates.
    #[must_use]
    pub fn clock_scroll_rate(&self, clock_hash: u32) -> Option<f32> {
        let clock_slots: Vec<u16> = self.patches(clock_hash).collect();
        let mut clock: Vec<Lane> = Vec::new();
        let mut scrolled: Vec<(Lane, f32)> = Vec::new();
        let mut found: Option<f32> = None;
        for insn in &self.instructions {
            if insn.is_texture()
                && let Source::Register { index, half } = insn.sources[0]
            {
                let u = (index, half, insn.swizzles[0][0]);
                if let Some(&(_, rate)) = scrolled.iter().find(|(lane, _)| *lane == u) {
                    if found.is_some_and(|seen| seen != rate) {
                        return None;
                    }
                    found = Some(rate);
                }
            }
            let mut loads = Vec::new();
            let mut products = Vec::new();
            for lane in (0..4u8).filter(|lane| insn.mask & (1 << lane) != 0) {
                let dst = (insn.dst, insn.dst_half, lane);
                match insn.name() {
                    Some("MOV") if self.is_patched_by(insn, &clock_slots) => loads.push(dst),
                    Some("MAD") => {
                        if let Some(rate) = self.clock_product(insn, usize::from(lane), &clock) {
                            products.push((dst, rate));
                        }
                    }
                    _ => {}
                }
            }
            let writes = |lane: &Lane| {
                lane.0 == insn.dst && lane.1 == insn.dst_half && insn.mask & (1 << lane.2) != 0
            };
            clock.retain(|lane| !writes(lane));
            scrolled.retain(|(lane, _)| !writes(lane));
            clock.extend(loads);
            scrolled.extend(products);
        }
        found
    }

    /// Whether `insn`'s only source is the inline constant a patch in `slots` fills.
    fn is_patched_by(&self, insn: &Instruction, slots: &[u16]) -> bool {
        insn.sources[0] == Source::Constant
            && insn.const_slot.is_some_and(|slot| slots.contains(&slot))
    }

    /// The literal a `MAD` multiplies the clock by on `lane`, when it does.
    fn clock_product(&self, insn: &Instruction, lane: usize, clock: &[Lane]) -> Option<f32> {
        let literal = insn.const_slot.filter(|&slot| {
            !self
                .parameter_patches
                .iter()
                .any(|&(patched, _)| patched == slot)
        });
        literal?;
        [(0, 1), (1, 0)]
            .into_iter()
            .find_map(|(of_clock, of_rate)| {
                let Source::Register { index, half } = insn.sources[of_clock] else {
                    return None;
                };
                let read = (index, half, insn.swizzles[of_clock][lane]);
                (clock.contains(&read) && insn.sources[of_rate] == Source::Constant)
                    .then(|| {
                        insn.constant?
                            .get(usize::from(insn.swizzles[of_rate][lane]))
                            .copied()
                    })
                    .flatten()
            })
    }
}
