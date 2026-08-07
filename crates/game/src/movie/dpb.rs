//! H.264 reference picture bookkeeping: which decoded pictures a new one may
//! reference, and when an old one falls out of the window.
//!
//! Pure and dependency-free on purpose, the same split [`super::Ring`] uses:
//! everything that decides *which* frame_num is a reference lives here and is
//! unit-tested without a decoder, and [`super::vaapi`] is the thin shell that
//! hands this real slice-header fields and gets back a picture list.
//!
//! # What this does not do
//!
//! Scoped to exactly what the real content needs - see
//! [ADR-0016](../../../../docs/architecture/adr/0016-platform-native-h264-decode.md),
//! which profiled the PSP intro directly: Main profile, I and P slices only,
//! zero B slices, no field pictures. So this implements only:
//!
//! - The default (unmodified) `RefPicList0` construction for a P slice,
//!   ordered by descending `PicNum` (spec 8.2.4.2.1).
//! - The sliding-window reference marking process (spec 8.2.5.3): the DPB
//!   holds at most `num_ref_frames` short-term references, and an IDR clears
//!   it outright.
//!
//! It does **not** implement `ref_pic_list_modification`, adaptive memory
//! control (MMCO), long-term references, B slices, or field/MBAFF coding.
//! [`Dpb::insert`] and [`Dpb::ref_pic_list0`] are correct only for content
//! that never exercises those; a caller parsing a real slice header is
//! responsible for checking for them and refusing rather than silently
//! mis-decoding - see `vaapi::VaapiDecoder`'s slice-header validation.

// `vaapi::VaapiDecoder`, the only intended caller, does not exist yet - see
// ADR-0016. Every item here is exercised by its own unit tests instead, so
// dead-code analysis does not get to stand in for that decoder actually
// being written. Drop this once `vaapi.rs` calls in.
#![allow(dead_code)]

use std::fmt;

/// One decoded picture the DPB is holding onto as a short-term reference.
#[derive(Debug, Clone, Copy)]
struct Reference<P> {
    /// The picture's own coded `frame_num`, not yet unwrapped against
    /// anything - see [`frame_num_wrap`].
    frame_num: u32,
    /// Whatever the caller uses to name the picture - a VA surface ID in
    /// `vaapi::VaapiDecoder`, a plain integer in tests.
    picture: P,
}

/// `frame_num` unwrapped relative to `current`, a.k.a. `PicNum` for a frame
/// picture (spec 8.2.4.1, no MBAFF/field coding).
///
/// `frame_num` is `log2_max_frame_num_minus4 + 4` bits wide and wraps at
/// `max_frame_num`. A reference coded *before* the wrap has a `frame_num`
/// that now reads as larger than the current picture's, even though it is
/// older - subtracting `max_frame_num` from it is what makes "smallest
/// `FrameNumWrap`" mean "oldest" again regardless of where the wrap fell.
fn frame_num_wrap(current: u32, reference: u32, max_frame_num: u32) -> i64 {
    if reference > current {
        i64::from(reference) - i64::from(max_frame_num)
    } else {
        i64::from(reference)
    }
}

/// The decoded picture buffer's short-term reference set, for a P-slice-only
/// stream. See the module documentation for what this deliberately does not
/// cover.
///
/// `P` is whatever the caller names a decoded picture by - kept generic so
/// this stays free of any VA-API or `cros-libva` type.
#[derive(Debug, Clone)]
pub struct Dpb<P> {
    refs: Vec<Reference<P>>,
    /// From the SPS: how many short-term references the sliding window
    /// keeps. At least one, even if the SPS somehow declared zero - a
    /// stream with `num_ref_frames == 0` still needs the picture it most
    /// recently decoded to fall out immediately rather than never entering.
    max_num_ref_frames: usize,
    /// `2^(log2_max_frame_num_minus4 + 4)`, from the SPS.
    max_frame_num: u32,
}

