//! A shot of several frames drawn again (`--from-png`): the motion blur frames photo mode
//! averaged, and the history a speed blur's zoom ring had.
//!
//! The capture drives the race to the first frame's tick and draws each frame of the shot's
//! plan as the race passes its tick, from the camera the shot had (a ship blur holds the camera
//! at the shot's tick; a track blur has it ride the craft, [`oag_raceplay::photo::mount`]).
//! Each frame is read back from the presentation target before the grade and without the HUD,
//! and the frames are averaged in 16 bits a channel (`crate::photo::average`), as the session's
//! shot is (`session::photo_rewind`). The capture then puts the average where the scene was and
//! draws the HUD over it once, at the shot's tick.

use anyhow::{Context, Result};
use oag_raceplay::photo::mount::Mount;
use oag_raceplay::photo::shot::Plan;

use super::spectrum;
use super::tick::advance_one_tick;
use super::*;
use crate::photo::average::Sum;

/// Frames the speed blur's ring is drawn at the shot's moment before the frames that are kept:
/// the history it feeds back (a weight of at most `0.25` a frame, `oag_post::hd_zoom`) has all
/// but settled by then. The paused mode has drawn it for as long as the person stayed in it.
const SETTLE_FRAMES: u64 = 40;

/// A shot to draw as the frames it was made of.
pub(super) struct Shot {
    plan: Plan,
    /// The camera that rides the craft: a track blur's.
    mount: Option<Mount>,
}

impl Shot {
    /// The shot the recorded text asks for at the capture's last tick, or `None` for the one
    /// frame: no recorded blur, a title that does not accumulate, or a race that is not played
    /// as one view.
    pub(super) fn of(
        options: &CaptureOptions,
        title: &oag_title::Title,
        race: &Race,
        aspect: f32,
    ) -> Option<Self> {
        let blurred = options.shot.blurred.as_ref()?;
        let Some(accumulation) = title.photo_shot else {
            println!("from-png: this title's photo mode does not accumulate frames; drawing one");
            return None;
        };
        if race.windowed() || race.view_state().viewport_count() > 1 {
            println!("from-png: a split screen is drawn as one frame");
            return None;
        }
        let plan = Plan::of(
            &blurred.effects,
            accumulation,
            u64::from(options.ticks),
            true,
            race.drawn_fov_degrees(aspect, options.fov),
        )?;
        if plan.frames() != blurred.frames {
            println!(
                "from-png: the shot was averaged from {} frames and this build's plan for it is {}; drawing the plan's",
                blurred.frames,
                plan.frames()
            );
        }
        Some(Self {
            plan,
            mount: blurred.mount,
        })
    }

    /// The tick before the first frame's: the race is driven this far before the frames begin,
    /// so that the first frame has a tick behind it to be drawn between.
    pub(super) fn driven(&self) -> u32 {
        u32::try_from(self.plan.first_stand().saturating_sub(1)).unwrap_or(u32::MAX)
    }
}

/// What a frame is drawn into and read back from.
pub(super) struct Rig<'a> {
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) scene: &'a mut Scene,
    pub(super) race: &'a mut Race,
    pub(super) held: &'a mut HeldButtons,
    pub(super) audio: &'a mut oag_sound::Audio,
    pub(super) options: &'a CaptureOptions,
    /// Where the scene is drawn.
    pub(super) view: &'a wgpu::TextureView,
    pub(super) viewport: (f32, f32, f32, f32),
    pub(super) camera_jitter: Option<u32>,
    /// Through the upscaler into the presentation target, when the capture is presented.
    pub(super) presented: Option<Resolve<'a>>,
    /// The capture's own target, read from when nothing is presented.
    pub(super) target: &'a wgpu::Texture,
}

/// The presentation target a presented capture's scene is resolved into.
pub(super) struct Resolve<'a> {
    pub(super) framebuffer: &'a mut oag_present::upscale::Framebuffer,
    pub(super) rect: (f32, f32, f32, f32),
    pub(super) presentation: oag_present::upscale::Presentation,
    pub(super) temporal_supported: bool,
}

/// The average of a shot's frames, and what its last frame drew.
pub(super) struct Averaged {
    pixels: Vec<u8>,
    size: (u32, u32),
    pub(super) stats: oag_present::perf::SceneStats,
}

impl Averaged {
    /// Records the copy of the average into the texture the HUD is drawn over.
    pub(super) fn encode(
        &self,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::Texture,
    ) {
        crate::photo::average::encode((device, queue), encoder, output, &self.pixels, self.size);
    }
}

