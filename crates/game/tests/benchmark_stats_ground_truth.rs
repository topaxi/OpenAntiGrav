//! The benchmark's stats screen, drawn on every title's own caption face into a phone's screen:
//! both pages and their button labels stay inside the panel, and no string is drawn over another
//! (`docs/tools/in-game-benchmark.md`, "Fitting the stats screen").
//!
//! The picture is rendered at 2340x1080 through the same overlay, face and letterbox the window
//! draws through. Disc-backed, and needs a GPU adapter.

use oag_game::benchmark::recorder::{Ending, Frame};
use oag_game::benchmark::report::{Context, Report};
use oag_game::benchmark::screen::{self, Choice, View};

/// A phone's screen in landscape, pixels. Density does not enter the layout.
const PHONE: (u32, u32) = (2340, 1080);

/// The long strings a phone or a laptop reports.
fn report(title: &'static str) -> Report {
    let settings: Vec<(String, String)> = (0..18)
        .map(|n| (format!("ANISOTROPIC FILTERING {n}"), "ORIGINAL".to_string()))
        .collect();
    Report {
        context: Context {
            title,
            track: Some("16_Track".into()),
            mode: "single_race",
            laps_target: Some(3),
            adapter: "{}".into(),
            adapter_name: "AMD Radeon RX 7800 XT (RADV NAVI32)".into(),
            driver: "radv Mesa 25.3.1-arch1.1".into(),
            device: "linux x86_64, AMD Ryzen 9 7900 12-Core Processor".into(),
            settings: Vec::new(),
            details: settings,
            track_name: Some("Talon's Junction White Reverse".into()),
            build_version: "0.1.0 (0123456789ab)",
            refresh: (60.0, "display"),
            frame_limit_hz: Some(60.0),
            target_fps_hz: Some(60.0),
            preset: None,
            mods: None,
            vsync: "off".into(),
            present_mode: "Immediate".into(),
            frame_limit: "60".into(),
            origin: "cli",
            command: Vec::new(),
            load: (String::new(), String::new()),
            git_hash: "0",
            wall_seconds: 1.0,
            people: 1,
            tick_cap: 40_000,
        },
        // Long enough to have a 0.1 % low, and on the frame limit.
        frames: (0..2000)
            .map(|index| Frame {
                frame_us: 16_700.0,
                lap: 1,
                ticks: 1,
                tick: index + 1,
            })
            .collect(),
        ending: Ending {
            tick: 2000,
            state_hash: 0,
            finished: true,
            flyby_ticks: 0,
        },
    }
}

/// Draws both pages of the screen for the title at `source` and checks them. Skips when the
/// source or a GPU adapter is missing.
fn check(source: &str, title: &'static str) {
    let Some(path) = oag_testdata::exact(source) else {
        return;
    };
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        println!("skipping: no GPU adapter");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&Default::default())).expect("a device");
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: path.display().to_string(),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .unwrap_or_else(|e| panic!("{source}: loading a race: {e:#}"));

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut overlay = oag_game::scoreboard::Overlay::new(&device, &queue, format, &loaded.hud)
        .expect("the overlay");
    let viewport = oag_display::display::viewport(PHONE, oag_display::display::Aspect::Psp);
    let english = |_: &str, fallback: &str| fallback.to_string();
    for details in [false, true] {
        let view = View {
            details,
            can_copy: true,
            copied: None,
        };
        let list = screen::draw_list(
            &report(title),
            overlay.face(),
            Choice::Back,
            Some("/data/oag/bench/run-1795576464/pulse.json"),
            &english,
            view,
        );
        let page = if details { "DETAILS" } else { "summary" };
        let bad = screen::misfits(&list, overlay.face(), true);
        assert!(bad.is_empty(), "{title} {page}: {bad:#?}");

        // The pixels too: nothing is lit outside the panel's own rectangle.
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("benchmark stats"),
            size: wgpu::Extent3d {
                width: PHONE.0,
                height: PHONE.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
            texture_binding_view_dimension: None,
        });
        let surface = target.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        }));
        overlay.draw_list(&device, &queue, &mut encoder, &surface, &list, viewport);
        let pixels =
            oag_game::capture::read_back(&device, &queue, encoder, &target, PHONE.0, PHONE.1)
                .expect("a readback");

        let space = oag_display::space::Space::PSP;
        let [left, top, width, height] = screen::PANEL;
        let (x0, y0) = oag_game::render::from_grid(space, viewport, (left, top));
        let (x1, y1) = oag_game::render::from_grid(space, viewport, (left + width, top + height));
        let mut lit = 0_usize;
        let mut outside = Vec::new();
        for (n, rgb) in pixels.as_chunks::<4>().0.iter().enumerate() {
            if rgb[..3].iter().all(|channel| *channel < 8) {
                continue;
            }
            lit += 1;
            let x = (n % PHONE.0 as usize) as f32 + 0.5;
            let y = (n / PHONE.0 as usize) as f32 + 0.5;
            if x < x0 - 1.0 || x > x1 + 1.0 || y < y0 - 1.0 || y > y1 + 1.0 {
                outside.push((x, y));
            }
        }
        assert!(
            lit > 10_000,
            "{title} {page}: only {lit} lit pixels, nothing drawn"
        );
        assert!(
            outside.is_empty(),
            "{title} {page}: {} pixels lit outside the panel ({x0}, {y0})..({x1}, {y1}), first {:?}",
            outside.len(),
            &outside[..outside.len().min(5)],
        );
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and a GPU"]
fn pulse_psp_fits_a_phone() {
    check("data/images/pulse-psp-eu.chd", "Wipeout Pulse");
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd and a GPU"]
fn pulse_ps2_fits_a_phone() {
    check("data/images/pulse-ps2-eu.chd", "Wipeout Pulse");
}

#[test]
#[ignore = "needs data/images/pure-psp-eu.chd and a GPU"]
fn pure_fits_a_phone() {
    check("data/images/pure-psp-eu.chd", "Wipeout Pure");
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU"]
fn hd_fits_a_phone() {
    check("data/images/hdfury-ps3-eu-dec.iso", "Wipeout HD");
}

#[test]
#[ignore = "needs the extracted 2048 package in data/extracted/vita/PCSF00007 and a GPU"]
fn the_2048_face_fits_a_phone() {
    check("data/extracted/vita/PCSF00007", "Wipeout 2048");
}

#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg and a GPU"]
fn omega_fits_a_phone() {
    check("data/images/omega-ps4-eu.pkg", "Wipeout: Omega Collection");
}
