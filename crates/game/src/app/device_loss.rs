//! A lost GPU device on a native build: what wgpu tells us, and the one place the game reads it.
//!
//! A driver reset, a GPU hang or an unplugged eGPU makes wgpu mark the device lost. wgpu-core
//! 30.0.2 reports that to the **device-lost callback** (`Device::set_device_lost_callback`) and
//! drops the matching error before the uncaptured-error handler (`ErrorType::DeviceLost`, "will be
//! surfaced via callback"), so the callback is the one source of truth here: it runs on any thread
//! and from inside `submit` or `poll`, and does nothing but record the reason in a flag the frame
//! loop reads (`Session::frame`, `App::after_loss`).
//!
//! The **uncaptured-error handler** keeps wgpu's own rule for every error on a live device (a
//! panic: a validation error is a bug we want to see) and swallows what a lost device's other
//! users (a worker, the old session's drop) keep reporting once the flag is set. An error is never
//! read as the loss by its text: a validation error that happens to say "device is lost" is still
//! fatal.
//!
//! The game never destroys its own device, so unlike the web build (`web_fault::device_lost`) a
//! `Destroyed` reason is a loss here too: it is what a test or the `lose-device` script step
//! causes on purpose, and the only other way to get it is something outside the game ending the
//! device. `docs/architecture/device-loss.md` has what is saved and what stays unrecoverable.

// The web build has its own handler (`web_fault`); on wasm32 only the types are used.
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

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
        let _ = error;
        if self.is_lost() {
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

/// The environment variable a relaunched game carries: set, the run was started by a game that
/// lost its device, and a second loss ends the run instead of starting another.
pub(crate) const RELAUNCHED_ENV: &str = "OAG_RELAUNCHED_AFTER_DEVICE_LOSS";

/// The profile folder the note lives in.
fn profile_dir() -> Option<std::path::PathBuf> {
    crate::profile::config_dir().map(|dir| dir.join("oag"))
}

const MARKER: &str = "device-lost.txt";

/// Leaves the note a next run reads ([`marker_present`]), for the player who starts the game again
/// by hand. A failed write is logged and does not stop the rest of the wind-down.
pub(crate) fn leave_marker(lost: &Lost) {
    if let Some(dir) = profile_dir() {
        leave_marker_in(&dir, lost);
    }
}

/// Whether the previous run ended on a lost device and no run has shown the note since.
pub(crate) fn marker_present() -> bool {
    profile_dir().is_some_and(|dir| marker_present_in(&dir))
}

/// Removes the note: called once the line it asks for is on its way to the screen.
pub(crate) fn clear_marker() {
    if let Some(dir) = profile_dir() {
        clear_marker_in(&dir);
    }
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

fn marker_present_in(dir: &std::path::Path) -> bool {
    crate::profile::read_to_string(dir.join(MARKER)).is_ok()
}

fn clear_marker_in(dir: &std::path::Path) {
    let path = dir.join(MARKER);
    if let Err(e) = std::fs::remove_file(&path) {
        log::warn!("could not remove {}: {e}", path.display());
    }
}

/// The arguments a restarted game may be given again: what a player launches with. A disc image
/// or any positional path, `--source`, `--dlc`, `--launcher`, `--launcher-list`, the language and intro switches,
/// `--no-audio`, `--no-soundtrack`, and `--script` (a timed button script that only presses buttons). Anything else
/// can do something a second time that must not happen twice: reopen a race (`--race`,
/// `--connect`, `--online`), overwrite a file (`--dump-audio`, `--tap-audio`, `--screenshot`), or
/// run a measurement. One such argument and the game does not restart.
pub(crate) fn restartable_args(args: &[std::ffi::OsString]) -> bool {
    const BARE: [&str; 9] = [
        "--launcher",
        "--launcher-list",
        "--no-audio",
        "--no-soundtrack",
        "--no-effects",
        "--no-intro",
        "--no-video",
        "--original-menu",
        "--pick-language",
    ];
    const VALUED: [&str; 3] = ["--source", "--dlc", "--script"];
    let mut args = args.iter().map(|arg| arg.to_string_lossy());
    while let Some(arg) = args.next() {
        if !arg.starts_with('-') || BARE.contains(&&*arg) {
            continue;
        }
        let (name, joined) = arg
            .split_once('=')
            .map_or((&*arg, false), |(n, _)| (n, true));
        if !VALUED.contains(&name) || (!joined && args.next().is_none()) {
            return false;
        }
    }
    true
}

/// Set once a restart has been started, so one loss starts one process however many frames see
/// it.
static RESTARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Starts this game again if it may be, once. Returns whether a restart has been started (now or
/// before). Not from a run that is itself a restart (a driver that loses the device again on a
/// fresh start ends the run), nor with arguments [`restartable_args`] refuses.
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
pub(crate) fn restart_once() -> bool {
    use std::sync::atomic::Ordering;
    if RESTARTED.load(Ordering::Acquire) {
        return true;
    }
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if std::env::var_os(RELAUNCHED_ENV).is_some() || !restartable_args(&args) {
        return false;
    }
    if RESTARTED.swap(true, Ordering::AcqRel) {
        return true;
    }
    match relaunch(&args) {
        Ok(child) => {
            log::info!(
                "restarting on a fresh graphics device (process {})",
                child.id()
            );
            true
        }
        Err(e) => {
            log::error!("could not start the game again: {e}");
            RESTARTED.store(false, Ordering::Release);
            false
        }
    }
}

/// Starts this game again with the arguments it was given, marked as a relaunch. The caller
/// leaves the event loop afterwards: the new process opens its own window on a fresh device.
///
/// Inside an AppImage the running executable lives in a mount that goes away with this process,
/// so the file itself (`$APPIMAGE`) is started instead.
///
/// # Errors
///
/// The executable cannot be found or started.
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
fn relaunch(args: &[std::ffi::OsString]) -> std::io::Result<std::process::Child> {
    let exe = match std::env::var_os("APPIMAGE") {
        Some(file) if !file.is_empty() => file.into(),
        _ => std::env::current_exe()?,
    };
    std::process::Command::new(exe)
        .args(args)
        .env(RELAUNCHED_ENV, "1")
        .spawn()
}

#[cfg(test)]
mod tests;
