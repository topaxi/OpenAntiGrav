//! Reading a draw list's video draw, split out of `capture.rs` under the size ratchet.

use oag_ui::frontend::Draw;

/// Which frame of its movie the list's video draw wants, if it has one.
pub(super) fn video_frame(list: &[Draw]) -> Option<usize> {
    list.iter().find_map(|draw| match draw {
        Draw::Video { frame, .. } => Some(*frame),
        _ => None,
    })
}

/// Which movie the list's video draw wants a frame of, if it has one.
pub(super) fn video_source(list: &[Draw]) -> Option<oag_ui::frontend::Video> {
    list.iter().find_map(|draw| match draw {
        Draw::Video { source, .. } => Some(*source),
        _ => None,
    })
}
