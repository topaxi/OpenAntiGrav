//! Where the downloadable campaigns' nodes sit on the campaign map.
//!
//! The two packages open two more campaigns, HD's and Fury's, whose cells are
//! Wipeout HD's own `PI_Cell`s (`Data\Plugins\grids\grid_NN.xml`, tagged
//! `Campaign="HD"` and `"Fury"`). The executable puts one hotspot per cell on
//! the same map the single-player events sit on:
//! `FE3DCanvas_AddHDCampaignEventButtons` (`0x810f817c`) and
//! `FE3DCanvas_AddFuryCampaignEventButtons` (`0x810f5000`) project a node
//! through one scale and bias and a per-grid offset
//! (`docs/ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md`,
//! confidence 84 - read off the initialiser's immediates, no memory dump).
//!
//! **Measured**: the scale, the bias and the sixteen per-grid offsets below,
//! and that they put every grid's nodes in the band just under its own group
//! wordmark: the eight HD and eight Fury `<CanvasLabel>`s `data2.psarc`'s
//! `NEWGUI/Definition.xml` authors (`UpliftLabel` at `(57, 27)`, `FrenzyLabel`
//! at `(99, 29)`, `MeltdownLabel` at `(96, 52)`, ...) each sit one to two units
//! above the first row of the nodes this places for their grid. **Chosen, not
//! measured**: what a canvas unit is on the screen, and that the bias is left
//! off here - [`step`] puts a grid's first node near the origin of its own
//! map, since the bias is the same for every node of a campaign and moves the
//! whole cluster rather than any node against another.

/// The `X` and `Y` scale of a node's cell: a cell step is three canvas units
/// across and two down.
pub const SCALE: (f32, f32) = (3.0, 2.0);

/// The constant added to every node's position, in canvas units. The same for
/// the HD and the Fury tier.
pub const BIAS: (f32, f32) = (55.0, 25.0);

/// The offset of the grid with index `n` (`grid0`..`grid15`), in canvas units.
/// HD's eight grids are `0..8` (the hotspot's node types `0..8`, table at
/// `0x818808e4`), Fury's `8..16` (types `8..16`, table at `0x81880670`).
pub const GROUP_OFFSETS: [(f32, f32); 16] = [
    (1.0, 1.0),
    (20.0, 6.0),
    (44.0, 4.0),
    (68.0, 6.0),
    (64.0, 16.0),
    (37.0, 17.0),
    (6.0, 18.0),
    (37.0, 27.0),
    (1.0, 1.0),
    (21.0, 3.0),
    (42.0, 0.0),
    (61.0, 5.0),
    (75.0, 11.0),
    (52.0, 16.0),
    (28.0, 16.0),
    (5.0, 17.0),
];

/// A node's position in canvas units: [`BIAS`] plus its grid's offset plus the
/// cell's own `(x, y)` scaled by [`SCALE`]. `None` for a grid past the sixteen
/// the executable has a table row for.
#[must_use]
pub fn node(grid: usize, cell: (u32, u32)) -> Option<(f32, f32)> {
    let (dx, dy) = *GROUP_OFFSETS.get(grid)?;
    #[expect(
        clippy::cast_precision_loss,
        reason = "a cell coordinate is a single digit"
    )]
    let (cx, cy) = (cell.0 as f32, cell.1 as f32);
    Some((BIAS.0 + dx + SCALE.0 * cx, BIAS.1 + dy + SCALE.1 * cy))
}

/// A node's position in cell steps, with the first row and column of the map at
/// `1`, the way a single-player event's `M_X` and `M_Y` are: [`node`] less
/// [`BIAS`], over [`SCALE`], and one more. `None` as [`node`].
#[must_use]
pub fn step(grid: usize, cell: (u32, u32)) -> Option<(f32, f32)> {
    let (x, y) = node(grid, cell)?;
    Some(((x - BIAS.0) / SCALE.0 + 1.0, (y - BIAS.1) / SCALE.1 + 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_node_is_the_bias_plus_the_grid_offset_plus_the_scaled_cell() {
        // `grid1_3_2`, Warped's third column and second row.
        assert_eq!(node(1, (3, 2)), Some((55.0 + 20.0 + 9.0, 25.0 + 6.0 + 4.0)));
    }

    #[test]
    fn a_grid_the_executable_has_no_row_for_has_no_node() {
        assert_eq!(node(16, (1, 1)), None);
    }
}
