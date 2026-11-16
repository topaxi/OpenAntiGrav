//! How a split screen records its views: [`Recording`], and the batched path of
//! [`Scene::render_slots`] that prepares every view first and records them at once.
//!
//! **The batched path.** Every view is prepared in order on the calling thread
//! ([`Scene::prepare_view`]): its writes, its passes into other targets (shadow maps, advert
//! cards) and its [`ViewPass`]. The first view's writes go to the queue as they always did; a
//! later view's are kept in a [`Log`]. The views' scene passes are then recorded, each into its
//! own encoder, by [`Recorder::record`], which reads only its view's `ViewPass` and GPU objects
//! no view changes. Last, in view order on the calling thread: the view's post passes are
//! encoded (the borrowed boost zoom, the GPU timer's resolve), their writes behind the view's.
//! `oag-game`'s `split_recording_ground_truth` compares the picture with the interleaved one.
//!
//! **One submission** ([`Folded`]). A later view's log and each post chain's become copies in a
//! command buffer just ahead of the commands that read them ([`Log::encode`]), which is the
//! order a submission each gave. Folding alone saves nothing (on the Deck HD reads +0.3 ms,
//! Pulse and Pure nothing). What it buys is that nothing waits for a view's commands before the
//! frame's own submit: a view's encoder goes to a finishing thread as soon as it is recorded,
//! its shadow maps and advert cards earlier still, and the post chains, the glow and the HUD run
//! while they finish (`docs/rendering/split-screen.md`, "One submission"). A window's split may
//! not wait for the frame's submit: its window submits on its own.
//! [`Recording::Interleaved`], so a phone and the web, and [`Recording::Overlapped`] keep a
//! submission per view.
//!
//! **The default is [`Recording::Interleaved`]; equal cores take [`Recording::Threaded`].**
//! [`Recording::default_for_target`] picks threads only on positive evidence of equal cores:
//! native Linux, at least two cores, every `cpu_capacity` readable and the same (the Steam Deck
//! and a desktop Zen report 1024 on each), while `THREADS_ON_EQUAL_CORES` is set. Interleaved
//! stays everywhere else: capacities that differ (a hybrid Intel part, a phone's big.LITTLE),
//! capacities that cannot be read, one core, Android, Windows, macOS and the web. On the Deck
//! threads lowered the frame thread by 0.4 to 5.5 ms a frame at two to four views in HD and
//! Pure, except Pure's three views, where they changed nothing
//! (`docs/rendering/split-screen.md`, "Threaded recording on equal cores").
//! On an Intel Core Ultra 7 265H notebook (a hybrid part, `powersave` governor, pinned to the
//! P-cores) they cost 10 to 100% more CPU a frame: Pulse two views 1,527 against 2,025 us, HD
//! two views 2,093 against 2,521, HD four 3,633 against 4,033, Pure two views 593 against 1,192
//! (`docs/tools/render-bench.md`, "Threaded views"). The loss there is the second core, not
//! contention: one view at a time on a second thread is as slow as both at once, threads pinned
//! to one core match serial, and an empty scoped spawn and join between frames costs about
//! 200 us. Each view's recording is a burst of 0.2 to 0.4 ms once a frame, which a sleeping
//! core does not wake and clock up for. `OAG_SPLIT_RECORDING` or [`Scene::set_split_recording`]
//! picks either on any machine. One view is drawn as one view whatever the choice.
//!
//! **[`Recording::Overlapped`]** records view `k` on a worker while view `k + 1` is
//! prepared. It lost more than threads did on the same notebook (Pulse two views 1,272 against
//! 2,123 us, HD two views 1,987 against 4,177): pinned to one core it is within 7 to 11 % of
//! interleaved, so the loss is a worker woken once per view (`docs/tools/render-bench.md`,
//! "Overlapped recording"). Selectable, not the default.
//!
//! **On the web the views are always recorded on the page's thread**: wgpu's browser objects
//! belong to it, and the page may not block on a worker, so `threaded` records serially there.

use oag_gpu::split_finish::Closed;
use oag_gpu::writes::Log;

use super::record::{Recorder, ViewPass};
use super::viewports::Layer;
use super::*;

