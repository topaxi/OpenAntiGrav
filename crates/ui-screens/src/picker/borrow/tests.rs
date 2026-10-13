//! The borrowed shared ship page: shape, scale and the single-player names.

use super::*;

fn halves(grid: [f32; 2]) -> [Layout; 2] {
    read_borrowed(
        &HashMap::new(),
        &StringTable::from_xml(
            r#"<Strings><Entry ID="RC_SHIPSEL" String="SHIP SELECT"></Entry></Strings>"#,
        ),
        FaceScales::default(),
        grid,
    )
    .expect("the numbers read back as a split page")
}

fn names(layout: &Layout) -> Vec<String> {
    let mut names: Vec<String> = layout
        .screen
        .images
        .iter()
        .filter_map(|image| image.name.clone())
        .chain(
            layout
                .screen
                .texts
                .iter()
                .filter_map(|text| text.name.clone()),
        )
        .collect();
    names.sort();
    names
}

/// Each half is the single-player screen's widgets under their single-player names, the
/// second half also holding the lone `Suggest` line.
#[test]
fn each_half_holds_the_single_player_names() {
    let [first, second] = halves([480.0, 272.0]);
    for wanted in [
        "honey",
        "up arrow",
        "down arrow",
        "Speed Bar",
        "Thrust Bar",
        "Handling Bar",
        "Shield Bar",
        "Loyalty Bar",
        "Infohexgrid",
        "skin",
    ] {
        assert!(names(&first).iter().any(|name| name == wanted), "{wanted}");
        assert!(names(&second).iter().any(|name| name == wanted), "{wanted}");
    }
    assert!(names(&second).iter().any(|name| name == "Suggest"));
    assert_eq!(first.title, "SHIP SELECT");
    assert_eq!(second.title, "");
}

/// The panels are the PS2's, 9 and 204 of 448, scaled to the grid asked for: the second
/// half is below the first and the two crafts' rects do not meet.
#[test]
fn the_panels_sit_where_the_ps2_puts_them_scaled() {
    let [first, second] = halves([480.0, 272.0]);
    let ky = 272.0 / 448.0;
    assert!(
        (first.panel[1] - 9.0 * ky).abs() < 0.01,
        "{:?}",
        first.panel
    );
    assert!(
        (second.panel[1] - 204.0 * ky).abs() < 0.01,
        "{:?}",
        second.panel
    );
    assert!((first.panel[0] - 387.0 * 0.75).abs() < 0.01);
    assert!(first.preview[1] + first.preview[3] <= second.preview[1]);
}

/// A grid smaller than the PS2's draws the name and the stat rows' text at the grid ratio;
/// the PS2's own grid draws them as authored.
#[test]
fn text_is_drawn_smaller_only_on_a_smaller_grid() {
    let [psp, _] = halves([480.0, 272.0]);
    let [ps2, _] = halves(PS2_GRID);
    assert!((psp.name_scale - 272.0 / 448.0).abs() < 1e-6);
    assert!((ps2.name_scale - 1.0).abs() < f32::EPSILON);
    let speed = |layout: &Layout| {
        layout
            .screen
            .texts
            .iter()
            .find(|text| text.name.as_deref() == Some("Speed"))
            .map(|text| text.scale)
            .unwrap()
    };
    assert!((speed(&psp) - 272.0 / 448.0).abs() < 1e-6);
    assert!((speed(&ps2) - 1.0).abs() < f32::EPSILON);
}

/// The stat rows keep the PS2's 16 apart where the single screen's are 25: the tighter
/// design is the point of the split page.
#[test]
fn the_stat_rows_are_sixteen_apart_in_the_ps2s_grid() {
    let [first, _] = halves(PS2_GRID);
    let bar = |name: &str| {
        first
            .screen
            .images
            .iter()
            .find(|image| image.name.as_deref() == Some(name))
            .map(|image| image.y)
            .unwrap()
    };
    assert!((bar("Thrust Bar") - bar("Speed Bar") - 16.0).abs() < 0.01);
    assert!((bar("Shield Bar") - bar("Handling Bar") - 16.0).abs() < 0.01);
}
