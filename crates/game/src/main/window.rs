//! Choosing a monitor, placing a window on it, and how it presents.
//!
//! Beside [`crate::gpu`] rather than inside it: these answer questions about
//! the display the window is going onto, which the settings menu asks again
//! every time it is changed - see `Session::apply_window`.

use log::warn;
use oag_display::display;
use oag_game::icon;
use oag_present::perf;

/// The window's Wayland `app_id` and X11 `WM_CLASS` - the same field on both
/// backends' shared `PlatformSpecificWindowAttributes`, so one call to
/// `with_name` sets it for either. Matches `packaging/appimage/oag-game.desktop`'s
/// own filename (its desktop file ID) and `Icon=` key, and what
/// `just install-desktop-file` writes for a source checkout: winit's own docs
/// for `with_name` say the general name "should match the `.desktop` file
/// distributed with your program", because a Wayland compositor's taskbar has
/// no window icon to read at all (see [`window_icon`]'s doc) - it looks the
/// app up by this id and shows whatever icon that `.desktop` entry names.
#[cfg(target_os = "linux")]
pub(crate) const APP_ID: &str = "oag-game";

/// The window/taskbar icon, rasterised fresh every launch from
/// `assets/icons/64x64.svg` via [`oag_game::icon`].
///
/// **Only reaches the picture on X11 and Windows.** Wayland has no protocol
/// for a window to hand the compositor an icon - `winit`'s own Wayland
/// backend implements `set_window_icon` as an empty function - so on Wayland
/// (this includes niri) a taskbar's icon comes from the installed `.desktop`
/// entry keyed by [`APP_ID`] instead; see `just install-desktop-file`. Set
/// here anyway because it is free, it is what alt-tab and the titlebar read
/// everywhere it does apply, and a `cfg`-gated version that skipped Wayland
/// would be a second thing to keep in sync with winit's own platform support.
pub(crate) fn window_icon() -> winit::window::Icon {
    let icon::Rgba {
        width,
        height,
        pixels,
    } = icon::rasterize(64);
    winit::window::Icon::from_rgba(pixels, width, height)
        .expect("assets/icons/64x64.svg rasterizes to a 64x64 buffer, which from_rgba accepts")
}

/// What winit is asked for, for a mode and a chosen screen.
///
/// `Borderless(None)` means "the monitor this window is on", which is what
/// makes it unable to fail: exclusive fullscreen needs a `VideoMode` enumerated
/// off a monitor and can be refused, and this build does not offer it. See
/// [`display::WindowMode::ALL`]. Naming a monitor keeps that property - it is
/// still borderless, just on a screen this build has already confirmed exists.
pub(crate) fn fullscreen(
    mode: display::WindowMode,
    monitor: Option<winit::monitor::MonitorHandle>,
) -> Option<winit::window::Fullscreen> {
    match mode {
        display::WindowMode::Windowed => None,
        display::WindowMode::Borderless => Some(winit::window::Fullscreen::Borderless(monitor)),
    }
}

/// How one window opens: the same for window 1 and every further window, so they float, tile
/// or fill a monitor alike.
pub(crate) struct Look<'a> {
    pub(crate) title: &'a str,
    pub(crate) size: display::Size,
    pub(crate) mode: display::WindowMode,
    /// The monitor it opens on: borderless over all of it, or a window centred on it.
    pub(crate) monitor: Option<winit::monitor::MonitorHandle>,
    /// Where a windowed window is asked to sit instead of centred on `monitor`.
    pub(crate) position: Option<winit::dpi::PhysicalPosition<i32>>,
}