/// How [`Scene::render_slots`] records a split's views.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recording {
    /// Each view prepared, recorded and submitted before the next: the order the views were
    /// always drawn in, and the web's.
    Interleaved,
    /// Every view prepared, then recorded in order on the calling thread.
    Serial,
    /// Every view prepared, then recorded last view first on the calling thread: a recording
    /// that read another view's state would draw that view's picture, which a comparison with
    /// [`Recording::Interleaved`] shows.
    Reversed,
    /// Every view prepared, then recorded at once, a thread each (the first on the calling
    /// thread). Faster on the Steam Deck, slower on a hybrid notebook: see the module doc.
    Threaded,
    /// View `k` recorded on a worker while view `k + 1` is prepared on the calling thread, and
    /// each view submitted as soon as it is recorded; the last view is recorded on the calling
    /// thread. One worker a frame, started before the first view is prepared so its wake-up
    /// overlaps that prepare (`render_slots_overlapped`).
    Overlapped,
}

impl Recording {
    /// What a split records with when nothing chose: [`Recording::Threaded`] on a native Linux
    /// machine whose cores all report the same capacity, while [`THREADS_ON_EQUAL_CORES`]
    /// is set, and [`Recording::Interleaved`] everywhere else (see the module doc).
    /// `OAG_SPLIT_RECORDING` (`interleaved`, `serial`, `reversed`, `threaded`, `overlapped`)
    /// overrides it, for a comparison or a bench.
    pub fn default_for_target() -> Self {
        static CHOSEN: std::sync::OnceLock<Recording> = std::sync::OnceLock::new();
        *CHOSEN.get_or_init(|| {
            let over = std::env::var("OAG_SPLIT_RECORDING").ok();
            let capacities = native_core_capacities();
            let chosen = Self::select(over.as_deref(), &capacities, THREADS_ON_EQUAL_CORES);
            log::info!(
                "split recording: {chosen:?} ({}, core capacities {capacities:?})",
                if over.as_deref().and_then(Self::named).is_some() {
                    "OAG_SPLIT_RECORDING"
                } else {
                    "probe"
                }
            );
            chosen
        })
    }

    /// The recording `over` names, else [`Recording::Threaded`] when `threads_on_equal_cores`
    /// and `capacities` shows at least two cores and every one the same, else
    /// [`Recording::Interleaved`]. An empty list is a machine that does not say, and stays
    /// interleaved.
    fn select(over: Option<&str>, capacities: &[u32], threads_on_equal_cores: bool) -> Self {
        if let Some(named) = over.and_then(Self::named) {
            return named;
        }
        let uniform = capacities.len() > 1 && capacities.windows(2).all(|pair| pair[0] == pair[1]);
        if threads_on_equal_cores && uniform {
            Self::Threaded
        } else {
            Self::Interleaved
        }
    }

    fn named(name: &str) -> Option<Self> {
        match name {
            "interleaved" => Some(Self::Interleaved),
            "serial" => Some(Self::Serial),
            "reversed" => Some(Self::Reversed),
            "threaded" => Some(Self::Threaded),
            "overlapped" => Some(Self::Overlapped),
            _ => None,
        }
    }
}

/// Whether a machine with equal cores records a split with threads by default. On: the Steam
/// Deck's frame thread gains everywhere but Pure's three views at 1080p, whose p99 pays 0.7 ms
/// inside its frame budget (`docs/rendering/split-screen.md`, "Threaded recording on equal
/// cores").
const THREADS_ON_EQUAL_CORES: bool = true;

/// This machine's core capacities where threads may record a split by default: desktop Linux,
/// the Steam Deck. Android (phones are big.LITTLE), the web and every other OS read as empty.
fn native_core_capacities() -> Vec<u32> {
    #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
    {
        oag_gpu::split_finish::core_capacities()
    }
    #[cfg(not(all(target_os = "linux", not(target_arch = "wasm32"))))]
    {
        Vec::new()
    }
}

/// A split's commands held back for one submission, in the order they run, and the buffer the
/// later stages' writes go up through. Each stage's writes reach the GPU as copies just ahead of
/// its commands ([`Log::encode`]), which is the order one submission per stage gave them, and
/// the frame thread pays for one submission instead of one per view and per post chain.
pub(super) struct Folded {
    closed: Vec<Closed>,
    upload: oag_gpu::writes::UploadBuffer,
}

