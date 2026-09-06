//! Runs the boot sequence headless and writes one frame to a PNG.
//!
//! Three reasons this exists rather than being a debug convenience: it works over
//! SSH and in CI, it is how the boot sequence and the menu can be shown to
//! somebody without a display, and it goes through **the same** renderer the
//! window does, so what it captures is what the window draws. A separate capture
//! path would prove nothing.

use anyhow::{Context, Result};
use log::info;
use oag_render::mesh_render::Anisotropy;

use crate::boot::Boot;
use crate::frontend::Draw;
use crate::input::Input;
use crate::race;
use crate::render::{Renderer, VideoFormat};

mod menu_page;
use menu_page::menu_page;

/// What to capture.
#[derive(Debug, Clone)]
pub struct Options {
    /// Where to write the PNG.
    pub path: std::path::PathBuf,
    /// Capture the frame the way a window presents it - through the render
    /// scale, the upscaler, the grade and the aspect bars.
    ///
    /// Only reaches the race hand-off today. The front end's own capture path
    /// has no `Framebuffer` at all, and since
    /// [ADR-0038](../../../docs/architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md)
    /// that is correct rather than a gap: the front end has no 3D scene, so
    /// there is no render scale to apply and no upscaler that would ever run
    /// there. What a `--presented` front-end capture still misses is the grade
    /// and the aspect bars, which a windowed front-end frame does get from
    /// `Framebuffer::composite`.
    pub presented: bool,
    /// Run until this state is current, then capture.
    pub until: Option<String>,
    /// Run at least this many ticks first.
    ///
    /// A run that reaches `Launch Game` with a race to hand off to spends what is
    /// left of them on the race instead.
    pub ticks: u32,
    /// Pins the animation clock, in seconds, instead of deriving it from the
    /// tick. `None` derives it. See `Cli::anim_seconds`.
    pub anim_seconds: Option<f32>,
    /// Offset the camera by a sub-pixel each frame, on the race this hands off
    /// to. `--camera-jitter`; see `race::CaptureOptions::camera_jitter`.
    ///
    /// Carried rather than dropped because `--screenshot` reaching a race
    /// through `Launch Game` is one of the two ways a capture gets a race at
    /// all, and a flag honoured on one route and silently ignored on the other
    /// is how a comparison ends up differing by something nobody named. It
    /// reaches nothing on the front end itself: no stage there draws a 3D
    /// scene, per ADR-0038.
    pub camera_jitter: bool,
    /// Buttons held on every tick.
    pub held: u32,
    /// Buttons pressed and released on alternating ticks.
    ///
    /// A held button only produces one rising edge, so reaching a state that
    /// needs two presses - skip the intro, then pick a language - needs the
    /// button to be let go of in between.
    pub pressed: u32,
    /// Which control scheme the race this hands off to is driven with.
    pub scheme: oag_gameplay::ControlScheme,
    /// Print exits as well as entries.
    pub trace: bool,
    /// The race `Launch Game` hands off to, if this capture should follow it
    /// there.
    ///
    /// `None` captures the front end and nothing else, which is what a run that
    /// never reaches `Launch Game` does anyway.
    pub race: Option<race::Options>,
    /// With that handoff, print a telemetry line every this many ticks.
    pub log_every: u32,
    /// `--give`: keep the player's pickup slot topped up. See
    /// `race::CaptureOptions::give`.
    pub give: Option<oag_formats::weapons::Weapon>,
    /// `--autopilot`: fly the race this hands off to with an opponent's
    /// driver. See `race::CaptureOptions::autopilot`.
    pub autopilot: bool,
    /// Image size.
    pub size: (u32, u32),
    /// Render one named screen straight out of the XML and stop.
    ///
    /// A debugging view, not a step of the sequence: it does not run the state
    /// machine, take input or advance the movie. It exists so a screen the boot
    /// order does not reach yet - most of them - can still be looked at. See
    /// [`crate::frontend::Frontend::draw_screen`].
    pub screen: Option<String>,
    /// Anisotropic filtering level, only relevant if the handoff to
    /// [`Options::race`] happens.
    pub anisotropy: Anisotropy,
    /// Draw one page of **our own** menus instead of the sequence.
    ///
    /// The same kind of debugging view [`Options::screen`] is, for the other
    /// tree: a page id from `assets/ui/menu.toml`. It exists so a menu can be
    /// looked at without launching the game and walking to it, which is what
    /// iterating on a layout otherwise costs. Takes no input and runs no state
    /// machine.
    pub menu_page: Option<String>,
    /// How far into arriving that page is, `0..=1`. `None` draws it settled.
    ///
    /// See the CLI flag's own docs for why a still needs this at all.
    pub menu_anim_phase: Option<f32>,
    /// Which modal prompt to draw over that page: `rename`, `rename-note`,
    /// `delete` or `delete-built-in`.
    ///
    /// The same argument [`Self::menu_anim_phase`] makes, one step stronger. A
    /// prompt exists because a row was *activated*, and this path runs no
    /// state machine and calls no `Menu::update` - so the on-screen keyboard
    /// can never appear here on its own, and without this its layout is
    /// reviewable only by playing the game on a machine with a display.
    pub menu_prompt: Option<String>,
    /// The persisted settings, so `--menu-page` draws the rows a player would
    /// see rather than each list's first entry.
    pub settings: crate::settings::Settings,
    /// Which Pulse releases this machine has, so the MUSIC SOURCE row draws
    /// the same way it would in a live menu: with three values on a machine
    /// that has both discs, and empty on one that does not.
    pub music_discs: crate::audio::MusicDiscs,
}

