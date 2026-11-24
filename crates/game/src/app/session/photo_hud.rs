//! The HUD of an accumulated photo mode shot: drawn once, over the average of the frames, at the
//! tick the shot is taken at.
//!
//! Every frame of the shot is drawn without the HUD, because a HUD drawn into each is averaged
//! over the shutter (ghosted digits, a thrust gauge that reads the recorded input). The average
//! is copied into the presentation target by the frame that follows the last, which then draws
//! the HUD over it as it draws it for a single shot, and that frame is read back.

use super::Session;

/// The average of a finished shot, until the HUD is over it.
#[derive(Debug)]
pub(crate) struct Averaged {
    pub(super) pixels: Vec<u8>,
    pub(super) size: (u32, u32),
    pub(super) path: std::path::PathBuf,
    pub(super) pending: Option<super::photo_meta::Pending>,
    pub(super) frames: u32,
    pub(super) started: web_time::Instant,
    /// The frame in flight carries the average under its HUD.
    pub(super) under_hud: bool,
}

impl Session {
    /// Whether a shot still has a frame to draw: a frame of it, the one that puts the HUD over
    /// its average, the one that shows its bar before it begins, or the one a single
    /// frame shot is read from.
    pub(crate) fn photo_wants_frames(&self) -> bool {
        self.photo_job_running()
            || self.photo.as_ref().is_some_and(|open| {
                open.averaged.is_some() || open.announced.is_some() || open.shot.is_some()
            })
    }

    /// Whether the frame being drawn leaves the HUD out: a shot of several frames is under way.
    pub(super) fn photo_hides_hud(&self) -> bool {
        self.photo_job_running()
    }

    /// Records the copy of the finished average into the presentation target, after the scene is
    /// resolved into it and before the HUD goes on. Does nothing with no average waiting.
    pub(super) fn photo_encode_average(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let Some(averaged) = self.photo.as_mut().and_then(|open| open.averaged.as_mut()) else {
            return;
        };
        if self.framebuffer.output_size() != averaged.size {
            return;
        }
        crate::photo::average::encode(
            (&self.gpu.device, &self.gpu.queue),
            encoder,
            self.framebuffer.output_texture(),
            &averaged.pixels,
            averaged.size,
        );
        averaged.under_hud = true;
    }

    /// After the frame is submitted: writes the shot, from the frame the HUD went onto, or from
    /// the average itself when the frame could not carry it.
    pub(super) fn photo_finish_averaged(&mut self) {
        let Some(mut averaged) = self.photo.as_mut().and_then(|open| open.averaged.take()) else {
            return;
        };
        if averaged.under_hud {
            match self.read_photo_output() {
                Ok((pixels, ..)) => averaged.pixels = pixels,
                Err(why) => log::warn!("photo mode: cannot read the shot with its HUD: {why:#}"),
            }
        }
        self.photo_write_shot(averaged);
    }
}
