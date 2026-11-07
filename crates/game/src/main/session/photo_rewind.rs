//! The session's half of photo mode's rewind and accumulated shot: moving the race on screen to
//! the tick the rewind row names, and drawing a shot of several frames one frame at a time.
//!
//! The rules are `oag_raceplay::photo::{rewind, shot}`; this is the frame loop around them. A
//! shot runs behind a progress bar and any input abandons it, as 2048's accumulation does
//! (`docs/ghidra/functions/vita-2048-eu-v104/photo-mode.md`, state 3). Each frame is read back
//! from the presentation target before the grade and before anything is drawn over it, like a
//! single shot, and the frames are averaged in 16 bits a channel.

use log::{debug, warn};
use oag_gameplay::input::InputSnapshot;
use oag_raceplay::photo::rewind::Rewind;
use oag_raceplay::photo::shot::{Mover, Plan, Shot};
use oag_ui::pointer::Pointer;

use crate::stage::Stage;

/// The stick deflection that abandons a shot under way: 2048's own threshold.
const ABANDON_STICK: f32 = 0.2;

/// A shot of several frames under way.
#[derive(Debug)]
pub(super) struct Job {
    shot: Shot,
    /// The channels of the frames so far, summed. At most 130 frames of 255 fit 16 bits.
    sum: Vec<u16>,
    size: (u32, u32),
    path: std::path::PathBuf,
    started: web_time::Instant,
}

impl super::Session {
    /// Moves the race on screen to the tick the rewind row names, when that is not where it
    /// stands.
    pub(super) fn photo_rewind_sync(&mut self) {
        let (Stage::Race(stage), Some(open)) = (&mut self.stage, self.photo.as_mut()) else {
            return;
        };
        let Some(photo) = stage.race.photo() else {
            return;
        };
        let target = photo.effects().target_tick();
        if target == open.asked {
            return;
        }
        open.asked = target;
        let Some(keeper) = stage.keeper.as_mut() else {
            return;
        };
        if open.rewind.is_none() {
            open.rewind = Rewind::begin(keeper, &mut stage.race);
        }
        let Some(rewind) = open.rewind.as_mut() else {
            stage.race.photo_effects(|effects| effects.set_target_tick(effects.live()));
            return;
        };
        let started = web_time::Instant::now();
        let reached = rewind.seek(keeper, &mut stage.race, target);
        debug!(
            "photo mode: rewound to tick {reached} (asked {target}, live {}) in {:?}",
            rewind.live(),
            started.elapsed()
        );
        // With no recording the race stands at a keyframe: the row shows where it really is.
        if reached != target {
            open.asked = reached;
            stage
                .race
                .photo_effects(|effects| effects.set_target_tick(reached));
        }
    }

    /// Whether the player did anything that abandons a shot under way.
    pub(super) fn photo_input_abandons(
        pad: &oag_raceplay::photo::Pad,
        pointer: &Pointer,
        input: &InputSnapshot,
    ) -> bool {
        pad.shot
            || pad.leave
            || pad.help
            || pad.reset
            || pad.previous
            || pad.next
            || pointer.clicked
            || pointer.back
            || input.stick_x.abs() > ABANDON_STICK
            || input.stick_y.abs() > ABANDON_STICK
    }

    /// Whether a shot of several frames is under way.
    pub(super) fn photo_job_running(&self) -> bool {
        self.photo.as_ref().is_some_and(|open| open.job.is_some())
    }

    /// How far a shot of several frames has got: `(frames drawn, frames)`.
    pub(super) fn photo_progress(&self) -> Option<(u32, u32)> {
        let job = self.photo.as_ref()?.job.as_ref()?;
        Some((job.shot.next(), job.shot.plan().frames()))
    }

