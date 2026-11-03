//! Which output the sound is going to, for `auto` SPATIAL AUDIO.
//!
//! The probe runs on a worker ([`oag_audio::RouteWatcher`]); [`Follow::poll`]
//! takes its answer without waiting. Only a real device is watched: a silent
//! run or a dump has no output to ask about and stays `Unknown`.

use oag_audio::{Route, RouteKind, RouteWatcher};

/// The last known output, and the watcher that keeps it current.
#[derive(Debug, Default)]
pub struct Follow {
    watcher: Option<RouteWatcher>,
    route: Route,
}

impl Follow {
    /// Watches the output `watcher` probes.
    pub fn watching(watcher: RouteWatcher) -> Self {
        Self {
            watcher: Some(watcher),
            route: Route::default(),
        }
    }

    /// Takes a changed route, if the worker has one.
    pub fn poll(&mut self) -> Option<&Route> {
        let landed = self.watcher.as_mut()?.poll()?;
        self.route = landed;
        Some(&self.route)
    }

    /// The kind of the last known output.
    pub fn kind(&self) -> RouteKind {
        self.route.kind
    }

    /// The stable id of the last known output, if it has one.
    pub fn device(&self) -> Option<&str> {
        self.route.device.as_deref()
    }
}
