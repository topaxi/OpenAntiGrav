//! HD's Track Select fly-by: the selected circuit's `preview.bik` playing in
//! the `FlyByMovie` panel (`oag_ui_screens::picker::hd::track`).
//!
//! The clip is opened on a thread of its own through [`crate::clip`], looped,
//! and restarted from its first frame whenever the selection lands on another
//! circuit (**chosen**: the restart is not measured). Until the new circuit's
//! first frame has arrived the panel draws nothing, never the last circuit's
//! frame, and a circuit that ships no clip says so in the log.

use std::sync::Arc;

use anyhow::anyhow;
use log::warn;
use oag_ui_screens::picker;

use super::{PickerStage, PreviewSource};
use crate::app::gpu::Gpu;
use crate::clip::{self, Player};
use crate::render::Renderer;

/// The clip playing for the selected circuit.
pub(super) struct FlyBy {
    /// The archive entry the clip is read from.
    entry: String,
    player: Player,
    /// Seconds since the clip started.
    elapsed: f32,
}

impl PickerStage {
    /// Advances the clip's clock by `dt` seconds.
    pub(crate) fn tick_fly_by(&mut self, dt: f32) {
        if let Some(fly_by) = self.fly_by.as_mut() {
            fly_by.elapsed += dt;
        }
    }

    /// Opens the selected circuit's clip when the selection changed, and puts
    /// the frame due now into `renderer`'s fly-by planes. A screen with no
    /// `FlyByMovie` does nothing.
    pub(crate) fn pump_fly_by(&mut self, renderer: &mut Renderer, gpu: &Gpu) {
        let Some(clip) = self
            .previews
            .front_end
            .and_then(|front_end| front_end.circuit_fly_by)
        else {
            return;
        };
        if self
            .layout
            .hd_track
            .as_ref()
            .is_none_or(|screen| screen.fly_by.is_none())
        {
            return;
        }
        let Some(PreviewSource::Track { location, .. }) = self.sources.get(self.model.index())
        else {
            return;
        };
        let entry = picker::hd::track::fly_by_src(location, clip);
        if self.fly_by.as_ref().map(|playing| playing.entry.as_str()) != Some(entry.as_str()) {
            renderer.clear_fly_by();
            let archives = Arc::clone(&self.archives);
            let cache = oag_source::cache::default_cache_dir();
            let name = entry.clone();
            self.fly_by = Some(FlyBy {
                entry,
                player: Player::spawn("track select fly-by", move || {
                    let blob = archives
                        .lock()
                        .map_err(|_| anyhow!("the archives' lock is poisoned"))?
                        .read_name(&name)?;
                    clip::open(&name, &blob, &cache)
                }),
                elapsed: 0.0,
            });
        }
        let Some(fly_by) = self.fly_by.as_mut() else {
            return;
        };
        let Some((format, (rate, per))) = fly_by.player.shape() else {
            return;
        };
        if let Err(why) = renderer.set_fly_by(&gpu.device, format) {
            warn!("track select fly-by: {why:#}");
            return;
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a frame count from a few minutes of seconds at a clip's rate"
        )]
        let position = (f64::from(fly_by.elapsed) * rate as f64 / per.max(1) as f64) as u64;
        fly_by.player.pump(position, |frame| {
            renderer.upload_fly_by_frame(&gpu.queue, frame)
        });
    }
}
