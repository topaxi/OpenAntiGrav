//! The `Dynamic HUD` warp: `FE_CONCAVE` bends every HUD vertex towards the screen's middle row.
//!
//! Pulse's `Hud_WarpVertexY` (`0x08919c2c`) moves a vertex's `y` alone, by
//! `35 * sin(x * pi / 480) * (1 - y / 136)` in the PSP's 480x272 pixels: nothing at the left and
//! right edges, most at the middle column, pulling the top edge down and the bottom edge up and
//! leaving the middle row where it was. `Image_Draw` runs it over an image's vertices (its four
//! corners) and the glyph builder over each glyph's four corners, so a picture bends as a
//! quadrilateral; only `SpeedBarBg`, `SpeedBar`, `ShieldBarBg` and `ShieldBar` are cut into four
//! columns first (`Hud_UpdateSpeedBar`, ten vertices against four), so the bars curve. Live on
//! PPSSPP, the shield bar's top edge moved by the law to within a pixel at six columns
//! (`docs/ghidra/functions/psp-pulse-usa/hud-game-options.md`, "The warp").
//!
//! The PS2 pressing runs the same law (`FUN_001e5e98`): the same 35.0 (`0x00282ab0`), its centre
//! row half the screen's height and its period the screen's width, both read from the display's
//! globals (`0x00284fe0`, `0x00284fe4`), so on its 640x448 grid the edges move 35 of 448.
//!
//! Written over any grid, `x / width` and `y / height`, with the amplitude in the grid's units:
//! [`AMPLITUDE`] on Pulse's grids, [`amplitude_for`] scaling it to the share of the PSP's height on
//! a title that never had the warp.

/// How far the middle column's top and bottom edges move, in grid units, on both Pulse pressings.
pub const AMPLITUDE: f32 = 35.0;
/// The PSP's screen height, which the amplitude is a share of on a title without the warp.
const PSP_HEIGHT: f32 = 272.0;
/// How many columns a bar named in [`COLUMNED`] is cut into: `(10 / 2) - 1`.
pub const COLUMNS: usize = 4;
/// The images Pulse draws as a strip of [`COLUMNS`] columns while the warp is on.
pub const COLUMNED: [&str; 4] = ["SpeedBarBg", "SpeedBar", "ShieldBarBg", "ShieldBar"];

/// The amplitude on a grid `height` tall: [`AMPLITUDE`] where the title authors the warp (Pulse,
/// both pressings), else the same share of the screen's height as on the PSP.
#[must_use]
pub fn amplitude_for(authored: bool, height: f32) -> f32 {
    if authored {
        AMPLITUDE
    } else {
        AMPLITUDE * height / PSP_HEIGHT
    }
}

/// How far the warp moves a vertex at `(x, y)` down (negative: up), in grid units, on a grid of
/// `size` with the middle column's edges moving `amplitude`.
#[must_use]
pub fn dy(x: f32, y: f32, size: (f32, f32), amplitude: f32) -> f32 {
    let (width, height) = size;
    if width <= 0.0 || height <= 0.0 {
        return 0.0;
    }
    let half = height * 0.5;
    amplitude * (std::f32::consts::PI * x / width).sin() * (1.0 - y / half)
}

/// [`dy`] at the four corners of `rect` (`[x, y, width, height]`), in the order top-left,
/// top-right, bottom-left, bottom-right.
#[must_use]
pub fn corners(rect: [f32; 4], size: (f32, f32), amplitude: f32) -> [f32; 4] {
    let [x, y, w, h] = rect;
    [
        dy(x, y, size, amplitude),
        dy(x + w, y, size, amplitude),
        dy(x, y + h, size, amplitude),
        dy(x + w, y + h, size, amplitude),
    ]
}

