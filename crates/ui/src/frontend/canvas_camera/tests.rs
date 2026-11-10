//! The camera against values read from the running original
//! (`docs/ghidra/functions/vita-2048-eu-v104/campaign-map-projection.md`,
//! "Evidence" and "The scroll").

use super::*;

/// The cell of an event at `(m_x, m_y)` on the base map.
fn base_cell(m_x: f32, m_y: f32) -> (f32, f32) {
    (55.0 + 4.0 * m_x, 25.0 + 2.0 * m_y)
}

fn close(a: (f32, f32), b: (f32, f32), within: f32) -> bool {
    (a.0 - b.0).abs() <= within && (a.1 - b.1).abs() <= within
}

/// The view of the committed frame 13: the save's opening view.
const OPENING: (f32, f32) = (746.614, 2000.560);
/// The view of the committed frame 12: Event 1 selected from the opening view.
const EVENT_1_SELECTED: (f32, f32) = (425.094, 1714.214);

#[test]
fn the_span_is_the_one_the_item_stores() {
    let span = span();
    assert!(close(span, (16.511_15, 9.871_62), 0.001), "{span:?}");
}

#[test]
fn the_reset_camera_puts_the_current_event_where_the_item_stored_it() {
    let at = Camera::scrolled((0.0, 0.0)).project(cell_ground(base_cell(6.0, 6.0)));
    assert!(close(at, (1175.175, 2272.557), 0.05), "{at:?}");
}

#[test]
fn the_map_opens_where_the_original_comes_to_rest() {
    let offset = opening_offset(base_cell(6.0, 6.0));
    assert!(close(offset, OPENING, 0.05), "{offset:?}");
    // The eased step stops short: the event's ground point rests right of centre.
    let at = Camera::scrolled(offset).project(cell_ground(base_cell(6.0, 6.0)));
    assert!(close(at, (503.1, 272.0), 0.2), "{at:?}");
}

#[test]
fn selecting_an_event_scrolls_to_the_originals_target() {
    let offset = centred_on(base_cell(3.0, 3.0), OPENING);
    assert!(close(offset, EVENT_1_SELECTED, 0.05), "{offset:?}");
}

/// The top face's left edge, right edge and top edge on screen.
fn face_box(cell: (f32, f32), offset: (f32, f32)) -> (f32, f32, f32) {
    let centre = Camera::scrolled(offset).project(tile_top(cell));
    let face = tile_face();
    (
        centre.0 - face.0 * 0.5,
        centre.0 + face.0 * 0.5,
        centre.1 - face.1 * 0.5,
    )
}

#[test]
fn the_tiles_land_on_the_reference_frames_boxes() {
    // (m_x, m_y), view, then the measured left, right and top of the lit top
    // face in the frame. The right edge carries the 2 px of shear.
    let cases = [
        ((5.0, 5.0), OPENING, (352.0, 472.0, 127.0)),
        ((3.0, 3.0), EVENT_1_SELECTED, (439.0, 561.0, 222.0)),
        ((4.0, 4.0), EVENT_1_SELECTED, (554.0, 675.0, 318.0)),
        ((5.0, 5.0), EVENT_1_SELECTED, (669.0, 790.0, 413.0)),
    ];
    for ((m_x, m_y), view, (left, right, top)) in cases {
        let (ours_left, ours_right, ours_top) = face_box(base_cell(m_x, m_y), view);
        assert!(
            (ours_left - left).abs() < 2.0,
            "({m_x}, {m_y}) left {ours_left}"
        );
        assert!(
            (ours_right - right).abs() < 3.0,
            "({m_x}, {m_y}) right {ours_right}"
        );
        assert!(
            (ours_top - top).abs() < 2.0,
            "({m_x}, {m_y}) top {ours_top}"
        );
    }
}

#[test]
fn a_tile_is_a_hexagon_two_units_across() {
    let face = tile_face();
    assert!(close(face, (119.4, 95.4), 0.1), "{face:?}");
}
