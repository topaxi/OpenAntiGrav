//! The viewer's thumb-sized layout, off the grid alone.

use super::*;

const GRIDS: [(f32, f32); 3] = [(363.0, 272.0), (484.0, 272.0), (589.0, 272.0)];

fn overlap(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

#[test]
fn every_control_is_thumb_sized_and_inside_the_safe_area() {
    for grid in GRIDS {
        let (sx, sy) = insets(grid);
        for control in CONTROLS.into_iter().chain([Control::Bar]) {
            let [x, y, w, h] = rect(control, grid);
            assert!(h >= BIG * grid.1 - 1e-3, "{control:?} is {h} tall");
            assert!(x >= sx - 1e-3 && x + w <= grid.0 - sx + 1e-3, "{control:?}");
            assert!(y >= sy - 1e-3 && y + h <= grid.1 - sy + 1e-3, "{control:?}");
        }
    }
}

#[test]
fn nothing_overlaps_and_a_touch_in_the_middle_finds_its_control() {
    for grid in GRIDS {
        let all: Vec<Control> = CONTROLS.into_iter().chain([Control::Bar]).collect();
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert!(
                    !overlap(rect(*a, grid), rect(*b, grid)),
                    "{a:?} {b:?} {grid:?}"
                );
            }
            let [x, y, w, h] = rect(*a, grid);
            assert_eq!(control_at(grid, (x + w / 2.0, y + h / 2.0)), Some(*a));
        }
        assert_eq!(control_at(grid, (grid.0 / 2.0, grid.1 / 3.0)), None);
    }
}

#[test]
fn the_bar_maps_its_ends_to_the_start_and_the_end() {
    let grid = GRIDS[1];
    let [x, _, w, _] = bar_strip(grid);
    assert_eq!(tick_at(grid, x - 50.0, 6000), 0);
    assert_eq!(tick_at(grid, x + w / 2.0, 6000), 3000);
    assert_eq!(tick_at(grid, x + w + 50.0, 6000), 6000);
}

#[test]
fn a_double_tap_means_back_toggle_or_forward_by_third() {
    let grid = GRIDS[1];
    assert_eq!(zone_at(grid, 10.0), Zone::Back);
    assert_eq!(zone_at(grid, grid.0 / 2.0), Zone::Toggle);
    assert_eq!(zone_at(grid, grid.0 - 10.0), Zone::Forward);
}

#[test]
fn the_controls_hide_only_while_playing_and_untouched() {
    assert!(chrome_shown(false, false, 10_000));
    assert!(chrome_shown(false, true, CHROME_HIDE_TICKS - 1));
    assert!(!chrome_shown(false, true, CHROME_HIDE_TICKS));
    assert!(!chrome_shown(true, false, 0));
}
