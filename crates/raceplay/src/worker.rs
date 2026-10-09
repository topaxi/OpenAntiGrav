//! [`LoadWorker`]: reading a circuit off the disc on a thread of its own, so
//! the window keeps drawing while it happens.
//!
//! # Why this exists at all
//!
//! `Session::launch_race` used to be `self.stalled = true; race::load(options)?`
//! on the frame thread. Measured across every circuit on all four discs in hand,
//! that call is **1.5 to 5 seconds** - the PS3's circuits are the slow end - and
//! for all of it the event loop is not pumping. A window that stops answering
//! the compositor for four seconds is not a window that looks busy; on some it
//! is greyed out and offered for killing.
//!
//! So the load moves to a thread and the loading screen goes up over it, which
//! is the same shape `crate::boot::MediaWorker` already has for the boot's own
//! movies - deliberately, because the two waits are the same problem and a
//! second mechanism for it would be a second thing to keep in step with the
//! frame loop.
//!
//! # What it does not do
//!
//! **No progress counting** (stages are reported, see [`super::stages`]). `crate::loading::Phase::Race`'s own
//! documentation carries the reasoning: the other two waits this screen covers
//! are a transcode and a prefetch, neither of which any release has, and the
//! figures are what make them bearable. A race load is the wait the original
//! *also* has, and what the original puts up for it is a still and a word or a
//! wave and a tip - so a bar here would be this build decorating a screen the
//! disc authors. The circuit's name goes up, and nothing else this build made
//! up.
//!
//! **No cancellation.** `crate::prefetch`'s worker carries a stop flag because
//! it runs for ten minutes and a player may quit inside one; this runs for
//! seconds and its result is the thing being waited for. Dropping the handle
//! detaches the thread, which finishes its read and drops a `Loaded` nobody
//! wanted.

use std::sync::{Arc, Mutex};

use super::stages::LoadStages;
use super::{Loaded, Options, TextureSink};

/// A circuit being read, on a thread.
///
/// Holds no GPU and hands back none: [`super::load`] produces plain data -
/// vertex and index buffers, handling blocks, a spline - and the device only
/// enters at `Stage::race`, which runs on the frame thread with the result.
/// The one exception is an optional [`TextureSink`], which only *receives*
/// decoded textures.
/// That is what makes the split cheap rather than a rewrite; see
/// [`super::Setup`]'s own "no GPU anywhere in sight".
#[derive(Debug)]
pub struct LoadWorker {
    /// `None` once joined, which is what makes [`Self::join`] idempotent.
    #[cfg(not(target_arch = "wasm32"))]
    handle: Option<std::thread::JoinHandle<anyhow::Result<Loaded>>>,
    /// On the web the load runs on a Web Worker (`crate::web_thread`) and
    /// leaves its result here. The page's thread may not wait on a lock, so
    /// it reads `done` first and takes the result only once the worker has
    /// let go of it. See `docs/tools/web.md`, "Threads".
    #[cfg(target_arch = "wasm32")]
    handle: Option<Arc<WebResult>>,
    /// What the screen puts on the line where the other phases put an entry
    /// name. Shared rather than owned because the thread refines it: the
    /// circuit a caller *asked* for may be `None`, and only the load knows
    /// which one the title then defaulted to.
    current: Arc<Mutex<Option<String>>>,
    /// How far the load has got, reported from its thread. See
    /// [`super::stages`].
    stages: LoadStages,
}

impl LoadWorker {
    /// Starts the load in the background.
    ///
    /// `label` is what a player reads for this circuit - the RACE page's own
    /// row, so it is the disc's localised name rather than the `PI_Track` id.
    /// `None` where the caller has no name to give, which draws no line rather
    /// than a made-up one.
    #[must_use]
    ///
    /// `sink` is the device the scene will be built from, where the caller has
    /// one: every texture is then uploaded as it is decoded instead of being
    /// held for the scene - see [`super::TextureSink`].
    pub fn spawn(options: Options, label: Option<String>, sink: Option<TextureSink>) -> Self {
        // No sink on the web: the load runs on a worker there, and wgpu's web
        // types stay on the page's thread (docs/tools/web.md, "Threads").
        #[cfg(target_arch = "wasm32")]
        return {
            drop(sink);
            Self::spawn_with(label, move || super::load(&options))
        };
        #[cfg(not(target_arch = "wasm32"))]
        Self::spawn_with(label, move || {
            let _scope = TextureSink::open_if(sink.as_ref());
            super::load(&options)
        })
    }

    /// [`Self::spawn`] for a Wipeout 2048 campaign event, resolved on the
    /// thread through [`super::load_event`] - the same overlay `--event`
    /// applies, reached from the front end's own map.
    #[must_use]
    pub fn spawn_event(
        options: Options,
        event: String,
        label: Option<String>,
        sink: Option<TextureSink>,
    ) -> Self {
        #[cfg(target_arch = "wasm32")]
        return {
            drop(sink);
            Self::spawn_with(label, move || super::load_event(&options, &event))
        };
        #[cfg(not(target_arch = "wasm32"))]
        Self::spawn_with(label, move || {
            let _scope = TextureSink::open_if(sink.as_ref());
            super::load_event(&options, &event)
        })
    }

