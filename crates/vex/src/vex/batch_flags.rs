//! What a batch's own flag words say about how it is drawn: its blend class,
//! whether it is a cutout, whether it is back-face culled, and the alpha-test
//! reference the GE compares its fragments against.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`, and the seam is a real one: everything here
//! decodes bits of [`Batch::pass_mask`] and [`Batch::header_flags`] and reads
//! nothing else, so the parser stays in `vex.rs` and the *meaning* of what it
//! parsed lives here.
//!
//! Every reading is against `Gfx_BuildBatchStateList` (`0x0891f890`) in the PSP
//! Pulse executable - see `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`.

use super::Batch;

/// Which blend equation a transparent batch asks for.
///
/// Recovered from `Gfx_BuildBatchStateList`; see [`Batch::blend_class`] for the
/// bits and the branch order. Not a quality setting and not a choice - the
/// batch's own `pass_mask` picks one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendClass {
    /// `pass_mask & 0x100`: ordinary alpha blend, `SrcAlpha` over
    /// `OneMinusSrcAlpha`.
    AlphaOver,
    /// `pass_mask & 0x200`: additive and source-alpha weighted, `SrcAlpha`
    /// plus `One`. The boost plume's, and the engine flare's.
    Additive,
    /// `pass_mask & 0x400`: sorted with the transparent batches but drawn
    /// unblended.
    None,
}

impl Batch {
    /// Whether this batch is transparent and must be drawn after opaque ones.
    ///
    /// The `0x0700` mask is confirmed operationally: it is the exact test
    /// `Gfx_BuildBatchStateList` branches on. Which of the three bits is set
    /// decides *how* it blends - see [`Batch::blend_class`].
    #[must_use]
    pub fn is_transparent(&self) -> bool {
        self.pass_mask & 0x0700 != 0
    }

    /// Which blend equation a transparent batch asks for, or `None` when it is
    /// not transparent at all.
    ///
    /// The three bits inside `is_transparent`'s `0x0700` are not
    /// interchangeable, and the reimplementation drew all of them with one
    /// equation until this was recovered. From `Gfx_BuildBatchStateList`, whose
    /// branch order this method reproduces - `0x100` wins over `0x200`, which
    /// wins over `0x400`:
    ///
    /// | Bit | The original programs |
    /// | --- | --- |
    /// | `0x100` | `GU_ADD`, `GU_SRC_ALPHA` / `GU_ONE_MINUS_SRC_ALPHA`, colour test off |
    /// | `0x200` | `GU_ADD`, `GU_SRC_ALPHA` / `GU_FIX 0xffffff`, colour test on |
    /// | `0x400` | `Gu_Disable(GU_BLEND)` - in the transparent class, not blended |
    ///
    /// Only the blend equation and the colour test differ between them; the
    /// depth-write disable, the alpha test and the stencil setup are emitted
    /// outside the nest and are common to all three.
    ///
    /// See `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`, "Three
    /// `pass_mask` bits decoded, inside the `0x0700` transparent class".
    #[must_use]
    pub fn blend_class(&self) -> Option<BlendClass> {
        if self.pass_mask & 0x0100 != 0 {
            Some(BlendClass::AlphaOver)
        } else if self.pass_mask & 0x0200 != 0 {
            Some(BlendClass::Additive)
        } else if self.pass_mask & 0x0400 != 0 {
            Some(BlendClass::None)
        } else {
            None
        }
    }

    /// Whether this batch is alpha-tested rather than blended.
    #[must_use]
    pub fn is_alpha_tested(&self) -> bool {
        self.pass_mask & 0x0800 != 0
    }