impl Folded {
    /// Empty, with the scene's upload buffer.
    pub(super) fn begin(scene: &Scene) -> Self {
        let mut upload = std::mem::take(&mut *scene.split_upload.borrow_mut());
        upload.rewind();
        Self {
            closed: Vec::new(),
            upload,
        }
    }

    /// `log`'s writes as copies, next in line; it comes back empty.
    fn copies(&mut self, gpu: (&wgpu::Device, &wgpu::Queue), log: &mut Log) {
        if let Some(encoder) = log.encode(gpu.0, gpu.1, &mut self.upload) {
            self.closed.push(oag_gpu::split_finish::close(encoder));
        }
    }

    /// Submits it all, with the frame's own submission when `with_frame`, and gives the upload
    /// buffer back.
    pub(super) fn submit(self, scene: &Scene, queue: &wgpu::Queue, with_frame: bool) {
        if with_frame {
            oag_gpu::split_finish::defer(queue, self.closed);
        } else if !self.closed.is_empty() {
            queue.submit(self.closed.into_iter().map(Closed::buffer));
        }
        oag_gpu::perfprobe::mark("submit");
        *scene.split_upload.borrow_mut() = self.upload;
    }
}

/// Whether a split recorded with `recording` holds its commands for one submission: every
/// recording that makes all its views before it submits any. `OAG_SPLIT_SUBMIT=views` keeps a
/// submission per view and per post chain (an A/B switch for benches, nothing a player sets).
pub(super) fn folds(recording: Recording) -> bool {
    static PER_VIEW: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    recording != Recording::Overlapped
        && !*PER_VIEW
            .get_or_init(|| std::env::var("OAG_SPLIT_SUBMIT").is_ok_and(|value| value == "views"))
}

/// One view between its prepare and its recording: the encoder its prepare recorded the
/// view's other passes into, and its scene pass.
type Prepared<'a> = (wgpu::CommandEncoder, ViewPass<'a>);

/// One view recorded: its commands and what its scene pass drew.
type Recorded = (Closed, SceneStats);

impl Scene {
    /// Which [`Recording`] a split records with.
    pub fn split_recording(&self) -> Recording {
        self.split_recording.get()
    }

    /// Records a split's views with `recording` from now on.
    pub fn set_split_recording(&self, recording: Recording) {
        self.split_recording.set(recording);
    }

