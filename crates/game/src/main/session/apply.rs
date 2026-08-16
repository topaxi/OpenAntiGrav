//! Applying a changed setting to the live window, device and race.

use oag_game::{audio, display, menu, perf, settings};
use oag_gameplay::ControlScheme;
use oag_render::mesh_render::Anisotropy;

use crate::stage::Stage;
use crate::window::{centred_on, choose_monitor, fullscreen};

use super::Session;

impl Session {
    /// Puts the window onto the monitor, and into the mode and size, that the
    /// settings now hold.
    ///
    /// Immediately, unlike anisotropy and the language: a player who picks
    /// borderless and sees nothing happen will assume it is broken. The resize
    /// event the compositor sends back is what reconfigures the surface, so
    /// nothing here touches it.
    ///
    /// The size and the position are asked for only in windowed mode, and
    /// **every request here may be refused** - a compositor is allowed to
    /// ignore all three, and a tiling one will ignore at least two. Nothing
    /// depends on any of them being honoured.
    fn apply_window(&mut self) {
        let wanted = self.settings.display.clone();
        let monitor = choose_monitor(
            self.gpu.window.available_monitors().collect(),
            &wanted.monitor,
        );
        self.gpu
            .window
            .set_fullscreen(fullscreen(wanted.window_mode, monitor.clone()));
        let windowed = wanted.window_mode == display::WindowMode::Windowed;
        self.gpu.window.set_resizable(!windowed);
        if windowed {
            let size = wanted.window_size;
            let _ = self
                .gpu
                .window
                .request_inner_size(winit::dpi::LogicalSize::new(size.width, size.height));
            // Moved before it is resized as far as the compositor is concerned,
            // both being requests it may reorder or drop; the position is
            // computed from the size the settings hold rather than the one the
            // window currently has, so the two do not have to agree yet.
            if let Some(monitor) = &monitor {
                self.gpu
                    .window
                    .set_outer_position(centred_on(monitor, size));
            }
        }
    }

