use super::*;

const SKIN: &str = r#"<Screen name="FE Screen">
<BackgroundAnim name="bgAnim">
  <Values Src="Data\FE\FrontEndScene\A.vex" AnimLength="60.0" UseModelCamera="true" nearZ="0.5" farZ="50000.0"></Values>
  <Values Src="Data\FE\FrontEndScene\B_VR.vex" IsVR="true" UseModelCamera="true"></Values>
  <ScreenSetting name="default" blur="3" use_bands="false">
    <Main edge_level="0.3" fill_level="0.1" edge_width="1.5"></Main>
    <Band1 edge_level="0.2" fill_level="0.1" edge_width="3.0"></Band1>
  </ScreenSetting>
  <ScreenSetting name="Main Menu" use_bands="false" blur="0">
    <Main edge_level="0.6" fill_level="0.2" edge_width="0.5"></Main>
  </ScreenSetting>
  <ScreenSetting name="Wild" blur="0">
    <Main edge_level="7.0" fill_level="-2.0" edge_width="99.0"></Main>
  </ScreenSetting>
  <Model name="GroundPlane"><Values Src="Data\FE\frontendscene\ground_VR.vex"></Values></Model>
</BackgroundAnim>
</Screen>"#;

fn widget() -> Widget {
    Widget::read(&oag_tables::fexml::parse(SKIN)).expect("a widget")
}

#[test]
fn the_widget_reads_the_first_scene_that_is_not_for_vr() {
    let widget = widget();
    assert_eq!(widget.src, r"Data\FE\FrontEndScene\A.vex");
    assert_eq!(widget.anim_length, 60.0);
    assert_eq!(widget.depth, [0.5, 50000.0]);
    assert!(widget.model_camera);
    assert_eq!(widget.rows(), 3);
    assert!(!widget.bands());
}

#[test]
fn a_skin_with_no_widget_reads_none() {
    let skin = oag_tables::fexml::parse(r#"<Screen name="x"><Text name="a"></Text></Screen>"#);
    assert!(Widget::read(&skin).is_none());
}

#[test]
fn the_root_page_takes_the_main_menu_row_and_every_other_takes_default() {
    let widget = widget();
    let main = widget.look_for_root(true);
    assert_eq!(main.blur, 0.0);
    assert_eq!(main.main.edge_level, 0.6);
    assert_eq!(main.main.fill_level, 0.2);
    assert_eq!(main.main.edge_width, 0.5);
    let other = widget.look_for_root(false);
    assert_eq!(other.blur, 3.0);
    assert_eq!(other.main.edge_width, 1.5);
    // A screen with no row of its own falls back to `default`, not to zero.
    assert_eq!(widget.look_for("SoundTest"), other);
}

#[test]
fn levels_are_clamped_the_way_parse_screen_setting_clamps_them() {
    let wild = widget().look_for("Wild");
    assert_eq!(wild.main.edge_level, 1.0);
    assert_eq!(wild.main.fill_level, 0.0);
    assert_eq!(wild.main.edge_width, 24.0);
}

#[test]
fn the_working_copy_starts_empty_and_eases_toward_the_page_row() {
    let mut scene = Scene::new(widget());
    assert_eq!(scene.look(), Look::default());
    scene.tick(true);
    // One step is `PULL` of the way: 3% of the Main Menu row's 0.6.
    assert!((scene.look().main.edge_level - 0.018).abs() < 1e-6);
    for _ in 0..600 {
        scene.tick(true);
    }
    let settled = scene.widget().look_for_root(true);
    assert!((scene.look().main.edge_level - settled.main.edge_level).abs() < 1e-4);
    // Leaving the root retargets the same copy instead of jumping.
    scene.tick(false);
    assert!(scene.look().blur > 0.0 && scene.look().blur < 0.1);
}

#[test]
fn the_clock_loops_at_the_authored_length() {
    let mut scene = Scene::new(widget());
    for _ in 0..60 {
        scene.tick(true);
    }
    assert!((scene.seconds() - 1.0).abs() < 1e-4);
    assert_eq!(loop_position(61.5, 60.0), 1.5);
    assert_eq!(loop_position(5.0, 0.0), 0.0);
}

#[test]
fn a_settled_frame_carries_the_row_whole_and_the_looped_time() {
    let scene = Scene::new(widget());
    let frame = scene.settled(65.0, true, [[0.0; 4]; 4]);
    assert_eq!(frame.seconds, 5.0);
    assert_eq!(frame.look, scene.widget().look_for_root(true));
}
