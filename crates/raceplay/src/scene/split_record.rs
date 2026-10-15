//! How a split screen records its views: [`Recording`], and the batched path of
//! [`Scene::render_slots`] that prepares every view first and records them at once.
//!
//! **The batched path.** Every view is prepared in order on the calling thread
//! ([`Scene::prepare_view`]): its writes, its passes into other targets (shadow maps, advert
//! cards) and its [`ViewPass`]. The first view's writes go to the queue as they always did; a
//! later view's are kept in a [`Log`]. The views' scene passes are then recorded, each into its
//! own encoder, by [`Recorder::record`], which reads only its view's `ViewPass` and GPU objects
//! no view changes. Last, in view order on the calling thread: the view's log is replayed, its
//! post passes encoded (the borrowed boost zoom, the GPU timer's resolve), and both submitted.
//! What lands before each submission is what landed when the views were drawn one after
//! another, so the picture is the same; `oag-game`'s `split_recording_ground_truth` compares them.
//!
//! **The default is [`Recording::Interleaved`], one view after another, on every target.**
//! Measured on an Intel Core Ultra 7 265H notebook (NVIDIA RTX PRO 2000, `powersave` governor,
//! pinned to the P-cores, minimum of three interleaved rounds, 2026-10-15), threaded recording
//! cost 10 to 100% more CPU a frame than interleaved: Pulse two views 1,527 against 2,025 us, HD
//! two views 2,093 against 2,521, HD four 3,633 against 4,033, Pure two views 593 against 1,192
//! (`docs/tools/render-bench.md`, "Threaded views"). The loss is the second core, not contention:
//! one view at a time on a second thread is as slow as both at once, threads pinned to one core
//! match serial, and an empty scoped spawn and join between frames costs about 200 us there (a
//! parked worker gains nothing on a 2 x 250 us split, a spinning one halves it). Each view's
//! recording is a burst of 0.2 to 0.4 ms once a frame, which a sleeping core does not wake and
//! clock up for. **[`Recording::Threaded`] stays selectable** (`OAG_SPLIT_RECORDING=threaded`,
//! [`Scene::set_split_recording`]) for a machine where it measures faster, the Steam Deck first.
//!
//! **[`Recording::Overlapped`]** (2026-10-16) records view `k` on a worker while view `k + 1` is
//! prepared. It lost more than threads did on the same notebook (Pulse two views 1,272 against
//! 2,123 us, HD two views 1,987 against 4,177): pinned to one core it is within 7 to 11 % of
//! interleaved, so the loss is a worker woken once per view (`docs/tools/render-bench.md`,
//! "Overlapped recording"). Selectable, not the default.
//!
//! **On the web the views are always recorded on the page's thread**: wgpu's browser objects
//! belong to it, and the page may not block on a worker, so `threaded` records serially there.

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
    /// thread). Slower on the notebook it was measured on: see the module doc.
    Threaded,
    /// View `k` recorded on a worker while view `k + 1` is prepared on the calling thread, and
    /// each view submitted as soon as it is recorded; the last view is recorded on the calling
    /// thread. One worker a frame, started before the first view is prepared so its wake-up
    /// overlaps that prepare (`render_slots_overlapped`).
    Overlapped,
}

impl Recording {
    /// What a split records with when nothing chose: one view after another (see the module doc
    /// for why not threads).
    /// `OAG_SPLIT_RECORDING` (`interleaved`, `serial`, `reversed`, `threaded`, `overlapped`)
    /// overrides it, for
    /// a comparison or a bench.
    pub fn default_for_target() -> Self {
        static CHOSEN: std::sync::OnceLock<Recording> = std::sync::OnceLock::new();
        *CHOSEN.get_or_init(|| match std::env::var("OAG_SPLIT_RECORDING").as_deref() {
            Ok("interleaved") => Self::Interleaved,
            Ok("serial") => Self::Serial,
            Ok("reversed") => Self::Reversed,
            Ok("threaded") => Self::Threaded,
            Ok("overlapped") => Self::Overlapped,
            _ => Self::Interleaved,
        })
    }
}

/// One view between its prepare and its recording: the encoder its prepare recorded the
/// view's other passes into, and its scene pass.
type Prepared<'a> = (wgpu::CommandEncoder, ViewPass<'a>);

/// One view recorded: its commands and what its scene pass drew.
type Recorded = (wgpu::CommandBuffer, SceneStats);

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
    ) -> SceneStats {
        let stamps = timestamp_clones(view_timers, slots.len());
        let gpu = Gpu {
            device,
            queue,
            view,
        };
        let mut views = Views::begin(self);
        #[cfg(not(target_arch = "wasm32"))]
        if recording == Recording::Overlapped && slots.len() > 1 {
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
        let recorded = record_views(&self.recorder(), prepared, recording);
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
                    if recorded_tx.send(record_one(recorder, prepared)).is_err() {
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
                    ));
                }
                match mine {
                    None => owed = Some(log),
                    Some(prepared) => {
                        let commands = record_one(recorder, prepared);
                        stats.add(self.submit_view(
                            gpu,
                            views,
                            race,
                            (index, slot, rect),
                            commands,
                            log,
                            view_timers,
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

    /// View `index`'s log replayed, its post passes (the borrowed boost zoom, its GPU timer's
    /// resolve) encoded, and its commands submitted after them. In view order on the calling
    /// thread: the post passes read the scene's cells.
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
    ) -> SceneStats {
        if let Some(mut log) = log {
            log.replay(gpu.queue);
            views.spare.push(log);
        }
        // What follows the view's scene pass, recorded here because it reads the scene's
        // cells: the post chains of a split view (only the borrowed boost zoom draws; the
        // chains and the blur run after the last view) and the view's timer.
        self.enter_view(race, index, slot);
        let mut post = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("race viewport post"),
            });
        let (hd_bloom_encoded, blur_encoded) = self.finish_view(
            gpu.device,
            gpu.queue,
            &mut post,
            gpu.view,
            race,
            rect,
            (oag_display::display::MotionBlur::Off, Mat4::IDENTITY),
            None,
            None,
        );
        if let Some(timer) = view_timers.get_mut(index).and_then(Option::as_mut) {
            timer.resolve(&mut post);
        }
        gpu.queue.submit([commands, post.finish()]);
        oag_gpu::perfprobe::mark("submit");
        SceneStats {
            hd_bloom_encoded,
            blur_encoded,
            ..drawn
        }
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
}

impl Views {
    fn begin(scene: &Scene) -> Self {
        Self {
            staging: std::mem::take(&mut *scene.upload_staging.borrow_mut()),
            spare: std::mem::take(&mut *scene.split_logs.borrow_mut()),
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
        ((encoder, pass), log)
    }

    fn end(self, scene: &Scene) {
        *scene.upload_staging.borrow_mut() = self.staging;
        *scene.split_logs.borrow_mut() = self.spare;
    }
}

/// One view's scene pass recorded and its encoder finished.
fn record_one(recorder: &Recorder<'_>, (mut encoder, pass): Prepared<'_>) -> Recorded {
    let drawn = recorder.record(&pass, &mut encoder);
    drop(pass);
    oag_gpu::perfprobe::mark("scene-pass");
    let commands = encoder.finish();
    oag_gpu::perfprobe::mark("encoder-finish");
    (commands, drawn)
}

/// Records every prepared view's scene pass and finishes its encoder, in view order whatever
/// order they were recorded in.
fn record_views(
    recorder: &Recorder<'_>,
    prepared: Vec<Prepared<'_>>,
    recording: Recording,
) -> Vec<Recorded> {
    let one = |prepared: Prepared<'_>| record_one(recorder, prepared);
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
