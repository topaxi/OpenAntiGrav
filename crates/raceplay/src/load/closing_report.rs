//! The last lines of a load report: what the asset layers answered and what the
//! read memo kept.

use oag_assets::Archives;

/// Appends every name an asset layer answered (the player's override or this
/// project's fallback, `oag_assets::layers`) for the track's archives and the
/// craft's, then the memo summary. A mod or a substitute is never silent; a load
/// no layer answered for adds only the summary.
pub(super) fn finish(
    report: &mut Vec<String>,
    archives: &mut Archives,
    craft: Option<&mut Archives>,
) {
    report.extend(archives.take_layer_report());
    if let Some(craft) = craft {
        report.extend(craft.take_layer_report());
    }
    report.extend(archives.read_memo_summary());
}
