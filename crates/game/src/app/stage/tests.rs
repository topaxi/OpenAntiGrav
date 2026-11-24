use super::*;

/// The boot sequence's renderer carries the title's glyph slots and pens: on Pure the third slot
/// is the `Stats` face and the pens are snapped, so the live title screen's `HOLD ON!` draws as
/// the `--screen` capture of it does (`oag_pure::frontend::MENU_SKIN`).
///
/// **Skips with no adapter.**
#[test]
fn the_boot_sequences_renderer_installs_the_titles_faces() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&Default::default()))
    else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let skin = oag_pure::frontend::MENU_SKIN;
    assert!(skin.snapped_pens && skin.stats_font.is_some());
    let font = oag_ui::font::Atlas::build();
    let renderer = frontend_renderer(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba8Unorm,
        None,
        &FrontendFaces {
            font: &font,
            sprites: &oag_hud::sprite::Sheet::default(),
            skin,
            title_font: None,
            buttons_font: Some(&font),
        },
    )
    .expect("the ui pipeline");
    assert!(renderer.snapped_pens(), "Pure's pens are whole pixels");
    assert_eq!(
        renderer.third_slot_role(),
        skin.stats_font,
        "the third slot holds Pure's Stats face"
    );
}
