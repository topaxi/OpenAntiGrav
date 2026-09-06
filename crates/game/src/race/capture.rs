//! The headless capture path: advance a race a fixed number of ticks, draw one
//! frame without a window, and write it out.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;
use log::warn;

/// What a headless capture should do before it draws.
#[derive(Debug, Clone)]
pub struct CaptureOptions {
    /// Where to write the PNG.
    pub path: std::path::PathBuf,
    /// Ticks to advance the simulation first.
    pub ticks: u32,
    /// Buttons held on every one of those ticks.
    pub held: u32,
    /// Drive the run from a committed `.inputs` script instead of `held`/
    /// `pressed` - the same file `scripts/psp-trace.py --script` feeds the
    /// emulator, so one authored input produces both sides of a visual
    /// comparison. Ticks past the script's end coast with everything
    /// released.
    pub input_script: Option<oag_trace::script::Script>,
    /// Fly the player's craft with an opponent's driver. `--autopilot`.
    ///
    /// **The only way a capture can reach a finished race**, and therefore the
    /// only way `--screenshot` can show the results table: the flag falls when
    /// the player crosses the line for the last time, and `--hold cross` drives
    /// into the first wall. See [`Race::set_autopilot`], which is a verification
    /// aid rather than a mode the original has.
    pub autopilot: bool,
    /// Which control scheme maps the buttons. `[controls] scheme`.
    ///
    /// Here rather than left at the default because the novice sideshift is a
    /// *gesture*, and a headless run is the only way to exercise one without a
    /// window: `--scheme novice --hold cross,l --press left` is the flick.
    pub scheme: ControlScheme,
    /// Buttons pressed and released on alternating ticks, for gestures that read
    /// an edge rather than a level.
    ///
    /// The same convention the front-end capture uses, so `--press` means one
    /// thing across the whole tool. A tap every other tick is well inside the
    /// veteran sideshift's `0.25 s` window, which makes `--press l` a double-tap
    /// generator.
    pub pressed: u32,
    /// Image size.
    pub size: (u32, u32),
    /// Print a telemetry line every this many ticks. Zero prints none.
    pub log_every: u32,
    /// Keep the player's pickup slot topped up with this weapon.
    ///
    /// **A debug affordance for captures, and the reason it exists is worth
    /// keeping.** A weapon is only visible once something fires it, and a
    /// capture holds the throttle without steering, so it never crosses a
    /// `Weapon Pad` and never receives a pickup. Every visual change to a
    /// projectile was therefore unverifiable from a screenshot - two rocket
    /// changes shipped blind before this existed, and one was wrong: three
    /// pieces of track scenery were read as a fanned volley. See
    /// [`Race::rocket_model_matrices`].
    ///
    /// Written **outside** [`Race::tick`], only when the slot is already empty,
    /// so firing still spends it and nothing here reaches a determinism hash.
    pub give: Option<oag_formats::weapons::Weapon>,
    /// The shape to draw at inside the frame, leaving bars.
    ///
    /// A capture is a picture of a window, so it letterboxes the way a window
    /// does. At the default `--size`, which is the PSP's own shape, every value
    /// of this fills the frame and nothing changes.
    pub aspect: crate::display::Aspect,
    /// Anisotropic filtering level for the track and ship textures.
    pub anisotropy: Anisotropy,
    /// Whether the recovered bloom runs - see `crate::settings::Graphics::bloom`,
    /// which defaults it **off** until its magnitude is calibrated.
    pub bloom: bool,
    /// Which adapter to draw with, for the same reason `aspect` and `fov` are
    /// here: a capture is only evidence about what a player sees if it was
    /// drawn on the device they see it on. A driver is exactly the kind of
    /// thing a rendering difference gets blamed on, so a capture that quietly
    /// used a different one would be the wrong picture to argue from.
    ///
    /// There is no surface here, so an adapter that could not present is still
    /// eligible - which is the one way this list can be wider than the menu's.
    pub renderer: crate::display::Renderer,
    /// The field-of-view setting, for the same reason `aspect` is here: a
    /// capture should frame what a player at these settings would have seen.
    pub fov: crate::display::Fov,
    /// Whether the view frustum culls before the frame is drawn.
    ///
    /// Honoured rather than forced off, so that the screenshot comparison this
    /// project already claims for `[graphics] frustum_culling` can actually be
    /// run from a capture, and so a report of geometry going missing can be
    /// attributed to a tier rather than guessed at.
    pub frustum_culling: bool,
    /// Whether the authored PVS culls before the frame is drawn.
    ///
    /// Here, and honoured, so that `--screenshot` with `[graphics] pvs_culling`
    /// on and off produces two images to compare. **That comparison is the only
    /// way to show the association rule in `oag_render::pvs` places geometry in
    /// the right sections rather than merely in some section**, and it is the
    /// bar that setting has to clear before it can default on - the same one
    /// frustum culling passed. Frustum culling stays off in a capture either
    /// way, so the two images differ by this tier alone.
    pub pvs_culling: bool,
    /// Pins the animation clock, in seconds, instead of deriving it from the
    /// tick.
    ///
    /// `None` in a race, and set by `--anim-seconds`. Two captures at two times
    /// are how an animated surface is shown to move at all headlessly, and a
    /// comparison against a still of the original wants our phase matched to
    /// theirs rather than left where the tick put it. The tick already makes a
    /// capture reproducible; this makes it *aimable*.
    pub anim_seconds: Option<f32>,
    /// How strong the boost's field-of-view kick is. Honoured for a sharper
    /// version of the same reason: the effect is **authored**, so a capture meant
    /// to be compared against the running original wants it at
    /// [`crate::display::BoostFovKick::OFF`], and that comparison is the only
    /// way anyone will find out whether the original has something like it.
    pub boost_fov_kick: crate::display::BoostFovKick,
    /// Which of the three perspectives to render from.
    ///
    /// Honoured because a headless capture is the **only** way to get a frame of
    /// the cockpit view without a window, and therefore the only way anyone
    /// checks it: `--camera-view internal --screenshot`. `[graphics] camera_view`.
    pub camera_view: crate::display::CameraView,
    /// How many samples the rasterizer takes. Honoured for the same reason
    /// the two culling tiers are: a capture is how `msaa` gets compared
    /// against itself off and against the running original.
    pub msaa: crate::display::Msaa,
    /// Offset the camera by a sub-pixel each frame. `--camera-jitter`.
    ///
    /// Honoured here because a capture is the only way to *see* that jitter is
    /// live at all: two `--screenshot` runs differing only by this flag differ
    /// by well under a pixel, which is invisible in a window and obvious in a
    /// byte diff. The phase is [`Scene`]'s own frame counter, which starts at
    /// zero and advances on every `Scene::render` - **including the primer
    /// render above and each `OAG_RENDER_BENCH` iteration**, so it is a
    /// function of the whole invocation rather than of the tick count alone.
    /// The same command is reproducible; the same tick count reached a
    /// different way is not necessarily the same phase. See
    /// [`oag_render::jitter`].
    pub camera_jitter: bool,
    /// Force the exhaust into the state it holds this many seconds after a
    /// speed pad entry, at saturated intensity, before the frame is drawn.
    ///
    /// `--pose-boost`. A posed capture (`--pose-from --ticks 0`) never crosses
    /// a pad, so this is the only way a frame comparison can see the boost
    /// visuals at a chosen age. The state is reached by replaying
    /// [`Exhaust::advance`] rather than by poking fields, so what is captured
    /// is the same trajectory a real crossing produces.
    pub pose_boost: Option<f32>,
    /// With [`Self::pose_boost`]: the intensity at the entry tick, instead of a
    /// saturated ramp. `--pose-intensity`.
    ///
    /// See [`Race::force_boost_state`] for why a saturated default is the wrong
    /// one to compare a teleported capture against.
    pub pose_intensity: Option<f32>,
    /// With [`Self::pose_boost`]: the speed in units/s to advance the exhaust
    /// at, instead of `120`. `--pose-speed`.
    pub pose_speed: Option<f32>,
    /// Capture the frame the way a **window** presents it, rather than the
    /// scene the way it is drawn.
    ///
    /// `None` is the ordinary capture: the scene, straight out of the target it
    /// was drawn into, ungraded, at exactly `size`. That is the right default
    /// for a bug report, and it is deliberately not a picture of a window - see
    /// [`crate::upscale`].
    ///
    /// `Some` puts the whole presentation path in the way: the render scale,
    /// the upscaler, the grade and the aspect bars. **This is the only way to
    /// see an upscaler's output at all**, because the ordinary path never
    /// reaches the blit, and it is therefore what a still-frame comparison
    /// between resamplers has to use. It is also, necessarily, an sRGB pipeline
    /// throughout, exactly as a window is.
    pub presented: Option<Presented>,
    /// How hard the frame is smeared along each surface's own motion.
    ///
    /// Honoured the way `anti_aliasing` and the culling tiers are - a capture
    /// is how a graphics setting gets compared against itself off - but this
    /// one changes the *shape* of the run to do it: every velocity is a delta
    /// against the previous simulation tick, so a one-render capture has no
    /// previous tick and measures zero everywhere. With this on, the capture
    /// holds the last tick back, renders a **primer** frame at the
    /// tick-before-last pose (drawn and discarded - its only product is the
    /// previous-transform cache it seeds), runs the final tick, and renders
    /// the frame that is written out. See `oag_render::post::motion_blur` and
    /// `docs/rendering/motion-blur.md`.
    pub motion_blur: crate::display::MotionBlur,
    /// What casts a shadow in the captured frame: `--shadows`.
    ///
    /// Honoured the same way [`Self::motion_blur`] is, and here for the same
    /// reason: two captures differing only by this flag are how the tier gets
    /// compared against itself off. Unlike the blur it needs no primer frame -
    /// a blob is placed from the tick's own pose, with no history.
    pub shadows: crate::display::Shadows,
    /// Replaces the live audio spectrum with a fixed synthetic ramp before
    /// the frame is drawn. `--zone-spectrum-test`.
    ///
    /// **Exists because the live spectrum otherwise makes a Zone capture
    /// useless as evidence twice over.** On the null backend `--screenshot`
    /// always runs on - `audio.output().spectrum()` never publishes, since no
    /// render-ahead thread ever spawns without a device - so an ordinary
    /// capture shows no glow at all, whatever the scene actually authors.
    /// With a real device attached instead, the spectrum is live and
    /// non-deterministic, which is the wrong kind of input for a comparison
    /// two runs are supposed to agree on. A fixed ramp is neither: visible on
    /// the null backend and identical on every run, on the same terms
    /// `--zone-stage` exists to make the colour grade itself checkable
    /// without a live race to drive it there.
    pub zone_spectrum_test: bool,
}

