//! What a batch's own flag words say about how it is drawn: its blend class,
//! whether it is a cutout, whether it is back-face culled, and the alpha-test
//! reference the GE compares its fragments against.
//!
//! Split out of [`super`] under the 1,000-line rule: everything here decodes
//! bits of [`Batch::pass_mask`] and [`Batch::header_flags`] and reads nothing
//! else, so the parser stays in `vex.rs` and the *meaning* lives here.
//!
//! Every reading is against `Gfx_BuildBatchStateList` (`0x0891f890`) in the PSP
//! Pulse executable; see `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`.

use super::Batch;

/// Which blend equation a transparent batch asks for, from `Gfx_BuildBatchStateList`
/// (see [`Batch::blend_class`]). Not a quality setting: the batch's `pass_mask`
/// picks one.
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
    /// Whether this batch is transparent and must be drawn after opaque ones. The
    /// `0x0700` mask is the exact test `Gfx_BuildBatchStateList` branches on;
    /// which bit is set decides *how* it blends ([`Batch::blend_class`]).
    #[must_use]
    pub fn is_transparent(&self) -> bool {
        self.pass_mask & 0x0700 != 0
    }

    /// Which blend equation a transparent batch asks for, or `None` when it is not
    /// transparent.
    ///
    /// The three bits inside `is_transparent`'s `0x0700` are not interchangeable
    /// (all were once drawn with one equation). From `Gfx_BuildBatchStateList`,
    /// whose branch order this reproduces (`0x100` over `0x200` over `0x400`):
    ///
    /// | Bit | The original programs |
    /// | --- | --- |
    /// | `0x100` | `GU_ADD`, `GU_SRC_ALPHA` / `GU_ONE_MINUS_SRC_ALPHA`, colour test off |
    /// | `0x200` | `GU_ADD`, `GU_SRC_ALPHA` / `GU_FIX 0xffffff`, colour test on |
    /// | `0x400` | `Gu_Disable(GU_BLEND)` - in the transparent class, not blended |
    ///
    /// Only the blend equation and colour test differ; the depth-write disable,
    /// alpha test and stencil setup sit outside the nest, common to all three.
    /// See `mesh-draw.md`, "Three `pass_mask` bits decoded, inside the `0x0700`
    /// transparent class".
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
    /// **The sense of the bit is the opposite of what it looks like.**
    /// `Mesh_SetBatchDrawState` reads `if ((pass_mask & 0x20) == 0) { Gu_Enable(5)
    /// } else { Gu_Disable(5) }`, state `5` being `GU_CULL_FACE`: the bit **set**
    /// *disables* culling. Instruction level, confidence 90; see
    /// `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`, "The plume is
    /// two-sided".
    ///
    /// Nothing consumes this yet (every `mesh_render` pipeline sets `cull_mode:
    /// None`, since strip winding is reconstructed and culling would turn a
    /// winding mistake into missing geometry), so the old inversion never reached
    /// a picture. The first consumer would have culled the surfaces the original
    /// draws two-sided: on the boost plume, all 32 batches.
    #[must_use]
    pub fn is_culled(&self) -> bool {
        self.pass_mask & 0x0020 == 0
    }

    /// Whether the PSP draw path's per-batch state setup (`Mesh_SetBatchDrawState`)
    /// takes its pure-additive branch (`Gu_BlendFunc(GU_ADD, GU_FIX 0xffffff,
    /// GU_FIX 0xffffff)`, `dst + src`) rather than "replace" (`GU_FIX 0xffffff,
    /// GU_FIX 0`, `src` alone). Selected by `header_flags & 0x10`, clear for
    /// additive.
    ///
    /// Confidence 85 for the decode (branch selection and blend equation, against
    /// a live Ghidra decompile), not for any batch's real value: whether a model's
    /// batches read additive, and whether `GU_BLEND` is *enabled* then, are
    /// separate, lower-confidence claims. See
    /// `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`.
    ///
    /// **This reads the file's byte; the running game's can differ.**
    /// `Mesh_CountBatchesPerList` ORs `0x10` into every list-A batch with
    /// `pass_mask & 0x2000` and without `0x0800` at load, so those draw "replace"
    /// whatever the disc says (live on Assegai's hull: five batches at runtime,
    /// none on disc). See "The hull's extra pass" in that page.
    #[must_use]
    pub fn is_additive_blend(&self) -> bool {
        self.header_flags & 0x10 == 0
    }

    /// The alpha-test reference the GE compares this batch's fragment alpha
    /// against, or `None` when the test is off or `GU_ALWAYS`.
    ///
    /// The comparison is always `GU_GREATER` with mask `0xff`: a fragment survives
    /// when its 8-bit alpha is **strictly greater** than the returned value.
    ///
    /// # The branch, in the original's own order
    ///
    /// `Gfx_BuildBatchStateList` (`0x0891f890`) gets this batch's `pass_mask` and
    /// `header_flags` untransformed (`Mesh_InitBatch` `0x0890e8b4` passes them to
    /// `Gfx_AcquireBatchStateList` `0x0891df48`, which interns them, and
    /// `Gfx_CompileDirtyBatchStateLists` `0x0891e054` replays them), so this is a
    /// pure function of the two words the file authors:
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
    /// The `0x0700` row is why a transparent batch gets `0`: the enable and func
    /// for that class sit **outside** the `0x100`/`0x200`/`0x400` nest.
    ///
    /// # Confidence: 86
    ///
    /// The decompile is unambiguous and `Gfx_BuildBatchStateList` has exactly one
    /// argument-supplying chain, transforming neither word: the rubric's 84 for
    /// "decompilation only, consistent call sites". Two points more because the
    /// shipped data separates this reading from its rival, *rendered*, not only
    /// counted: every corpus authors three of the four selector-bit combinations,
    /// and the third (`pass_mask & 0x80` set, `header_flags & 0x20` **clear**) is
    /// what they disagree about. This branch gives `0`; a selector on
    /// `header_flags & 0x20` alone gives `0x7f`. Those batches are Wipeout Pure's
    /// `Speedup Pad` (349), whose glow texture tops out at alpha `58/255`, so the
    /// rival discards the pad whole, the regression
    /// `crates/render/tests/pad_alpha_test_ground_truth.rs` catches by counting
    /// lit pixels in a real capture. Short of 90: no runtime trace saw the GE
    /// programmed this way.
    ///
    /// The **counts** by pattern, across three corpora, are frozen in
    /// `crates/vex/tests/alpha_test_reference_ground_truth.rs`:
    ///
    /// | `pass_mask & 0x880` | `header_flags & 0x30` | reference | PSP Pulse | PSP Pure | PS2 Pulse |
    /// | --- | --- | --- | ---: | ---: | ---: |
    /// | `0x0800` | `0x00` | `0x7f` | 1,500 | 956 | 1,313 |
    /// | `0x0880` | `0x00` | `0` | - | 349 | 20 |
    /// | `0x0880` | `0x20` | `0x10` | 7,823 | 1,749 | 14,046 |
    ///
    /// The selector was read in the **PSP Pulse** executable only; PS2 Pulse and
    /// PSP Pure author the same patterns in the same fields (so no platform gate),
    /// but neither executable has been read.
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