impl<P: Copy + fmt::Debug> Dpb<P> {
    /// An empty DPB for a stream whose SPS declares these two values.
    #[must_use]
    pub fn new(max_num_ref_frames: usize, max_frame_num: u32) -> Self {
        Self {
            refs: Vec::new(),
            max_num_ref_frames: max_num_ref_frames.max(1),
            max_frame_num,
        }
    }

    /// How many short-term references are currently held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.refs.len()
    }

    /// Whether nothing is held yet - true right after construction and right
    /// after an IDR.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.refs.is_empty()
    }

    /// The default `RefPicList0` for a P slice belonging to the picture named
    /// `current_frame_num`: every held reference, most recently coded first.
    ///
    /// "Most recently coded" is by `PicNum` (descending), not by insertion
    /// order, precisely because a wrap can make an older `frame_num` numerically
    /// larger - see [`frame_num_wrap`]. An empty list means either nothing has
    /// been decoded yet or the last picture was an IDR; either way the caller
    /// has no valid P slice to decode against it.
    #[must_use]
    pub fn ref_pic_list0(&self, current_frame_num: u32) -> Vec<P> {
        let mut ordered: Vec<(i64, P)> = self
            .refs
            .iter()
            .map(|r| {
                (
                    frame_num_wrap(current_frame_num, r.frame_num, self.max_frame_num),
                    r.picture,
                )
            })
            .collect();
        ordered.sort_by_key(|entry| std::cmp::Reverse(entry.0));
        ordered.into_iter().map(|(_, picture)| picture).collect()
    }

    /// Records that `picture` (coded as `frame_num`) has been decoded.
    ///
    /// `idr` clears every existing reference first, per spec: an IDR picture
    /// cannot be predicted from anything before it, so nothing before it may
    /// go on being one either. `is_reference` is `nal_ref_idc != 0` from the
    /// NAL header; a disposable picture (`nal_ref_idc == 0`) is decoded and
    /// displayed like any other but never becomes a reference, so it is not
    /// stored here at all.
    ///
    /// The sliding window (spec 8.2.5.3) runs before insertion: if the DPB is
    /// already at `max_num_ref_frames`, the reference with the smallest
    /// `PicNum` relative to `frame_num` - the oldest, wrap accounted for - is
    /// evicted to make room.
    pub fn insert(&mut self, frame_num: u32, picture: P, is_reference: bool, idr: bool) {
        if idr {
            self.refs.clear();
        }
        if !is_reference {
            return;
        }
        if self.refs.len() >= self.max_num_ref_frames {
            let oldest = self
                .refs
                .iter()
                .enumerate()
                .min_by_key(|(_, r)| frame_num_wrap(frame_num, r.frame_num, self.max_frame_num))
                .map(|(index, _)| index);
            if let Some(index) = oldest {
                self.refs.remove(index);
            }
        }
        self.refs.push(Reference { frame_num, picture });
    }
}

#[cfg(test)]
mod tests {
    use super::Dpb;

    /// `log2_max_frame_num_minus4 = 0`, the smallest legal value: `frame_num`
    /// wraps at 16. Small on purpose, so a test can walk past a wrap in a few
    /// lines rather than sixteen thousand.
    const MAX_FRAME_NUM: u32 = 16;

    #[test]
    fn a_fresh_dpb_has_no_references() {
        let dpb: Dpb<u32> = Dpb::new(4, MAX_FRAME_NUM);
        assert!(dpb.is_empty());
        assert_eq!(dpb.ref_pic_list0(0), Vec::<u32>::new());
    }

    #[test]
    fn a_disposable_picture_is_never_stored() {
        let mut dpb: Dpb<u32> = Dpb::new(4, MAX_FRAME_NUM);
        dpb.insert(0, 100, false, false);
        assert!(
            dpb.is_empty(),
            "nal_ref_idc == 0 must not become a reference"
        );
    }

