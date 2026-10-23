//! Choosing a monitor, placing a window on it, and how it presents.
//!
//! Beside [`crate::gpu`] rather than inside it: these answer questions about
//! the display the window is going onto, which the settings menu asks again
//! every time it is changed - see `Session::apply_window`.

use log::{info, warn};
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

/// What the session says about the compositor, as plain data so the decision is testable.
pub(crate) struct Signals<'a> {
    /// `GAMESCOPE_WAYLAND_DISPLAY` is set: gamescope's own session (a game launched by Steam on
    /// a Deck or a Linux box in gaming mode).
    pub(crate) gamescope_display: bool,
    /// `XDG_CURRENT_DESKTOP`.
    pub(crate) desktop: Option<&'a str>,
    /// The name winit reports for the monitor the window is on.
    pub(crate) monitor: Option<&'a str>,
}

/// Whether the window is being opened under gamescope, by gamescope's own signals and never by
/// the machine being a Deck. The monitor name is the one that still holds for a launch with a
/// bare environment over ssh, which is how the problem was first seen.
pub(crate) fn is_gamescope(signals: &Signals<'_>) -> bool {
    signals.gamescope_display
        || signals.desktop.is_some_and(|d| d.eq_ignore_ascii_case("gamescope"))
        || signals.monitor.is_some_and(|m| m.eq_ignore_ascii_case("gamescope"))
}