/// How many ticks the runner will take before giving up on `until`.
///
/// The boot movie is forty seconds, and the `--reel` leg is eight plus three
/// two-second holds, so a minute of simulated time covers either and is still
/// bounded.
const MAX_TICKS: u32 = 60 * 60;

/// Runs the sequence and writes one frame.
///
/// `audio` is stepped once per simulation tick, in the same loop the front end
/// is stepped in. It is threaded through rather than made here because the
/// handoff to [`race::capture`] continues into the *same* buffer: a
/// `--dump-audio` run that reaches `Launch Game` would otherwise lose whichever
/// leg made its own. The caller writes the file once, after both.
pub fn run(
    loaded: Boot,
    video_format: Option<VideoFormat>,
    options: &Options,
    audio: &mut crate::audio::Audio,
) -> Result<()> {
    let Boot {
        title,
        languages,
        strings,
        tracks,
        teams,
        mut frontend,
        movie,
        movie_sound,
        after_language_movie,
        after_language_movie_sound,
        backdrop,
        font,
        sprites,
        menu_skin,
        // Renamed on the way in: `frame` is taken in this function by the
        // *movie* frame a backdrop is showing, and two things called `frame` in
        // one scope is how the wrong one gets passed.
        frame: menu_frame,
        menu_font,
        ..
    } = loaded;

    // Checked before anything is printed or stepped: a name that matches
    // nothing would otherwise draw the backdrop and nothing else, which looks
    // exactly like a screen that is empty. The list of what does exist is short
    // enough to just print.
    if let Some(name) = &options.screen
        && !frontend
            .screens()
            .screens
            .iter()
            .any(|s| s.name == *name || s.path == *name)
    {
        anyhow::bail!(
            "no screen named {name:?}; this XML has {}",
            frontend
                .screens()
                .screens
                .iter()
                .map(|s| format!("{:?}", s.path))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let dt = 1.0 / 60.0;
    let mut input = Input::new();
    let mut ticks = 0u32;

    // Each movie's sound keyed to the screen that plays it, the same way
    // `App::tick` installs them - kept in step with that loop deliberately, since
    // a movie sounding on one path and not the other is the divergence
    // `movie_playhead`'s own doc comment warns about.
    //
    // The boot screen gets no `Enter` event, being where the machine starts, so
    // its own movie's sound (if it has one) starts here. On Pulse that is the
    // intro; on Pure the boot screen is the picker and plays nothing.
    let movie_states = frontend.movie_states();
    let mut pending_sound: Vec<(&'static str, Option<crate::at3::Pcm>)> = Vec::new();
    for (at, sound) in [(0usize, movie_sound), (1, after_language_movie_sound)] {
        if let Some(state) = movie_states.get(at).copied() {
            pending_sound.push((state, sound));
        }
    }
    if let Some(current) = frontend.machine().current().map(str::to_string)
        && let Some(at) = pending_sound
            .iter()
            .position(|(state, _)| *state == current)
    {
        let (state, sound) = pending_sound.remove(at);
        audio.start_boot_movie(state, sound);
    }

    // Those two draw one thing and step nothing, so no movie ever ends to hand
    // the music its cue the way the loop below does. A `--dump-audio` run of
    // either would otherwise be silence.
    if options.screen.is_some() || options.menu_page.is_some() {
        audio.start_music(
            &options.music_discs,
            options.settings.audio.music_source,
            &crate::boot::default_audio_cache_dir(),
        );
    }

    // `--screen` and `--menu-page` both draw one thing and nothing else, so the
    // sequence is not run at all: stepping it would only move the state machine
    // somewhere the capture then ignores.
    while options.screen.is_none() && options.menu_page.is_none() {
        // `Launch Game` ends the front end's leg whatever `until` and `ticks` say,
        // so the ticks they asked for are spent on the race rather than on a state
        // whose whole content is the word LAUNCH GAME.
        if options.race.is_some() && frontend.is_finished() {
            break;
        }

        let reached = options
            .until
            .as_deref()
            .is_some_and(|name| frontend.machine().is(name));
        if reached && ticks >= options.ticks {
            break;
        }
        if options.until.is_none() && ticks >= options.ticks {
            break;
        }
        if ticks >= MAX_TICKS {
            if let Some(name) = &options.until {
                anyhow::bail!(
                    "never reached {name:?} in {MAX_TICKS} ticks; got as far as {:?}",
                    frontend.machine().current().unwrap_or("nothing")
                );
            }
            break;
        }

        let pulse = if ticks.is_multiple_of(2) {
            options.pressed
        } else {
            0
        };
        input.begin_frame(options.held | pulse);
        // The playhead is read **before** the mixer is advanced, so it is where
        // the sound had got to at the end of the previous tick - the last
        // moment it is a measurement rather than a prediction. See
        // `movie::Player::follow`.
        let events = frontend.update(dt, &mut input, audio.movie_playhead());
        // Every entered screen offered its own movie's track, the same handover
        // `App::tick` does.
        for event in &events {
            if let crate::state_machine::Event::Enter(name) = event
                && let Some(at) = pending_sound.iter().position(|(state, _)| state == name)
            {
                let (state, sound) = pending_sound.remove(at);
                audio.start_boot_movie(state, sound);
            }
        }
        // In the tick loop, next to the state machine it belongs to. See
        // [`crate::audio`] for why nothing here is per frame.
        audio.tick();
        // The movie's sound outlives neither leg: see
        // `Frontend::is_playing_movie` for why the state is what is asked
        // rather than the player.
        if !frontend.is_playing_movie() {
            audio.stop_movie();
        }
        // The menu music's cue is **no movies left**, kept in step with
        // `App::tick`'s own: "not in a movie" is true before a picker-first
        // title's reel has played at all, and starting the loop there puts it
        // under the reel.
        if !frontend.is_playing_movie() && pending_sound.is_empty() {
            audio.start_music(
                &options.music_discs,
                options.settings.audio.music_source,
                &crate::boot::default_audio_cache_dir(),
            );
        }
        crate::report(&events, options.trace);
        for note in frontend.take_notes() {
            info!("{note}");
        }
        ticks += 1;
    }

    match &options.screen {
        Some(name) => println!("drawing screen {name:?} from the XML, sequence not run"),
        None => println!(
            "after {ticks} tick(s), state {:?}",
            frontend.machine().current().unwrap_or("nothing")
        ),
    }

    let (width, height) = options.size;

    // The handoff, and the only place a capture leaves the front end. What it
    // writes is a race frame drawn through the same scene the window draws, for
    // the same reason the rest of this file goes through the front end's own
    // renderer.
    if frontend.is_finished()
        && let Some(race_options) = &options.race
    {
        let loaded = race::load(race_options)?;
        for line in &loaded.report {
            info!("{line}");
        }
        // The same handoff `App::launch_race` makes: the menu voice this loop
        // started above stops, and the race playlist takes over - one music
        // rule for every way a race is reached, screenshot captures included.
        audio.start_race_music(
            &options.music_discs,
            options.settings.audio.music_source,
            &crate::boot::default_audio_cache_dir(),
        );
        // The five render-profile settings, resolved against this title -
        // see `crate::settings::RenderProfile`. Read once rather than inline
        // below, since `anti_aliasing` is needed twice: once for the scene
        // pipeline and once for `Presentation`'s own resolve pass.
        let render_profile = options
            .settings
            .render_profiles
            .get(title.name)
            .cloned()
            .unwrap_or_default();
        return race::capture(
            loaded,
            &race::CaptureOptions {
                aspect: options.settings.display.aspect,
                path: options.path.clone(),
                ticks: options.ticks.saturating_sub(ticks),
                held: options.held,
                pressed: options.pressed,
                input_script: None,
                autopilot: options.autopilot,
                scheme: options.scheme,
                size: (width, height),
                log_every: options.log_every,
                give: options.give,
                anisotropy: options.anisotropy,
                renderer: options.settings.graphics.renderer.clone(),
                fov: options.settings.graphics.fov,
                frustum_culling: options.settings.graphics.frustum_culling,
                pvs_culling: options.settings.graphics.pvs_culling,
                anim_seconds: options.anim_seconds,
                camera_jitter: options.camera_jitter,
                bloom: options.settings.graphics.bloom,
                boost_fov_kick: options.settings.graphics.boost_fov_kick,
                camera_view: options.settings.graphics.camera_view,
                msaa: render_profile.msaa,
                motion_blur: render_profile.motion_blur,
                shadows: render_profile.shadows,
                // The front-end capture path never poses a ship, so there is
                // nothing for a forced boost state to be aged relative to.
                pose_boost: None,
                pose_intensity: None,
                pose_speed: None,
                presented: options.presented.then_some(race::Presented {
                    render_scale: render_profile.render_scale,
                    presentation: crate::upscale::Presentation {
                        reconstruction: render_profile.reconstruction,
                        sharpness: render_profile.upscale_sharpness.stops(),
                        brightness: options.settings.display.brightness,
                        gamma: options.settings.display.gamma,
                    },
                }),
                // This path has no CLI flag of its own for it, the same gap
                // `zone_stage` already has here - see `main/headless.rs` for
                // the one capture path that does.
                zone_spectrum_test: false,
            },
            audio,
        );
    }

    // **`--menu-page`'s picture comes off a different movie than the sequence's
    // does.** A menu page's `Draw::Video` is the disc's looping menu backdrop,
    // not the intro reel, so the movie the frame is read from and the plane
    // geometry the pipeline is built for both have to be the backdrop's. Getting
    // that wrong is not a compile error and not a crash: it reads a frame of the
    // intro into planes sized for the backdrop, and the flag quietly stops
    // showing what a player would see - which is exactly what this module exists
    // not to do.
    //
    // Frame zero and not a moving one: a capture is one picture, and there is
    // nothing here for a playhead to be advanced by.
    // The grid the chosen list's rects are in - the source's, on all three
    // paths. `--menu-page` used to pin `Space::PSP` here on the grounds that
    // this project's menu layout is authored at 480x272; that is still where the
    // layout is *written*, but `menu::Skin` now scales it into the source's grid
    // so that the disc's own `FEGlobals` and its own faces are drawn at the
    // numbers the disc states. A capture that got this wrong would disagree with
    // the window, which is the divergence this module exists to prevent - so
    // this reads the same value `Session::open_menus` hands its renderer.
    let space = frontend.space();
    let (mut movie, video_format, list, space) = match (&options.menu_page, &options.screen) {
        (Some(page), _) => {
            let showing = backdrop.as_ref().filter(|movie| movie.frames.is_some());
            let frame = showing.map(|movie| crate::menu::Backdrop {
                rect: crate::frontend::pillarbox_in(space, movie.display_aspect),
                frame: 0,
                // Position zero with the frame, there being no playhead here to
                // have got anywhere: a capture reads the frame straight out of
                // the `FrameStore` rather than off a `Feed`.
                position: 0,
            });
            let format = showing.and_then(VideoFormat::of);
            let list = menu_page(
                &options.settings,
                options.anisotropy,
                title.name,
                page,
                &tracks,
                &teams,
                &languages,
                &strings,
                &options.music_discs,
                frame,
                // The face the rows are drawn in, not the front end's
                // default: the pitch comes off its line height, so reading
                // the wrong one spaces the rows for a font nothing draws.
                &crate::menu::Skin::new(
                    menu_skin,
                    space,
                    menu_font.as_ref().unwrap_or(&font).line_height,
                ),
                &|text| crate::font::measure(menu_font.as_ref().unwrap_or(&font), text),
                &menu_frame,
                options.menu_anim_phase,
                options.menu_prompt.as_deref(),
            )?;
            (backdrop, format, list, space)
        }
        (None, Some(name)) => {
            let list = frontend.draw_screen(name);
            (movie, video_format, list, frontend.space())
        }
        (None, None) => {
            let list = frontend.draw_list();
            // **The sequence itself now has the same split.** Its last screen,
            // `Show Logo`, sits on the looping backdrop rather than on the
            // intro - so a capture that stops there has to read its frame from
            // the backdrop and build its planes for the backdrop, exactly as
            // `--menu-page` does. The draw says which movie it means, so this
            // follows the list rather than guessing from the state.
            match video_source(&list) {
                Some(crate::frontend::Video::Backdrop) => {
                    let showing = backdrop.as_ref().filter(|movie| movie.frames.is_some());
                    let format = showing.and_then(VideoFormat::of);
                    (backdrop, format, list, frontend.space())
                }
                // **`Video::Intro` names two different movies over a boot.** The
                // first one, and - on a title whose chain has two - the second.
                // The draw cannot say which, the variant being reused rather
                // than a third added, so the state picks, exactly as
                // `App::tick`'s own feed swap does. Without this the capture
                // reads the *first* movie's frame at the *second* movie's
                // playhead: the wrong picture, silently, which is the class of
                // mistake this module's own docs above exist to rule out.
                //
                // **Asked as "is this the chain's second movie screen?"**, which
                // is the same question `boot::assemble` answers when it hands
                // each step its plan, and so cannot disagree with it.
                //
                // It used to ask whether the state was the one the picker
                // confirms into, and that was **never the same question** -
                // `language_confirm_target` is the step *after* the picker,
                // which on Pure is `Developer Publisher Screen`, the screen that
                // plays movie **zero**. So the old form selected the second
                // movie for the first movie's screen. Verified against the disc
                // rather than reasoned about: a capture of Pure's dev/pub screen
                // is byte-for-byte identical either way, both cuts being 480x272
                // and that screen's parent fills covering the frame at the tick
                // this stops on - which is exactly why it survived.
                //
                // On Wipeout HD it does not survive. Its chain has one movie and
                // its `Studio Logo` *is* the after-language screen, so the old
                // form handed this path `after_language_movie`, which is `None`
                // there: the renderer was built with no video pipeline and every
                // capture of the logo reel came out black with the frame counter
                // drawn over it.
                Some(crate::frontend::Video::Intro)
                    if frontend
                        .machine()
                        .current()
                        .is_some_and(|state| frontend.movie_states().get(1) == Some(&state)) =>
                {
                    let showing = after_language_movie
                        .as_ref()
                        .filter(|movie| movie.frames.is_some());
                    let format = showing.and_then(VideoFormat::of);
                    (after_language_movie, format, list, frontend.space())
                }
                _ => (movie, video_format, list, frontend.space()),
            }
        }
    };

    // The same adapter the window would have drawn with, so a screenshot is a
    // picture of what a player sees rather than of whatever wgpu picked here.
    // No surface to be compatible with, which is the only difference.
    let instance = crate::adapter::instance();
    let adapter =
        crate::adapter::choose(&instance, None, &options.settings.graphics.renderer)?.adapter;
    let (device, queue) = pollster::block_on(adapter.request_device(
        &oag_render::mesh_render::device_descriptor("oag-game offscreen", &adapter),
    ))
    .context("requesting the device")?;

    // Rgba8Unorm rather than the surface's sRGB format: the readback is written
    // straight into a PNG, so a second gamma encode would double-correct.
    let format = wgpu::TextureFormat::Rgba8Unorm;
    // Menu pages draw in the title's own menu face; every other capture in
    // this path - a boot screen, the picker - draws in the default one. The
    // face has to match the one whose line height set the row pitch above,
    // or the rows are spaced for a font nothing draws them in.
    let face = if options.menu_page.is_some() {
        menu_font.clone().unwrap_or_else(|| font.clone())
    } else {
        font.clone()
    };
    let mut renderer = Renderer::new(&device, &queue, format, video_format, face, &sprites)?;
    renderer.set_space(space);

    if let (Some(frames), Some(wanted)) = (
        movie.as_mut().and_then(|movie| movie.frames.as_mut()),
        video_frame(&list),
    ) {
        let mut picture = crate::movie::VideoFrame::default();
        frames.read_frame(wanted.min(frames.len - 1), &mut picture)?;
        renderer.upload_frame(&queue, &picture)?;
    }

    let target = offscreen(&device, format, width, height);
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("capture"),
    });
    // Shaped the same way a window is, for the same reason the race capture is:
    // a screenshot should frame what a player would have seen at that size. At
    // the default `--size`, which is the PSP's own shape, every aspect fills the
    // frame and nothing changes.
    renderer.render(
        &device,
        &queue,
        &mut encoder,
        &view,
        &list,
        crate::display::viewport((width, height), options.settings.display.aspect),
        // A capture is one static frame with no `MenuStage` clock behind it,
        // so there is nothing here for a value marquee to be mid-scroll of.
        None,
    );
    let pixels = read_back(&device, &queue, encoder, &target, width, height)?;
    write_png(&options.path, width, height, &pixels)
}

/// The texture a headless capture draws into.
///
/// `Rgba8Unorm` at both call sites rather than the surface's sRGB format: the
/// readback is written straight into a PNG, so a second gamma encode would
/// double-correct.
fn offscreen(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("capture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// Submits `encoder`, then copies the drawn texture back as tightly packed RGBA.
///
/// Copies out of a texture must have rows aligned to 256 bytes, so the readback
/// buffer is usually wider than the image and needs unpadding.
fn read_back(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut encoder: wgpu::CommandEncoder,
    target: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<Vec<u8>> {
    let unpadded = width as usize * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded = unpadded.div_ceil(align) * align;

    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * height as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .context("waiting for the GPU")?;

    let mapped = slice
        .get_mapped_range()
        .context("mapping the readback buffer")?;
    let mut pixels = Vec::with_capacity(unpadded * height as usize);
    for row in mapped.chunks(padded).take(height as usize) {
        pixels.extend_from_slice(&row[..unpadded]);
    }
    drop(mapped);
    readback.unmap();
    Ok(pixels)
}

fn write_png(path: &std::path::Path, width: u32, height: u32, pixels: &[u8]) -> Result<()> {
    // A screenshot is opaque. The frame's alpha channel is the bloom mask and
    // not coverage - see `oag_render::capture::make_opaque` - so encoding it
    // straight from the readback writes a fully transparent PNG.
    let mut pixels = pixels.to_vec();
    oag_render::capture::make_opaque(&mut pixels);
    let png = oag_formats::png::encode_rgba(width, height, &pixels);
    std::fs::write(path, png).with_context(|| format!("writing {}", path.display()))?;
    println!("wrote {} ({width}x{height})", path.display());
    Ok(())
}

/// What `--loading-screen` draws.
#[derive(Debug, Clone)]
pub struct LoadingOptions {
    /// Where to write the PNG.
    pub path: std::path::PathBuf,
    /// Image size.
    pub size: (u32, u32),
    /// How many frames the screen has been up.
    ///
    /// Both the wave's phase and the tip on show are functions of this, so it
    /// is how a capture reaches a particular beat of the heartbeat: the
    /// envelope peaks at 5 and 11 of its 24 frames, and phase 0 idles at the
    /// amplitude floor.
    pub ticks: u32,
    /// The conversion state to draw.
    ///
    /// **Stated rather than observed, and that is the flag's whole reason for
    /// existing.** A warm cache reaches a real mid-conversion state for a
    /// fraction of a second and a cold one takes ten minutes to leave it, so
    /// neither is a way to look at the screen. What is drawn from here is the
    /// window's own [`crate::loading::Screen::draw_list`] with a known input,
    /// not a second layout.
    pub progress: crate::prefetch::Progress,
    /// Which of the screen's two waits to draw, and what the current load is
    /// doing - `--loading-step`. Stated for the same reason `progress` is.
    pub phase: crate::loading::Phase,
    /// Which adapter to draw with, from `[graphics] renderer`.
    pub renderer: crate::display::Renderer,
    /// The shape the game is drawn in, from `[display] aspect`.
    pub aspect: crate::display::Aspect,
}

/// Draws the loading screen once and writes it, without a window or a worker.
///
/// The same two passes the window makes, in the same order: the UI list clears
/// the frame and draws the text, and the wave goes over it additively with no
/// depth attachment. A capture that composited them differently would prove
/// nothing about what a player sees, which is the rule the rest of this module
/// follows.
///
/// # Errors
///
/// Propagates the adapter, the device and the file. Also fails when a screen
/// that **has** a wave produced no geometry for it:
/// [`oag_render::loading::Pipeline::draw`] draws nothing at all when nothing
/// was uploaded, which would otherwise write a perfectly plausible
/// text-on-black PNG with no error anywhere.
///
/// **A title that authors no wave is not that failure**, and telling the two
/// apart is why the check reads [`crate::loading::Screen::has_wave`] rather
/// than the vertex count alone. Wipeout HD's screen is a full-screen still and
/// a caption with no wave at all - see `docs/formats/hd-loading.md` - so an
/// empty upload there is the correct picture rather than a missing one, and
/// refusing it refused the very screen this capture exists to look at.
pub fn loading(
    assets: &crate::loading::Assets,
    font: crate::font::Atlas,
    sprites: &crate::sprite::Sheet,
    options: &LoadingOptions,
) -> Result<()> {
    // Seed 0: a capture has to be reproducible, and which feature it draws is
    // part of the picture. See `loading::Screen::new`. Language `None`: this
    // CLI capture has no `Settings` to read one from, so it takes the same
    // English fallback `crate::strings::project_table` gives any other
    // caller with nothing to name.
    let mut screen = crate::loading::Screen::new(assets, font.line_height, 0, None);
    // Stepped rather than jumped to: the tip rotation counts frames, and the
    // wave draws from its own `Rng` on every one of them, so frame `n` is only
    // reachable by having drawn the `n - 1` before it.
    let mut quads = screen.quads();
    for _ in 0..options.ticks {
        screen.advance(options.progress.finished);
        quads = screen.quads();
    }
    let vertices = oag_render::loading::vertices(&quads);
    anyhow::ensure!(
        !vertices.is_empty() || !screen.has_wave(),
        "the wave produced no geometry, so there would be nothing to draw"
    );

    let (width, height) = options.size;
    let instance = crate::adapter::instance();
    let adapter = crate::adapter::choose(&instance, None, &options.renderer)?.adapter;
    let (device, queue) = pollster::block_on(adapter.request_device(
        &oag_render::mesh_render::device_descriptor("oag-game loading screen", &adapter),
    ))
    .context("requesting the device")?;

    let format = wgpu::TextureFormat::Rgba8Unorm;
    // Kept before the renderer takes it: the layout measures its own wrapping
    // and eliding through the atlas that will draw it, and `Renderer` owns
    // rather than borrows one.
    let atlas = font.clone();
    // The feature illustration's own sheet where the title ships one, exactly
    // as `Stage::loading` and `Stage::race_loading` choose - `Draw::Sprite`
    // addresses whichever sheet the renderer was built with, so a capture given
    // the front end's would draw the loading screen's picture from the wrong
    // atlas and silently miss it.
    let sprites = assets.art.as_ref().map_or(sprites, |art| &art.sheet);
    let mut renderer = Renderer::new(&device, &queue, format, None, font, sprites)?;
    // Sample count 1, matching `upscale::Framebuffer`'s own target and this
    // capture's texture. The wave's pipeline bakes it in, so a mismatch here is
    // a validation error rather than a soft failure.
    let mut wave = oag_render::loading::Pipeline::new(&device, &queue, format, &assets.strip, 1);
    wave.upload(&queue, [1.0, 1.0, 1.0, screen.opacity()], &vertices);

    let target = offscreen(&device, format, width, height);
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let viewport = crate::display::viewport((width, height), options.aspect);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("loading screen"),
    });
    renderer.render(
        &device,
        &queue,
        &mut encoder,
        &view,
        &screen.draw_list(options.phase, &options.progress, &atlas),
        viewport,
        None,
    );
    draw_wave(&mut encoder, &view, &wave, viewport);

    let pixels = read_back(&device, &queue, encoder, &target, width, height)?;
    write_png(&options.path, width, height, &pixels)
}

/// Adds the wave's pass to `encoder`, over whatever is already in `view`.
///
/// One place rather than two so the window and the capture cannot drift apart,
/// and three things it has to get right:
///
/// - **Load, not clear**: the wave sits over the text the UI pass drew.
/// - **No depth attachment.** `oag_render::loading::Pipeline` is built with
///   `depth_stencil: None` and a pass that attached one would not match it.
/// - **The same viewport the UI pass used.** The wave's vertices are normalised
///   `0..1` with no idea where the game's rectangle is, so without this it
///   spans the whole surface while the text sits letterboxed inside it. Invisible
///   at the PSP's own aspect, where the two rectangles are the same.
pub fn draw_wave(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    wave: &oag_render::loading::Pipeline,
    viewport: (f32, f32, f32, f32),
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("loading wave"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_viewport(viewport.0, viewport.1, viewport.2, viewport.3, 0.0, 1.0);
    wave.draw(&mut pass);
}

fn video_frame(list: &[Draw]) -> Option<usize> {
    list.iter().find_map(|draw| match draw {
        Draw::Video { frame, .. } => Some(*frame),
        _ => None,
    })
}

/// Which movie the list's video draw wants a frame of, if it has one.
fn video_source(list: &[Draw]) -> Option<crate::frontend::Video> {
    list.iter().find_map(|draw| match draw {
        Draw::Video { source, .. } => Some(*source),
        _ => None,
    })
}
