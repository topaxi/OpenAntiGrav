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
}

impl Recording {
    /// What a split records with when nothing chose: one view after another (see the module doc
    /// for why not threads).
    /// `OAG_SPLIT_RECORDING` (`interleaved`, `serial`, `reversed`, `threaded`) overrides it, for
    /// a comparison or a bench.
    pub fn default_for_target() -> Self {
        static CHOSEN: std::sync::OnceLock<Recording> = std::sync::OnceLock::new();
        *CHOSEN.get_or_init(|| match std::env::var("OAG_SPLIT_RECORDING").as_deref() {
            Ok("interleaved") => Self::Interleaved,
            Ok("serial") => Self::Serial,
            Ok("reversed") => Self::Reversed,
            Ok("threaded") => Self::Threaded,
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
        let mut staging = std::mem::take(&mut *self.upload_staging.borrow_mut());
        let mut spare = std::mem::take(&mut *self.split_logs.borrow_mut());
        let mut prepared: Vec<Prepared<'_>> = Vec::with_capacity(slots.len());
        let mut logs: Vec<Option<Log>> = Vec::with_capacity(slots.len());
        for (index, (&slot, &rect)) in slots.iter().zip(rects).enumerate() {
            self.enter_view(race, index, slot);
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("race viewport"),
            });
            let uploads = Uploads::begin(
                queue,
                &self.uniform_blocks,
                SceneBlocks::view(race.viewed_slot()),
                staging,
            );
            // The first view's writes land before the first submission, which is its own: only
            // a later view's have to wait.
            let uploads = if index == 0 {
                uploads
            } else {
                uploads.logged(spare.pop().unwrap_or_default())
            };
            let timestamps = view_timers
                .get(index)
                .and_then(Option::as_ref)
                .and_then(oag_gpu::timing::PassTimer::writes);
            let pass = self.prepare_view(
                &uploads,
                &mut encoder,
                view,
                race,
                rect,
                settings,
                timestamps,
            );
            let (back, log) = uploads.into_parts();
            staging = back;
            prepared.push((encoder, pass));
            logs.push(log);
        }
        *self.upload_staging.borrow_mut() = staging;
        oag_gpu::perfprobe::mark("views-prepared");
        let borrowed = self.borrow_for_recording();
        let recorded = record_views(&borrowed.recorder(), prepared, recording);
        drop(borrowed);
        let mut stats = SceneStats::default();
        for (index, (((commands, drawn), log), (&slot, &rect))) in recorded
            .into_iter()
            .zip(logs)
            .zip(slots.iter().zip(rects))
            .enumerate()
        {
            if let Some(mut log) = log {
                log.replay(queue);
                spare.push(log);
            }
            // What follows the view's scene pass, recorded here because it reads the scene's
            // cells: the post chains of a split view (only the borrowed boost zoom draws; the
            // chains and the blur run after the last view) and the view's timer.
            self.enter_view(race, index, slot);
            let mut post = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("race viewport post"),
            });
            let (hd_bloom_encoded, blur_encoded) = self.finish_view(
                device,
                queue,
                &mut post,
                view,
                race,
                rect,
                (oag_display::display::MotionBlur::Off, Mat4::IDENTITY),
                None,
                None,
            );
            if let Some(timer) = view_timers.get_mut(index).and_then(Option::as_mut) {
                timer.resolve(&mut post);
            }
            stats.add(SceneStats {
                hd_bloom_encoded,
                blur_encoded,
                ..drawn
            });
            queue.submit([commands, post.finish()]);
            oag_gpu::perfprobe::mark("submit");
        }
        *self.split_logs.borrow_mut() = spare;
        stats
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

/// Records every prepared view's scene pass and finishes its encoder, in view order whatever
/// order they were recorded in.
fn record_views(
    recorder: &Recorder<'_>,
    prepared: Vec<Prepared<'_>>,
    recording: Recording,
) -> Vec<Recorded> {
    let one = |(mut encoder, pass): Prepared<'_>| -> Recorded {
        let drawn = recorder.record(&pass, &mut encoder);
        drop(pass);
        oag_gpu::perfprobe::mark("scene-pass");
        let commands = encoder.finish();
        oag_gpu::perfprobe::mark("encoder-finish");
        (commands, drawn)
    };
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