/// `rect` and its sheet window `uv` (`[x, y, width, height]` each) cut into [`COLUMNS`] equal
/// columns, left to right, each column's window the matching share of `uv`. The shared edges
/// are computed once, so neighbouring columns meet without a seam.
#[must_use]
pub fn columns(rect: [f32; 4], uv: [f32; 4]) -> [([f32; 4], [f32; 4]); COLUMNS] {
    let edge = |i: usize| i as f32 / COLUMNS as f32;
    std::array::from_fn(|i| {
        let (a, b) = (edge(i), edge(i + 1));
        let x0 = rect[0] + rect[2] * a;
        let x1 = rect[0] + rect[2] * b;
        let u0 = uv[0] + uv[2] * a;
        let u1 = uv[0] + uv[2] * b;
        ([x0, rect[1], x1 - x0, rect[3]], [u0, uv[1], u1 - u0, uv[3]])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PSP: (f32, f32) = (480.0, 272.0);
    const A: f32 = AMPLITUDE;

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1.0e-3
    }

    /// The constants read off `0x08abf580` (35.0), `0x08abf588` (136.0) and `0x08abf58c`
    /// (`pi / 480`): the middle column's top edge comes down 35, its bottom edge up 35.
    #[test]
    fn the_middle_column_moves_its_top_down_and_bottom_up_by_35() {
        assert!(near(dy(240.0, 0.0, PSP, A), 35.0));
        assert!(near(dy(240.0, 272.0, PSP, A), -35.0));
        assert!(near(dy(240.0, 136.0, PSP, A), 0.0));
    }

    #[test]
    fn the_left_and_right_edges_do_not_move() {
        for y in [0.0, 60.0, 200.0, 272.0] {
            assert!(near(dy(0.0, y, PSP, A), 0.0));
            assert!(dy(480.0, y, PSP, A).abs() < 1.0e-4);
        }
    }

    /// The live check: the shield bar's top edge at `y = 247` on PPSSPP moved by these at
    /// `x = 310, 360, 420` (measured `-25, -20, -11`).
    #[test]
    fn the_shield_bar_columns_move_as_the_original_measured() {
        for (x, measured) in [(310.0, -25.0), (360.0, -20.0), (420.0, -11.0)] {
            let law: f32 = dy(x, 247.0, PSP, A);
            assert!(
                (law - measured).abs() <= 1.0,
                "x {x}: {law} against {measured}"
            );
        }
    }

    /// A title without the warp bends the same share of its screen as the PSP; the PS2 pressing
    /// authors its own 35 on a 448-tall grid.
    #[test]
    fn a_title_without_the_warp_bends_the_psp_share_and_the_ps2_its_own_35() {
        let hd = (1280.0, 720.0);
        let scaled = amplitude_for(false, hd.1);
        assert!(near(dy(640.0, 0.0, hd, scaled), 35.0 * 720.0 / 272.0));
        assert!(near(
            dy(320.0, 180.0, hd, scaled) / 720.0,
            dy(120.0, 68.0, PSP, A) / 272.0
        ));
        let ps2 = (640.0, 448.0);
        assert!(near(dy(320.0, 0.0, ps2, amplitude_for(true, ps2.1)), 35.0));
    }

    #[test]
    fn corners_are_top_left_top_right_bottom_left_bottom_right() {
        let c = corners([200.0, 10.0, 80.0, 250.0], PSP, A);
        assert_eq!(c[0], dy(200.0, 10.0, PSP, A));
        assert_eq!(c[1], dy(280.0, 10.0, PSP, A));
        assert_eq!(c[2], dy(200.0, 260.0, PSP, A));
        assert_eq!(c[3], dy(280.0, 260.0, PSP, A));
    }

    #[test]
    fn four_columns_tile_the_rect_and_its_window() {
        let cut = columns([300.0, 240.0, 140.0, 8.0], [10.0, 20.0, 70.0, 4.0]);
        assert_eq!(cut[0].0[0], 300.0);
        for pair in cut.windows(2) {
            assert!(near(pair[0].0[0] + pair[0].0[2], pair[1].0[0]));
            assert!(near(pair[0].1[0] + pair[0].1[2], pair[1].1[0]));
        }
        assert!(near(cut[3].0[0] + cut[3].0[2], 440.0));
        assert!(near(cut[3].1[0] + cut[3].1[2], 80.0));
        assert!(
            cut.iter()
                .all(|(r, uv)| r[1] == 240.0 && r[3] == 8.0 && uv[3] == 4.0)
        );
    }
}
