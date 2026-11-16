//! The media phase as a request (`MediaPlan`) and as a thread (`MediaWorker`).

use std::sync::{Arc, Mutex};

use super::progress::{MediaProgress, lock_media};
use super::{Media, Options, Shell, load_media};

/// What the media phase is being asked to read, resolved by the shell phase.
///
/// A struct rather than three arguments because the three travel together across
/// a thread boundary and are all answers to the same question - which movies this
/// title's chain names - decided where the chain is.
#[derive(Debug, Clone)]
pub struct MediaPlan {
    pub movie_name: Option<String>,
    pub second_movie_name: Option<&'static str>,
    /// See [`Shell::movie_region`].
    pub movie_region: &'static str,
    pub menu_backdrop: Option<&'static str>,
}

impl MediaPlan {
    /// How many loads [`load_media`] will attempt for this plan.
    ///
    /// Counted from the names rather than from what succeeds, because this is
    /// the denominator of a bar that goes up while the loads are still running:
    /// a total that shrank when a movie failed would make the fraction jump
    /// backwards. A load that degrades to `None` still counts as done - the wait
    /// it represents happened either way, which is the only thing the bar
    /// measures.
    ///
    /// Each named movie is **two** loads, its picture and its ATRAC3+ track, and
    /// they are counted apart because they are two out-of-process decodes with
    /// nothing but the name in common. The backdrop has no track and is one.
    #[must_use]
    pub fn loads(&self) -> usize {
        usize::from(self.movie_name.is_some()) * 2
            + usize::from(self.second_movie_name.is_some()) * 2
            + usize::from(self.menu_backdrop.is_some())
    }
}

/// [`load_media`] running on a thread of its own, so a window can open first.
///
/// **This is the whole reason the boot is in halves.** On a `native-video`
/// build the movies are 4.9 of a 5.0-second boot of the EU disc - `GstDecoder`
/// decodes every frame of both reels into memory before it returns - and all
/// of that used to happen before winit had been told to make a window. There
/// was nothing on screen to say the game had started because there was no
/// screen.
#[derive(Debug)]
pub struct MediaWorker {
    /// `None` once joined, which is what makes [`Self::join`] idempotent.
    #[cfg(not(target_arch = "wasm32"))]
    handle: Option<std::thread::JoinHandle<Media>>,
    /// The web build has no threads, so [`Self::spawn`] loads the media
    /// inline and the result waits here. See docs/tools/web.md.
    #[cfg(target_arch = "wasm32")]
    handle: Option<Media>,
    /// What the loading screen draws its bar from while the thread runs.
    progress: Arc<Mutex<MediaProgress>>,
}

impl MediaWorker {
    /// Starts the media phase in the background.
    ///
    /// Takes copies rather than borrows because the thread outlives this call
    /// by seconds; `Screens` is a dozen parsed widget trees and `Options` a
    /// handful of paths, which is nothing beside what the thread then decodes.
    #[must_use]
    pub fn spawn(archives: oag_assets::Archives, shell: &Shell, options: &Options) -> Self {
        let screens = shell.screens.clone();
        let plan = shell.media_plan();
        let options = options.clone();
        // The total is filled in on the thread, by `load_media` - but it is
        // known here, and a bar that only appeared once the thread had been
        // scheduled would flicker in on the first frame. So it starts correct.
        let progress = Arc::new(Mutex::new(MediaProgress {
            total: plan.loads(),
            ..MediaProgress::default()
        }));
        #[cfg(target_arch = "wasm32")]
        let handle = load_media(archives, &screens, &plan, &options, &progress);
        #[cfg(not(target_arch = "wasm32"))]
        let handle = std::thread::Builder::new()
            // Named so it is obvious in a debugger and in `top` which thread
            // the boot is waiting on, the same way `movie-decode` is.
            .name("boot-media".to_string())
            .spawn({
                let progress = Arc::clone(&progress);
                move || load_media(archives, &screens, &plan, &options, &progress)
            })
            .expect("spawning the boot's media thread");
        Self {
            handle: Some(handle),
            progress,
        }
    }

    /// Whether the movies have arrived, so the loading screen may start fading.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        return true;
        #[cfg(not(target_arch = "wasm32"))]
        self.handle
            .as_ref()
            .is_none_or(std::thread::JoinHandle::is_finished)
    }

    /// How far the movies have got, right now.
    #[must_use]
    pub fn progress(&self) -> MediaProgress {
        lock_media(&self.progress).clone()
    }

    /// Waits for the movies and takes them.
    ///
    /// **A worker that panicked yields a boot with no movies rather than a
    /// panic here**, because the alternative is a loading screen that never
    /// fades: the sequence copes with `None` on every one of these fields
    /// already - that is what a source with no backdrop, or a machine with no
    /// `ffmpeg`, has always produced - so the game reaches its menus and says
    /// what happened. A second call yields the same empty `Media`, which
    /// cannot happen today and would otherwise be a second panic.
    pub fn join(&mut self) -> Media {
        let Some(handle) = self.handle.take() else {
            return Self::nothing("the boot's movies were already taken");
        };
        #[cfg(target_arch = "wasm32")]
        return handle;
        #[cfg(not(target_arch = "wasm32"))]
        match handle.join() {
            Ok(media) => media,
            Err(_) => Self::nothing(
                "the thread loading the movies panicked, so the sequence plays with no picture",
            ),
        }
    }

    fn nothing(why: &str) -> Media {
        Media {
            report: vec![why.to_string()],
            ..Media::default()
        }
    }
}
