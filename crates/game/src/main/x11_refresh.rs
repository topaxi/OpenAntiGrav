//! The refresh rate of the output an X11 window is on, read through RandR.
//!
//! winit's monitor report on X11 is taken when the window is created and refreshed only by a
//! synthetic configure event a window manager sent, which gamescope does not. A dock, an
//! undock or `xrandr --rate` after that never reaches `current_monitor()` or
//! `available_monitors()`, so the rate the policy reads (`oag_present::refresh`) would be stale
//! for the life of the process. The server is asked directly instead, over a connection of our
//! own, on a thread of its own: a round trip never lands on a frame.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;
use x11rb::connection::Connection;
use x11rb::protocol::randr::{self, ConnectionExt as _, ModeFlag};
use x11rb::protocol::xproto::{ConnectionExt as _, Window as XWindow};
use x11rb::rust_connection::RustConnection;

/// How often the server is asked.
const POLL_EVERY: Duration = Duration::from_secs(1);

/// No answer yet, or none the server could give.
const UNKNOWN: u32 = 0;

/// A thread asking the X server for the window's output rate once a second.
pub(crate) struct LiveRefresh {
    millihertz: Arc<AtomicU32>,
    /// Dropping it ends the thread at its next wake.
    _stop: Sender<()>,
}

impl LiveRefresh {
    /// Starts on an X11 window; `None` on any other window system or when no server answers.
    pub(crate) fn start(window: &Window) -> Option<Self> {
        let id = match window.window_handle().ok()?.as_raw() {
            RawWindowHandle::Xlib(handle) => u32::try_from(handle.window).ok()?,
            RawWindowHandle::Xcb(handle) => handle.window.get(),
            _ => return None,
        };
        let (connection, screen) = RustConnection::connect(None).ok()?;
        let root = connection.setup().roots.get(screen)?.root;
        let millihertz = Arc::new(AtomicU32::new(UNKNOWN));
        let (stop, wake) = mpsc::channel::<()>();
        let shared = Arc::clone(&millihertz);
        std::thread::Builder::new()
            .name("x11-refresh".into())
            .spawn(move || {
                loop {
                    // A failed round keeps the last answer: the fallback is a frozen monitor
                    // report, which is wrong through a dock.
                    if let Some(rate) = read(&connection, root, id)
                        && shared.swap(rate, Ordering::Relaxed) != rate
                    {
                        log::debug!("X server reports {rate} mHz for the window's output");
                    }
                    if wake.recv_timeout(POLL_EVERY) != Err(RecvTimeoutError::Timeout) {
                        return;
                    }
                }
            })
            .ok()?;
        Some(Self {
            millihertz,
            _stop: stop,
        })
    }

    /// The last rate the server gave for the window's output, `None` before its first answer.
    pub(crate) fn millihertz(&self) -> Option<u32> {
        Some(self.millihertz.load(Ordering::Relaxed)).filter(|&mhz| mhz != UNKNOWN)
    }
}

/// An axis-aligned rectangle in root coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl Rect {
    /// The area shared with `other`, in pixels.
    fn overlap(self, other: Rect) -> u64 {
        let span = |a: i32, a_len: u32, b: i32, b_len: u32| {
            let lo = i64::from(a).max(i64::from(b));
            let hi = (i64::from(a) + i64::from(a_len)).min(i64::from(b) + i64::from(b_len));
            (hi - lo).max(0) as u64
        };
        span(self.x, self.width, other.x, other.width)
            * span(self.y, self.height, other.y, other.height)
    }
}

/// The candidate with the most overlap with the window, the first one when the window touches
/// none (a window off every output is still shown by something).
fn pick(window: Rect, outputs: &[(Rect, u32)]) -> Option<u32> {
    let mut best: Option<(u64, u32)> = None;
    for &(rect, millihertz) in outputs {
        let area = window.overlap(rect);
        if best.is_none_or(|(most, _)| area > most) {
            best = Some((area, millihertz));
        }
    }
    best.map(|(_, millihertz)| millihertz)
}