/// The attributes every window of a run is built from (window 1, [`crate::gpu::Gpu::new`], and
/// each further window, `Gpu::open_output`).
///
/// **A windowed window is fixed size, and that is a measurement.** Setting the minimum and
/// maximum to the same thing (`with_resizable(false)`) is the signal a tiling compositor floats
/// a window on rather than squeezing it into a column - measured under niri, which tiles it to a
/// portrait slot without this and honours the requested size with it. Borderless has to be
/// resizable, because the compositor is about to resize it to the monitor. A window built
/// without this one (the further windows were, until 2026-10-14) tiles on niri while window 1
/// floats.
///
/// The icon and the app id are the same for every window; a windowed position is asked for at
/// creation (`with_position`), which X11 and Windows honour and Wayland has no protocol for - the
/// compositor places the window there.
pub(crate) fn attributes(look: Look<'_>) -> winit::window::WindowAttributes {
    let Look {
        title,
        size,
        mode,
        monitor,
        position,
    } = look;
    let borderless = mode == display::WindowMode::Borderless;
    let mut attributes = winit::window::Window::default_attributes()
        .with_title(title)
        .with_inner_size(winit::dpi::LogicalSize::new(size.width, size.height))
        .with_resizable(borderless)
        .with_window_icon(Some(window_icon()))
        .with_fullscreen(fullscreen(mode, monitor.clone()));
    // The Wayland/X11 app id - see [`APP_ID`]'s doc. `WindowAttributesExtWayland` rather than
    // the X11 twin because both write the same underlying field; cfg-gated because the trait
    // only exists when winit is built with a Linux backend.
    #[cfg(target_os = "linux")]
    {
        use winit::platform::wayland::WindowAttributesExtWayland;
        attributes = attributes.with_name(APP_ID, APP_ID);
    }
    // Borderless carries the monitor in the fullscreen request; a windowed window has nothing to
    // carry it, so it is placed on the screen instead, at creation rather than moved afterwards
    // (a window moved off another has already told it `Occluded`, which pauses the race).
    if !borderless {
        if let Some(position) = position {
            attributes = attributes.with_position(position);
        } else if let Some(monitor) = &monitor {
            attributes = attributes.with_position(centred_on(monitor, size));
        }
    }
    attributes
}

/// What each monitor is called on the menu and in the settings file.
///
/// A screen the platform has no name for is numbered instead, so the list has
/// no blank rows. That number is positional and a settings file holding one is
/// therefore as fragile as an index would have been - which is why it is the
/// fallback and not the scheme; see [`display::Monitor`].
pub(crate) fn monitor_names(monitors: &[winit::monitor::MonitorHandle]) -> Vec<String> {
    monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| {
            monitor
                .name()
                .unwrap_or_else(|| format!("screen {}", index + 1))
        })
        .collect()
}

/// The monitor a setting names, or `None` for "let the compositor decide".
///
/// A name this machine does not have is a note and the default, not an error:
/// the ordinary way to get one is to unplug a screen, and refusing to open a
/// window over it would be punishing a player for their own desk. The note
/// lists what is there, because the next thing anyone wants is the spelling.
pub(crate) fn choose_monitor(
    monitors: Vec<winit::monitor::MonitorHandle>,
    setting: &display::Monitor,
) -> Option<winit::monitor::MonitorHandle> {
    let names = monitor_names(&monitors);
    if let Some(index) = setting.choose(&names) {
        return monitors.into_iter().nth(index);
    }
    if let Some(wanted) = setting.name() {
        warn!(
            "no monitor named {wanted:?}; using the default (this machine has: {})",
            names.join(", ")
        );
    }
    None
}

/// Where a windowed window goes to sit on `monitor`.
///
/// The arithmetic is [`display::centred`], which is tested; this is the part
/// that reads winit's own rectangle and cannot be. A monitor's scale factor is
/// what turns the setting's logical size into the physical pixels the position
/// is measured in - getting that wrong offsets the window by the difference on
/// any screen that is not at 100 %.
///
/// **Centred on the window's inner extent and applied to its outer one**, so a
/// decorated window sits high by about a title bar. Not corrected, because the
/// correction is not knowable before the window exists and the frame size is
/// the compositor's to decide anyway - this is a request it may refuse
/// outright, and being a title bar off "centred" is the smallest of the ways
/// that can go.
pub(crate) fn centred_on(
    monitor: &winit::monitor::MonitorHandle,
    size: display::Size,
) -> winit::dpi::PhysicalPosition<i32> {
    let scale = monitor.scale_factor();
    let physical = |value: u32| (f64::from(value) * scale).round().max(0.0) as u32;
    let origin = monitor.position();
    let area = monitor.size();
    let (x, y) = display::centred(
        (origin.x, origin.y),
        (area.width, area.height),
        (physical(size.width), physical(size.height)),
    );
    winit::dpi::PhysicalPosition::new(x, y)
}

