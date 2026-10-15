//! Draw calls that continue the one before are one draw call.
//!
//! A model's batches arrive in runs of one texture, and a batch's triangles sit right after
//! the previous batch's in the index buffer, so a circuit's several hundred batches are far
//! fewer `draw_indexed` calls. wgpu validates every recorded command when the encoder
//! finishes (about 0.6 us each on the NVIDIA desktop driver, HD's 800 batches a view are
//! half of the view's `encoder.finish`), so a call not recorded is the cost not paid.
//!
//! **The triangles reach the rasteriser in the same order**: only a range that starts exactly
//! where the pending one ends joins it, in the same pipeline state with the same texture
//! group, so the merged call is the two calls back to back.

/// The draw calls of one [`Drawable::draw_lists`], coalesced.
pub(super) struct Runs<'a> {
    textures: &'a [wgpu::BindGroup],
    /// Which texture group is bound to group 1 right now.
    bound: Option<usize>,
    /// The call not yet recorded: its texture slot and index range.
    pending: Option<(usize, std::ops::Range<u32>)>,
    binds: oag_gpu::perfprobe::Binds,
}

impl<'a> Runs<'a> {
    pub(super) fn new(textures: &'a [wgpu::BindGroup]) -> Self {
        Self {
            textures,
            bound: None,
            pending: None,
            binds: oag_gpu::perfprobe::Binds::default(),
        }
    }

    /// Records that group 1 already holds `slot` (bound by the caller, as the depth prepass
    /// binds the white fallback once for draws that read no texture).
    pub(super) fn assume_bound(&mut self, slot: usize) {
        self.bound = Some(slot);
    }

    /// Queues the draw of `range` with texture `slot`: it joins the pending call when it
    /// continues it, and otherwise the pending call is recorded first.
    pub(super) fn push(
        &mut self,
        pass: &mut wgpu::RenderPass<'_>,
        slot: usize,
        range: std::ops::Range<u32>,
    ) {
        self.binds.record(slot);
        oag_gpu::perfprobe::draw_queued();
        match &mut self.pending {
            Some((pending_slot, pending))
                if *pending_slot == slot && pending.end == range.start =>
            {
                pending.end = range.end;
            }
            _ => {
                self.flush(pass);
                self.pending = Some((slot, range));
            }
        }
    }

    /// Records the pending call. **Before any change of pipeline or index buffer**, since the
    /// call belongs to the state it was queued under.
    pub(super) fn flush(&mut self, pass: &mut wgpu::RenderPass<'_>) {
        let Some((slot, range)) = self.pending.take() else {
            return;
        };
        // Switched only when the slot changes, the same elision a per-batch bind ran:
        // batches sharing a texture arrive in runs - a model's own batch order groups them -
        // so this is not a bet. Carried across the lists rather than reset per list, because
        // a `set_pipeline` does not unbind anything: every pipeline a drawable owns is built
        // from one layout, which is also what lets groups 0, 2 and 3 be bound once at the
        // top. It restarts at `None` per `Drawable::draw_lists` call, where the texture
        // array itself changes.
        if self.bound != Some(slot) {
            pass.set_bind_group(1, &self.textures[slot], &[]);
            self.bound = Some(slot);
        }
        oag_gpu::perfprobe::draw_call();
        pass.draw_indexed(range, 0, 0..1);
    }
}
