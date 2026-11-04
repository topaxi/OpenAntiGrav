//! The overlay keeps a mod's vector icons at the pixel size the HUD is drawn at: a draw at
//! another viewport starts a raster on a worker and the finished sheet is swapped in.

use oag_hud::svg::VectorIcon;
use oag_hud::{Layout, Readout};

const LAYOUT: &str = r#"
<Screen>
<Image name="PickupBackground">
<Values x="240" y="35" Centred="true" width="66" height="60" U="3" V="111" TxtrWidth="66" TxtrHeight="60" Src="Data\HUD\Textures\PulseHUD.mip"></Values>
</Image>
<Image name="TurboIcon">
<Values x="240" y="35" Centred="true" width="48" height="48" U="70" V="111" TxtrWidth="48" TxtrHeight="48" Src="Data\HUD\Textures\PulseHUD.mip"></Values>
</Image>
</Screen>
"#;

const SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="white"/></svg>"#;
const SRC: &str = "mod:test/bar";

fn assets() -> oag_hud::Assets {
    let layout = Layout::from_xml(LAYOUT);
    let (width, height) = (128_u32, 160_u32);
    let mut rgba = vec![0_u8; (width * height * 4) as usize];
    for y in 111 + 5..111 + 5 + 27 {
        for x in 70 + 14..70 + 14 + 20 {
            rgba[((y * width + x) * 4 + 3) as usize] = 255;
        }
    }
    let sheet = oag_hud::sprite::Sheet::build_with(
        &[],
        vec![oag_hud::sprite::DecodedImage {
            src: "Data\\HUD\\Textures\\PulseHUD.mip".into(),
            width,
            height,
            rgba,
            quad_extent: None,
            quad_centre: None,
            blend: None,
        }],
        &mut Vec::new(),
    );
    let font = oag_ui::font::Atlas::build();
    let mut assets = oag_hud::Assets {
        layout: Some(layout),
        sheet,
        font: font.clone(),
        small_font: font.clone(),
        default_font: font.clone(),
        skin_font: font,
        strings: oag_ui::language::StringTable::default(),
        space: oag_display::space::Space::default(),
        art: oag_pulse::hud::ART,
        split: None,
        split_columns: None,
        split_hud: None,
        vector_icons: vec![VectorIcon::parse(SRC.into(), SVG).unwrap()],
        vector_base: None,
    };
    assets.rasterise_vector_icons(1.0, &mut Vec::new());
    assets
}

#[test]
fn a_draw_at_another_size_rasterises_the_vector_icons_again_and_swaps_the_sheet() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        println!("skipping: no GPU adapter");
        return;
    };
    let Ok((device, queue)) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
    else {
        println!("skipping: no device");
        return;
    };
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut overlay = super::Overlay::new(
        &device,
        &queue,
        format,
        &assets(),
        oag_display::display::HudScale::Linear,
    )
    .expect("the overlay")
    .expect("a layout");
    assert_eq!(overlay.vector_density(), Some(1.0), "the load's raster");
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("retarget"),
        size: wgpu::Extent3d {
            width: 1920,
            height: 1080,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let readout = Readout::default();
    let mut draw_until = |viewport: (f32, f32, f32, f32), wanted: f32| {
        for _ in 0..2000 {
            let mut encoder = device.create_command_encoder(&Default::default());
            overlay.draw(
                &device,
                &queue,
                &mut encoder,
                &view,
                &readout,
                None,
                viewport,
            );
            queue.submit([encoder.finish()]);
            if overlay
                .vector_density()
                .is_some_and(|d| (d - wanted).abs() < 0.01)
            {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        false
    };
    // 960x544 shows two pixels of every unit of the 480x272 grid; 1920x1080 about four.
    assert!(draw_until((0.0, 0.0, 960.0, 544.0), 2.0), "2 px/unit");
    assert!(
        draw_until((0.0, 0.0, 1920.0, 1080.0), 1080.0 / 272.0),
        "4 px/unit"
    );
}
