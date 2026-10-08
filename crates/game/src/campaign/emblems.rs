//! `Cell Selection`'s emblem art on the campaign's sheet: the mode, class and
//! weapons icons, the corner mask, the barcode and the white emblem of every
//! circuit a cell names. See `oag_ui_screens::campaign::hd::cell_emblems`.

use std::collections::HashMap;

use oag_tables::race_campaign::Grid;
use oag_ui_screens::campaign::hd::{cell_brackets, cell_emblems};

use super::read_hd_texture;

/// A circuit's white emblem `src` by lowercased circuit id.
pub(super) fn circuit_emblems(
    tracks: &[oag_raceplay::catalogue::Track],
) -> HashMap<String, String> {
    tracks
        .iter()
        .map(|track| {
            (
                track.id.to_lowercase(),
                cell_emblems::track_emblem_src(&track.location),
            )
        })
        .collect()
}

/// Reads every picture the screen can name into `blobs`, keyed by the `src` it
/// is asked for under; one that will not read is logged and its widget draws
/// nothing.
pub(super) fn push_blobs(
    archives: &mut oag_assets::Archives,
    grids: &[Grid],
    circuit_emblems: &HashMap<String, String>,
    blobs: &mut Vec<(String, Vec<u8>)>,
) {
    let mut sources = cell_emblems::sheet_sources();
    sources.extend(
        grids
            .iter()
            .flat_map(|grid| &grid.cells)
            .filter_map(|cell| cell.track.as_deref())
            .filter_map(|id| circuit_emblems.get(&id.to_lowercase()).cloned()),
    );
    sources.push(cell_brackets::SRC.to_string());
    sources.push(cell_emblems::BARCODE_SRC.to_string());
    sources.sort_unstable();
    sources.dedup();
    for src in sources {
        match read_hd_texture(archives, &src) {
            Ok(blob) => blobs.push((src, blob)),
            Err(error) => log::warn!("{src}: {error:#} - the emblem it is for draws nothing"),
        }
    }
}
