//! Applying a changed setting to the live window, device and race.

use log::{error, warn};
use oag_display::display;
use oag_game::settings::TriggerSensitivity;
use oag_game::{input, settings};
use oag_gameplay::ControlScheme;
use oag_input::Controls;
use oag_input::pad::TriggerMode;
use oag_mesh::mesh_render::Anisotropy;
use oag_present::drs;
use oag_present::perf;
use oag_ui::menu;

use crate::stage::Stage;
use crate::window::{centred_on, choose_monitor, fullscreen};

use super::Session;

impl Session {
    /// Puts the window onto the monitor, and into the mode and size, that the
    /// settings now hold.
    ///
    /// Immediately, unlike anisotropy: a player who picks borderless and sees
    /// nothing happen will assume it is broken. The resize event the
    /// compositor sends back is what reconfigures the surface, so nothing
    /// here touches it.
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

    /// The [`settings::RenderProfile`] the five relocated `graphics.*` rows
    /// below read and write, for whichever title `self.shell` currently
    /// names - see [`settings::Settings::render_profiles`].
    ///
    /// `None` only when this run has no menus at all (`self.shell` is
    /// `None`), the same case every other per-shell row in this file already
    /// falls back on: a graphics row can only be touched with menus open, so
    /// this never actually returns `None` from a live keypress.
    fn render_profile_mut(&mut self) -> Option<&mut settings::RenderProfile> {
        let shell = self.shell.as_ref()?;
        let key = settings::profile_key(shell.title, shell.platform);
        Some(self.settings.render_profiles.entry(key).or_default())
    }

