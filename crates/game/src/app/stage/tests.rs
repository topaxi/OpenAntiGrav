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
            menu_font: None,
            label_face: None,
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

/// Pulse's boot sequence draws `font="Menu"` texts in the menu face, which the renderer holds in
/// its second slot (the primary atlas is `Default`); a title that does not draw the sequence
/// that way keeps what its skin names there.
///
/// **Skips with no adapter.**
#[test]
fn the_boot_sequences_second_slot_holds_the_menu_face_where_the_skin_draws_it() {
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
    let font = oag_ui::font::Atlas::build();
    let slot = |skin: &'static oag_title::MenuSkin| {
        frontend_renderer(
            &device,
            &queue,
            wgpu::TextureFormat::Rgba8Unorm,
            None,
            &FrontendFaces {
                font: &font,
                sprites: &oag_hud::sprite::Sheet::default(),
                skin,
                menu_font: Some(&font),
                label_face: None,
                title_font: None,
                buttons_font: None,
            },
        )
        .expect("the ui pipeline")
        .face_slot_role()
    };
    let pulse = oag_pulse::frontend::MENU_SKIN;
    assert!(pulse.menu_face_in_boot);
    assert_eq!(
        slot(pulse),
        pulse.menu_font,
        "Pulse's second slot is the menu face"
    );
    assert_eq!(
        slot(oag_pure::frontend::MENU_SKIN),
        None,
        "Pure's second slot is its title face, not loaded here"
    );
    // A touch front end's tile label face takes the slot, whatever the skin.
    let label = (font.clone(), "NEOSANS_BOLD");
    let touch = frontend_renderer(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba8Unorm,
        None,
        &FrontendFaces {
            font: &font,
            sprites: &oag_hud::sprite::Sheet::default(),
            skin: &oag_ui::placeholder::MENU_SKIN,
            menu_font: None,
            label_face: Some(&label),
            title_font: None,
            buttons_font: None,
        },
    )
    .expect("the ui pipeline");
    assert_eq!(touch.face_slot_role(), Some("NEOSANS_BOLD"));
}