    /// `slots[i]`'s view drawn into `rects[i]`, every view prepared before any is recorded: see
    /// the module doc. What [`Scene::render_slots`] does for every [`Recording`] but
    /// [`Recording::Interleaved`]; the caller has set the frame's work and histories up.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_slots_batched(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        race: &Race,
        slots: &[usize],
        rects: &[(f32, f32, f32, f32)],
        settings: &frame::ViewSettings<'_>,
        view_timers: &mut [Option<oag_gpu::timing::PassTimer>],
        recording: Recording,
        folded: &mut Option<Folded>,
    ) -> SceneStats {
        let stamps = timestamp_clones(view_timers, slots.len());
        let gpu = Gpu {
            device,
            queue,
            view,
        };
        let mut views = Views::begin(self, folded.is_some());
        // The draw dump appends a line a draw to one file, which two threads would interleave.
        #[cfg(not(target_arch = "wasm32"))]
        if recording == Recording::Overlapped
            && slots.len() > 1
            && std::env::var_os("OAG_DRAW_DUMP").is_none()
        {
            let stats = self.render_slots_overlapped(
                gpu,
                &mut views,
                race,
                slots,
                rects,
                settings,
                &stamps,
                view_timers,
            );
            views.end(self);
            return stats;
        }
        let mut prepared: Vec<Prepared<'_>> = Vec::with_capacity(slots.len());
        let mut logs: Vec<Option<Log>> = Vec::with_capacity(slots.len());
        for (index, (&slot, &rect)) in slots.iter().zip(rects).enumerate() {
            let (view_prepared, log) = views.prepare(
                self,
                gpu,
                race,
                (index, slot, rect),
                settings,
                &stamps[index],
            );
            prepared.push(view_prepared);
            logs.push(log);
        }
        oag_gpu::perfprobe::mark("views-prepared");
        // Under one submission nothing waits for a view's commands until that submission, so
        // their `finish`, by far the larger part of a view, overlaps what follows the views.
        let hand_off = folded.is_some() && oag_gpu::split_finish::frame_open();
        let recorded = record_views(&self.recorder(), prepared, recording, hand_off);
        let mut stats = SceneStats::default();
        for (index, ((recorded, log), (&slot, &rect))) in recorded
            .into_iter()
            .zip(logs)
            .zip(slots.iter().zip(rects))
            .enumerate()
        {
            stats.add(self.submit_view(
                gpu,
                &mut views,
                race,
                (index, slot, rect),
                recorded,
                log,
                view_timers,
                folded.as_mut(),
            ));
        }
        views.end(self);
        stats
    }

    /// [`Recording::Overlapped`]: a worker records view `k` while this thread prepares view
    /// `k + 1`, and this thread submits view `k` (its log replayed, its post passes encoded) as
    /// soon as the worker hands it back, so the GPU starts on the first view early, as
    /// [`Recording::Interleaved`] has it. The last view is recorded here: nothing is left to
    /// prepare beside it. What lands before each submission is what lands in the batched path.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::too_many_arguments)]
    fn render_slots_overlapped<'a>(
        &'a self,
        gpu: Gpu<'a>,
        views: &mut Views,
        race: &'a Race,
        slots: &[usize],
        rects: &[(f32, f32, f32, f32)],
        settings: &frame::ViewSettings<'_>,
        stamps: &'a [Stamp],
        view_timers: &mut [Option<oag_gpu::timing::PassTimer>],
    ) -> SceneStats {
        let recorder = self.recorder();
        let last = slots.len() - 1;
        let mut stats = SceneStats::default();
        std::thread::scope(|scope| {
            let (jobs, to_record) = std::sync::mpsc::channel::<Prepared<'a>>();
            let (recorded_tx, recorded) = std::sync::mpsc::channel::<Recorded>();
            let recorder = &recorder;
            // Spawned before the first prepare, so the worker is awake by the time it is done.
            let worker = scope.spawn(move || {
                for prepared in to_record {
                    if recorded_tx
                        .send(record_one(recorder, prepared, false))
                        .is_err()
                    {
                        break;
                    }
                }
            });
            // The view on the worker, and its log, owed a submission.
            let mut owed: Option<Option<Log>> = None;
            for (index, (&slot, &rect)) in slots.iter().zip(rects).enumerate() {
                let (prepared, log) = views.prepare(
                    self,
                    gpu,
                    race,
                    (index, slot, rect),
                    settings,
                    &stamps[index],
                );
                let mine = if index < last {
                    jobs.send(prepared)
                        .expect("the recording worker is running");
                    None
                } else {
                    Some(prepared)
                };
                if let Some(previous_log) = owed.take() {
                    let previous = recorded.recv().expect("the recording worker is running");
                    stats.add(self.submit_view(
                        gpu,
                        views,
                        race,
                        (index - 1, slots[index - 1], rects[index - 1]),
                        previous,
                        previous_log,
                        view_timers,
                        None,
                    ));
                }
                match mine {
                    None => owed = Some(log),
                    Some(prepared) => {
                        let commands = record_one(recorder, prepared, false);
                        stats.add(self.submit_view(
                            gpu,
                            views,
                            race,
                            (index, slot, rect),
                            commands,
                            log,
                            view_timers,
                            None,
                        ));
                    }
                }
            }
            drop(jobs);
            worker
                .join()
                .unwrap_or_else(|e| std::panic::resume_unwind(e));
        });
        stats
    }

    /// View `index`'s post passes (the borrowed boost zoom, its GPU timer's resolve) encoded,
    /// their writes behind the rest of the view's, and all of it handed to `folded`, or the
    /// log replayed and submitted when there is none. In view order on the calling thread: the
    /// post passes read the scene's cells.
    #[allow(clippy::too_many_arguments)]
    fn submit_view(
        &self,
        gpu: Gpu<'_>,
        views: &mut Views,
        race: &Race,
        (index, slot, rect): (usize, usize, (f32, f32, f32, f32)),
        (commands, drawn): Recorded,
        log: Option<Log>,
        view_timers: &mut [Option<oag_gpu::timing::PassTimer>],
        folded: Option<&mut Folded>,
    ) -> SceneStats {
        // What follows the view's scene pass, recorded here because it reads the scene's
        // cells: the post chains of a split view (only the borrowed boost zoom draws; the
        // chains and the blur run after the last view) and the view's timer.
        self.enter_view(race, index, slot);
        let mut post = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("race viewport post"),
            });
        let draws = self.view_post_draws(race);
        let log = log.map(std::cell::RefCell::new);
        let writes: &dyn oag_gpu::writes::Writes = match &log {
            Some(log) => log,
            None => gpu.queue,
        };
        let (hd_bloom_encoded, blur_encoded) = self.finish_view(
            gpu.device,
            writes,
            &mut post,
            gpu.view,
            race,
            rect,
            (oag_display::display::MotionBlur::Off, Mat4::IDENTITY),
            None,
            None,
        );
        let timed = match view_timers.get_mut(index).and_then(Option::as_mut) {
            Some(timer) => {
                timer.resolve(&mut post);
                true
            }
            None => false,
        };
        let mut log = log.map(std::cell::RefCell::into_inner);
        match folded {
            Some(folded) => {
                if let Some(log) = &mut log {
                    folded.copies((gpu.device, gpu.queue), log);
                }
                folded.closed.extend(views.frame_passes.take());
                folded.closed.push(commands);
                // Most frames a view has no post pass, and an encoder is a command pool the
                // driver resets when the GPU is done.
                if timed || draws {
                    folded.closed.push(oag_gpu::split_finish::close(post));
                }
            }
            None => {
                if let Some(log) = &mut log {
                    log.replay(gpu.queue);
                }
                gpu.queue.submit([commands.buffer(), post.finish()]);
                oag_gpu::perfprobe::mark("submit");
            }
        }
        views.spare.extend(log);
        SceneStats {
            hd_bloom_encoded,
            blur_encoded,
            ..drawn
        }
    }

    /// Whether [`Scene::finish_view`] draws anything for a view of a split: the flare the view
    /// placed, or the borrowed boost zoom. Asked before it runs, since it takes the flare.
    fn view_post_draws(&self, race: &Race) -> bool {
        self.lens_flare_placed()
            || (race.borrowed_zoom_tuning().is_some() && race.hd_zoom_frame().is_some())
    }

    /// A stage of a split's post work that needs its own writes ahead of its own commands:
    /// `encode` records it with its writes going to `folded`'s next copies, or, with no
    /// `folded`, to the queue and submitted alone. It answers whether it recorded anything.
    pub(super) fn post_stage(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        folded: Option<&mut Folded>,
        label: &str,
        encode: impl FnOnce(&dyn oag_gpu::writes::Writes, &mut wgpu::CommandEncoder) -> bool,
    ) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
        let Some(folded) = folded else {
            if encode(queue, &mut encoder) {
                queue.submit(Some(encoder.finish()));
            }
            return;
        };
        let spare = self.split_logs.borrow_mut().pop().unwrap_or_default();
        let log = std::cell::RefCell::new(spare);
        let recorded = encode(&log, &mut encoder);
        let mut log = log.into_inner();
        folded.copies((device, queue), &mut log);
        if recorded {
            folded.closed.push(oag_gpu::split_finish::close(encoder));
        }
        self.split_logs.borrow_mut().push(log);
    }

    /// Points the scene's per-view cells at view `index` of a split, following `slot`.
    pub(super) fn enter_view(&self, race: &Race, index: usize, slot: usize) {
        race.set_viewpoint(slot);
        // This seat's own camera history: the previous view-projection its velocity is
        // measured against and the shake the blur takes out, never another view's.
        self.history.set(motion::split_history(slot));
        self.layer.set(if index == 0 {
            Layer::First
        } else {
            Layer::Next
        });
    }
}

