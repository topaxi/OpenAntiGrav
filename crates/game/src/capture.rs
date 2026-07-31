//! Runs the boot sequence headless and writes one frame to a PNG.
//!
//! Three reasons this exists rather than being a debug convenience: it works over
//! SSH and in CI, it is how the boot sequence and the menu can be shown to
//! somebody without a display, and it goes through **the same** renderer the
//! window does, so what it captures is what the window draws. A separate capture
//! path would prove nothing.

use anyhow::{Context, Result};
use oag_render::mesh_render::Anisotropy;

use crate::boot::Boot;
use crate::frontend::Draw;
use crate::input::Input;
use crate::race;
use crate::render::{Renderer, VideoFormat};

/// What to capture.
#[derive(Debug, Clone)]
pub struct Options {
    /// Where to write the PNG.
    pub path: std::path::PathBuf,
    /// Capture the frame the way a window presents it - through the render
    /// scale, the upscaler, the grade and the aspect bars.
    ///
    /// Only reaches the race hand-off today. The front end's own capture path
    /// has no `Framebuffer` either, and giving it one is the same piece of work
    /// as the UI-compositing restructure.
    pub presented: bool,
    /// Run until this state is current, then capture.
    pub until: Option<String>,
    /// Run at least this many ticks first.
    ///
    /// A run that reaches `Launch Game` with a race to hand off to spends what is
    /// left of them on the race instead.
    pub ticks: u32,
    /// Buttons held on every tick.
    pub held: u32,
    /// Buttons pressed and released on alternating ticks.
    ///
    /// A held button only produces one rising edge, so reaching a state that
    /// needs two presses - skip the intro, then pick a language - needs the
    /// button to be let go of in between.
    pub pressed: u32,
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
    /// The persisted settings, so `--menu-page` draws the rows a player would
    /// see rather than each list's first entry.
    pub settings: crate::settings::Settings,
}

/// How many ticks the runner will take before giving up on `until`.
///
/// The boot movie is forty seconds, and the `--reel` leg is eight plus three
/// two-second holds, so a minute of simulated time covers either and is still
/// bounded.
const MAX_TICKS: u32 = 60 * 60;

