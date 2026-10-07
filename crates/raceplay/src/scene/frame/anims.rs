//! The scenery's per-frame animation tables, pulled out of `frame.rs` under
//! the 1,000-line rule in `scripts/check-file-size.py` - a move, with no
//! behaviour change.

impl super::super::Scene {
    /// Samples both animation mechanisms of every piece of scenery off the one
    /// clock and uploads them.
    pub(super) fn write_scenery_anims(&self, queue: &wgpu::Queue, seconds: f32) {
        for drawable in [
            Some(&self.track),
            self.sky.as_ref(),
            self.collision.as_ref(),
            self.pads.as_ref(),
            self.weapon_pads.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            drawable.write_anims(queue, seconds);
            // The scenery that *moves* rides the same clock as the scenery
            // that scrolls, so `--anim-seconds` aims both at once and a race
            // runs both off the tick.
            drawable.write_node_anims(queue, seconds);
        }
    }
}
