//! The `perf-probe` renders of a captured frame (`bench`): the harness `capture` calls once its
//! race has run, over the scene it drew.

use super::*;

/// What the harness draws with.
pub(super) struct Probe<'a> {
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) adapter: wgpu::AdapterInfo,
    pub(super) scene: &'a mut Scene,
    pub(super) race: &'a Race,
    pub(super) view: &'a wgpu::TextureView,
    pub(super) viewport: (f32, f32, f32, f32),
    pub(super) options: &'a CaptureOptions,
    pub(super) camera_jitter: Option<u32>,
    pub(super) spectrum: &'a spectrum::Bands,
}

/// Benches the frame: `OAG_RENDER_BENCH` and `OAG_RENDER_GPU_BENCH`, see [`bench`].
pub(super) fn run(probe: Probe<'_>) {
    let Probe {
        device,
        queue,
        adapter,
        scene,
        race,
        view,
        viewport,
        options,
        camera_jitter,
        spectrum,
    } = probe;
    let split = race.view_state().viewport_count() > 1;
    let pacer = bench::TickPacer::from_env();
    if !split && !race.windowed() {
        bench::run(device, queue, |encoder, timestamps| {
            if pacer.starts_tick() {
                scene.forget_shadow_gathers();
            }
            scene.render(
                device,
                queue,
                encoder,
                view,
                race,
                viewport,
                options.fov,
                options.frustum_culling,
                options.pvs_culling,
                options.anim_seconds,
                options.motion_blur,
                options.shadows,
                camera_jitter,
                spectrum,
                timestamps.scene,
                timestamps.blur,
                timestamps.bloom,
            );
        });
    }
    // The whole frame as the window submits it, one view or many: what
    // `bench::run` leaves out (finishing and submitting the encoder).
    // A windowed race is benched window by window, each its own encoder and submission
    // as the window's frame loop makes them (the scene side of the chain: the resolve,
    // HUD and composite are the same per window whatever this lane changes).
    let windowed = race.windowed();
    let seats: Vec<usize> = (0..race.windows().len())
        .map(|window| race.window_drawn(window).len())
        .collect();
    let bench_views = if windowed {
        seats.iter().sum()
    } else {
        race.views_drawn()
    };
    bench::run_split(device, queue, &adapter, bench_views, |timers| {
        if pacer.starts_tick() {
            scene.forget_shadow_gathers();
        }
        if windowed {
            let mut rest = &mut *timers;
            // The first window that draws does the frame's shared work, the rest read it.
            let mut shared = false;
            for (window, &count) in seats.iter().enumerate() {
                let (own, tail) = rest.split_at_mut(count.min(rest.len()));
                rest = tail;
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("race bench window"),
                });
                scene.render_window(
                    device,
                    queue,
                    &mut encoder,
                    view,
                    race,
                    window,
                    viewport,
                    options.fov,
                    options.frustum_culling,
                    options.pvs_culling,
                    options.anim_seconds,
                    options.shadows,
                    spectrum,
                    shared,
                    oag_raceplay::WindowEffects {
                        motion_blur: options.motion_blur,
                        camera_jitter,
                        ..Default::default()
                    },
                    own,
                );
                shared |= count > 0;
                queue.submit(Some(encoder.finish()));
            }
            return;
        }
        if split {
            scene.render_viewports(
                device,
                queue,
                view,
                race,
                viewport,
                options.fov,
                options.frustum_culling,
                options.pvs_culling,
                options.anim_seconds,
                options.shadows,
                spectrum,
                oag_raceplay::WindowEffects {
                    motion_blur: options.motion_blur,
                    camera_jitter,
                    ..Default::default()
                },
                timers,
            );
            return;
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("race bench frame"),
        });
        let timer = timers.first_mut().and_then(Option::as_mut);
        scene.render(
            device,
            queue,
            &mut encoder,
            view,
            race,
            viewport,
            options.fov,
            options.frustum_culling,
            options.pvs_culling,
            options.anim_seconds,
            options.motion_blur,
            options.shadows,
            camera_jitter,
            spectrum,
            timer
                .as_deref()
                .and_then(oag_gpu::timing::PassTimer::writes),
            None,
            None,
        );
        if let Some(timer) = timer {
            timer.resolve(&mut encoder);
        }
        queue.submit(Some(encoder.finish()));
    });
}
