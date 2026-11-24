//! A lost GPU device on a native build: what wgpu tells us, and the one place the game reads it.
//!
//! A driver reset, a GPU hang or an unplugged eGPU makes wgpu mark the device lost. From then on
//! every call into it fails, and wgpu reports each failure as a validation error ("Parent device
//! is lost") to the uncaptured-error handler, whose default is to panic. Two hooks turn that
//! into a flag the frame loop reads:
//!
//! - **The device-lost callback** (`Device::set_device_lost_callback`) records why the device
//!   went. It may run on any thread and from inside `submit` or `poll`, so it does nothing but
//!   set the flag; saving and rebuilding happen on the frame thread
//!   (`Session::frame`, `App::recover_device`).
//! - **The uncaptured-error handler** keeps wgpu's own rule for every error before a loss (a
//!   panic: a validation error is a bug we want to see), and swallows what a lost device keeps
//!   reporting afterwards. An error whose text names a lost device counts as the loss itself,
//!   for the case where it reaches us before the callback has run.
//!
//! The game never destroys its own device, so unlike the web build (`web_fault::device_lost`) a
//! `Destroyed` reason is a loss here too: it is what a test or the `lose-device` script step
//! causes on purpose, and the only other way to get it is something outside the game ending the
//! device. `docs/architecture/device-loss.md` has what is saved and what stays unrecoverable.

use std::sync::{Arc, Mutex, PoisonError};

/// Why the device went, as the callback gave it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Lost {
    pub(crate) reason: wgpu::DeviceLostReason,
    pub(crate) message: String,
}

/// The loss, once it has happened. Cloned into the callbacks; the frame loop keeps the original.
#[derive(Clone, Debug, Default)]
pub(crate) struct DeviceLoss {
    seen: Arc<Mutex<Seen>>,
}

/// The loss and whether the frame loop has taken it. A taken loss stays a loss: the dying
/// device's other users (a worker, the old session's drop) keep reporting into the handler.
#[derive(Debug, Default)]
struct Seen {
    lost: Option<Lost>,
    taken: bool,
}

/// What the uncaptured-error handler does with an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// The device is gone, or this error says so: record it and carry on.
    Lost,
    /// A lost device keeps reporting its own loss; nothing to add.
    Swallow,
    /// An error on a live device: wgpu's default, a panic.
    Fatal,
}

impl DeviceLoss {
    /// Hooks `device`: its loss callback and its uncaptured-error handler, both feeding the
    /// returned flag.
    pub(crate) fn install(device: &wgpu::Device) -> Self {
        let loss = Self::default();
        let on_lost = loss.clone();
        device.set_device_lost_callback(move |reason, message| on_lost.record(reason, message));
        let on_error = loss.clone();
        device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
            match on_error.judge(&error) {
                Verdict::Lost => {
                    on_error.record(wgpu::DeviceLostReason::Unknown, error.to_string())
                }
                Verdict::Swallow => log::trace!("wgpu: after the device was lost: {error}"),
                Verdict::Fatal => {
                    log::error!("Handling wgpu errors as fatal by default");
                    panic!("wgpu error: {error}\n");
                }
            }
        }));
        loss
    }

    /// Notes the loss. The first reason stands: the callback and the error that follows it are
    /// one event.
    pub(crate) fn record(&self, reason: wgpu::DeviceLostReason, message: String) {
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        if seen.lost.is_none() {
            log::debug!("the GPU device was lost ({reason:?}): {message}");
            seen.lost = Some(Lost { reason, message });
        }
    }

    /// What to do with `error` given what is known so far.
    pub(crate) fn judge(&self, error: &wgpu::Error) -> Verdict {
        if names_a_lost_device(error) {
            Verdict::Lost
        } else if self.is_lost() {
            Verdict::Swallow
        } else {
            Verdict::Fatal
        }
    }

    /// Whether a loss has been recorded, taken or not.
    pub(crate) fn is_lost(&self) -> bool {
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .lost
            .is_some()
    }

    /// The loss, once: the frame loop that takes it owns the reaction.
    pub(crate) fn take(&self) -> Option<Lost> {
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        if seen.taken || seen.lost.is_none() {
            return None;
        }
        seen.taken = true;
        seen.lost.clone()
    }
}

/// Whether wgpu's text for `error` is its "device is lost" one. wgpu has no typed error for it:
/// the lost state reaches a caller as a validation error carrying wgpu-core's
/// `DeviceError::Lost` text, "Parent device is lost".
fn names_a_lost_device(error: &wgpu::Error) -> bool {
    matches!(error, wgpu::Error::Validation { .. })
        && error
            .to_string()
            .to_ascii_lowercase()
            .contains("device is lost")
}

/// The environment variable a relaunched game carries: set, the run was started by a game that
/// lost its device, and a second loss ends the run instead of starting another.
pub(crate) const RELAUNCHED_ENV: &str = "OAG_RELAUNCHED_AFTER_DEVICE_LOSS";

/// The profile folder the note lives in.
fn profile_dir() -> Option<std::path::PathBuf> {
    crate::profile::config_dir().map(|dir| dir.join("oag"))
}

const MARKER: &str = "device-lost.txt";

/// Leaves the note a next run reads ([`take_marker`]), for the player who starts the game again
/// by hand. A failed write is logged and does not stop the rest of the wind-down.
pub(crate) fn leave_marker(lost: &Lost) {
    if let Some(dir) = profile_dir() {
        leave_marker_in(&dir, lost);
    }
}

/// Reads and removes the note [`leave_marker`] left: whether the previous run ended on a lost
/// device.
pub(crate) fn take_marker() -> bool {
    profile_dir().is_some_and(|dir| take_marker_in(&dir))
}

fn leave_marker_in(dir: &std::path::Path, lost: &Lost) {
    if let Err(e) = crate::profile::create_dir_all(dir) {
        log::warn!("could not make {}: {e}", dir.display());
        return;
    }
    let path = dir.join(MARKER);
    let text = format!("{:?}\n{}\n", lost.reason, lost.message.replace('\n', " "));
    if let Err(e) = crate::profile::write(&path, text) {
        log::warn!("could not leave {}: {e}", path.display());
    }
}

fn take_marker_in(dir: &std::path::Path) -> bool {
    let path = dir.join(MARKER);
    if crate::profile::read_to_string(&path).is_err() {
        return false;
    }
    if let Err(e) = std::fs::remove_file(&path) {
        log::warn!("could not remove {}: {e}", path.display());
    }
    true
}

/// Starts this game again with the arguments it was given, marked as a relaunch. The caller
/// leaves the event loop afterwards: the new process opens its own window on a fresh device.
///
/// # Errors
///
/// The executable cannot be found or started.
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
pub(crate) fn relaunch() -> std::io::Result<std::process::Child> {
    std::process::Command::new(std::env::current_exe()?)
        .args(std::env::args_os().skip(1))
        .env(RELAUNCHED_ENV, "1")
        .spawn()
}

#[cfg(test)]
mod tests;
