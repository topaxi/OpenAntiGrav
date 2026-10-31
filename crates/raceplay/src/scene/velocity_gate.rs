//! Whether the frame being drawn reads the velocity buffer, and so whether the
//! track and the hulls write it.
//!
//! The velocity attachment used to be written on every frame, by design, so that
//! it was always there for whatever came to read it. It cost 0.3 to 0.5 ms of a
//! 3.5 to 4.5 ms Wipeout HD race pass at 1080p (about a tenth, `docs/rendering/hd-gpu-cost.md`)
//! on frames where nothing read it, which is every frame at the defaults: the blur
//! is off and the upscaler is not temporal. The track and the hulls, where nearly
//! every velocity fragment is, are now built [`mesh_render::Velocity::Gated`] and
//! draw with the write masked unless this frame has a reader. Nothing else about the
//! draw changes: colour and depth are bit-identical either way.
//!
//! **[`Readers`] is the one list of what reads the buffer.** Adding a consumer means
//! adding a field here, and `Scene::render` derives the gate from [`Readers::any`] and
//! from nothing else.
//!
//! **No stale read on the frame a reader switches on**: the buffer is cleared at the
//! top of every race pass and the gate is decided from the same frame's arguments as
//! the readers themselves, so a reader that runs this frame finds this frame's motion,
//! never the previous frame's and never a hole.

use super::*;

/// Everything that reads [`Scene::velocity`] after the race pass.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Readers {
    /// The motion blur's prepare stage (`Scene::encode_motion_blur`): the chain is
    /// built and the strength is above zero. A split view is drawn with the blur off
    /// and the blur runs after the last view, so a split's strength comes through
    /// `Scene::split_blur_reads`.
    pub motion_blur: bool,
    /// A temporal upscaler, through `Scene::temporal`: it hands the velocity buffer
    /// out only for a frame drawn with camera jitter, and jitter is what a temporal
    /// upscaler asks for (`--camera-jitter` alone also asks, and gets the write).
    pub temporal_upscaler: bool,
}

impl Readers {
    /// Whether anything reads the buffer this frame.
    pub(crate) fn any(self) -> bool {
        let Self {
            motion_blur,
            temporal_upscaler,
        } = self;
        motion_blur || temporal_upscaler
    }
}

impl Scene {
    /// The readers of this frame's velocity buffer, from the arguments
    /// [`Scene::render`] draws it with.
    pub(crate) fn velocity_readers(
        &self,
        motion_blur: oag_display::display::MotionBlur,
        camera_jitter: Option<u32>,
    ) -> Readers {
        Readers {
            motion_blur: self.motion_blur.is_some()
                && (motion_blur.shutter() > 0.0 || self.split_blur_reads.get()),
            temporal_upscaler: camera_jitter.is_some(),
        }
    }

    /// Tells the gated drawables whether this frame's draws write the velocity buffer.
    pub(super) fn gate_velocity(&self, readers: Readers) {
        let read = readers.any();
        self.track.set_velocity_read(read);
        for hull in &self.ships {
            hull.set_velocity_read(read);
        }
    }

    /// Whether the last frame told the track to write velocity.
    #[cfg(test)]
    pub(crate) fn velocity_gate_open(&self) -> bool {
        self.track.velocity_read()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_reader_alone_opens_the_gate_and_none_closes_it() {
        assert!(!Readers::default().any());
        assert!(
            Readers {
                motion_blur: true,
                ..Readers::default()
            }
            .any()
        );
        assert!(
            Readers {
                temporal_upscaler: true,
                ..Readers::default()
            }
            .any()
        );
    }
}