/// [`Presented`] with the scene size worked out.
#[derive(Debug, Clone, Copy)]
struct PresentedState {
    scene_size: (u32, u32),
    presentation: crate::upscale::Presentation,
}

/// The settings a `--presented` capture needs that an ordinary one does not.
#[derive(Debug, Clone, Copy)]
pub struct Presented {
    /// What fraction of the aspect rectangle the scene is drawn at.
    pub render_scale: crate::display::Scale,
    /// The upscaler, its sharpness, and the grade.
    pub presentation: crate::upscale::Presentation,
}

/// Runs a race headless and writes one frame to a PNG.
///
/// Through the same [`Scene`] the window draws, for the same reason
/// [`crate::capture`] goes through the same renderer the front end's window does: a
/// separate capture path would prove nothing about what a player sees.
///
/// `audio` is advanced one step per simulation tick, in the same loop, so a
/// `--dump-audio` capture is as long as the ticks it was given whatever the
/// machine's speed. Writing the WAV is the caller's, not this function's: a
/// capture reached from [`crate::capture::run`] has already accumulated the
/// front end's ticks into the same buffer, and finishing here would truncate
/// the file to the race leg.
///
/// # Errors
///
/// Propagates adapter and device creation, pipeline building, the readback map and
/// the file write.
pub fn capture(
    loaded: Loaded,
    options: &CaptureOptions,
    audio: &mut crate::audio::Audio,
) -> Result<()> {
    let (width, height) = options.size;
    let Loaded {
        shadows,
        shadow_hulls,
        setup,
        hud,
        track_model,
        liveries,
        collision_model,
        sky_model,
        pad_model,
        weapon_pad_model,
        gantry,
        rocket_model,
        mine_model,
        bomb_model,
        shield_cockpit,
        countdown_model,
        fog_volumes,
        light,
        authored_fog,
        hd_bloom,
        zone_grade,
        visibility,
        flare,
        noise,
        trail_blend,
        trail_shape,
        ..
    } = loaded;
    let mode = setup.mode;
    let mut race = Race::start(setup);
    race.set_boost_fov_kick(options.boost_fov_kick);
    race.set_sight_fov(options.fov);
    race.set_sight_screen(hud.space.size);
    race.set_camera_view(options.camera_view);
    race.set_control_scheme(options.scheme);
    race.set_autopilot(options.autopilot);

    let mut held = HeldButtons::new(options.held);
    // A motion blur capture holds the **last** tick back: every velocity is a
    // delta against the previous tick, so the primer frame below has to be
    // rendered at the tick-before-last pose before that tick runs. Every
    // other capture drives all its ticks here, exactly as before.
    let deferred_tick = (options.motion_blur != crate::display::MotionBlur::Off
        && options.ticks > 0)
        .then(|| options.ticks - 1);
    let driven = deferred_tick.unwrap_or(options.ticks);
    let mut finished_early = false;
    for tick in 0..driven {
        advance_one_tick(&mut race, &mut held, audio, options, tick);
        // The race ended inside the requested tick count, so the rest of it is
        // not driven - exactly as the window stops stepping a finished race, and
        // for the same reason: a board taken at the last crossing drawn over a
        // world that went on moving for another two thousand ticks would be a
        // picture of two different moments. Said out loud, because a capture
        // that quietly ran short would otherwise look like a lost tick.
        if race.finished() {
            println!(
                "the race finished on tick {} of the {} asked for; the rest are not driven",
                race.world.tick, options.ticks
            );
            finished_early = true;
            break;
        }
    }

    let instance = crate::adapter::instance();
    let adapter = crate::adapter::choose(&instance, None, &options.renderer)?.adapter;
    // Asked here because this is where this path's adapter is - the capture
    // builds its own rather than borrowing the window's. See
    // `upscale::Temporal`.
    let temporal_supported = oag_render::post::fsr3::supported(&adapter);
    let (device, queue) = pollster::block_on(adapter.request_device(
        &oag_render::mesh_render::device_descriptor("oag-game race offscreen", &adapter),
    ))
    .context("requesting the device")?;

    // **`Rgba8Unorm` on both paths now**, because every shader in this pipeline
    // writes gamma-space values and nothing may encode them again - see
    // [ADR-0020](../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md).
    //
    // `--presented` used to be `Rgba8UnormSrgb` on the grounds that it is a
    // picture of a *window* and should take the window's format. It still is,
    // and it still does: the window stopped encoding in the same change. That
    // the two agreed on the label and not on the value is what made a
    // `--screenshot` and a `--presented` capture of the same frame disagree
    // about the boost plume by up to 73/255, the plume being the one surface
    // whose texels were already re-encoded to compensate for the old upload.
    // The offscreen target's non-sRGB twin, and therefore FSR 1, still work -
    // `remove_srgb_suffix` on a format that has no suffix is the identity. See
    // `crate::upscale`.
    let format = wgpu::TextureFormat::Rgba8Unorm;
    // The scene's own size, which presented is the aspect rectangle scaled and
    // otherwise is the whole capture.
    let presented = options.presented.map(|state| PresentedState {
        scene_size: crate::upscale::target_size(
            crate::display::viewport((width, height), options.aspect),
            state.render_scale,
            device.limits().max_texture_dimension_2d,
        ),
        presentation: state.presentation,
    });
    // What the scene - and so its depth buffer - is actually drawn at. A depth
    // attachment whose size does not match the colour one is a validation
    // error, not a bad picture.
    let scene_size = presented.map_or((width, height), |state| state.scene_size);
    let mut scene = Scene::new(
        &device,
        &queue,
        track_model,
        &liveries,
        collision_model,
        sky_model,
        pad_model,
        weapon_pad_model,
        gantry,
        mode,
        rocket_model,
        mine_model,
        bomb_model,
        shield_cockpit,
        flare,
        noise,
        trail_blend,
        trail_shape,
        format,
        scene_size,
        options.anisotropy,
        options.bloom,
        visibility,
        options.msaa,
        fog_volumes,
        light,
        authored_fog,
        hd_bloom,
        zone_grade,
        shadows,
        shadow_hulls,
    )?;

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("race capture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        // `TEXTURE_BINDING` because the bloom's bright pass samples the frame
        // it was just drawn into - see `oag_render::post::bloom`.
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let surface = target.create_view(&wgpu::TextureViewDescriptor::default());
    // Where the scene is drawn. Presented, that is an offscreen target at the
    // render scale which the blit later stretches into the aspect rectangle -
    // the same two-step a window does. Otherwise it is the capture texture
    // itself, and the scene draws into a sub-rectangle of it directly.
    let mut framebuffer = match presented {
        Some(state) => Some(
            crate::upscale::Framebuffer::new(&device, format, state.scene_size)
                .context("building the upscale pipeline")?,
        ),
        None => None,
    };
    let view = match &framebuffer {
        Some(framebuffer) => framebuffer.view().clone(),
        None => surface.clone(),
    };

    // Texture copies want rows aligned to 256 bytes, so the readback buffer is
    // usually wider than the image and needs unpadding.
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
        label: Some("race capture"),
    });
    // Shaped the same way a window is, so a screenshot frames what a player
    // would have seen at that size rather than a differently-cropped picture.
    // Presented, the offscreen target *is* that rectangle and the bars are what
    // the blit clears around it, so the scene fills its target instead.
    let rect = crate::display::viewport((width, height), options.aspect);
    let viewport = match presented {
        Some(state) => (
            0.0,
            0.0,
            state.scene_size.0 as f32,
            state.scene_size.1 as f32,
        ),
        None => rect,
    };
    // Computed once for all three `Scene::render` calls below, so the primer,
    // the bench loop and the real frame cannot disagree about the sequence -
    // they already share `Scene`'s own frame counter, and a differing phase
    // count would desynchronise the offsets rather than the indices.
    //
    // **`Reconstruction::Off` on an ordinary capture, and that is correct
    // rather than a shortcut**: without `--presented` nothing resolves the
    // frame at all, so nothing is reconstructing and only the flag can ask for
    // jitter. See `upscale::jitter_phases`.
    let camera_jitter = crate::upscale::jitter_phases(
        options.camera_jitter,
        presented.map_or(crate::display::Reconstruction::Off, |state| {
            state.presentation.reconstruction
        }),
        temporal_supported,
        (viewport.2 as u32, viewport.3 as u32),
        (rect.2 as u32, rect.3 as u32),
    );

    // The primer frame a motion blur capture needs: the scene at the
    // tick-before-last pose, drawn into the same target and then entirely
    // overwritten by the real frame's clear. Every pixel it produces is
    // discarded; its whole product is CPU-side, `Scene::render` folding this
    // pose into `race::scene::frame::MotionState` so the real frame has a
    // previous tick to measure its `prev_mvp` velocities against. Without it
    // a one-render capture measures against itself and the velocity buffer
    // comes out zero everywhere.
    //
    // Its own encoder, submitted before the final frame's, because two
    // renders in one submission would interleave their `write_buffer`
    // uploads: a queue write lands before the submission's commands, so the
    // final frame's uniforms would reach both passes. That was already true
    // of the camera tier's one uniform buffer and is more so now - every
    // drawable's `prev_mvp` goes through the same queue.
    //
    // Then the held-back tick runs, so the frame written out is the state at
    // exactly `--ticks`, the same as a capture with the blur off.
    if let Some(tick) = deferred_tick
        && !finished_early
    {
        let mut primer = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("race capture primer"),
        });
        let spectrum = zone_spectrum(options, audio);
        scene.render(
            &device,
            &queue,
            &mut primer,
            &view,
            &race,
            viewport,
            options.fov,
            options.frustum_culling,
            options.pvs_culling,
            options.anim_seconds,
            options.motion_blur,
            options.shadows,
            camera_jitter,
            &spectrum,
            // **No timestamps on any capture path**, deliberately: this
            // project compares captures byte for byte, and a measurement is
            // the input a resolution controller would eventually act on. See
            // `oag_render::timing::PassTimer` and `perf.rs`'s own argument for
            // why the overlay is window-only. All three pairs: the scene
            // pass's, the motion-blur chain's, and the HD/Fury bloom chain's.
            None,
            None,
            None,
        );
        queue.submit(Some(primer.finish()));
        advance_one_tick(&mut race, &mut held, audio, options, tick);
    }
    if let Some(age) = options.pose_boost {
        race.force_boost_state(age, options.pose_intensity, options.pose_speed);
    }
    println!(
        "after {} tick(s): {}",
        race.world.tick,
        describe(&race.telemetry())
    );

    // Both tiers follow their settings, because two captures differing only by
    // one of them are how that tier gets validated - see
    // `CaptureOptions::frustum_culling` and `CaptureOptions::pvs_culling`.
    // As in `RaceStage::render`: the grade follows the zone the race reached.
    scene.sync_zone_grade(&race);
    let spectrum = zone_spectrum(options, audio);
    oag_render::perfprobe::reset();
    oag_render::perfprobe::mark("frame-start");
    // `OAG_RENDER_BENCH=N` re-records the same frame N times into throwaway
    // encoders and reports the CPU cost of `Scene::render` alone. No GPU
    // submission and no presentation, so this is the command-recording half of
    // a frame and nothing else - which is the half every finding here is about.
    // `cfg!` rather than `#[cfg]`: without `perf-probe` this is `if false` and
    // the optimiser drops all of it, while the compiler still type-checks it -
    // so the harness cannot rot between the runs that use it.
    if cfg!(feature = "perf-probe")
        && let Ok(runs) = std::env::var("OAG_RENDER_BENCH")
            .unwrap_or_default()
            .parse::<u32>()
    {
        let mut samples = Vec::with_capacity(runs as usize);
        for _ in 0..runs {
            let mut bench = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("race bench"),
            });
            let start = std::time::Instant::now();
            scene.render(
                &device,
                &queue,
                &mut bench,
                &view,
                &race,
                viewport,
                options.fov,
                options.frustum_culling,
                options.pvs_culling,
                options.anim_seconds,
                options.motion_blur,
                options.shadows,
                camera_jitter,
                &spectrum,
                // `OAG_RENDER_BENCH` measures the CPU side of encoding this
                // pass and says so; a GPU timestamp is a different number
                // about a different thing, and mixing them into one loop's
                // output is how a bench stops meaning anything.
                None,
                None,
                None,
            );
            samples.push(start.elapsed().as_secs_f64() * 1e6);
            drop(bench);
        }
        samples.sort_by(f64::total_cmp);
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        println!(
            "bench Scene::render over {} runs: mean {mean:.1} us, median {:.1} us, p10 {:.1} us, p90 {:.1} us",
            samples.len(),
            samples[samples.len() / 2],
            samples[samples.len() / 10],
            samples[samples.len() * 9 / 10],
        );
    }
    scene.render(
        &device,
        &queue,
        &mut encoder,
        &view,
        &race,
        viewport,
        options.fov,
        options.frustum_culling,
        options.pvs_culling,
        options.anim_seconds,
        // Honoured, and honest since the primer render above exists: with the
        // setting on, this render is the second of two distinct cameras, so
        // the smear a player sees is the smear the PNG shows. Off, the pass
        // never observed a previous camera and encodes nothing.
        options.motion_blur,
        options.shadows,
        camera_jitter,
        &spectrum,
        // Untimed, as the primer above is and for the same reason.
        None,
        None,
        None,
    );
    oag_render::perfprobe::report_frame(race.world.tick);

    // The HUD, into the same target. Without this a race screenshot would show
    // the track and no HUD at all, because unlike the front end's capture this
    // path has no `Framebuffer` and so no overlay pass of its own - which would
    // make `--screenshot` useless for the one thing it is most wanted for.
    //
    // **Every path is `Rgba8Unorm` since ADR-0020** - this capture, a
    // `--presented` capture and the window alike - so `Renderer::new`'s fork of
    // the sprite sheet's texture format on `format.is_srgb()` now always takes
    // the raw side, and the HUD's art reaches all three the same way. It used
    // to differ: an ordinary capture was raw while the window and `--presented`
    // were sRGB, which is exactly the disagreement the ADR removed (measured at
    // up to `73/255` on the plume). Text and fills go through the R8 coverage
    // atlas and were unaffected either way. See `docs/ui/hud.md`.
    //
    // **Or the results table, once the race has one.** A capture whose
    // `--ticks` reach past the last crossing is a picture of a finished race,
    // and the same rule the window follows applies here: the HUD is reporting a
    // simulation that has stopped, so the board replaces it rather than sitting
    // under it. That is what makes `--screenshot` able to show a scoreboard at
    // all - see `crate::scoreboard`.
    // The upscaler and the blit, through exactly the calls the window's frame
    // loop makes - and in the same order, which is the point. Presented, the
    // scene resolves into the presentation target *first* and the HUD goes on
    // afterwards at presentation size, per
    // [ADR-0036](../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md);
    // the grade comes last, in `composite`, so the HUD is inside it.
    if let (Some(framebuffer), Some(state)) = (framebuffer.as_mut(), presented) {
        framebuffer.resize_output(&device, (width, height));
        // **A `--presented` capture is the one place an upscaler's output can
        // be looked at**, which is what this flag exists for - so the temporal
        // path has to be reachable here too, not just from the window.
        let temporal = temporal_supported.then(|| scene.temporal()).flatten().map(
            |(depth, velocity, frame)| crate::upscale::Temporal {
                depth,
                velocity,
                sample_count: scene.sample_count(),
                camera: frame.camera,
                jitter: frame.jitter,
                phase_count: frame.phase_count,
                reset: frame.reset,
            },
        );
        // No timer to abandon a claim on: a capture has no frame loop to pace
        // and no `PassTimer` to pace it with - that is per-window state the
        // session owns. The `false` this can return on a build failure is
        // exactly the picture already carrying on down the ladder; nothing
        // here claimed a slot for it to give back.
        let _ = framebuffer.resolve_scene(
            &device,
            &queue,
            &mut encoder,
            rect,
            &state.presentation,
            temporal,
            None,
        );
    }
    // Where the HUD goes, which is not where the scene went. Presented, that is
    // the presentation target at the aspect rectangle; otherwise it is the
    // capture texture the scene drew straight into, which has no resolve in
    // front of it and so was already at native size - an ordinary
    // `--screenshot` is byte-for-byte what it was before the HUD moved.
    let (hud_view, hud_viewport) = match framebuffer.as_ref() {
        Some(framebuffer) => (framebuffer.output().clone(), rect),
        None => (view.clone(), viewport),
    };
    match race.results() {
        Some(board) => match crate::scoreboard::Overlay::new(&device, &queue, format, &hud) {
            Ok(mut overlay) => {
                overlay.draw(
                    &device,
                    &queue,
                    &mut encoder,
                    &hud_view,
                    board,
                    hud_viewport,
                );
            }
            Err(why) => warn!("the scoreboard did not build ({why}); capturing without one"),
        },
        None => match crate::hud::Overlay::new(&device, &queue, format, &hud) {
            Ok(Some(mut overlay)) => {
                let mut readout = race.readout();
                // The rung the grade is showing - see `Scene::zone_stage`. A
                // `--zone-stage` capture reads its own class name back here.
                readout.zone_stage = scene.zone_stage().unwrap_or(0);
                readout.zone_next_in =
                    scene.zones_to_next_stage(u16::try_from(readout.zone).unwrap_or(u16::MAX));
                overlay.draw(
                    &device,
                    &queue,
                    &mut encoder,
                    &hud_view,
                    &readout,
                    hud_viewport,
                );
                // The countdown, on the same terms `RaceStage::draw_hud` draws
                // it on - see `oag_game::hud::countdown`,
                // `oag_race::RaceState::thrust_gated` and `Scene::draws_gantry`,
                // which stands this overlay down where the circuit's own gantry
                // is already showing the count.
                if let (Some((model, widget)), true) = (
                    countdown_model,
                    RaceState::thrust_gated(readout.race_ticks) && !scene.draws_gantry(),
                ) {
                    match crate::hud::Countdown::new(&device, &queue, format, model, &widget) {
                        Ok(mut countdown) => countdown.draw(
                            &device,
                            &queue,
                            &mut encoder,
                            &hud_view,
                            readout.race_ticks as f32 / 60.0,
                            hud_viewport,
                            (width, height),
                        ),
                        Err(why) => {
                            warn!(
                                "the countdown overlay did not build ({why}); capturing without one"
                            );
                        }
                    }
                }
            }
            Ok(None) => warn!("no HUD layout: capturing without one"),
            Err(why) => warn!("the HUD overlay did not build ({why}); capturing without one"),
        },
    }
    if let (Some(framebuffer), Some(state)) = (framebuffer.as_mut(), presented) {
        framebuffer.composite(
            &queue,
            &mut encoder,
            &surface,
            state.presentation.brightness,
            state.presentation.gamma,
        );
    }

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

    // A screenshot is opaque. The frame's alpha channel is the bloom mask and
    // not coverage - see `oag_render::capture::make_opaque` - so encoding it
    // straight from the readback writes a fully transparent PNG.
    oag_render::capture::make_opaque(&mut pixels);
    let png = oag_formats::png::encode_rgba(width, height, &pixels);
    std::fs::write(&options.path, png)
        .with_context(|| format!("writing {}", options.path.display()))?;
    println!("wrote {} ({width}x{height})", options.path.display());
    Ok(())
}

