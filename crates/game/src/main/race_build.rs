//! Building a race scene off the frame thread, behind the loading screen.
//!
//! **This is the freeze the loading screen used to end on.** Building the
//! scene - every mesh uploaded, every pipeline created, and wgpu parsing and
//! translating the WGSL behind each of those pipelines - measured 8.0 s in a
//! release build of Pulse PSP on a desktop, and it ran inside one frame of the
//! loading screen: the wave stopped, the window stopped answering, and only
//! then did the fade start. See `docs/architecture/race-load-transition.md`.
//!
//! So the build runs here, on a thread of its own, and the frame loop only
//! ever asks [`BuildWorker::take`] whether it has landed - a poll, never a
//! wait. `wgpu::Device` and `wgpu::Queue` are `Send + Sync`, and nothing the
//! scene holds is tied to the thread that made it, so the stage that comes
//! back is the same `RaceStage` the frame thread would have built.

use web_time::{Duration, Instant};

use log::debug;

use oag_game::settings;
use oag_gameplay::ControlScheme;
use oag_mesh::mesh_render::Anisotropy;
use oag_raceplay as race;

use crate::gpu::Handles;
use crate::loading_stage::RaceBuildError;
use crate::race_stage::RaceStage;
use crate::stage::Stage;

/// Everything [`Stage::build_race_stage`] and [`RaceStage::warm_up`] read,
/// copied off the session on the frame the circuit's load lands.
///
/// Copied rather than borrowed because the build outlives that frame. The
/// settings are the ones in force at that moment; a menu row changed while the
/// loading screen is up has nothing to reach anyway.
pub(crate) struct Request {
    pub(crate) gpu: Handles,
    /// The scene target's allocation, which the scene's depth attachment has
    /// to match - see [`Stage::race`].
    pub(crate) allocation: (u32, u32),
    /// The viewport the warmup draws: the framebuffer's extent, not its
    /// allocation. See `Session::advance_race_build`.
    pub(crate) extent: (u32, u32),
    /// Where the warmup draws: the scene target, idle while the loading
    /// screen draws straight to the presentation target (ADR-0038).
    pub(crate) warm_target: wgpu::TextureView,
    pub(crate) anisotropy: Anisotropy,
    pub(crate) settings: settings::Settings,
    pub(crate) render_profile: settings::RenderProfile,
    pub(crate) scheme: ControlScheme,
    pub(crate) autopilot: bool,
    pub(crate) autopilot_pilot: Option<oag_ai::Pilot>,
    pub(crate) autopilot_skill: Option<oag_ai::Difficulty>,
    pub(crate) track_entry: Option<String>,
    pub(crate) pvs_culling: bool,
    pub(crate) anim_seconds: Option<f32>,
}

/// What comes back: the stage, what it was built against, and how long the
/// two halves took on the worker.
pub(crate) struct Built {
    pub(crate) stage: Result<Box<RaceStage>, RaceBuildError>,
    /// The allocation the scene was sized for. A resize that landed while the
    /// build ran means the frame thread has to resize it before use.
    pub(crate) allocation: (u32, u32),
    pub(crate) build: Duration,
    pub(crate) warm_up: Duration,
}

/// The build, running.
pub(crate) struct BuildWorker {
    #[cfg(not(target_arch = "wasm32"))]
    handle: Option<std::thread::JoinHandle<Built>>,
    /// The web build has no threads, so the scene is built inside
    /// [`Self::spawn`] with its big uploads parked, and each [`Self::take`]
    /// writes a frame's worth of them. See [`Staged`].
    #[cfg(target_arch = "wasm32")]
    staged: Option<Staged>,
    /// Open from the build until the last parked write has landed.
    #[cfg(target_arch = "wasm32")]
    uploads: Option<oag_gpu::deferred_upload::Scope>,
    /// Frames the parked uploads have taken, for the log line.
    #[cfg(target_arch = "wasm32")]
    upload_frames: u32,
    allocation: (u32, u32),
}

/// What one frame of [`BuildWorker::take`] may spend writing parked uploads on
/// the web. **Chosen, not measured**: a third of a 60 Hz frame, so the loading
/// screen's own frame (a bar and a wave) still fits beside it.
#[cfg(target_arch = "wasm32")]
const UPLOAD_BUDGET: Duration = Duration::from_millis(5);

