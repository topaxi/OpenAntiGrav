//! [`super::ranked`]'s rule and the rows' text, pinned against the live frame.

use super::{KillTag, ranked};
use crate::hud::Readout;

/// The live PPSSPP frame (`AG Systems 1`, `Feisar 1`, `Qirex 1`, `AAA 0`,
/// `Triakis 0`, `Goteki 45 0`, `Piranha 0`, `EG-X 0`, top to bottom) is the
/// column this rule produces from a grid array in which the equal-kill craft
/// sit in the order `EG-X`, `Piranha`, `Goteki 45`, `Triakis`, `AAA` and
/// `Qirex`, `Feisar`, `AG Systems`: earlier in the array is lower.
#[test]
fn equal_kills_stack_with_the_later_craft_on_top() {
    // Array order: 0 EG-X, 1 Piranha, 2 Goteki 45, 3 Triakis, 4 AAA,
    // 5 Qirex, 6 Feisar, 7 AG Systems.
    let kills = [0, 0, 0, 0, 0, 1, 1, 1];
    assert_eq!(ranked(&kills), [7, 6, 5, 4, 3, 2, 1, 0]);
}

#[test]
fn more_kills_rank_higher_whatever_the_array_order() {
    // Craft 2 leads on three, craft 0 and 3 tie on one, craft 1 has none.
    assert_eq!(ranked(&[1, 0, 3, 1]), [2, 3, 0, 1]);
}

#[test]
fn a_lone_craft_is_row_zero() {
    assert_eq!(ranked(&[4]), [0]);
    assert!(ranked(&[]).is_empty());
}

fn strings() -> oag_ui::language::StringTable {
    oag_ui::language::StringTable::from_xml(
        r#"<Screen name="Top"><Entry ID="Feisar" String="Feisar"/><Entry ID="AG_Systems" String="AG Systems"/></Screen>"#,
    )
}

/// The player's row has two spaces before the count and full alpha; an
/// opponent's has one and `0.8`; a row past the field has neither.
#[test]
fn the_rows_read_name_then_kills_and_dim_the_opponents() {
    let readout = Readout {
        kill_tags: vec![
            KillTag {
                team: "AG_Systems".into(),
                kills: 1,
                player: false,
            },
            KillTag {
                team: "Feisar".into(),
                kills: 0,
                player: true,
            },
        ],
        ..Readout::blank()
    };
    let strings = strings();
    assert_eq!(
        super::text(&readout, "PosTag0", &strings).as_deref(),
        Some("AG Systems 1")
    );
    assert_eq!(
        super::text(&readout, "PosTag1", &strings).as_deref(),
        Some("Feisar  0")
    );
    assert_eq!(super::text(&readout, "PosTag2", &strings), None);
    assert_eq!(super::text(&readout, "Position", &strings), None);
    let white = [1.0; 4];
    assert_eq!(
        super::colour(&readout, "PosTag0", white),
        Some([1.0, 1.0, 1.0, 0.8])
    );
    assert_eq!(super::colour(&readout, "PosTag1", white), Some(white));
    assert_eq!(super::colour(&readout, "PosTag5", white), None);
}