    #[cfg(target_arch = "wasm32")]
    fn spawn_with(
        label: Option<String>,
        load: impl FnOnce() -> anyhow::Result<Loaded> + Send + 'static,
    ) -> Self {
        let current = Arc::new(Mutex::new(label));
        let stages = LoadStages::default();
        let result = Arc::new(WebResult::default());
        let spawned = crate::web_thread::spawn({
            let stages = stages.clone();
            let result = Arc::clone(&result);
            move || {
                let loaded = {
                    let _scope = stages.open();
                    load()
                };
                *crate::web_thread::lock_spinning(&result.loaded) = Some(loaded);
                result
                    .done
                    .store(true, std::sync::atomic::Ordering::Release);
            }
        });
        if let Err(why) = &spawned {
            log::error!("race load: {why}");
        }
        Self {
            handle: spawned.ok().map(|()| result),
            current,
            stages,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn spawn_with(
        label: Option<String>,
        load: impl FnOnce() -> anyhow::Result<Loaded> + Send + 'static,
    ) -> Self {
        let current = Arc::new(Mutex::new(label));
        let stages = LoadStages::default();
        let handle = std::thread::Builder::new()
            // Named for the same reason `boot-media` is: it should be obvious
            // in a debugger and in `top` which thread the window is waiting on.
            .name("race-load".to_string())
            .spawn({
                let stages = stages.clone();
                move || {
                    let _scope = stages.open();
                    load()
                }
            })
            .ok();
        Self {
            handle,
            current,
            stages,
        }
    }

    /// Whether the load has returned.
    ///
    /// `true` for a worker whose thread failed to spawn at all, so a caller
    /// polling this cannot wait forever on a thread that never started - the
    /// error surfaces from [`Self::join`] instead, where it can be reported.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        return self
            .handle
            .as_ref()
            .is_none_or(|result| result.done.load(std::sync::atomic::Ordering::Acquire));
        #[cfg(not(target_arch = "wasm32"))]
        self.handle
            .as_ref()
            .is_none_or(std::thread::JoinHandle::is_finished)
    }

    /// Takes the result, waiting if it is not in yet.
    ///
    /// **Never actually waits in the frame loop**, which only calls this once
    /// [`Self::is_finished`] is true - the same contract
    /// `crate::boot::MediaWorker::join` has. `None` on the second call, and
    /// on a worker whose thread would not spawn.
    pub fn join(&mut self) -> Option<anyhow::Result<Loaded>> {
        #[cfg(target_arch = "wasm32")]
        return self.handle.take().and_then(|result| {
            // wasm32 caps the shared memory at 4 GiB and it never shrinks, so
            // its high-water mark is the number a load has to stay under.
            log::info!(
                "web: module memory {} MiB after the race load",
                core::arch::wasm32::memory_size::<0>() / 16
            );
            // Only reached once `done`, so the worker has let go: never waits.
            crate::web_thread::lock_spinning(&result.loaded).take()
        });
        #[cfg(not(target_arch = "wasm32"))]
        let handle = self.handle.take()?;
        #[cfg(not(target_arch = "wasm32"))]
        Some(match handle.join() {
            Ok(loaded) => loaded,
            // A panic on the load thread is reported as a failed load rather
            // than resumed here, which would take the window down with it. The
            // caller's own error path puts the player back in the menus.
            Err(_) => Err(anyhow::anyhow!("the circuit load panicked")),
        })
    }

    /// What the screen should name as loading, right now.
    #[must_use]
    pub fn current(&self) -> Option<String> {
        match self.current.lock() {
            Ok(current) => current.clone(),
            // A poisoned lock means the load thread panicked while holding it.
            // The name is a caption; losing it is not worth a second panic, and
            // `join` above is where that failure is actually reported.
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// What the loading screen shows for this wait.
    ///
    /// Deliberately uncounted: there is no total, so the screen draws no figures.
    /// The load's stage rides in [`LoadProgress::stage`] for a bar that is
    /// stage-driven, which is Wipeout HD's. See this module's own documentation
    /// for why nothing is counted.
    #[must_use]
    pub fn progress(&self) -> LoadProgress {
        LoadProgress {
            current: self.current(),
            finished: self.is_finished(),
            stage: self.stages.reached(),
        }
    }
}

/// Where a load on a Web Worker leaves its result.
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Default)]
struct WebResult {
    loaded: Mutex<Option<anyhow::Result<Loaded>>>,
    /// Set once `loaded` is filled and its lock released.
    done: std::sync::atomic::AtomicBool,
}

/// A race load's progress as the host's loading screen reads it.
///
/// Plain data, so the host (`oag_game`, whose `prefetch::Progress` the screen
/// polls) builds its own snapshot from it and this crate knows no loading screen.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadProgress {
    /// What the load is doing right now, when it has said.
    pub current: Option<String>,
    /// Nothing left to do, for any reason.
    pub finished: bool,
    /// The stage reached, `0` before the first.
    pub stage: u8,
}