/// The device, the queue and the target every view of a frame draws with.
#[derive(Clone, Copy)]
struct Gpu<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    view: &'a wgpu::TextureView,
}

/// A view's scene-pass timestamps, a clone of its timer's query set and the two indices: held
/// apart from the timer, so a later view's pass can carry its timestamps while an earlier
/// view's timer is resolved.
type Stamp = Option<(wgpu::QuerySet, Option<u32>, Option<u32>)>;

fn timestamp_clones(timers: &[Option<oag_gpu::timing::PassTimer>], views: usize) -> Vec<Stamp> {
    (0..views)
        .map(|index| {
            timers
                .get(index)
                .and_then(Option::as_ref)
                .and_then(oag_gpu::timing::PassTimer::writes)
                .map(|writes| {
                    (
                        writes.query_set.clone(),
                        writes.beginning_of_pass_write_index,
                        writes.end_of_pass_write_index,
                    )
                })
        })
        .collect()
}

/// The upload staging and the later views' logs, out of the scene for one frame's views.
struct Views {
    staging: Staging,
    spare: Vec<Log>,
    folding: bool,
    /// The first view's passes into other targets, closed ahead of its scene pass.
    frame_passes: Option<Closed>,
}

impl Views {
    fn begin(scene: &Scene, folding: bool) -> Self {
        Self {
            staging: std::mem::take(&mut *scene.upload_staging.borrow_mut()),
            spare: std::mem::take(&mut *scene.split_logs.borrow_mut()),
            folding,
            frame_passes: None,
        }
    }