    /// Begins the shot the mode asks for, written to `path` when it is done. `false` when it is
    /// the one frame on screen, which the caller then saves as it always did.
    pub(super) fn start_photo_job(&mut self, path: &std::path::Path) -> bool {
        let (Stage::Race(stage), Some(open)) = (&mut self.stage, self.photo.as_mut()) else {
            return false;
        };
        let Some(photo) = stage.race.photo() else {
            return false;
        };
        if photo.effects().blur().is_some()
            && open.rewind.is_none()
            && let Some(keeper) = stage.keeper.as_mut()
        {
            open.rewind = Rewind::begin(keeper, &mut stage.race);
        }
        let resimulates = open.rewind.as_ref().is_some_and(Rewind::resimulates);
        let tick = stage.race.sim.world.tick;
        let Some(photo) = stage.race.photo() else {
            return false;
        };
        let Some(plan) = Plan::new(photo, tick, resimulates) else {
            return false;
        };
        debug!(
            "photo mode: a shot of {} frames at tick {tick} (blur {:?}, lens {})",
            plan.frames(),
            plan.blur(),
            plan.has_lens()
        );
        let mover = open.rewind.as_mut().zip(stage.keeper.as_mut());
        let shot = Shot::start(
            plan,
            mover.map(|(rewind, keeper)| Mover { rewind, keeper }),
            &mut stage.race,
        );
        open.job = Some(Job {
            shot,
            sum: Vec::new(),
            size: (0, 0),
            path: path.to_path_buf(),
            started: web_time::Instant::now(),
        });
        true
    }

    /// Before a frame is drawn: stands the race at the shot's next frame and returns how far
    /// between its ticks to draw it. `None` with no shot under way.
    pub(super) fn photo_job_prepare(&mut self) -> Option<f32> {
        let (Stage::Race(stage), Some(open)) = (&mut self.stage, self.photo.as_mut()) else {
            return None;
        };
        let job = open.job.as_mut()?;
        let mover = open.rewind.as_mut().zip(stage.keeper.as_mut());
        Some(job.shot.prepare(
            mover.map(|(rewind, keeper)| Mover { rewind, keeper }),
            &mut stage.race,
        ))
    }

    /// After a frame is drawn and submitted: takes it into the sum, and writes the picture when
    /// it was the last.
    pub(super) fn photo_job_collect(&mut self) {
        if !self.photo_job_running() {
            return;
        }
        let frame = self.read_photo_output();
        let Some(open) = self.photo.as_mut() else {
            return;
        };
        let Some(job) = open.job.as_mut() else {
            return;
        };
        match frame {
            Ok((pixels, width, height)) => {
                if job.sum.is_empty() {
                    job.sum = vec![0; pixels.len()];
                    job.size = (width, height);
                }
                if job.size == (width, height) && job.sum.len() == pixels.len() {
                    for (sum, &channel) in job.sum.iter_mut().zip(&pixels) {
                        *sum += u16::from(channel);
                    }
                    job.shot.advance();
                }
            }
            Err(why) => {
                warn!("photo mode: cannot read a frame of the shot: {why:#}");
                self.photo_job_end(false);
                return;
            }
        }
        if job.shot.done() {
            self.photo_job_end(true);
        }
    }

    /// Abandons a shot under way: nothing is written.
    pub(super) fn photo_job_cancel(&mut self) {
        if self.photo_job_running() {
            self.photo_job_end(false);
        }
    }

    /// Ends the shot: the average of its frames is written when `finished`, and the race is put
    /// back at the tick the shot was taken at.
    fn photo_job_end(&mut self, finished: bool) {
        let Some(job) = self.photo.as_mut().and_then(|open| open.job.take()) else {
            return;
        };
        let Job {
            shot,
            sum,
            size,
            path,
            started,
        } = job;
        let frames = u32::from(finished) * shot.plan().frames();
        if let (Stage::Race(stage), Some(open)) = (&mut self.stage, self.photo.as_mut()) {
            let mover = open.rewind.as_mut().zip(stage.keeper.as_mut());
            shot.finish(
                mover.map(|(rewind, keeper)| Mover { rewind, keeper }),
                &mut stage.race,
            );
        }
        if !finished {
            self.photo_message(oag_game::photo::strings::CANCELLED);
            return;
        }
        let half = frames / 2;
        let pixels: Vec<u8> = sum
            .iter()
            .map(|&total| ((u32::from(total) + half) / frames) as u8)
            .collect();
        match oag_game::capture::write_png(&path, size.0, size.1, &pixels) {
            Ok(()) => {
                debug!(
                    "photo mode: saved {} ({frames} frames in {:?})",
                    path.display(),
                    started.elapsed()
                );
                self.photo_message(oag_game::photo::strings::SAVED);
            }
            Err(why) => {
                warn!("photo mode: cannot save {}: {why:#}", path.display());
                self.photo_message(oag_game::photo::strings::FAILED);
            }
        }
    }
}
