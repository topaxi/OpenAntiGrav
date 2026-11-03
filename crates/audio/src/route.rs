//! What the default output is: headphones, speakers or unknown.
//!
//! `auto` spatial audio follows this. The probe asks the platform and runs off
//! the frame loop ([`RouteWatcher`] takes the answer a frame later); the render
//! thread and the device callback never wait on it.
//!
//! cpal does not help for the default device: its PipeWire and PulseAudio hosts
//! name the graph's `output_default` node whatever is plugged in, and the
//! Android and iOS hosts name "Default Device". So the platform is asked
//! itself: on Linux PipeWire pushes every change (`live`, reduced by `graph`,
//! no process is spawned), on Windows the `WASAPI` form factor is polled.
//! Anything unreadable is [`RouteKind::Unknown`], which `auto` treats as
//! speakers.

use std::sync::Arc;

use oag_thread::Task;

#[cfg(all(target_os = "linux", not(target_os = "android")))]
mod graph;
#[cfg(all(target_os = "linux", not(target_os = "android")))]
mod live;

/// What kind of output the sound is going to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RouteKind {
    /// Headphones, a headset or a Bluetooth headphone profile.
    Headphones,
    /// Speakers, or a display or receiver the sound goes out of the room on.
    Speakers,
    /// The platform did not say.
    #[default]
    Unknown,
}

/// The default output: a stable id to remember a choice by, and its kind.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Route {
    /// Stable for one physical output and port; `None` when the platform gave
    /// no id, in which case nothing is remembered for it.
    pub device: Option<String>,
    /// Headphones, speakers or unknown.
    pub kind: RouteKind,
}

/// A port type or form factor that names headphones.
fn names_headphones(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "headphones" | "headphone" | "headset" | "hands-free" | "handsfree"
    )
}

/// A port type or form factor that names a loudspeaker or a screen.
fn names_speakers(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "speaker" | "hdmi" | "tv" | "hifi" | "car" | "computer" | "spdif" | "displayport"
    )
}

/// Asks the platform for the default output. May block, so only
/// [`RouteWatcher`]'s worker calls it.
#[must_use]
pub fn probe() -> Route {
    probe_platform()
}

#[cfg(target_os = "windows")]
fn probe_platform() -> Route {
    use cpal::traits::{DeviceTrait, HostTrait};
    let Some(device) = cpal::default_host().default_output_device() else {
        return Route::default();
    };
    let Ok(description) = device.description() else {
        return Route::default();
    };
    let kind = match description.device_type() {
        cpal::DeviceType::Headphones | cpal::DeviceType::Headset => RouteKind::Headphones,
        cpal::DeviceType::Speaker | cpal::DeviceType::Dock => RouteKind::Speakers,
        _ => RouteKind::Unknown,
    };
    Route {
        device: Some(description.name().to_string()),
        kind,
    }
}

// Linux is pushed by `live`. macOS, Android, iOS and the browser: nothing is
// read yet, so the answer is `Unknown` and `auto` is the rear cue. The
// per-platform queries are listed in the spatial audio handover thread.
#[cfg(not(target_os = "windows"))]
fn probe_platform() -> Route {
    Route::default()
}

/// Watches the default output for a change, off the frame loop.
pub struct RouteWatcher {
    probe: Arc<dyn Fn() -> Route + Send + Sync>,
    every: u32,
    countdown: u32,
    task: Option<Task<Route>>,
    last: Option<Route>,
    /// A source that pushes changes; when set, `probe` is never run.
    #[cfg(all(target_os = "linux", not(target_os = "android")))]
    live: Option<live::Live>,
}

impl std::fmt::Debug for RouteWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouteWatcher")
            .field("last", &self.last)
            .finish_non_exhaustive()
    }
}

impl RouteWatcher {
    /// Polls roughly every two seconds at 60 calls per second (Windows; Linux
    /// is pushed).
    pub const POLL_EVERY: u32 = 120;

    /// A watcher over the platform probe. In the browser nothing is probed.
    #[must_use]
    pub fn new() -> Self {
        if cfg!(target_arch = "wasm32") {
            return Self::with_probe(Arc::new(Route::default), u32::MAX);
        }
        #[cfg(all(target_os = "linux", not(target_os = "android")))]
        {
            let mut watcher = Self::with_probe(Arc::new(Route::default), u32::MAX);
            watcher.live = Some(live::Live::start());
            watcher
        }
        #[cfg(not(all(target_os = "linux", not(target_os = "android"))))]
        Self::with_probe(Arc::new(probe), Self::POLL_EVERY)
    }

    /// A watcher over `probe`, started every `every` calls of [`Self::poll`]
    /// (the first call starts one).
    #[must_use]
    pub fn with_probe(probe: Arc<dyn Fn() -> Route + Send + Sync>, every: u32) -> Self {
        Self {
            probe,
            every: every.max(1),
            countdown: 0,
            task: None,
            last: None,
            #[cfg(all(target_os = "linux", not(target_os = "android")))]
            live: None,
        }
    }

    /// Call once a frame. Returns the new route when the probe's answer differs
    /// from the last one (the first answer always counts), and never waits.
    pub fn poll(&mut self) -> Option<Route> {
        #[cfg(all(target_os = "linux", not(target_os = "android")))]
        if let Some(live) = &self.live {
            let route = live.latest()?;
            return (self.last.as_ref() != Some(&route)).then(|| {
                self.last = Some(route.clone());
                route
            });
        }
        if let Some(task) = self.task.as_mut()
            && task.is_finished()
        {
            let landed = task.join();
            self.task = None;
            if let Some(Ok(route)) = landed
                && self.last.as_ref() != Some(&route)
            {
                self.last = Some(route.clone());
                return Some(route);
            }
        }
        if self.task.is_none() {
            if self.countdown == 0 {
                self.countdown = self.every;
                let probe = Arc::clone(&self.probe);
                self.task = Some(Task::spawn("audio-route", move || probe()));
            }
            self.countdown = self.countdown.saturating_sub(1);
        }
        None
    }
}

impl Default for RouteWatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