    /// Whether back-face culling is enabled. Set means two-sided.
    ///
    /// **The sense of the bit is the opposite of what it looks like, and this
    /// method used to have it backwards.** `Mesh_SetBatchDrawState` reads
    /// `if ((pass_mask & 0x20) == 0) { Gu_Enable(5) } else { Gu_Disable(5) }`,
    /// state index `5` being `GU_CULL_FACE` - so the bit **set** *disables*
    /// culling. Read at instruction level, confidence 90; see
    /// `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`, "The plume is
    /// two-sided".
    ///
    /// Nothing in this workspace consumes this yet - every `mesh_render`
    /// pipeline sets `cull_mode: None` deliberately, because strip winding is
    /// reconstructed rather than read from the file and culling would turn a
    /// winding mistake into missing geometry. So the inversion never reached a
    /// picture. It is corrected here because the first consumer would have
    /// culled exactly the surfaces the original draws two-sided, and on the
    /// boost plume - all 32 of whose batches carry the bit - that means every
    /// batch.
    #[must_use]
    pub fn is_culled(&self) -> bool {
        self.pass_mask & 0x0020 == 0
    }

    /// Whether the PSP draw path's shared per-batch state setup
    /// (`Mesh_SetBatchDrawState`) takes its pure-additive blend branch
    /// (`Gu_BlendFunc(GU_ADD, GU_FIX 0xffffff, GU_FIX 0xffffff)`, i.e.
    /// `dst + src`) rather than its "replace" branch (`GU_FIX 0xffffff,
    /// GU_FIX 0`, i.e. `src` alone, discarding `dst`). Selected by
    /// `header_flags & 0x10`, clear for additive.
    ///
    /// Confidence 85 for what this method decodes: the branch selection and
    /// blend-equation decode are confirmed against a live Ghidra decompile.
    /// That number is not a claim about any specific batch's real value -
    /// whether a given model's batches actually read additive, and whether
    /// `GU_BLEND` is actually *enabled* when they do, are separate,
    /// lower-confidence claims. See
    /// `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`.
    #[must_use]
    pub fn is_additive_blend(&self) -> bool {
        self.header_flags & 0x10 == 0
    }