impl BuildWorker {
    /// Starts building `loaded` into a race scene.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn spawn(request: Request, loaded: race::Loaded) -> Self {
        let allocation = request.allocation;
        let uploads = oag_gpu::deferred_upload::Scope::open(&request.gpu.queue);
        let staged = Some(build_stage(request, loaded));
        Self {
            staged,
            uploads: Some(uploads),
            upload_frames: 0,
            allocation,
        }
    }

    /// Starts building `loaded` into a race scene.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn spawn(request: Request, loaded: race::Loaded) -> Self {
        let allocation = request.allocation;
        let handle = std::thread::Builder::new()
            // Named like `race-load` beside it, so a debugger and `top` say
            // which thread the loading screen is waiting on.
            .name("race-build".to_string())
            .spawn(move || build(request, loaded))
            .ok();
        Self { handle, allocation }
    }

    /// The result, once the build has returned; `None` while it is still
    /// running. **Never waits**, which is the whole point: the frame loop
    /// calls this once a frame and the loading screen keeps drawing.
    ///
    /// A worker whose thread would not spawn, or that panicked, reports a
    /// [`RaceBuildError::Gpu`] - the same fatal class a pipeline that would
    /// not build always was.
    ///
    /// On the web each call writes parked uploads for [`UPLOAD_BUDGET`], and
    /// the warmup draw waits until the last of them has landed: it would draw
    /// zeros otherwise.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn take(&mut self) -> Option<Built> {
        let Some(staged) = self.staged.take() else {
            return Some(Built {
                stage: Err(RaceBuildError::Gpu(anyhow::anyhow!(
                    "the race scene was already taken"
                ))),
                allocation: self.allocation,
                build: Duration::ZERO,
                warm_up: Duration::ZERO,
            });
        };
        if staged.stage.is_ok() && oag_gpu::deferred_upload::pending_bytes() > 0 {
            let start = Instant::now();
            let left = oag_gpu::deferred_upload::drain(|| start.elapsed() < UPLOAD_BUDGET);
            self.upload_frames += 1;
            if left > 0 {
                self.staged = Some(staged);
                return None;
            }
            debug!(
                "race scene uploads landed over {} frames, {:?} in the last",
                self.upload_frames,
                start.elapsed()
            );
        }
        self.uploads = None;
        Some(warm(staged))
    }

    /// The result, once the build has returned; `None` while it is still
    /// running.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn take(&mut self) -> Option<Built> {
        if self.handle.as_ref().is_some_and(|h| !h.is_finished()) {
            return None;
        }
        let failed = |why: &str| Built {
            stage: Err(RaceBuildError::Gpu(anyhow::anyhow!("{why}"))),
            allocation: self.allocation,
            build: Duration::ZERO,
            warm_up: Duration::ZERO,
        };
        Some(match self.handle.take() {
            Some(handle) => handle
                .join()
                .unwrap_or_else(|_| failed("the race scene's build thread panicked")),
            None => failed("the race scene's build thread would not start"),
        })
    }
}

/// A scene built but not yet warmed up: the half of [`build`] that has to
/// finish before the other can start, because on the web the first half leaves
/// uploads parked (see [`BuildWorker::take`]).
struct Staged {
    stage: Result<Box<RaceStage>, RaceBuildError>,
    gpu: Handles,
    warm_target: wgpu::TextureView,
    allocation: (u32, u32),
    extent: (u32, u32),
    fov: oag_display::display::Fov,
    frustum_culling: bool,
    pvs_culling: bool,
    anim_seconds: Option<f32>,
    build: Duration,
}

fn build(request: Request, loaded: race::Loaded) -> Built {
    warm(build_stage(request, loaded))
}

fn build_stage(request: Request, loaded: race::Loaded) -> Staged {
    let Request {
        gpu,
        allocation,
        extent,
        warm_target,
        anisotropy,
        settings,
        render_profile,
        scheme,
        autopilot,
        autopilot_pilot,
        autopilot_skill,
        track_entry,
        pvs_culling,
        anim_seconds,
    } = request;
    let start = Instant::now();
    let built = Stage::build_race_stage(
        &gpu,
        loaded,
        allocation,
        anisotropy,
        &settings,
        &render_profile,
        scheme,
        autopilot,
        autopilot_pilot,
        autopilot_skill,
        track_entry.as_deref(),
    );
    let build = start.elapsed();
    debug!("race scene built in {build:?}, off the frame thread");
    Staged {
        stage: built.map_err(RaceBuildError::Gpu),
        gpu,
        warm_target,
        allocation,
        extent,
        fov: settings.graphics.fov,
        frustum_culling: settings.graphics.frustum_culling,
        pvs_culling,
        anim_seconds,
        build,
    }
}

fn warm(staged: Staged) -> Built {
    let Staged {
        mut stage,
        gpu,
        warm_target,
        allocation,
        extent,
        fov,
        frustum_culling,
        pvs_culling,
        anim_seconds,
        build,
    } = staged;
    let warm_start = Instant::now();
    // **Building the pipeline objects is not the same as the driver having
    // compiled them.** Several backends defer that to the first real draw, so
    // the scene draws one frame here, still behind the loading screen. See
    // `RaceStage::warm_up`.
    if let Ok(race_stage) = &mut stage {
        race_stage.warm_up(
            &gpu,
            &warm_target,
            allocation,
            (0.0, 0.0, extent.0 as f32, extent.1 as f32),
            fov,
            frustum_culling,
            pvs_culling,
            anim_seconds,
        );
    }
    let warm_up = warm_start.elapsed();
    debug!("race scene warmed up in {warm_up:?}, off the frame thread");
    Built {
        stage,
        allocation,
        build,
        warm_up,
    }
}

/// Drops a race stage on a thread of its own, so releasing its GPU resources
/// does not cost the frame that let go of it.
///
/// A thread that will not spawn drops its closure, and the stage with it,
/// right here: a hitch rather than a leak.
#[cfg(target_arch = "wasm32")]
pub(crate) fn drop_off_thread(stage: Box<RaceStage>) {
    drop(stage);
}

/// See the wasm twin above: the web build has no thread to drop it on.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn drop_off_thread(stage: Box<RaceStage>) {
    let spawned = std::thread::Builder::new()
        .name("race-drop".to_string())
        .spawn(move || drop(stage));
    if let Err(e) = spawned {
        log::warn!("could not release the parked race off the frame thread: {e}");
    }
}

// `#[path]` for the reason `headless.rs` gives for its own: this module was
// loaded via `#[path = "main/race_build.rs"]`, so a bare `mod tests;` would
// resolve to `src/main/tests.rs`.
#[cfg(test)]
#[path = "race_build/tests.rs"]
mod tests;
