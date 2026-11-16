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

/// The frame of a `len`-frame looping movie a still shows: `wanted` wrapped into
/// the loop, or the first frame when no frame was asked for.
pub(super) fn still_frame(wanted: Option<usize>, len: usize) -> usize {
    wanted.unwrap_or(0).checked_rem(len).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::still_frame;

    #[test]
    fn a_still_shows_the_frame_it_asked_for_and_wraps_at_the_loop() {
        assert_eq!(still_frame(None, 270), 0);
        assert_eq!(still_frame(Some(38), 270), 38);
        assert_eq!(still_frame(Some(270), 270), 0);
        assert_eq!(still_frame(Some(308), 270), 38);
    }

    #[test]
    fn a_movie_with_no_frames_asks_for_the_first() {
        assert_eq!(still_frame(Some(38), 0), 0);
    }
}
