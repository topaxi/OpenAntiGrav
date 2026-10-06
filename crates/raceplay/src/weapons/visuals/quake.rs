//! The Quake wave's riding effect: [`Race::advance_quake_visual`].
//!
//! Split out of `visuals.rs` under the 1,000-line rule; a move.

use super::*;

impl Race {
    /// Keeps [`Trigger::Quake`] and its own transform riding the travelling
    /// wave, one instance for the whole race.
    ///
    /// Needs `self.sim.course` (to place the wave along the ring) and
    /// `self.sim.spline` (to read the track's own width there), which is why this
    /// cannot live in `oag_weapons::projectile::quake` at all - see that
    /// module's own doc comment on the split.
    ///
    /// **Recovered position, scale and orientation.** `Quake_Update`
    /// builds its transform from two edge points sampled across the track at
    /// the wave's own position: position is their midpoint, scale is their
    /// separation divided by `50.0` -
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`'s "What
    /// `Quake_Update` builds from those two points" section. This engine has
    /// no per-progress edge-point record of its own the way the original's
    /// `SplinePt` does; it recovers the same two points from
    /// [`Spline`]'s own sample at the ring point nearest the wave's own
    /// progress - `pos`/`lateral`/`half_width_left`/`half_width_right`, the
    /// same fields [`oag_render::track::build_model`] already draws the
    /// ribbon's own edges from.
    ///
    /// **What the `/ 50` scales, read 2026-09-24:** the instance's extent
    /// co-factor (`+0x2c`) and nothing else, so `WO_QUAKE`'s line emitters
    /// spread their fire edge to edge while each fireball keeps its authored
    /// size - see `oag_fx::psys::spawn`. Until then this fed the value
    /// in as severity, which made every fireball `width / 50` too big and
    /// stacked all of them on the midpoint. **Pulse on the PSP only**: every
    /// other source still feeds it in as severity, by choice, until its own
    /// executable is read (`oag_fx::psys::Effect::without_extents`). **The frame's `X` is measured**,
    /// `normalize(B - A)`; its `Y` is the track's own up at the midpoint,
    /// read 2026-10-06 (see the `orient` call below).
    ///
    /// Releases the instance the tick the wave goes away
    /// (`self.sim.world.quake` becomes `None`), as `Quake_Update` does
    /// (`Psys_ReleaseHandle`, `now != 0`): the same "hand the slot back, free
    /// the templates, let the particles fade" shape
    /// [`Race::advance_projectile_flares`] takes.
    pub(crate) fn advance_quake_visual(&mut self) {
        let Some(wave) = self.sim.world.quake else {
            self.view.quake_point = None;
            if let Some(playing) = self.view.quake_effect.take() {
                self.view.stage.release(playing);
            }
            return;
        };
        let Some(course) = self.sim.course.as_ref() else {
            return;
        };
        let Some(index) = course_index_near_progress(course, wave.progress) else {
            return;
        };
        let Some(position) = course.position(index) else {
            return;
        };
        let Some((_, sample, _)) = self.sim.spline.nearest(position) else {
            return;
        };
        let lateral = Vec3::from_array(sample.lateral);
        let centre = Vec3::from_array(sample.pos);
        let left = centre - lateral * sample.half_width_left;
        let right = centre + lateral * sample.half_width_right;
        let midpoint = (left + right) * 0.5;
        self.view.quake_point = Some(midpoint);
        let scale = (right - left).length() / 50.0;
        // The frame's `X` runs edge to edge: `Quake_Update`'s basis starts
        // from `normalize(B - A)` as its first row (`0x0891da08`, read
        // 2026-09-24).
        let across = right - left;

        let Some(effect) = self.view.handles.get(Trigger::Quake).cloned() else {
            return;
        };
        // Where the extent law is on (Pulse on the PSP), severity stays
        // `1.0` and the `/ 50` lands on the instance's extent co-factor, not
        // on size or speed - see `oag_fx::psys::spawn`. Everywhere else
        // the effect keeps what it did before that law was read: the `/ 50`
        // as severity, by the lead's choice until those executables are read
        // (`psys::Effect::without_extents`).
        let stretches = effect.has_extents();
        let severity = if stretches { 1.0 } else { scale };
        let playing = match self.view.quake_effect {
            None => {
                self.view.quake_effect = self.view.stage.attach(&effect, midpoint, severity);
                self.view.quake_effect
            }
            Some(playing) => {
                self.view.stage.follow(playing, midpoint);
                self.view.stage.rescale(playing, severity);
                Some(playing)
            }
        };
        // `Quake_Update` (`0x0891d268`) takes row 1 from the track record
        // `AiTrack_LocatePosition` fills at the midpoint: `-(+0x20)`, the
        // interpolated `down` axis negated (`vneg.q` at `0x0891daac`). Row 2
        // is `across x Y`, row 0 `Y x row 2` - the frame `frame_x` builds.
        // On Pulse's PSP only, with the extent law: no other source's frame
        // was read.
        if let (true, Some(playing)) = (stretches, playing) {
            self.view.stage.stretch(playing, scale, across);
            let up = (-Vec3::from_array(sample.down)).try_normalize();
            self.view.stage.orient(playing, up.unwrap_or(Vec3::Y));
        }
        // Every frame the wave's instance lives, at the span's first edge
        // point - `left` here, the same reading `across` makes (chosen: which
        // of `Quake_SampleSpan`'s two points is `A` is not read).
        if let (Some(_), Some(flash)) = (playing, &mut self.view.screen_flash) {
            flash.start(oag_fx::flash::QUAKE, left);
        }
    }
}