    /// The alpha-test reference the GE compares this batch's fragment alpha
    /// against, or `None` when the batch is drawn with the alpha test off or
    /// with a function of `GU_ALWAYS`.
    ///
    /// The comparison is always `GU_GREATER` and the mask always `0xff`, so
    /// the reference is the whole of it: a fragment survives when its 8-bit
    /// alpha is **strictly greater** than the value returned here.
    ///
    /// # The branch, in the original's own order
    ///
    /// `Gfx_BuildBatchStateList` (`0x0891f890`) is handed this batch's
    /// `pass_mask` and `header_flags` directly - `Mesh_InitBatch`
    /// (`0x0890e8b4`) passes `*batch` and `*(u8 *)(batch + 3)` into
    /// `Gfx_AcquireBatchStateList` (`0x0891df48`), which interns them, and
    /// `Gfx_CompileDirtyBatchStateLists` (`0x0891e054`) replays them into the
    /// builder unchanged. No transform sits in between, which is what lets
    /// this method be a pure function of the two words the file authors:
    ///
    /// | Test, in order | What the original programs |
    /// | --- | --- |
    /// | `header_flags & 0x10` set | `Gu_Disable(GU_ALPHA_TEST)` - no test |
    /// | `pass_mask & 0x0700` set | `Gu_AlphaFunc(GU_GREATER, 0, 0xff)` |
    /// | `pass_mask & 0x0800` clear | `Gu_AlphaFunc(GU_ALWAYS, 0, 0xff)` - no test |
    /// | `header_flags & 0x20` set | `Gu_AlphaFunc(GU_GREATER, 0x10, 0xff)` |
    /// | `pass_mask & 0x0080` set | `Gu_AlphaFunc(GU_GREATER, 0, 0xff)` |
    /// | otherwise | `Gu_AlphaFunc(GU_GREATER, 0x7f, 0xff)` |
    ///
    /// The `0x0700` row is why a transparent batch gets `0` and never one of
    /// the other two: the enable and the func for that class sit **outside**
    /// the `0x100`/`0x200`/`0x400` nest, so every blended batch in the game
    /// shares them.
    ///
    /// # Confidence: 84
    ///
    /// Static decompilation with consistent call sites, which is exactly the
    /// rubric's ceiling for that evidence - `Gfx_BuildBatchStateList` has one
    /// argument-supplying chain and it does not transform either word. Not
    /// runtime-verified, so capped below 85.
    ///
    /// **The disc census does not raise it, and must not be cited as though it
    /// does.** Across PSP Pulse (8,561 cutout batches), PSP Pure (2,705) and
    /// PS2 Pulse (14,247), `pass_mask & 0x80` and `header_flags & 0x20` are
    /// *always set together and always clear together* - only `0x0800`/`0x00`
    /// and `0x0880`/`0x20` ever occur. That is agreement, not a test: were the
    /// selector `pass_mask & 0x80` alone, or the branch order reversed, the
    /// census would read identically. It does say the reading is
    /// self-consistent on every corpus this project can reach, and it leaves
    /// the branch **order** untestable here - a title that separated the two
    /// bits would settle it.
    ///
    /// The selector was read in the **PSP Pulse** executable only. PS2 Pulse
    /// and PSP Pure author the same two bit patterns in the same fields, which
    /// is why this method is not gated by platform, but neither of their
    /// executables has been read.
    #[must_use]
    pub fn alpha_test_reference(&self) -> Option<u8> {
        if self.header_flags & 0x10 != 0 {
            None
        } else if self.pass_mask & 0x0700 != 0 {
            Some(0)
        } else if self.pass_mask & 0x0800 == 0 {
            None
        } else if self.header_flags & 0x20 != 0 {
            Some(0x10)
        } else if self.pass_mask & 0x0080 != 0 {
            Some(0)
        } else {
            Some(0x7f)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch(pass_mask: u16, header_flags: u8) -> Batch {
        Batch {
            pass_mask,
            material_index: 0,
            primitive_type: super::super::PRIM_TRIANGLES,
            vertex_type: 0,
            scale: 1.0,
            vertices: Vec::new(),
            declared_vertex_count: 0,
            bounds: ([0.0; 3], [0.0; 3]),
            header_flags,
        }
    }

    /// The two patterns every corpus actually authors, per the census in
    /// `crates/vex/tests/alpha_test_reference_ground_truth.rs`.
    #[test]
    fn the_two_authored_patterns_pick_the_two_authored_references() {
        assert_eq!(batch(0x0800, 0x00).alpha_test_reference(), Some(0x7f));
        assert_eq!(batch(0x0880, 0x20).alpha_test_reference(), Some(0x10));
    }

    #[test]
    fn a_transparent_batch_gets_zero_whatever_else_it_sets() {
        for pass_mask in [0x0100, 0x0200, 0x0400, 0x0980] {
            assert_eq!(batch(pass_mask, 0x00).alpha_test_reference(), Some(0));
            assert_eq!(batch(pass_mask, 0x20).alpha_test_reference(), Some(0));
        }
    }

    #[test]
    fn a_batch_without_the_cutout_bit_is_not_alpha_tested() {
        assert_eq!(batch(0x0000, 0x00).alpha_test_reference(), None);
        assert_eq!(batch(0x0080, 0x20).alpha_test_reference(), None);
    }

    /// `header_flags & 0x10` is the outermost branch, so it wins over both the
    /// transparent class and the cutout bit.
    #[test]
    fn the_additive_header_bit_turns_the_test_off_entirely() {
        assert_eq!(batch(0x0800, 0x10).alpha_test_reference(), None);
        assert_eq!(batch(0x0200, 0x10).alpha_test_reference(), None);
    }

    /// The branch order the census cannot reach: a batch that sets
    /// `pass_mask & 0x80` **without** `header_flags & 0x20` is what separates
    /// the two candidate selectors, and no disc this project reads authors one.
    #[test]
    fn the_unauthored_pattern_follows_the_decompiles_own_order() {
        assert_eq!(batch(0x0880, 0x00).alpha_test_reference(), Some(0));
        assert_eq!(batch(0x0800, 0x20).alpha_test_reference(), Some(0x10));
    }
}
