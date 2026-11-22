//! The thumb-sized layout's rules, off the grid alone.

use super::*;

const GRIDS: [(f32, f32); 3] = [(363.0, 272.0), (484.0, 272.0), (589.0, 272.0)];

fn overlap(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

#[test]
fn every_target_is_thumb_sized_and_inside_the_safe_area() {
    for grid in GRIDS {
        let (sx, sy) = insets(grid);
        for chip in chips(true) {
            let [x, y, w, h] = rect(chip, grid);
            assert!(h >= BIG * grid.1 - 1e-3, "{chip:?} is {h} tall");
            assert!(w >= BIG * grid.1 - 1e-3, "{chip:?} is {w} wide");
            assert!(
                x >= sx - 1e-3 && x + w <= grid.0 - sx + 1e-3,
                "{chip:?} {x} {w}"
            );
            assert!(
                y >= sy - 1e-3 && y + h <= grid.1 - sy + 1e-3,
                "{chip:?} {y} {h}"
            );
        }
    }
}

#[test]
fn no_two_targets_overlap() {
    for grid in GRIDS {
        let all = chips(true);
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert!(
                    !overlap(rect(*a, grid), rect(*b, grid)),
                    "{a:?} and {b:?} overlap on {grid:?}"
                );
            }
        }
    }
}

#[test]
fn a_tap_on_a_targets_middle_finds_it_and_the_drag_chip_is_free_lens_only() {
    for grid in GRIDS {
        for chip in chips(true) {
            let [x, y, w, h] = rect(chip, grid);
            assert_eq!(chip_at(grid, (x + w / 2.0, y + h / 2.0), true), Some(chip));
        }
        let [x, y, w, h] = rect(Chip::Drag, grid);
        assert_eq!(chip_at(grid, (x + w / 2.0, y + h / 2.0), false), None);
        assert_eq!(chip_at(grid, (grid.0 / 2.0, grid.1 / 3.0), true), None);
    }
}

#[test]
fn the_panel_fits_ten_rows_and_clears_the_chips() {
    for grid in GRIDS {
        let geom = panel(grid);
        let last = geom.rect(9, grid);
        assert!(last[1] + last[3] <= grid.1 - SAFE_Y * grid.1 + 1e-3);
        let first = geom.rect(0, grid);
        let whole = [first[0], first[1], first[2], last[1] + last[3] - first[1]];
        for chip in [
            Chip::Back,
            Chip::Hud,
            Chip::Drag,
            Chip::Camera,
            Chip::Take,
            Chip::Effects,
        ] {
            assert!(
                !overlap(whole, rect(chip, grid)),
                "panel covers {chip:?} on {grid:?}"
            );
        }
    }
}