    /// [`Self::render_profile_mut`]'s read-only sibling, for the frame loop
    /// and every other reader that only needs a value rather than a place to
    /// write one back.
    ///
    /// Defaults rather than `None` on a shell-less run (a `--dry-run` or a
    /// capture with no menus): the render loop needs *a* value to draw with
    /// every frame, and [`settings::RenderProfile::default`] is exactly what
    /// `Graphics`'s own fields fell back to before this split.
    pub(crate) fn render_profile(&self) -> settings::RenderProfile {
        self.shell
            .as_ref()
            .map(|shell| (shell.title, shell.platform))
            // **The `--race` route, which has no shell and is not profile-less.**
            // Falling straight to the default here meant that route drew with
            // `render_scale` 100 and no reconstruction whatever the file or the
            // CLI said - see `App::race_title`.
            .or(self.race_title.zip(self.race_platform))
            .and_then(|(title, platform)| {
                self.settings
                    .render_profiles
                    .get(&settings::profile_key(title, platform))
            })
            .cloned()
            .unwrap_or_default()
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
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "display.window_size" => match text.parse::<display::Size>() {
                Ok(size) => {
                    self.settings.display.window_size = size;
                    self.apply_window();
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "display.aspect" => match text.parse::<display::Aspect>() {
                // Applied by the next frame, because every stage takes its
                // viewport from this on the way into its pass.
                Ok(aspect) => self.settings.display.aspect = aspect,
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
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
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "display.gamma" => match text.parse::<display::Gamma>() {
                Ok(gamma) => self.settings.display.gamma = gamma,
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Applied on the spot rather than by the next frame: a bus gain is
            // read by whatever the mixer renders next, which on a device is
            // already in flight. That is what makes the row audible while the
            // player is standing on it, the way the two above are visible.
            "audio.music_volume" => match text.parse::<oag_sound::Volume>() {
                Ok(volume) => {
                    self.settings.audio.music_volume = volume;
                    self.audio.apply(&self.settings.audio);
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // The other half of the original's two volumes, and the same
            // mechanism: straight onto `Bus::Sfx`'s gain. A held `~ENGINE`
            // voice follows it without being restarted, because a bus gain is
            // applied at mix time rather than at play time.
            "audio.sfx_volume" => match text.parse::<oag_sound::Volume>() {
                Ok(volume) => {
                    self.settings.audio.sfx_volume = volume;
                    self.audio.apply(&self.settings.audio);
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // The voice bus, which a cue reaches by living in `speech.bnk`
            // rather than by being listed anywhere - see `sfx::Cue::bus`.
            "audio.speech_volume" => match text.parse::<oag_sound::Volume>() {
                Ok(volume) => {
                    self.settings.audio.speech_volume = volume;
                    self.audio.apply(&self.settings.audio);
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // The third row, and the one that is ours: it moves the mixer's
            // master rather than a bus, so a player who finds the race
            // distorting turns one thing down instead of keeping two in step.
            // Same mechanism as the two above.
            "audio.master_volume" => match text.parse::<oag_sound::Volume>() {
                Ok(volume) => {
                    self.settings.audio.master_volume = volume;
                    self.audio.apply(&self.settings.audio);
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Applied on the spot, and **seeked rather than restarted**: the
            // two releases' encodes of a track agree in length to 11 ms, so
            // carrying the playhead across lands in the same bar. The first
            // move onto a release reads it off its disc - measured at 2.0 s
            // for the PS2's 36 MiB of PCM and 0.4 s for the PSP's cached
            // decode - and every move after that is instant, because the sound
            // is held. See `oag_sound::Audio::set_music_source`.
            // **Re-read rather than remembered.** The illustration is a decoded
            // image off the disc, so changing the styling means opening the
            // source again - cheap, and done here in the menus rather than at
            // the front of a load where it would sit between a press and a
            // race.
            "display.front_end_style" => {
                self.settings.display.front_end_style = text.to_string();
                self.reload_loading_assets();
            }
            "audio.music_source" => match text.parse::<oag_sound::MusicSource>() {
                Ok(source) => {
                    self.settings.audio.music_source = source;
                    self.audio.set_music_source(
                        &self.music_discs,
                        source,
                        &oag_source::cache::default_audio_cache_dir(),
                    );
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
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
                Ok(scale) => {
                    if let Some(profile) = self.render_profile_mut() {
                        profile.render_scale = scale;
                    }
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.target_fps" => match text.parse::<drs::Target>() {
                // Applied by the next frame, and the reset is what makes
                // turning it *off* immediate: `frame` re-applies the extent
                // every frame, and off means the ceiling.
                Ok(target) => {
                    if let Some(profile) = self.render_profile_mut() {
                        profile.target_fps = target;
                    }
                    self.drs.reset();
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.minimum_resolution" => match text.parse::<display::Scale>() {
                // A floor at or above the render scale is stored rather than
                // refused - the menu warns about it, the way the upscaler row
                // warns at scales where it does nothing - and `drs::Limits`
                // brings it under the ceiling where the two meet. Both land in
                // the render profile, beside the ceiling they are compared
                // against.
                Ok(scale) => {
                    if let Some(profile) = self.render_profile_mut() {
                        profile.minimum_resolution = scale;
                    }
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Every value on this axis is applied by the next frame:
            // `Framebuffer::resolve_scene` reads it fresh and builds each
            // pass's pipelines lazily. Choosing `fxaa`, `fsr1` or `fsr3` for
            // the first time compiles them inside that frame, which is a stall
            // a player may notice once and never again.
            "graphics.reconstruction" => match text.parse::<display::Reconstruction>() {
                Ok(reconstruction) => {
                    if let Some(profile) = self.render_profile_mut() {
                        profile.reconstruction = reconstruction;
                    }
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.upscale_sharpness" => match text.parse::<display::Sharpness>() {
                // Applied by the next frame: it is one float in the upscaler's
                // uniform, rewritten only when it moves.
                Ok(sharpness) => {
                    if let Some(profile) = self.render_profile_mut() {
                        profile.upscale_sharpness = sharpness;
                    }
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // **Always waits for the next race**, because this row is nothing
            // but the sample count the scene's pipelines are built with - see
            // `race::Scene::new`. Since ADR-0041 there is no half of it that
            // applies live, which is what the row's unconditional restart note
            // now says; see `Session::open_menus`.
            "graphics.msaa" => match text.parse::<display::Msaa>() {
                Ok(mode) => {
                    if let Some(profile) = self.render_profile_mut() {
                        profile.msaa = mode;
                    }
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Applied by the next frame the race draws, which hands the
            // strength to `Scene::render` fresh each time - the pass itself
            // is built with every race, whatever this says. See
            // `race::Scene::motion_blur`.
            "graphics.motion_blur" => match text.parse::<display::MotionBlur>() {
                Ok(strength) => {
                    if let Some(profile) = self.render_profile_mut() {
                        profile.motion_blur = strength;
                    }
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Live as well: the frame hands it to the race's blur pass before
            // every draw - see `race::Scene::set_blur_resolution`.
            "graphics.motion_blur_resolution" => match text.parse::<display::BlurResolution>() {
                Ok(resolution) => {
                    if let Some(profile) = self.render_profile_mut() {
                        profile.motion_blur_resolution = resolution;
                    }
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Applied by the next frame the race draws: the session hands the
            // preset to `Race::set_model_detail` every frame, and every model
            // is built with all its tiers whatever this says.
            "graphics.model_detail" => match text.parse::<oag_mesh::mesh::ModelDetail>() {
                Ok(detail) => {
                    if let Some(profile) = self.render_profile_mut() {
                        profile.model_detail = detail;
                    }
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Applied by the next frame the race draws: the session hands the
            // preset to `Race::set_texture_detail` every frame, which writes
            // it into the scene uniform the fragment stage reads.
            "graphics.texture_detail" => {
                match text.parse::<oag_mesh::mesh_render::TextureDetail>() {
                    Ok(detail) => {
                        if let Some(profile) = self.render_profile_mut() {
                            profile.texture_detail = detail;
                        }
                    }
                    Err(e) => {
                        warn!("ignoring {setting} = {text:?}: {e}");
                        return;
                    }
                }
            }
            // Applied by the next frame the race draws, which reads the
            // tier fresh - the pass is built with every race whatever this
            // says, the same shape `graphics.motion_blur` above has. See
            // `race::Scene::shadows`.
            "graphics.shadows" => match text.parse::<display::Shadows>() {
                Ok(tier) => {
                    if let Some(profile) = self.render_profile_mut() {
                        profile.shadows = tier;
                    }
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Applied by the next frame's composite, which resolves the name
            // through the catalogue and rebuilds the pass if it changed - so
            // the menu the player is standing on is drawn through the filter
            // they just picked, which is the only way to choose one.
            "graphics.screen_filter" => {
                if let Some(profile) = self.render_profile_mut() {
                    profile.screen_filter = text.clone();
                }
            }
            "graphics.screen_filter_strength" => match text.parse::<display::FilterStrength>() {
                Ok(strength) => {
                    if let Some(profile) = self.render_profile_mut() {
                        profile.screen_filter_strength = strength;
                    }
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.fov" => match text.parse::<display::Fov>() {
                // Applied by the next frame the race draws, which builds its
                // projection from this every time. Nothing else uses it: the
                // front end and the menus are drawn flat.
                Ok(fov) => self.settings.graphics.fov = fov,
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "display.pause_on_focus_loss" => match text.as_str() {
                // Read at the next focus event; nothing to apply now.
                "on" => self.settings.display.pause_on_focus_loss = true,
                "off" => self.settings.display.pause_on_focus_loss = false,
                _ => {
                    warn!("ignoring {setting} = {text:?}: expected on or off");
                    return;
                }
            },
            "graphics.perf_overlay" => match text.parse::<perf::Overlay>() {
                // Applied by the next frame, which draws it or does not.
                Ok(mode) => self.settings.graphics.perf_overlay = mode,
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
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
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "display.frame_limit" => match text.parse::<perf::FrameLimit>() {
                // Applied by the next `about_to_wait`, which is what waits.
                Ok(limit) => self.settings.display.frame_limit = limit,
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "graphics.anisotropy" => match text.parse::<Anisotropy>() {
                Ok(level) => {
                    self.anisotropy = level;
                    self.settings.graphics.anisotropy = level;
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Baked into the race at `Race::start` (see `Self::race`), so a
            // change here has no effect on the one already running - the same
            // as `graphics.anti_aliasing`'s MSAA levels above.
            "graphics.boost_fov_kick" => match text.parse::<display::BoostFovKick>() {
                Ok(kick) => self.settings.graphics.boost_fov_kick = kick,
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
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
                    warn!("ignoring {setting} = {text:?}: {e}");
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
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            // Both trigger rows apply *now*, unlike the scheme above: nothing
            // carries across a tick either way - `pad::resolve` is a function
            // of one frame's readings - so there is no half-finished gesture to
            // strand, and a player tuning the feel of a brake needs to feel it.
            "controls.triggers" => match text.parse::<TriggerMode>() {
                Ok(mode) => {
                    self.settings.controls.triggers = mode.name().to_string();
                    self.controls.set_trigger_mode(mode);
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "controls.touch_go_zones" => match text.as_str() {
                // Read by the next tick's `feed_touch`.
                "on" => self.settings.controls.touch_go_zones = true,
                "off" => self.settings.controls.touch_go_zones = false,
                _ => {
                    warn!("ignoring {setting} = {text:?}: expected on or off");
                    return;
                }
            },
            "controls.touch_opacity" => match text.parse::<u8>() {
                // Read by the next frame's overlay draw.
                Ok(percent) if (10..=100).contains(&percent) => {
                    self.settings.controls.touch_opacity = percent;
                }
                _ => {
                    warn!("ignoring {setting} = {text:?}: expected a percentage from 10 to 100");
                    return;
                }
            },
            "controls.trigger_sensitivity" => match text.parse::<TriggerSensitivity>() {
                Ok(sensitivity) => {
                    self.settings.controls.trigger_sensitivity = sensitivity;
                    self.controls.set_trigger_curve(sensitivity.exponent());
                }
                Err(e) => {
                    warn!("ignoring {setting} = {text:?}: {e}");
                    return;
                }
            },
            "race.mode" => {
                self.settings.race.mode = text;
                // The CIRCUIT row's own list depends on this setting - Zone
                // circuits for a title whose Zone environments are separate
                // from its race ones, the race list otherwise - and nothing
                // else re-supplies it when MODE changes underneath an open
                // menu. See `Session::resupply_tracks_for_mode`.
                self.resupply_tracks_for_mode();
            }
            "race.class" => self.settings.race.class = text,
            // VARIANT's own list depends on which team this names - the
            // same reason `race.mode` above resupplies CIRCUIT.
            "race.team" => {
                self.settings.race.team = text;
                self.resupply_race_variant();
            }
            "race.track" => self.settings.race.track = text,
            "race.variant" => self.settings.race.variant = text,
            "race.kill_target" => self.settings.race.kill_target = text,
            "race.weapons" => self.settings.race.weapons = text,
            "ai.difficulty" => self.settings.ai.difficulty = text,
            // The two title rows resupply their scoped TRACK/TEAM the same
            // way `race.mode` resupplies the ordinary TRACK row above -
            // `Menu::supply` only runs at menu-open, and nothing else tells
            // RemixTracks/RemixTeams that the title underneath them moved.
            "remix.track_title" => {
                self.settings.remix.track_title = text;
                self.resupply_remix_tracks();
            }
            "remix.craft_title" => {
                self.settings.remix.craft_title = text;
                self.resupply_remix_teams();
                // VARIANT depends on the team, which just moved to whichever
                // this craft title's roster offers first.
                self.resupply_remix_variant();
            }
            "remix.track" => self.settings.remix.track = text,
            "remix.team" => {
                self.settings.remix.team = text;
                self.resupply_remix_variant();
            }
            "remix.variant" => self.settings.remix.variant = text,
            // **Applied immediately now, not deferred to the next boot** -
            // this row used to be a bare settings write, and `Self::apply_window`'s
            // own doc comment named it beside anisotropy as the two rows a
            // player would see nothing happen for. `Session::resupply_language`
            // is the reload: the disc's own string table and overlay, the
            // mode/team/circuit/front-end-style labels, the ticker and nav
            // legend, and the menu's own faces, without losing where the
            // player is standing in the tree. See that function's own doc for
            // what it deliberately still leaves for the next boot.
            "language" => {
                self.settings.language = Some(text);
                self.resupply_language();
            }
            // The AI PILOTS page's own four rows - see `super::pilot_editor`.
            // None of the four is part of `self.settings`: which pilot and
            // which axis are on screen is not persisted, so both return
            // before the save below rather than writing an unrelated file on
            // every LOW/HIGH nudge. PILOT and AXIS additionally re-supply
            // LOW/HIGH for whatever they just moved onto.
            "pilot.selected" | "pilot.axis" => {
                self.resupply_pilot_bounds();
                return;
            }
            "pilot.low" | "pilot.high" => return,
            other => {
                warn!("nothing applies {other}");
                return;
            }
        }
        if let Err(e) = settings::save(&self.settings) {
            error!("could not save settings: {e:#}");
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
            error!("could not save settings: {e:#}");
        }
    }

    /// Starts capturing a new key for the CONTROLS page's selected row, if it
    /// is a binding row and the player just confirmed it.
    ///
    /// Consumes Cross or Start directly off the shared `Input` rather than
    /// going through `Menu::update` - the same pattern `Keyboard::buttons_mut`'s
    /// own doc comment describes for the front end's skip-intro press - so
    /// `Entry::Binding` can stay the harmless `Menu::activate` no-op it always
    /// was. Does nothing while a capture is already in flight, so holding
    /// Cross down through the capture cannot immediately restart it.
    ///
    /// **Releases every key the moment a capture opens.** `app.rs` diverts
    /// every keyboard event away from `Controls::set_key` while a capture is
    /// live, so the eventual release of whatever key confirmed this row - and
    /// of anything else a player happened to be holding - would otherwise
    /// never reach `Keyboard`, exactly the stuck-held-forever failure
    /// `Keyboard::release_all`'s own doc comment describes for a focus loss.
    /// Releasing here means that swallowed key-up lands on an already-clear
    /// bit instead.
    ///
    /// Takes `awaiting_binding` and `controls` apart from `self` rather than
    /// as a `&mut self` method: `Session::frame` calls this from inside a
    /// `match &mut self.stage` arm, and a `&mut self` receiver there would
    /// borrow a field the match already holds.
    pub(crate) fn maybe_begin_binding(
        awaiting_binding: &mut Option<input::Button>,
        controls: &mut Controls,
        menu: &menu::Menu,
    ) {
        if awaiting_binding.is_some() {
            return;
        }
        let Some(menu::Entry::Binding { button, .. }) = menu.page().entries.get(menu.selected())
        else {
            return;
        };
        let button = *button;
        let taken = controls.buttons_mut();
        if taken.take(input::Button::Cross) || taken.take(input::Button::Start) {
            *awaiting_binding = Some(button);
            controls.release_all();
        }
    }

    /// Applies a captured rebind and persists it.
    ///
    /// Written on the keypress that made it, not on the way out - the same
    /// rule every other setting in this file follows and the same reason:
    /// there is no way out guaranteed to run.
    pub(crate) fn rebind(&mut self, button: input::Button, name: &'static str) {
        self.controls.bindings_mut().rebind(button, name);
        self.settings.controls.bindings = self.controls.bindings().to_pairs();
        self.awaiting_binding = None;
        if let Err(e) = settings::save(&self.settings) {
            error!("could not save settings: {e:#}");
        }
    }

    /// Stops capturing without changing anything - escape's own job while a
    /// binding is being captured, in place of backing out of the menu.
    pub(crate) fn cancel_binding(&mut self) {
        self.awaiting_binding = None;
    }
}
