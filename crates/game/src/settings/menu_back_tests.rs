//! MENU BACK's settings: where it is seeded and what a profile reads.

use super::*;

#[test]
fn the_menu_back_row_opens_on_original_and_follows_the_setting() {
    let seed = |settings: &Settings| {
        menu_seeds(
            settings,
            Anisotropy::default(),
            oag_pulse::TITLE,
            oag_disc::Platform::Ps2,
        )
        .into_iter()
        .find(|(setting, _)| *setting == "controls.menu_back")
        .map(|(_, value)| value)
    };
    let text = |token: &str| Some(oag_ui::menu::Value::Text(token.to_string()));
    let mut settings = Settings::default();
    assert_eq!(seed(&settings), text("original"));
    settings.controls.menu_back = "triangle".to_string();
    assert_eq!(seed(&settings), text("triangle"));
    // A token this build does not know reads as the default rather than failing the boot.
    settings.controls.menu_back = "b".to_string();
    assert_eq!(seed(&settings), text("original"));
}