    /// View `index` prepared: its writes (the first view's to the queue, a later view's into a
    /// log of its own, returned) and its scene pass.
    fn prepare<'a>(
        &mut self,
        scene: &'a Scene,
        gpu: Gpu<'a>,
        race: &'a Race,
        (index, slot, rect): (usize, usize, (f32, f32, f32, f32)),
        settings: &frame::ViewSettings<'_>,
        stamp: &'a Stamp,
    ) -> (Prepared<'a>, Option<Log>) {
        scene.enter_view(race, index, slot);
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("race viewport"),
            });
        let uploads = Uploads::begin(
            gpu.queue,
            &scene.uniform_blocks,
            SceneBlocks::view(race.viewed_slot()),
            std::mem::take(&mut self.staging),
        );
        // The first view's writes land before the first submission, which is its own: only
        // a later view's have to wait.
        let uploads = if index == 0 {
            uploads
        } else {
            uploads.logged(self.spare.pop().unwrap_or_default())
        };
        let timestamps =
            stamp
                .as_ref()
                .map(|(query_set, begin, end)| wgpu::RenderPassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: *begin,
                    end_of_pass_write_index: *end,
                });
        let pass = scene.prepare_view(
            &uploads,
            &mut encoder,
            gpu.view,
            race,
            rect,
            settings,
            timestamps,
        );
        let (back, log) = uploads.into_parts();
        self.staging = back;
        // The first view's shadow maps and advert cards, closed now: they finish while the
        // views after it are prepared, and do not wait behind its scene pass.
        if self.folding && index == 0 && oag_gpu::split_finish::frame_open() {
            let next = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("race viewport"),
                });
            let passes = std::mem::replace(&mut encoder, next);
            self.frame_passes = Some(oag_gpu::split_finish::close(passes));
        }
        ((encoder, pass), log)
    }

    fn end(self, scene: &Scene) {
        *scene.upload_staging.borrow_mut() = self.staging;
        *scene.split_logs.borrow_mut() = self.spare;
    }
}

/// One view's scene pass recorded and its encoder finished, here or, with `hand_off`, on a
/// finishing thread.
fn record_one(
    recorder: &Recorder<'_>,
    (mut encoder, pass): Prepared<'_>,
    hand_off: bool,
) -> Recorded {
    let drawn = recorder.record(&pass, &mut encoder);
    drop(pass);
    oag_gpu::perfprobe::mark("scene-pass");
    let commands = if hand_off {
        oag_gpu::split_finish::finish_elsewhere(encoder)
    } else {
        Closed::Finished(encoder.finish())
    };
    oag_gpu::perfprobe::mark("encoder-finish");
    (commands, drawn)
}

