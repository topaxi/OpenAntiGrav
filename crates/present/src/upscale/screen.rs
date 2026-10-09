//! The screen filter's place in the frame: between the presentation target
//! and the grade.
//!
//! Split out of `upscale.rs` for the 1,000-line file ratchet
//! (`just check-size`), the same reason `blit.rs` was - this is the parent's
//! own pass ordering and nothing else uses it.
//!
//! # Where it sits, and why
//!
//! `Framebuffer::composite` is the one pass every frame goes through and the
//! last one, which is what put brightness and gamma there; a display
//! simulation wants the same two properties for the same reason - a CRT
//! showed the menus too - and one more: it has to come *before* the grade.
//! The grade is a calibration of the player's monitor, applied to the
//! finished picture; the filter *is* the picture, a different display drawn
//! onto this one. So the order is presentation target, filter, grade,
//! surface, and the cost is one more presentation-sized pass and the pass's
//! own two history targets, paid only while a preset is selected. The
//! performance overlay draws onto the surface after all of this and stays
//! outside it, on purpose - it is an instrument. See
//! [ADR-0053](../../../../docs/architecture/adr/0053-screen-filters-are-loadable-wgsl-after-the-composite.md).
//!
//! # A pass built once per revision
//!
//! [`Framebuffer::set_screen_filter`] is called every frame with whatever
//! preset the profile names, and rebuilds the pass only when the id or the
//! [`Preset::revision`] moved - a settings change, or a user file saved. A
//! preset whose shader will not build is reported once and the frame is
//! drawn unfiltered until its revision moves again, which is the contract
//! `oag_post::screen` states and `crate::screen` relies on for
//! edit-save-look authoring.

use web_time::Instant;

use log::{debug, warn};

use oag_display::display::{Brightness, FilterStrength, Gamma};
use oag_post::screen::{self, Preset};

/// The preset a [`super::Framebuffer`] is currently filtering with, and the
/// pass built from it.
#[derive(Debug)]
pub(super) struct ScreenFilter {
    /// The `(id, revision)` the pass was built from - what decides whether a
    /// preset handed over next frame is the same shader.
    id: String,
    revision: u64,
    /// The preset's tunables, as the uniform wants them.
    params: [[f32; 4]; 4],
    /// `Err` is kept rather than dropped so a shader that will not build is
    /// reported once and then skipped, not retried every frame.
    pass: Result<screen::Screen, ()>,
    /// When the pass was built: the origin of the uniform's `time`.
    built: Instant,
}

impl ScreenFilter {
    fn same(&self, preset: &Preset) -> bool {
        self.id == preset.id && self.revision == preset.revision
    }
}

/// What a frame hands the filter beyond the picture.
#[derive(Debug, Clone, Copy)]
pub struct ScreenFrame {
    /// The grid the running title authors in - `oag_display::space::Space::size`.
    pub native: (f32, f32),
    /// The profile's strength row.
    pub strength: FilterStrength,
}

/// Everything [`super::Framebuffer::composite`] is told about the frame it
/// puts on the surface: the player's grade, and what the filter is handed.
#[derive(Debug, Clone, Copy)]
pub struct Composite {
    pub brightness: Brightness,
    pub gamma: Gamma,
    pub screen: ScreenFrame,
}

impl super::Framebuffer {
    /// Makes sure the filter [`composite`](super::Framebuffer::composite)
    /// runs is `preset`, building or rebuilding the pass if it is not.
    ///
    /// `None` drops the pass and its two targets: `off` costs nothing.
    pub fn set_screen_filter(&mut self, device: &wgpu::Device, preset: Option<&Preset>) {
        let Some(preset) = preset else {
            if self.screen.take().is_some() {
                debug!("screen filter: off");
            }
            return;
        };
        if self
            .screen
            .as_ref()
            .is_some_and(|filter| filter.same(preset))
        {
            return;
        }
        let pass = match screen::Screen::new(device, self.format, preset) {
            Ok(pass) => {
                debug!(
                    "screen filter: {} ({}, revision {})",
                    preset.name, preset.id, preset.revision
                );
                Ok(pass)
            }
            // The one place a player's own file reaches the GPU, and the one
            // place its mistakes must land as a line rather than a crash.
            Err(why) => {
                warn!(
                    "screen filter {} did not build; drawing unfiltered until it changes:\n{why:#}",
                    preset.id
                );
                Err(())
            }
        };
        self.screen = Some(ScreenFilter {
            id: preset.id.clone(),
            revision: preset.revision,
            params: preset.defaults(),
            pass,
            built: Instant::now(),
        });
    }

    /// Whether a filter is selected *and* its pass built - what a test asks
    /// to tell "filtered" from "fell back".
    #[must_use]
    pub fn screen_filter_active(&self) -> bool {
        self.screen
            .as_ref()
            .is_some_and(|filter| filter.pass.is_ok())
    }

    /// Runs the filter over the presentation target, if one is built, and
    /// returns the bind group the grade should read instead of the target's
    /// own. `None` means "read the target".
    pub(super) fn filter_output(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: ScreenFrame,
    ) -> Option<wgpu::BindGroup> {
        let filter = self.screen.as_mut()?;
        let pass = filter.pass.as_mut().ok()?;
        pass.render(
            device,
            queue,
            encoder,
            screen::Frame {
                source: &self.output.view,
                size: self.output.size,
                native: frame.native,
                time: filter.built.elapsed().as_secs_f32(),
                strength: frame.strength.factor(),
                params: filter.params,
            },
        );
        pass.output().map(|view| {
            super::bind(
                device,
                &self.layout,
                &self.sampler,
                &self.output.grade,
                view,
            )
        })
    }
}
