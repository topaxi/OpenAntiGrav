//! Disc-backed proof that building a race scene never costs the frame thread
//! a frame: [`BuildWorker::take`] returns at once every time it is polled,
//! while the build itself takes seconds.
//!
//! `#[ignore]`d and needs `data/images/pulse-psp-eu.chd` and a GPU adapter;
//! see `oag_testdata`'s own doc for the skip-or-fail contract. Run it with:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(oag-game) and test(race_build)'
//! ```

use web_time::{Duration, Instant};

use crate::settings;
use oag_raceplay as race;

use super::{BuildWorker, Request};
use crate::app::gpu::Handles;

/// The longest one poll of the build may take on the test thread.
///
/// **Chosen, not measured**, and deliberately loose: one 60 Hz frame is
/// 16.7 ms, and `just test-data` runs this beside four thousand other tests on
/// a contended machine, so a tight bound would fail on scheduling rather than
/// on anything this code does. What it guards against is the regression it
/// exists for - a poll that waits on the build, which measured 8.0 s.
const POLL_BUDGET: Duration = Duration::from_millis(50);

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    pollster::block_on(
        adapter.request_device(&oag_mesh::mesh_render::device_descriptor(
            "race-build test",
            &adapter,
        )),
    )
    .ok()
}

#[test]
#[ignore = "needs a disc image and a GPU adapter"]
fn race_build_polls_never_wait_on_the_build() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let Some((device, queue)) = device() else {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "no GPU adapter to build a race scene on"
        );
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        ..race::Options::default()
    })
    .expect("loading the race");

    let format = wgpu::TextureFormat::Bgra8Unorm;
    let allocation = (480, 272);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("race-build test target"),
        size: wgpu::Extent3d {
            width: allocation.0,
            height: allocation.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
        texture_binding_view_dimension: None,
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let request = Request {
        gpu: Handles {
            device: device.clone(),
            queue: queue.clone(),
            format,
        },
        allocation,
        extent: allocation,
        warm_target: view.clone(),
        anisotropy: oag_mesh::mesh_render::Anisotropy::default(),
        settings: settings::Settings::default(),
        render_profile: settings::RenderProfile::default(),
        scheme: oag_gameplay::ControlScheme::default(),
        autopilot: false,
        autopilot_pilot: None,
        autopilot_skill: None,
        track_entry: None,
        pvs_culling: true,
        anim_seconds: None,
    };

    let releases_before = crate::heap::releases();
    let started = Instant::now();
    let mut worker = BuildWorker::spawn(request, loaded);
    let mut polls = 0u32;
    let mut slowest = Duration::ZERO;
    let built = loop {
        // A frame's worth of this thread's own GPU work between polls - a
        // clear of the same target - so the build shares the device and the
        // queue with something, the way it does behind the loading screen.
        let mut encoder = device.create_command_encoder(&Default::default());
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("race-build test frame"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        }));
        queue.submit(Some(encoder.finish()));

        let poll = Instant::now();
        let taken = worker.take();
        slowest = slowest.max(poll.elapsed());
        polls += 1;
        if let Some(built) = taken {
            break built;
        }
        std::thread::sleep(Duration::from_millis(16));
    };
    let total = started.elapsed();

    assert!(
        built.stage.is_ok(),
        "the scene built: {:?}",
        built.stage.err().map(|e| match e {
            crate::app::loading_stage::RaceBuildError::Load(e)
            | crate::app::loading_stage::RaceBuildError::Gpu(e) => format!("{e:#}"),
        })
    );
    // The build hands the heap its decode buffers freed back before it returns.
    assert!(
        crate::heap::releases() > releases_before,
        "the build ended without returning the freed heap"
    );
    // Structural, and immune to a contended machine: the build took many
    // frames, so the first poll cannot have been the one that waited for it.
    assert!(
        polls > 1,
        "the first poll returned the build, so it waited for it ({total:?})"
    );
    assert!(
        slowest < POLL_BUDGET,
        "a poll took {slowest:?}, over the {POLL_BUDGET:?} chosen budget, across {polls} polls \
         of a {total:?} build"
    );
}

/// What a race leaves behind is released, and the heap it freed is handed back, by the time the
/// drop thread ends. A count rather than a resident-set threshold: the claim is that the call is
/// made once per race let go, which a changed allocator or a busier machine cannot move.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_race_let_go_is_dropped_and_the_heap_returned_by_the_drop_thread() {
    let race = std::sync::Arc::new(());
    let weak = std::sync::Arc::downgrade(&race);
    let before = crate::heap::releases();

    let handle = super::drop_then_release(race).expect("the drop thread spawns");
    handle.join().expect("the drop thread does not panic");

    assert!(
        weak.upgrade().is_none(),
        "the value outlived its drop thread"
    );
    assert_eq!(
        crate::heap::releases(),
        before + 1,
        "the heap is returned once per race dropped"
    );
}