    /// Applies a changed setting and writes it back.
    ///
    /// Persisted on every keypress rather than on the way out, because there is
    /// no way out that is guaranteed to run: a player quits with the window
    /// button as often as with the menu. The file is a few hundred bytes.
    pub(crate) fn apply_setting(&mut self, setting: &str, value: &menu::Value) {
        let text = value.to_string();
        match setting {
            // `display.monitor` never fails to parse - a monitor name is
            // whatever the platform says it is - so a name this machine does
            // not have is reported by `choose_monitor` when the window is
            // moved, not here. See `display::Monitor`.
            "display.monitor" => {
                self.settings.display.monitor = display::Monitor::from(text);
                self.apply_window();
            }
            "display.window_mode" => match text.parse::<display::WindowMode>() {
                Ok(mode) => {
                    self.settings.display.window_mode = mode;
                    self.apply_window();
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "display.window_size" => match text.parse::<display::Size>() {
                Ok(size) => {
                    self.settings.display.window_size = size;
                    self.apply_window();
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "display.aspect" => match text.parse::<display::Aspect>() {
                // Applied by the next frame, because every stage takes its
                // viewport from this on the way into its pass.
                Ok(aspect) => self.settings.display.aspect = aspect,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Both applied by the next frame: the blit pass reads them on its
            // way onto the surface, so a change shows on whatever is on screen
            // - including the menu the player is standing on, which is the
            // point of putting them there rather than behind a race.
            "display.brightness" => match text.parse::<display::Brightness>() {
                Ok(brightness) => self.settings.display.brightness = brightness,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "display.gamma" => match text.parse::<display::Gamma>() {
                Ok(gamma) => self.settings.display.gamma = gamma,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Applied on the spot rather than by the next frame: a bus gain is
            // read by whatever the mixer renders next, which on a device is
            // already in flight. That is what makes the row audible while the
            // player is standing on it, the way the two above are visible.
            "audio.music_volume" => match text.parse::<audio::Volume>() {
                Ok(volume) => {
                    self.settings.audio.music_volume = volume;
                    self.audio.apply(&self.settings.audio);
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Applied on the spot, and **seeked rather than restarted**: the
            // two releases' encodes of a track agree in length to 11 ms, so
            // carrying the playhead across lands in the same bar. The first
            // move onto a release reads it off its disc - measured at 2.0 s
            // for the PS2's 36 MiB of PCM and 0.4 s for the PSP's cached
            // decode - and every move after that is instant, because the sound
            // is held. See `audio::Audio::set_music_source`.
            "audio.music_source" => match text.parse::<audio::MusicSource>() {
                Ok(source) => {
                    self.settings.audio.music_source = source;
                    self.audio.set_music_source(
                        &self.music_discs,
                        source,
                        &oag_game::boot::default_audio_cache_dir(),
                    );
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // **Applied on the next launch**, and the only setting here that
            // cannot be applied at all this run: the device, every pipeline and
            // every uploaded mesh hang off the adapter chosen at boot, so
            // switching would mean tearing down the surface, the framebuffer
            // and whatever stage is on screen. Stored now, drawn with next time.
            // `Gpu::new` retries on the default if this one will not start, so
            // choosing an adapter that cannot serve is recoverable from inside
            // the game rather than only from the settings file.
            "graphics.renderer" => self.settings.graphics.renderer = display::Renderer::from(text),
            "graphics.render_scale" => match text.parse::<display::Scale>() {
                // Applied by the next frame: `frame` sizes the target from this
                // every time and rebuilds it when the answer changes.
                Ok(scale) => self.settings.graphics.render_scale = scale,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.upscaler" => match text.parse::<display::Upscaler>() {
                // Applied by the next frame. Choosing `fsr1` for the first time
                // builds its two pipelines inside that frame, which is a shader
                // compilation a player may notice once and never again.
                Ok(upscaler) => self.settings.graphics.upscaler = upscaler,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.upscale_sharpness" => match text.parse::<display::Sharpness>() {
                // Applied by the next frame: it is one float in the upscaler's
                // uniform, rewritten only when it moves.
                Ok(sharpness) => self.settings.graphics.upscale_sharpness = sharpness,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // `off`, `fxaa` and `smaa` are applied by the next frame, the
            // same as the upscaler: `Framebuffer::resolve` reads this fresh
            // and builds FXAA's pipeline lazily, the same way it does FSR 1's.
            // **Only moving to or between the two MSAA levels waits for the
            // next race**, because that is what rebuilds the scene pipelines
            // MSAA's sample count is baked into - see `race::Scene::new`. The
            // row's restart note distinguishes the two cases; see
            // `Session::open_menus`.
            "graphics.anti_aliasing" => match text.parse::<display::AntiAliasing>() {
                Ok(mode) => self.settings.graphics.anti_aliasing = mode,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.fov" => match text.parse::<display::Fov>() {
                // Applied by the next frame the race draws, which builds its
                // projection from this every time. Nothing else uses it: the
                // front end and the menus are drawn flat.
                Ok(fov) => self.settings.graphics.fov = fov,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.perf_overlay" => match text.parse::<perf::Overlay>() {
                // Applied by the next frame, which draws it or does not.
                Ok(mode) => self.settings.graphics.perf_overlay = mode,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "display.vsync" => match text.parse::<perf::Vsync>() {
                Ok(vsync) => {
                    self.settings.display.vsync = vsync;
                    // Applied immediately, by reconfiguring the surface. A
                    // player who turns vsync off and sees nothing change will
                    // assume it is broken, and the frame limiter that comes
                    // with it would then look broken too.
                    self.gpu.set_vsync(vsync);
                    // The frames either side of a surface reconfigure are not
                    // frames anyone is going to present at that rate.
                    self.stalled = true;
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "display.frame_limit" => match text.parse::<perf::FrameLimit>() {
                // Applied by the next `about_to_wait`, which is what waits.
                Ok(limit) => self.settings.display.frame_limit = limit,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.anisotropy" => match text.parse::<Anisotropy>() {
                Ok(level) => {
                    self.anisotropy = level;
                    self.settings.graphics.anisotropy = level;
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Baked into the race at `Race::start` (see `Self::race`), so a
            // change here has no effect on the one already running - the same
            // as `graphics.anti_aliasing`'s MSAA levels above.
            "graphics.boost_fov_kick" => match text.parse::<display::BoostFovKick>() {
                Ok(kick) => self.settings.graphics.boost_fov_kick = kick,
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Applied to the race already running, unlike the two rows above:
            // the original binds this to a button precisely so it can be changed
            // while flying, and a row that deferred it to the next race would be
            // the odd one out rather than the careful one. Nothing it touches is
            // simulation state - see `race::Race::set_camera_view`.
            "graphics.camera_view" => match text.parse::<display::CameraView>() {
                Ok(view) => {
                    self.settings.graphics.camera_view = view;
                    if let Stage::Race(stage) = &mut self.stage {
                        stage.race.set_camera_view(view);
                    }
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // The scheme the *next* race starts with. Not applied to a race
            // already running: `Race::set_control_scheme` is called once before
            // the first tick, and swapping mid-race would leave a half-finished
            // gesture armed in `ShipState`.
            "controls.scheme" => match text.parse::<ControlScheme>() {
                Ok(scheme) => {
                    self.settings.controls.scheme = scheme.name().to_string();
                    self.scheme = scheme;
                }
                Err(e) => {
                    eprintln!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "race.mode" => self.settings.race.mode = text,
            "race.class" => self.settings.race.class = text,
            "race.team" => self.settings.race.team = text,
            "race.track" => self.settings.race.track = text,
            "ai.difficulty" => self.settings.ai.difficulty = text,
            "language" => self.settings.language = Some(text),
            other => {
                eprintln!("note: nothing applies {other}");
                return;
            }
        }
        if let Err(e) = settings::save(&self.settings) {
            eprintln!("could not save settings: {e:#}");
        }
    }

    /// Moves the in-race camera on one perspective and persists the choice.
    ///
    /// **This is the original's SELECT.** `Camera_UpdatePlayerView`
    /// (`0x0883c0cc`) tests abstract button index `0xf`, consumes the press,
    /// rotates its profile setting through three values and sets the profile's
    /// dirty byte - so the write back to the settings file here is a reproduction
    /// and not a convenience. Confidence **88** for the cycle and its order; see
    /// `docs/ghidra/functions/psp-pulse-usa/camera.md`.
    ///
    /// The order lives on [`display::CameraView::next`], so this function decides
    /// nothing about it: the button, the `CAMERA VIEW` menu row and the type's own
    /// test all walk one sequence.
    ///
    /// Saving on every press is deliberate. The alternative - saving on exit -
    /// loses the choice to a crash or a `kill`, and the file is a few hundred
    /// bytes written at most once per press of one button.
    pub(crate) fn cycle_camera_view(&mut self) {
        let next = self.settings.graphics.camera_view.next();
        self.settings.graphics.camera_view = next;
        if let Stage::Race(stage) = &mut self.stage {
            stage.race.set_camera_view(next);
        }
        // Nothing re-seeds the menu here: `settings::menu_seeds` reads
        // `self.settings` when the page opens, so the `CAMERA VIEW` row already
        // opens on whatever the player last flew with.
        if let Err(e) = settings::save(&self.settings) {
            eprintln!("could not save settings: {e:#}");
        }
    }
}