/// A mode's refresh in millihertz: the dot clock over the total scan, as `xrandr` prints it,
/// with a doubled scan counted twice and an interlaced one as fields.
fn mode_millihertz(mode: &randr::ModeInfo) -> Option<u32> {
    let flags = u32::from(mode.mode_flags);
    let mut vtotal = u64::from(mode.vtotal);
    if flags & u32::from(ModeFlag::DOUBLE_SCAN) != 0 {
        vtotal *= 2;
    }
    let mut clock = u64::from(mode.dot_clock) * 1000;
    if flags & u32::from(ModeFlag::INTERLACE) != 0 {
        clock *= 2;
    }
    let scan = u64::from(mode.htotal) * vtotal;
    (scan != 0)
        .then(|| (clock + scan / 2) / scan)
        .and_then(|mhz| u32::try_from(mhz).ok())
}

/// One round of questions: the window's rectangle on the root, and each active output's
/// rectangle and rate.
fn read(connection: &RustConnection, root: XWindow, window: XWindow) -> Option<u32> {
    let geometry = connection.get_geometry(window).ok()?;
    let origin = connection.translate_coordinates(window, root, 0, 0).ok()?;
    let resources = connection.randr_get_screen_resources_current(root).ok()?;
    let (geometry, origin, resources) = (geometry.reply().ok()?, origin.reply().ok()?, resources.reply().ok()?);
    let infos: Vec<_> = resources
        .crtcs
        .iter()
        .filter_map(|&crtc| {
            connection
                .randr_get_crtc_info(crtc, resources.config_timestamp)
                .ok()
        })
        .collect();
    let outputs: Vec<(Rect, u32)> = infos
        .into_iter()
        .filter_map(|cookie| {
            let info = cookie.reply().ok().filter(|info| info.mode != 0)?;
            let mode = resources.modes.iter().find(|mode| mode.id == info.mode)?;
            let rect = Rect {
                x: i32::from(info.x),
                y: i32::from(info.y),
                width: u32::from(info.width),
                height: u32::from(info.height),
            };
            Some((rect, mode_millihertz(mode)?))
        })
        .collect();
    let window = Rect {
        x: i32::from(origin.dst_x),
        y: i32::from(origin.dst_y),
        width: u32::from(geometry.width),
        height: u32::from(geometry.height),
    };
    pick(window, &outputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, width: u32, height: u32) -> Rect {
        Rect { x, y, width, height }
    }

    fn mode(dot_clock: u32, htotal: u16, vtotal: u16, flags: u32) -> randr::ModeInfo {
        randr::ModeInfo {
            id: 1,
            width: 0,
            height: 0,
            dot_clock,
            hsync_start: 0,
            hsync_end: 0,
            htotal,
            hskew: 0,
            vsync_start: 0,
            vsync_end: 0,
            vtotal,
            name_len: 0,
            mode_flags: randr::ModeFlag::from(flags),
        }
    }

    #[test]
    fn a_mode_is_its_dot_clock_over_its_scan() {
        // 3840x2160 timing at 594 MHz: 4400 x 2250 is 60.000 Hz.
        assert_eq!(mode_millihertz(&mode(594_000_000, 4400, 2250, 0)), Some(60_000));
        assert_eq!(mode_millihertz(&mode(0, 4400, 2250, 0)), Some(0));
        assert_eq!(mode_millihertz(&mode(1, 0, 0, 0)), None);
    }

    #[test]
    fn a_doubled_scan_halves_and_an_interlaced_one_doubles() {
        let plain = mode_millihertz(&mode(148_500_000, 2200, 1125, 0)).unwrap();
        let doubled = mode_millihertz(&mode(148_500_000, 2200, 1125, u32::from(ModeFlag::DOUBLE_SCAN))).unwrap();
        let interlaced = mode_millihertz(&mode(148_500_000, 2200, 1125, u32::from(ModeFlag::INTERLACE))).unwrap();
        assert_eq!(plain, 60_000);
        assert_eq!(doubled, 30_000);
        assert_eq!(interlaced, 120_000);
    }

    #[test]
    fn the_output_under_most_of_the_window_gives_the_rate() {
        let outputs = [(rect(0, 0, 1920, 1080), 60_000), (rect(1920, 0, 1280, 800), 90_000)];
        assert_eq!(pick(rect(1900, 0, 1000, 700), &outputs), Some(90_000));
        assert_eq!(pick(rect(0, 0, 800, 600), &outputs), Some(60_000));
        assert_eq!(pick(rect(9000, 9000, 10, 10), &outputs), Some(60_000));
        assert_eq!(pick(rect(0, 0, 10, 10), &[]), None);
    }
}