/// Records every prepared view's scene pass and closes its encoder, in view order whatever
/// order they were recorded in.
fn record_views(
    recorder: &Recorder<'_>,
    prepared: Vec<Prepared<'_>>,
    recording: Recording,
    hand_off: bool,
) -> Vec<Recorded> {
    let one = |prepared: Prepared<'_>| record_one(recorder, prepared, hand_off);
    // The draw dump appends a line a draw to one file, which several threads would interleave.
    let dumping = std::env::var_os("OAG_DRAW_DUMP").is_some();
    match recording {
        Recording::Threaded if !dumping && prepared.len() > 1 => threaded(prepared, one),
        Recording::Reversed => {
            let mut recorded: Vec<Recorded> = prepared.into_iter().rev().map(one).collect();
            recorded.reverse();
            recorded
        }
        _ => prepared.into_iter().map(one).collect(),
    }
}

/// Each view after the first on a scoped thread of its own, the first on this one.
#[cfg(not(target_arch = "wasm32"))]
fn threaded<'a>(
    prepared: Vec<Prepared<'a>>,
    one: impl Fn(Prepared<'a>) -> Recorded + Copy + Send,
) -> Vec<Recorded> {
    std::thread::scope(|scope| {
        let mut views = prepared.into_iter();
        let first = views.next();
        let rest: Vec<_> = views.map(|view| scope.spawn(move || one(view))).collect();
        let mut recorded: Vec<Recorded> = first.into_iter().map(one).collect();
        recorded.extend(rest.into_iter().map(|handle| {
            handle
                .join()
                .unwrap_or_else(|e| std::panic::resume_unwind(e))
        }));
        recorded
    })
}

/// The web records on the page's thread: see the module doc.
#[cfg(target_arch = "wasm32")]
fn threaded<'a>(
    prepared: Vec<Prepared<'a>>,
    one: impl Fn(Prepared<'a>) -> Recorded,
) -> Vec<Recorded> {
    prepared.into_iter().map(one).collect()
}

#[cfg(test)]
mod tests {
    use super::{Recording, THREADS_ON_EQUAL_CORES};

    const DECK: [u32; 8] = [1024; 8];
    /// An Intel Core Ultra 7 265H, a hybrid part, as the probe reads it (core order).
    const CORE_ULTRA_265H: [u32; 16] = [
        1024, 1024, 1024, 1024, 1024, 1024, 758, 758, 758, 758, 758, 758, 758, 758, 301, 301,
    ];
    const GALAXY_S24: [u32; 10] = [253, 253, 253, 253, 653, 653, 653, 730, 730, 1024];

    /// Only a readable list of two or more equal cores records with threads, and only once
    /// the rule is on.
    #[test]
    fn only_equal_cores_that_say_so_record_with_threads() {
        assert_eq!(Recording::select(None, &DECK, true), Recording::Threaded);
        for capacities in [&CORE_ULTRA_265H[..], &GALAXY_S24, &[], &[1024]] {
            assert_eq!(
                Recording::select(None, capacities, true),
                Recording::Interleaved,
                "{capacities:?}"
            );
        }
        assert_eq!(
            Recording::select(None, &DECK, false),
            Recording::Interleaved
        );
    }

    /// Whatever the rule, a machine with differing or unreadable capacities keeps interleaved.
    #[test]
    fn this_notebook_and_a_phone_keep_interleaved_by_default() {
        for capacities in [&CORE_ULTRA_265H[..], &GALAXY_S24, &[]] {
            assert_eq!(
                Recording::select(None, capacities, THREADS_ON_EQUAL_CORES),
                Recording::Interleaved
            );
        }
    }

    /// `OAG_SPLIT_RECORDING` wins over the probe both ways; a name it does not know is ignored.
    #[test]
    fn the_variable_overrides_the_probe_both_ways() {
        assert_eq!(
            Recording::select(Some("interleaved"), &DECK, true),
            Recording::Interleaved
        );
        for capacities in [&CORE_ULTRA_265H[..], &GALAXY_S24, &[]] {
            for rule in [false, true] {
                assert_eq!(
                    Recording::select(Some("threaded"), capacities, rule),
                    Recording::Threaded
                );
            }
        }
        assert_eq!(
            Recording::select(Some("overlapped"), &DECK, true),
            Recording::Overlapped
        );
        assert_eq!(
            Recording::select(Some("fast"), &DECK, true),
            Recording::Threaded
        );
        assert_eq!(
            Recording::select(Some("fast"), &CORE_ULTRA_265H, true),
            Recording::Interleaved
        );
    }
}