    #[test]
    fn an_idr_clears_whatever_came_before_it() {
        let mut dpb: Dpb<u32> = Dpb::new(4, MAX_FRAME_NUM);
        dpb.insert(0, 100, true, false);
        dpb.insert(1, 101, true, false);
        assert_eq!(dpb.len(), 2);

        dpb.insert(0, 200, true, true);
        assert_eq!(dpb.len(), 1, "the IDR itself is still inserted");
        assert_eq!(dpb.ref_pic_list0(0), vec![200]);
    }

    #[test]
    fn list0_orders_by_descending_pic_num_not_insertion_order() {
        let mut dpb: Dpb<u32> = Dpb::new(4, MAX_FRAME_NUM);
        // Inserted out of frame_num order, on purpose.
        dpb.insert(2, 102, true, false);
        dpb.insert(0, 100, true, false);
        dpb.insert(1, 101, true, false);

        assert_eq!(
            dpb.ref_pic_list0(3),
            vec![102, 101, 100],
            "most recently coded (highest PicNum) first"
        );
    }

    #[test]
    fn the_sliding_window_evicts_the_oldest_frame_num_once_full() {
        let mut dpb: Dpb<u32> = Dpb::new(2, MAX_FRAME_NUM);
        dpb.insert(0, 100, true, false);
        dpb.insert(1, 101, true, false);
        assert_eq!(dpb.len(), 2, "at capacity, nothing evicted yet");

        dpb.insert(2, 102, true, false);
        assert_eq!(dpb.len(), 2, "capacity is a hard bound");
        assert_eq!(
            dpb.ref_pic_list0(2),
            vec![102, 101],
            "frame_num 0 (100) was the oldest and is gone"
        );
    }

    #[test]
    fn a_zero_capacity_sps_still_lets_the_newest_picture_reference_nothing_stale() {
        // A degenerate but legal SPS: num_ref_frames == 0. Clamped to 1 so a
        // reference picture is at least visible to the very next one, then
        // evicted before the one after that.
        let mut dpb: Dpb<u32> = Dpb::new(0, MAX_FRAME_NUM);
        dpb.insert(0, 100, true, false);
        assert_eq!(dpb.ref_pic_list0(1), vec![100]);

        dpb.insert(1, 101, true, false);
        assert_eq!(
            dpb.ref_pic_list0(2),
            vec![101],
            "100 has fallen out already"
        );
    }

    #[test]
    fn frame_num_wrapping_still_orders_the_wrapped_reference_as_older() {
        // MAX_FRAME_NUM is 16. A reference coded at frame_num 15 just before
        // the counter wraps, then the current picture is frame_num 1 (having
        // wrapped through 0). Naively comparing raw frame_num would rank 15
        // as newer than 1; FrameNumWrap must not.
        let mut dpb: Dpb<u32> = Dpb::new(4, MAX_FRAME_NUM);
        dpb.insert(15, 115, true, false);
        dpb.insert(0, 200, true, false);

        assert_eq!(
            dpb.ref_pic_list0(1),
            vec![200, 115],
            "200 (frame_num 0, coded after the wrap) is newer than 115 (frame_num 15, before it)"
        );
    }

    #[test]
    fn the_sliding_window_respects_the_wrap_too() {
        let mut dpb: Dpb<u32> = Dpb::new(2, MAX_FRAME_NUM);
        dpb.insert(14, 114, true, false);
        dpb.insert(15, 115, true, false);
        // Wraps: frame_num goes 15 -> 0. Relative to frame_num 0, 14's
        // FrameNumWrap (14 - 16 = -2) is older than 15's (15 - 16 = -1), so
        // 14 is the one evicted, not 15 despite 15 having the larger raw
        // frame_num.
        dpb.insert(0, 100, true, false);

        assert_eq!(dpb.ref_pic_list0(0), vec![100, 115]);
    }
}