/// The spectrum to feed `Scene::render` with: a fixed, deterministic ramp
/// under [`CaptureOptions::zone_spectrum_test`], or [`Output::spectrum`]'s
/// live one otherwise. See that field's own doc comment for why a capture
/// wants the override.
///
/// [`Output::spectrum`]: oag_audio::Output::spectrum
fn zone_spectrum(options: &CaptureOptions, audio: &crate::audio::Audio) -> [f32; oag_audio::BANDS] {
    if options.zone_spectrum_test {
        std::array::from_fn(|i| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "oag_audio::BANDS is 16; no precision lost casting a band index"
            )]
            let t = i as f32 / (oag_audio::BANDS - 1) as f32;
            t
        })
    } else {
        audio.output().spectrum().levels()
    }
}

/// One simulation tick of a capture: `tick`'s input, the race step, and the
/// audio that rides it.
///
/// Extracted from the tick loop so a motion blur capture can defer exactly
/// one tick past the primer render without duplicating the input logic -
/// a second copy of the script/pulse arithmetic is how the two would drift.
fn advance_one_tick(
    race: &mut Race,
    held: &mut HeldButtons,
    audio: &mut crate::audio::Audio,
    options: &CaptureOptions,
    tick: u32,
) {
    if let Some(script) = &options.input_script {
        held.set_held(script.at(tick as usize).buttons);
    } else {
        held.pulse(options.pressed, options.held, tick.is_multiple_of(2));
    }
    let snapshot = held.snapshot();
    // Before the tick, so `spend_pickup` can fire it on this tick's edge.
    if let Some(weapon) = options.give
        && race.world.ships[0].pickup.weapon.is_none()
    {
        race.world.ships[0].pickup.weapon = Some(weapon);
    }
    race.tick(&snapshot);
    // The race's own voices, on the tick that raised them - the same call
    // the windowed loop makes immediately after `Race::tick` in
    // `main::session::frame`. Without it a `--dump-audio` capture of a race
    // carried the music and nothing else: no engines, no collisions, no
    // speech, and the cues piled up undrained in the race. The dump is the
    // only end-to-end evidence a headless run has that a sound was made at
    // all, so a capture that silently held only half the mix is worse than
    // no capture.
    audio.race_tick(race);
    // Beside the race step and not outside the loop, for the reason the
    // exhaust and the chase camera are advanced from inside `Race::tick`:
    // what a capture produces has to be a function of the tick count and
    // nothing else, or the same command line gives a different file on a
    // slower machine.
    audio.tick();
    // The same two calls `main::session::frame` makes, so that a headless
    // capture is a usable instrument and not only a picture. `--tap-audio`
    // needs them in particular: the recording is written from whichever loop
    // is running, and this one is the only loop a machine with no window has.
    audio.output().report_health();
    audio.output().flush_tap();
    if options.log_every > 0 && race.world.tick.is_multiple_of(u64::from(options.log_every)) {
        println!("{}", describe(&race.telemetry()));
    }
}

/// One telemetry line, for a log or a report.
#[must_use]
pub fn describe(telemetry: &Telemetry) -> String {
    format!(
        "tick {:>5}  speed {:>8.2}  grounded {:>3.1}  spline {:>8.2}  height {:>8.2}  at {:.1}\
         {}",
        telemetry.tick,
        telemetry.speed,
        telemetry.grounded,
        telemetry.spline_distance,
        telemetry.height_above_spline,
        telemetry.position,
        match (telemetry.pickup, telemetry.projectiles) {
            (None, 0) => String::new(),
            (held, count) => {
                let held = held.map_or("-", oag_formats::weapons::Weapon::as_type);
                format!("  holding {held}  in the air {count}")
            }
        },
    )
}