/// What each vsync setting asks the surface for, best first.
///
/// Named modes rather than wgpu's `Auto` pair, because the three settings are
/// three specific behaviours and `AutoNoVsync` picks between two of them: it
/// prefers `Mailbox` and falls back to `Immediate`, so asking for it made
/// "off" mean *either* "tear for the lowest latency" or "never tear", driver
/// depending. That is exactly the distinction this row now exists to let a
/// player make.
///
/// The cost of naming them is that **only `Fifo` is guaranteed** - Vulkan
/// requires it and makes the other two optional - and configuring a surface
/// with a mode it does not offer is a panic, not an error. Hence a chain per
/// setting and [`Gpu::present_mode`] walking it against what the surface
/// actually reported.
pub(crate) fn present_modes(vsync: perf::Vsync) -> &'static [wgpu::PresentMode] {
    match vsync {
        // Second choice is `Mailbox` and not `Fifo`: what "off" is asked for is
        // a loop that is never blocked, and mailbox keeps that while fifo
        // destroys it.
        perf::Vsync::Off => &[wgpu::PresentMode::Immediate, wgpu::PresentMode::Mailbox],
        perf::Vsync::On => &[wgpu::PresentMode::Fifo],
        perf::Vsync::Smooth => &[wgpu::PresentMode::Mailbox, wgpu::PresentMode::Fifo],
    }
}

/// What a further window of a multi-window race asks the surface for, best first.
///
/// **A mode that does not block**, whatever the VSYNC row says: the frame loop is
/// paced by window 1, and a second `Fifo` surface acquired from the same thread would
/// make the loop wait on that display's vblank as well, in turn. `Mailbox` keeps a
/// seat tear-free without waiting; `Immediate` tears but never waits; `Fifo` is what
/// is left when the surface offers neither, and the log says so. **Chosen, not
/// measured** on real displays - see `docs/rendering/multi-window.md`, "Pacing".
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
pub(crate) fn seat_present_modes() -> &'static [wgpu::PresentMode] {
    &[wgpu::PresentMode::Mailbox, wgpu::PresentMode::Immediate]
}

/// How many frames the CPU may queue ahead of the display, for the mode
/// [`present_modes`] resolved to.
///
/// Vulkan's swapchain holds this plus one images. `Mailbox` needs three to
/// discard into, and `smooth` falling back to `Fifo` keeps three so it still
/// never halves; both stay at wgpu's default of 2. `on` already accepts the
/// half-rate cliff, so a second queued frame only adds latency there, and
/// `Immediate` never waits on an image at all. **Chosen, not measured**: no
/// input-to-photon number backs the frame this saves yet.
pub(crate) fn frame_latency(vsync: perf::Vsync, mode: wgpu::PresentMode) -> u32 {
    match (vsync, mode) {
        (perf::Vsync::Smooth, _) | (_, wgpu::PresentMode::Mailbox) => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_mode_that_discards_or_must_not_halve_queues_two_frames() {
        use wgpu::PresentMode::{Fifo, Immediate, Mailbox};
        assert_eq!(frame_latency(perf::Vsync::On, Fifo), 1);
        assert_eq!(frame_latency(perf::Vsync::Off, Immediate), 1);
        assert_eq!(frame_latency(perf::Vsync::Off, Mailbox), 2);
        assert_eq!(frame_latency(perf::Vsync::Smooth, Mailbox), 2);
        assert_eq!(frame_latency(perf::Vsync::Smooth, Fifo), 2);
    }

    fn look(mode: display::WindowMode) -> Look<'static> {
        Look {
            title: "t",
            size: display::Size {
                width: 1440,
                height: 816,
            },
            mode,
            monitor: None,
            position: None,
        }
    }

    /// Every window, the first and the further ones alike, is built by [`attributes`]: a windowed
    /// one is fixed size (a tiling compositor floats it, as it floats window 1) and a borderless
    /// one is resizable and fullscreen.
    #[test]
    fn a_windowed_window_is_fixed_size_and_a_borderless_one_is_resizable_and_fullscreen() {
        let windowed = attributes(look(display::WindowMode::Windowed));
        assert!(!windowed.resizable, "fixed size is the float signal");
        assert!(windowed.fullscreen.is_none());
        let borderless = attributes(look(display::WindowMode::Borderless));
        assert!(borderless.resizable, "the compositor resizes it to the monitor");
        assert!(matches!(
            borderless.fullscreen,
            Some(winit::window::Fullscreen::Borderless(None))
        ));
    }
}