/// Draws one page of our own menus, with the source's own lists supplied.
///
/// The lists matter even for a still: a circuit row with nothing in it and one
/// showing this disc's twenty-four circuits are different pictures, and the
/// point of the flag is to look at the real one.
#[allow(
    clippy::too_many_arguments,
    reason = "every one of these is a separate thing the page needs, and a struct \
              for one call site would name the grouping without clarifying it"
)]
fn menu_page(
    settings: &crate::settings::Settings,
    anisotropy: Anisotropy,
    page: &str,
    tracks: &[crate::catalogue::Track],
    languages: &[crate::language::Language],
    strings: &crate::language::StringTable,
    backdrop: Option<crate::menu::Backdrop>,
) -> Result<Vec<crate::frontend::Draw>> {
    let definition = crate::menu::Definition::parse(crate::menu::BUILT_IN)
        .context("parsing the built-in menu definition")?;
    if definition.page(page).is_none() {
        anyhow::bail!(
            "no menu page named {page:?}; this definition has {}",
            definition
                .pages
                .iter()
                .map(|p| format!("{:?}", p.id))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let mut model = crate::menu::Menu::new(definition);
    model.supply(
        crate::menu::ValueSource::Tracks,
        &tracks
            .iter()
            .map(|track| crate::menu::Choice::labelled(&track.id, strings.get_or_id(&track.id)))
            .collect::<Vec<_>>(),
    );
    model.supply(
        crate::menu::ValueSource::Languages,
        &languages
            .iter()
            .map(|language| crate::menu::Choice::labelled(&language.name, &language.native_name))
            .collect::<Vec<_>>(),
    );
    // No window here, so no screens to enumerate: the list is `default` plus
    // whatever the settings already name. That is enough for the row to draw
    // the player's own value, which is all `--menu-page` is for, and it does
    // not invent a monitor this machine may not have.
    model.supply(
        crate::menu::ValueSource::Monitors,
        &crate::display::Monitor::offered(
            &settings
                .display
                .monitor
                .name()
                .map(ToString::to_string)
                .into_iter()
                .collect::<Vec<_>>(),
        )
        .into_iter()
        .map(crate::menu::Choice::plain)
        .collect::<Vec<_>>(),
    );
    // Seeded after supplying, and from the same list the live menus use, so
    // what the flag draws is what a player would see rather than whatever each
    // row's list happened to start on.
    for (key, value) in crate::settings::menu_seeds(settings, anisotropy) {
        model.seed(key, &value);
    }
    model.open(page);
    Ok(crate::menu::draw_list(
        &model,
        &oag_input::keys::bound_keys,
        backdrop,
    ))
}

/// Runs the sequence and writes one frame.
pub fn run(loaded: Boot, video_format: Option<VideoFormat>, options: &Options) -> Result<()> {
    let Boot {
        languages,
        strings,
        tracks,
        mut frontend,
        movie,
        backdrop,
        font,
        sprites,
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
        let events = frontend.update(dt, &mut input);
        crate::report(&events, options.trace);
        for note in frontend.take_notes() {
            println!("{note}");
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
            println!("{line}");
        }
        return race::capture(
            loaded,
            &race::CaptureOptions {
                aspect: options.settings.display.aspect,
                path: options.path.clone(),
                ticks: options.ticks.saturating_sub(ticks),
                held: options.held,
                size: (width, height),
                log_every: options.log_every,
                anisotropy: options.anisotropy,
                fov: options.settings.graphics.fov,
                frustum_culling: options.settings.graphics.frustum_culling,
                pvs_culling: options.settings.graphics.pvs_culling,
                animated_textures: options.settings.graphics.animated_textures,
                presented: options.presented.then_some(race::Presented {
                    render_scale: options.settings.graphics.render_scale,
                    presentation: crate::upscale::Presentation {
                        upscaler: options.settings.graphics.upscaler,
                        sharpness: options.settings.graphics.upscale_sharpness,
                        brightness: options.settings.display.brightness,
                        gamma: options.settings.display.gamma,
                    },
                }),
            },
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
    let (mut movie, video_format, list) = match (&options.menu_page, &options.screen) {
        (Some(page), _) => {
            let showing = backdrop.as_ref().filter(|movie| movie.frames.is_some());
            let frame = showing.map(|movie| crate::menu::Backdrop {
                rect: crate::frontend::pillarbox(crate::frontend::SCREEN, movie.display_aspect),
                frame: 0,
            });
            let format = showing.and_then(VideoFormat::of);
            let list = menu_page(
                &options.settings,
                options.anisotropy,
                page,
                &tracks,
                &languages,
                &strings,
                frame,
            )?;
            (backdrop, format, list)
        }
        (None, Some(name)) => {
            let list = frontend.draw_screen(name);
            (movie, video_format, list)
        }
        (None, None) => {
            let list = frontend.draw_list();
            (movie, video_format, list)
        }
    };

    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    }))
    .context("no GPU adapter available")?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("oag-game offscreen"),
        ..Default::default()
    }))
    .context("requesting the device")?;

    // Rgba8Unorm rather than the surface's sRGB format: the readback is written
    // straight into a PNG, so a second gamma encode would double-correct.
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = Renderer::new(&device, &queue, format, video_format, font, &sprites)?;

    if let (Some(frames), Some(wanted)) = (
        movie.as_mut().and_then(|movie| movie.frames.as_mut()),
        video_frame(&list),
    ) {
        let mut bytes = Vec::new();
        frames.read_frame(wanted.min(frames.len - 1), &mut bytes)?;
        renderer.upload_frame(&queue, &bytes)?;
    }

    let target = device.create_texture(&wgpu::TextureDescriptor {
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
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());

    // Copies out of a texture must have rows aligned to 256 bytes, so the
    // readback buffer is usually wider than the image and needs unpadding.
    let unpadded = width as usize * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded = unpadded.div_ceil(align) * align;

    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * height as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

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
    );
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

    let png = oag_formats::png::encode_rgba(width, height, &pixels);
    std::fs::write(&options.path, png)
        .with_context(|| format!("writing {}", options.path.display()))?;
    println!("wrote {} ({width}x{height})", options.path.display());
    Ok(())
}

fn video_frame(list: &[Draw]) -> Option<usize> {
    list.iter().find_map(|draw| match draw {
        Draw::Video { frame, .. } => Some(*frame),
        _ => None,
    })
}