/// [`is_gamescope`] over this process's environment and the given monitor.
pub(crate) fn under_gamescope(monitor: Option<&winit::monitor::MonitorHandle>) -> bool {
    #[cfg(target_os = "linux")]
    {
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
        let name = monitor.and_then(|m| m.name());
        is_gamescope(&Signals {
            gamescope_display: std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some(),
            desktop: desktop.as_deref(),
            monitor: name.as_deref(),
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = monitor;
        false
    }
}

/// [`under_gamescope`], asked once per process: it cannot change under a running
/// game, and the frame loop asks every frame.
pub(crate) fn under_gamescope_cached(monitor: Option<&winit::monitor::MonitorHandle>) -> bool {
    static GAMESCOPE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *GAMESCOPE.get_or_init(|| under_gamescope(monitor))
}

/// The scale to turn a logical size into pixels with. Under gamescope it is 1, whatever winit
/// reports: gamescope's game Xwayland advertises its 3840x2160 screen as 100mm x 160mm, which
/// winit's X11 backend reads as a scale of 6, so a 1440x816 window opened as 8640x4896 pixels
/// (measured on a Steam Deck, 2026-10-14; `docs/tools/packaging.md`, "Under gamescope").
pub(crate) fn effective_scale(reported: f64, gamescope: bool) -> f64 {
    if gamescope { 1.0 } else { reported }
}

/// The mode a window is opened in. Under gamescope a window frame means nothing (gamescope draws
/// one window per output and no decoration), so `Windowed` is opened as `Borderless`, filling the
/// output; the saved setting is left alone, so the same file still means windowed on a desktop
/// (maintainer decision, 2026-10-14).
pub(crate) fn effective_mode(mode: display::WindowMode, gamescope: bool) -> display::WindowMode {
    if gamescope { display::WindowMode::Borderless } else { mode }
}

/// Whether a window is fullscreen right now: it is, by the compositor's report, or it is under
/// gamescope, which opens every window borderless whatever the saved mode says
/// ([`effective_mode`]). The saved `window_mode` alone misses both gamescope and an F11 toggle.
pub(crate) fn is_fullscreen(window: &winit::window::Window) -> bool {
    window.fullscreen().is_some() || under_gamescope_cached(window.current_monitor().as_ref())
}

/// The shape the game is drawn at: the setting, or in a fullscreen window a fixed fullscreen
/// resolution's own shape under `free` ([`display::FullscreenResolution::letterboxed`]).
pub(crate) fn scene_aspect(settings: &oag_game::settings::Display, fullscreen: bool) -> display::Aspect {
    if fullscreen {
        settings.fullscreen_resolution.letterboxed(settings.aspect)
    } else {
        settings.aspect
    }
}

/// The aspect rectangle the scene's resolution is measured against: the surface's, or in a
/// fullscreen window the fixed `display.fullscreen_resolution` when the player picked one.
pub(crate) fn scene_rect_of(
    settings: &oag_game::settings::Display,
    fullscreen: bool,
    surface: (u32, u32),
) -> (f32, f32, f32, f32) {
    let basis = if fullscreen {
        settings.fullscreen_resolution.target(surface)
    } else {
        surface
    };
    display::viewport(basis, scene_aspect(settings, fullscreen))
}

/// The windows a multi-window layout opens. Under gamescope it is one: every window of a client
/// is mapped full-output at 0,0 and gamescope shows one of them (measured on a Steam Deck,
/// 2026-10-14; `docs/rendering/multi-window.md`, "Under gamescope"), so the seats of the windows
/// that could not be seen are cut up in the one window instead, as a split screen is. Window 1
/// keeps its cut. **Chosen, not measured.**
pub(crate) fn one_window_under_gamescope(
    layout: Vec<oag_game::split_screen::WindowSeats>,
    gamescope: bool,
) -> Vec<oag_game::split_screen::WindowSeats> {
    if !gamescope || layout.len() < 2 {
        return layout;
    }
    let count = layout.iter().map(|window| window.count).sum();
    vec![oag_game::split_screen::WindowSeats { count, cut: layout[0].cut }]
}

/// The size asked of winit for a window of `size`: logical on a desktop, which is what the
/// setting means there, and physical pixels under gamescope, held to the monitor so a saved size
/// larger than the output cannot ask for more pixels than gamescope displays.
pub(crate) fn request_size(
    size: display::Size,
    gamescope: bool,
    monitor: Option<(u32, u32)>,
) -> winit::dpi::Size {
    if !gamescope {
        return winit::dpi::LogicalSize::new(size.width, size.height).into();
    }
    let (width, height) = match monitor {
        Some((w, h)) if w > 0 && h > 0 => (size.width.min(w), size.height.min(h)),
        _ => (size.width, size.height),
    };
    winit::dpi::PhysicalSize::new(width, height).into()
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

/// What F11 asks of a window: borderless on the monitor it is on now (`current`), or windowed
/// again when it is fullscreen already. `None` means "windowed". The monitor is the window's
/// own at the moment of the press, which is what lets a player drag each window to its screen
/// first. Per window and per session: the saved WINDOW MODE setting is not rewritten
/// (**chosen, not measured**).
pub(crate) fn toggled_fullscreen(
    now: Option<winit::window::Fullscreen>,
    current: Option<winit::monitor::MonitorHandle>,
) -> Option<winit::window::Fullscreen> {
    match now {
        Some(_) => None,
        None => Some(winit::window::Fullscreen::Borderless(current)),
    }
}

/// Whether `key` is the fullscreen hotkey, on a fresh press (not an auto-repeat).
pub(crate) fn is_fullscreen_key(key: &winit::keyboard::Key, pressed: bool, repeat: bool) -> bool {
    pressed && !repeat && *key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::F11)
}

/// Flips `window` between borderless fullscreen on its current monitor and windowed.
///
/// A windowed window is fixed size (min = max, the float signal, see [`attributes`]) and winit
/// keeps that pair as the window's own limits, so going fullscreen clears both (`set_resizable`
/// alone restores the pair and the window manager then holds the window at its old size -
/// measured under openbox) and the way back pins them to `restore`, the inner size remembered
/// on the way in, rather than to whatever size the window has while it is still leaving
/// fullscreen.
pub(crate) fn toggle_fullscreen(
    window: &winit::window::Window,
    restore: &mut Option<winit::dpi::PhysicalSize<u32>>,
) {
    let next = toggled_fullscreen(window.fullscreen(), window.current_monitor());
    if next.is_some() {
        *restore = Some(window.inner_size());
        window.set_min_inner_size(None::<winit::dpi::Size>);
        window.set_max_inner_size(None::<winit::dpi::Size>);
        window.set_resizable(true);
        window.set_fullscreen(next);
    } else {
        window.set_fullscreen(None);
        if let Some(size) = restore.take() {
            let _ = window.request_inner_size(size);
            window.set_min_inner_size(Some(size));
            window.set_max_inner_size(Some(size));
        }
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
    let gamescope = under_gamescope(look.monitor.as_ref());
    attributes_in(look, gamescope)
}

/// [`attributes`] with the compositor decided by the caller.
/// `inner` held to the monitor's size under gamescope, and untouched elsewhere. winit's X11
/// backend reports a freshly fullscreened window as the monitor's size times its scale, so under
/// gamescope's scale of 6 a 3840x2160 output reads 23040x12960 until the first `Resized`
/// corrects it (measured on a Steam Deck, 2026-10-14). A surface configured from that first
/// reading asked the driver for 6x the pixels and relied on the device limit to catch it.
pub(crate) fn held_to_monitor(
    inner: winit::dpi::PhysicalSize<u32>,
    gamescope: bool,
    monitor: Option<(u32, u32)>,
) -> winit::dpi::PhysicalSize<u32> {
    match monitor {
        Some((w, h)) if gamescope && w > 0 && h > 0 => {
            winit::dpi::PhysicalSize::new(inner.width.min(w), inner.height.min(h))
        }
        _ => inner,
    }
}

/// [`held_to_monitor`] for a live window: the size a surface over it is configured with.
pub(crate) fn surface_size(window: &winit::window::Window) -> winit::dpi::PhysicalSize<u32> {
    let monitor = window.current_monitor();
    held_to_monitor(
        window.inner_size(),
        under_gamescope_cached(monitor.as_ref()),
        monitor.map(|m| (m.size().width, m.size().height)),
    )
}

fn attributes_in(look: Look<'_>, gamescope: bool) -> winit::window::WindowAttributes {
    let Look {
        title,
        size,
        mode,
        monitor,
        position,
    } = look;
    let mode = effective_mode(mode, gamescope);
    let borderless = mode == display::WindowMode::Borderless;
    let mut attributes = winit::window::Window::default_attributes()
        .with_title(title)
        .with_inner_size(request_size(
            size,
            gamescope,
            monitor.as_ref().map(|m| (m.size().width, m.size().height)),
        ))
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

/// Logs what the window system reported for a freshly created window: every monitor's size and
/// scale, the window's own scale and inner size, and the requested one.
pub(crate) fn log_geometry(
    label: &str,
    window: &winit::window::Window,
    monitors: &[winit::monitor::MonitorHandle],
    requested: display::Size,
) {
    let inner = surface_size(window);
    let raw = window.inner_size();
    info!(
        "{label}: requested {}x{} logical, scale {}, inner {}x{} (winit reports {}x{}), fullscreen {}",
        requested.width,
        requested.height,
        window.scale_factor(),
        inner.width,
        inner.height,
        raw.width,
        raw.height,
        window.fullscreen().is_some(),
    );
    for monitor in monitors {
        let size = monitor.size();
        info!(
            "{label}: monitor {:?} {}x{} scale {}",
            monitor.name(),
            size.width,
            size.height,
            monitor.scale_factor(),
        );
    }
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

/// The MONITOR row's options. Under gamescope the compositor shows one output, so the choice
/// means nothing: the row offers only `default`, marked unusable for `reason` (dimmed, with the
/// reason in the note slot, `oag_ui::menu::Choice::unusable`). The stored name is untouched.
pub(crate) fn monitor_choices(
    names: &[String],
    gamescope: bool,
    reason: &str,
) -> Vec<oag_ui::menu::Choice> {
    if gamescope {
        return vec![oag_ui::menu::Choice::plain(display::Monitor::DEFAULT.to_string()).unusable(reason)];
    }
    display::Monitor::offered(names)
        .into_iter()
        .map(oag_ui::menu::Choice::plain)
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
    let scale = effective_scale(monitor.scale_factor(), under_gamescope(Some(monitor)));
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
    fn gamescope_offers_the_default_monitor_alone_marked_unusable() {
        let names = vec!["DP-1".to_string(), "HDMI-1".to_string()];
        let desk = monitor_choices(&names, false, "why");
        assert_eq!(desk.len(), 3);
        assert!(desk.iter().all(|choice| choice.unavailable.is_none()));
        let scope = monitor_choices(&names, true, "why");
        assert_eq!(scope.len(), 1);
        assert_eq!(scope[0].value, display::Monitor::DEFAULT);
        assert_eq!(scope[0].unavailable.as_deref(), Some("why"));
    }

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
    fn px(size: winit::dpi::Size) -> (u32, u32, bool) {
        match size {
            winit::dpi::Size::Physical(p) => (p.width, p.height, true),
            winit::dpi::Size::Logical(l) => (l.width as u32, l.height as u32, false),
        }
    }

    #[test]
    fn gamescope_is_told_by_its_own_signals() {
        let none = Signals { gamescope_display: false, desktop: Some("GNOME"), monitor: Some("DP-1") };
        assert!(!is_gamescope(&none));
        assert!(is_gamescope(&Signals { gamescope_display: true, ..none }));
        assert!(is_gamescope(&Signals { desktop: Some("gamescope"), ..none }));
        assert!(is_gamescope(&Signals { monitor: Some("gamescope"), ..none }));
    }

    #[test]
    fn under_gamescope_a_size_is_physical_pixels_held_to_the_output() {
        let size = display::Size { width: 2560, height: 1440 };
        assert_eq!(px(request_size(size, false, Some((3840, 2160)))), (2560, 1440, false));
        assert_eq!(px(request_size(size, true, Some((3840, 2160)))), (2560, 1440, true));
        assert_eq!(px(request_size(size, true, Some((1280, 800)))), (1280, 800, true));
        assert_eq!(px(request_size(size, true, None)), (2560, 1440, true));
        assert_eq!(effective_scale(6.0, true), 1.0);
        let six = winit::dpi::PhysicalSize::new(23040, 12960);
        let held = |gs, m| {
            let s = held_to_monitor(six, gs, m);
            (s.width, s.height)
        };
        assert_eq!(held(true, Some((3840, 2160))), (3840, 2160));
        assert_eq!(held(false, Some((3840, 2160))), (23040, 12960));
        assert_eq!(held(true, None), (23040, 12960));
        let ok = winit::dpi::PhysicalSize::new(1280, 800);
        assert_eq!(held_to_monitor(ok, true, Some((3840, 2160))), ok);
        assert_eq!(effective_scale(2.0, false), 2.0);
    }

    #[test]
    fn a_window_built_under_gamescope_asks_for_pixels_and_a_desktop_one_for_logical() {
        let under = attributes_in(look(display::WindowMode::Windowed), true);
        assert_eq!(px(under.inner_size.unwrap()), (1440, 816, true));
        let desktop = attributes_in(look(display::WindowMode::Windowed), false);
        assert_eq!(px(desktop.inner_size.unwrap()), (1440, 816, false));
    }

    #[test]
    fn under_gamescope_windowed_opens_borderless_and_a_desktop_keeps_the_setting() {
        use display::WindowMode::{Borderless, Windowed};
        assert_eq!(effective_mode(Windowed, true), Borderless);
        assert_eq!(effective_mode(Borderless, true), Borderless);
        assert_eq!(effective_mode(Windowed, false), Windowed);
        assert_eq!(effective_mode(Borderless, false), Borderless);
        let under = attributes_in(look(Windowed), true);
        assert!(under.resizable);
        assert!(matches!(under.fullscreen, Some(winit::window::Fullscreen::Borderless(None))));
        let desktop = attributes_in(look(Windowed), false);
        assert!(!desktop.resizable && desktop.fullscreen.is_none());
    }

    #[test]
    fn under_gamescope_the_windows_of_a_layout_become_one_window_holding_every_seat() {
        use oag_game::split_screen::WindowSeats;
        use oag_render::viewport::Cut;
        let two = |count, cut| WindowSeats { count, cut };
        let layout = vec![two(4, Cut::Horizontal), two(2, Cut::Vertical)];
        assert_eq!(one_window_under_gamescope(layout.clone(), false), layout);
        assert_eq!(
            one_window_under_gamescope(layout, true),
            vec![two(6, Cut::Horizontal)]
        );
        let single = vec![two(3, Cut::Vertical)];
        assert_eq!(one_window_under_gamescope(single.clone(), true), single);
        assert!(one_window_under_gamescope(Vec::new(), true).is_empty());
    }

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

    #[test]
    fn f11_goes_borderless_on_the_current_monitor_and_back_to_windowed() {
        use winit::window::Fullscreen;
        let on = toggled_fullscreen(None, None);
        assert!(matches!(on, Some(Fullscreen::Borderless(None))));
        assert!(toggled_fullscreen(on, None).is_none());
    }

    #[test]
    fn only_a_fresh_f11_press_is_the_fullscreen_key() {
        use winit::keyboard::{Key, NamedKey};
        let f11 = Key::Named(NamedKey::F11);
        assert!(is_fullscreen_key(&f11, true, false));
        assert!(!is_fullscreen_key(&f11, false, false));
        assert!(!is_fullscreen_key(&f11, true, true));
        assert!(!is_fullscreen_key(&Key::Named(NamedKey::F10), true, false));
    }
}
