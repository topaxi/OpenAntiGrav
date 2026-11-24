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
    effects: oag_raceplay::photo::effects::Effects,
    accumulation: oag_title::photo::Accumulation,
    aspect: f32,
    /// The field the lens ring was first sized for, on the grid.
    first_fov: f32,
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
        let first_fov = race.drawn_fov_degrees(aspect, options.fov);
        let shot = Self {
            plan: Plan::of(
                &blurred.effects,
                accumulation,
                u64::from(options.ticks),
                true,
                first_fov,
            )?,
            mount: blurred.mount,
            effects: blurred.effects,
            accumulation,
            aspect,
            first_fov,
        };
        if shot.plan.frames() != blurred.frames {
            println!(
                "from-png: the shot was averaged from {} frames and this build's plan for it is {}; drawing the plan's",
                blurred.frames,
                shot.plan.frames()
            );
        }
        Some(shot)
    }

    /// Sizes the lens ring in the field the race draws with now: live photo mode sizes it at the
    /// shot's tick, where the speed has widened the field, and the race is driven to the tick
    /// before the first frame by the time this is called again (`Self::driven` does not depend
    /// on it).
    pub(super) fn refit(&mut self, race: &Race, options: &CaptureOptions) {
        let fov = race.drawn_fov_degrees(self.aspect, options.fov);
        if let Some(plan) = Plan::of(
            &self.effects,
            self.accumulation,
            self.plan.tick(),
            true,
            fov,
        ) {
            if plan.has_lens() {
                println!(
                    "from-png: the lens ring is sized for a field of {fov:.2} degrees at tick {} (the grid's is {:.2})",
                    race.core.sim.world.tick, self.first_fov
                );
            }
            self.plan = plan;
        }
    }

    /// The tick before the first frame's: the race is driven this far before the frames begin,
    /// so that the first frame has a tick behind it to be drawn between.
    pub(super) fn driven(&self) -> u32 {
        u32::try_from(self.plan.first_stand().saturating_sub(1)).unwrap_or(u32::MAX)
    }
}

/// One thing the capture does to the race on the way to a shot's frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// A tick of the race, the camera of a track blur on the craft before it and after.
    Tick,
    /// The track blur's camera on the craft as the race stands: before every frame, because a
    /// frame whose tick the race is already at has had no tick to put it there.
    Ride,
    /// Draw frame `index` of the plan.
    Frame(u32),
}

/// The steps from a race at tick `now` through every frame of `plan`.
fn steps(plan: &Plan, mut now: u64, riding: bool) -> Vec<Step> {
    let mut steps = Vec::new();
    for index in 0..plan.frames() {
        while now < plan.frame(index).stand {
            steps.push(Step::Tick);
            now += 1;
        }
        if riding {
            steps.push(Step::Ride);
        }
        steps.push(Step::Frame(index));
    }
    steps
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
    /// shot's frames do. The frames keep that tick: live photo mode's shot runs with the mode's
    /// own tick standing still (`session::photo`'s `photo_wants_frames` returns before the mode
    /// steps), so the ring's history is not stepped between its frames. Does nothing without a
    /// speed blur.
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
        let start = self.race.core.sim.world.tick;
        for step in steps(plan, start, shot.mount.is_some()) {
            match step {
                Step::Tick => self.tick(shot.mount.as_ref()),
                Step::Ride => {
                    if let Some(mount) = &shot.mount {
                        self.race.ride(mount);
                    }
                }
                Step::Frame(index) => {
                    let frame = plan.frame(index);
                    if index == 0 {
                        self.settle_zoom();
                    }
                    self.race.photo_set_shift(frame.lens);
                    self.race.set_draw_alpha(frame.alpha);
                    stats = Some(self.draw());
                    let (pixels, size) = self.read().context("reading a frame of the shot")?;
                    anyhow::ensure!(sum.add(&pixels, size), "a frame of the shot changed size");
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use oag_raceplay::photo::effects::{Effects, Row};

    fn plan(tick: u64) -> Plan {
        let mut effects = Effects::new().offering(true, true);
        effects.set(Row::MotionBlur, 1);
        effects.set(Row::Shutter, 100);
        Plan::of(
            &effects,
            oag_title::photo::Accumulation::HD,
            tick,
            true,
            65.0,
        )
        .expect("a track blur")
    }

    #[test]
    fn a_frame_the_race_already_stands_at_has_the_camera_ridden_onto_it() {
        // The shot is early enough that its first frames stand at tick 0, where the race begins:
        // no tick runs before them.
        let early = plan(10);
        assert_eq!(early.first_stand(), 0);
        let steps = steps(&early, 0, true);
        assert_eq!(steps[..2], [Step::Ride, Step::Frame(0)]);
        for (at, step) in steps.iter().enumerate() {
            if matches!(step, Step::Frame(_)) {
                assert_eq!(steps[at - 1], Step::Ride, "{at}");
            }
        }
        assert_eq!(
            steps.iter().filter(|step| **step == Step::Tick).count(),
            10,
            "the race is driven to the shot's tick"
        );
    }

    #[test]
    fn a_ship_blur_rides_nothing() {
        let steps = steps(&plan(300), 285, false);
        assert!(!steps.contains(&Step::Ride));
        assert_eq!(steps.last(), Some(&Step::Frame(119)));
    }
}
