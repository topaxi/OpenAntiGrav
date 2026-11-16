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
//! at `(99, 29)`, `MeltdownLabel` at `(96, 52)`, ...) each sit within a unit of
//! the first row of the nodes this places for their grid
//! (`vita_2048_dlc_menus_ground_truth`).
//!
//! A canvas unit is a cell of the `Canvas3D` triangle lattice: a node is the
//! column and row of the first of the six cells its hexagon is painted on, and
//! the single-player events sit on the same canvas through [`MAP`]
//! (`docs/ghidra/functions/vita-2048-eu-v104/campaign-map-projection.md`,
//! "The law").

use oag_title::NodeMap;

/// The projection both campaigns share, by grid index: HD's eight grids are
/// `0..8` (the hotspot's node types `0..8`, table at `0x818808e4`), Fury's
/// `8..16` (types `8..16`, table at `0x81880670`). A cell step is three canvas
/// units across and two down.
pub const NODES: NodeMap = NodeMap {
    scale: (3, 2),
    bias: (55, 25),
    group_offsets: &[
        (1, 1),
        (20, 6),
        (44, 4),
        (68, 6),
        (64, 16),
        (37, 17),
        (6, 18),
        (37, 27),
        (1, 1),
        (21, 3),
        (42, 0),
        (61, 5),
        (75, 11),
        (52, 16),
        (28, 16),
        (5, 17),
    ],
};

/// The single-player map's projection: an event's `M_X`/`M_Y` step is four
/// columns across and two rows down from the same bias, with no grid offset
/// (the quad at `0x81880448`, confidence 92), and its `M_CANVASTWEAK_X`/`_Y`
/// is added in canvas units ([`NodeMap::nudged`]).
pub const MAP: NodeMap = NodeMap {
    scale: (4, 2),
    bias: (55, 25),
    group_offsets: &[(0, 0)],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_events_cell_is_the_quad_plus_its_tweak() {
        // `2048 - Event 4` sits at (6, 6); read live as cell (79, 37).
        assert_eq!(MAP.nudged(0, (6, 6), (0, 0)), Some((79, 37)));
        // An event at (4, 8) nudged by (-1, -1); read live as cell (70, 40).
        assert_eq!(MAP.nudged(0, (4, 8), (-1, -1)), Some((70, 40)));
        assert_eq!(MAP.nudged(0, (-1, 3), (0, 0)), None);
    }

    #[test]
    fn a_node_is_the_bias_plus_the_grid_offset_plus_the_scaled_cell() {
        // `grid1_3_2`, Warped's third column and second row.
        assert_eq!(NODES.node(1, (3, 2)), Some((55 + 20 + 9, 25 + 6 + 4)));
    }

    #[test]
    fn a_grid_the_executable_has_no_row_for_has_no_node() {
        assert_eq!(NODES.node(16, (1, 1)), None);
    }
}
