//! A cell's picture and padlock: which art, over which texels, in which colours
//! (`docs/ghidra/functions/ps3-hdfury-eu/ship-select-ring.md`, "The cells").

use super::*;
use oag_ui::screen::argb_to_rgba;

const LOCK_COL: u32 = 0xff80_8080;
const LOCK_FADE: u32 = 0xff24_2424;

/// Each draw over `uv`, as its rect and the colours of its left and right sides, left column first.
fn over(draws: &[Draw], uv: [f32; 4]) -> Vec<([f32; 4], [f32; 4], [f32; 4])> {
    let mut found: Vec<_> = draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite {
                rect,
                uv: at,
                color,
            } if *at == uv => Some((*rect, *color, *color)),
            Draw::GradientSprite {
                rect,
                uv: at,
                left,
                right,
            } if *at == uv => Some((*rect, *left, *right)),
            _ => None,
        })
        .collect();
    found.sort_by(|a, b| a.0[0].total_cmp(&b.0[0]).then(a.0[1].total_cmp(&b.0[1])));
    found
}

fn draws_of(picker: &Picker) -> Vec<Draw> {
    let layout = layout();
    let extra = layout.hd.as_deref().unwrap();
    body(picker, &layout, extra, &Frame::default(), &|n| sheet(n))
}

fn close(got: [f32; 4], want: [f32; 4]) -> bool {
    got.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-3)
}

/// `share` of the way from `from` to `to`, per channel.
fn between(from: u32, to: u32, share: f32) -> [f32; 4] {
    let (a, b) = (argb_to_rgba(from), argb_to_rgba(to));
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * share)
}

/// The five columns' sides, as [`hex::edge_share`] runs: 0 at the outer edges, 1 at the selected
/// column's, so a column is `(left, right)` of these.
const SIDES: [(f32, f32); 5] = [(0.0, 0.5), (0.5, 1.0), (1.0, 1.0), (1.0, 0.5), (0.5, 0.0)];

/// A padlock is `hexlock_HD.gtf`'s own 75x64 texel window on the cell's 92x80 quad, flat in
/// `SelectedLockCol` on the selected column and running from `SelectedLockColFade` to it across
/// each other column.
#[test]
fn a_padlock_is_the_widgets_own_art_shaded_from_the_faded_colour() {
    assert_eq!(LOCK_SRC, r"Data\FE\Images\hexlock_HD.gtf");
    let picker = picker_of(&["A", "B", "C", "D", "E", "F"], "C");
    let art = sheet(LOCK_SRC).unwrap().sheet_rect([0.0, 0.0, 75.0, 64.0]);
    let locks = over(&draws_of(&picker), art);
    // Rows 0, 1, 4, 5 and 6 are locked in every column.
    assert_eq!(locks.len(), 5 * 5);
    for (column, (left, right)) in SIDES.into_iter().enumerate() {
        for (rect, got_left, got_right) in &locks[column * 5..column * 5 + 5] {
            assert_eq!(rect[2..], [92.0, 80.0]);
            let selected = column == 2;
            let (want_left, want_right) = if selected {
                (argb_to_rgba(LOCK_COL), argb_to_rgba(LOCK_COL))
            } else {
                (
                    between(LOCK_FADE, LOCK_COL, left),
                    between(LOCK_FADE, LOCK_COL, right),
                )
            };
            assert!(
                close(*got_left, want_left) && close(*got_right, want_right),
                "column {column}: {got_left:?} {got_right:?}"
            );
        }
    }
}

/// Every model has its picture, locked or not: white on the selected column's open rows, black at
/// 100/255 on its locked ones, and black running from 18/255 to 100/255 on the other columns.
/// The picture is under the padlock.
#[test]
fn a_locked_cell_keeps_its_silhouette_under_the_padlock() {
    let picker = picker_of(&["A", "B", "C", "D", "E", "F"], "C");
    let draws = draws_of(&picker);
    let layout = layout();
    let grid = layout.hd.as_deref().unwrap().hex.unwrap();
    let cells = grid.cells(picker.index(), picker.entries().len(), 0.0);
    // The test sheet places every thumbnail at one spot, so draws are told apart by position.
    let art = sheet("thumb").unwrap().sheet_rect([0.0, 0.0, 128.0, 64.0]);
    let pictures = over(&draws, art);
    assert_eq!(pictures.len(), 35);
    for (column, (left, right)) in SIDES.into_iter().enumerate() {
        for (row, (_, got_left, got_right)) in
            pictures[column * 7..column * 7 + 7].iter().enumerate()
        {
            let open = row == 2 || row == 3;
            let (want_left, want_right) = match column {
                2 if open => ([1.0; 4], [1.0; 4]),
                2 => (argb_to_rgba(0x6400_0000), argb_to_rgba(0x6400_0000)),
                _ => (
                    between(0x1200_0000, 0x6400_0000, left),
                    between(0x1200_0000, 0x6400_0000, right),
                ),
            };
            assert!(
                close(*got_left, want_left) && close(*got_right, want_right),
                "column {column} row {row}: {got_left:?} {got_right:?}"
            );
        }
    }
    // The first selected cell is locked: its picture is drawn before its padlock.
    let lock = sheet(LOCK_SRC).unwrap().sheet_rect([0.0, 0.0, 75.0, 64.0]);
    let first = cells
        .iter()
        .find(|c| c.offset == 0 && c.row == 0)
        .expect("the selected column");
    let centre_x = first.rect[0] + first.rect[2] * 0.5;
    let centre_y = first.rect[1] + first.rect[3] * 0.5;
    let at = |uv: [f32; 4]| {
        draws
            .iter()
            .position(|draw| match draw {
                Draw::Sprite { rect, uv: a, .. } | Draw::GradientSprite { rect, uv: a, .. } => {
                    *a == uv
                        && (rect[0] + rect[2] * 0.5 - centre_x).abs() < 1e-3
                        && (rect[1] + rect[3] * 0.5 - centre_y).abs() < 1e-3
                }
                _ => false,
            })
            .expect("drawn")
    };
    assert!(at(art) < at(lock));
}