impl Rig<'_> {
    /// Draws the race as it stands into the output, without the HUD.
    fn draw(&mut self) -> oag_present::perf::SceneStats {
        let (device, queue) = (self.device, self.queue);
        if self.scene.sync_zone_grade(self.race) {
            self.scene.rebind_zone_art(device, queue);
        }
        let spectrum = spectrum::feed(self.options, self.audio, self.scene);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("race capture frame"),
        });
        let options = self.options;
        let stats = self.scene.render(
            device,
            queue,
            &mut encoder,
            self.view,
            self.race,
            self.viewport,
            options.fov,
            options.frustum_culling,
            options.pvs_culling,
            options.anim_seconds,
            options.motion_blur,
            options.shadows,
            self.camera_jitter,
            &spectrum,
            None,
            None,
            None,
        );
        if let Some(resolve) = self.presented.as_mut() {
            let temporal = resolve
                .temporal_supported
                .then(|| self.scene.temporal(0))
                .flatten();
            let _ = resolve.framebuffer.resolve_scene(
                device,
                queue,
                &mut encoder,
                resolve.rect,
                &resolve.presentation,
                temporal,
                None,
            );
        }
        queue.submit(Some(encoder.finish()));
        stats
    }

    /// The texture the finished frame is read from: the presentation target, before the grade
    /// and anything drawn over it, or the capture's target.
    fn output(&self) -> &wgpu::Texture {
        match &self.presented {
            Some(resolve) => resolve.framebuffer.output_texture(),
            None => self.target,
        }
    }

    /// Reads the output back as RGBA.
    fn read(&self) -> Result<(Vec<u8>, (u32, u32))> {
        let output = self.output();
        let size = output.size();
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("race capture frame readback"),
            });
        let pixels = crate::capture::read_back(
            self.device,
            self.queue,
            encoder,
            output,
            size.width,
            size.height,
        )?;
        Ok((pixels, (size.width, size.height)))
    }

    /// One tick of the race, with the camera of a track blur on the craft before it and after,
    /// as the mode's rewind puts it (`photo::rewind`): the poses a frame blends from are taken
    /// under the camera of the tick before.
    fn tick(&mut self, mount: Option<&Mount>) {
        if let Some(mount) = mount {
            self.race.ride(mount);
        }
        let tick = u32::try_from(self.race.core.sim.world.tick).unwrap_or(u32::MAX);
        advance_one_tick(self.race, self.held, self.audio, self.options, tick);
        if let Some(mount) = mount {
            self.race.ride(mount);
        }
    }

    /// Draws the speed blur's ring for [`SETTLE_FRAMES`] ticks of its own at the moment the race
    /// stands at, so the history it feeds back is what the paused mode's had, then leaves the
    /// ring on the tick after: the frames that follow add one more step of it, as a
    /// shot's frames do. Does nothing without a speed blur.
    pub(super) fn settle_zoom(&mut self) {
        let blur = self
            .options
            .shot
            .picture
            .as_ref()
            .is_some_and(|picture| picture.speed_blur > 0);
        if !blur || !self.race.has_fixed_picture() {
            return;
        }
        self.race.set_draw_alpha(1.0);
        for age in 0..SETTLE_FRAMES {
            self.race.picture_set_age(age);
            self.draw();
        }
        self.race.picture_set_age(SETTLE_FRAMES);
    }

    /// Draws each frame of `shot` as the race passes its tick, and averages them.
    pub(super) fn run(&mut self, shot: &Shot) -> Result<Averaged> {
        let plan = &shot.plan;
        let mut sum = Sum::default();
        let mut stats = None;
        let mut settled = false;
        for index in 0..plan.frames() {
            let frame = plan.frame(index);
            while self.race.core.sim.world.tick < frame.stand {
                self.tick(shot.mount.as_ref());
            }
            if !settled {
                self.settle_zoom();
                settled = true;
            }
            self.race.photo_set_shift(frame.lens);
            self.race.set_draw_alpha(frame.alpha);
            stats = Some(self.draw());
            let (pixels, size) = self.read().context("reading a frame of the shot")?;
            anyhow::ensure!(sum.add(&pixels, size), "a frame of the shot changed size");
        }
        self.race.photo_set_shift(None);
        self.race.set_draw_alpha(1.0);
        println!(
            "from-png: drew {} frames from tick {} to tick {}",
            plan.frames(),
            plan.first_stand(),
            self.race.core.sim.world.tick
        );
        Ok(Averaged {
            pixels: sum.average(plan.frames()),
            size: sum.size(),
            stats: stats.context("the shot has no frame")?,
        })
    }
}
