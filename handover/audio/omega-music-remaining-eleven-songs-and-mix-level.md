---
status: re-first
titles: [omega]
updated: 2026-11-23
summary: "The 11 eight-channel songs play folded by a chosen downmix; read the track base parameters (property 13) that set the real fold and mix level"
difficulty: medium (35)
---

# Omega music plays 28 of 29 songs; the fold and the mix level are chosen, not measured

2026-10-05. A race plays the `PI_Music` songs whose segment is stereo stems, and
the front end plays its loop. Evidence is in `docs/formats/wwise.md`, "Music".

## Open

- 11 songs are one eight-channel ATRAC9 file. They play since 2026-11-23
  through `oag_music::omega::fold_to_stereo` (front pair 0 dB, centre and
  sides -3 dB, backs -6 dB, LFE dropped), **chosen, not measured**: how the
  original folds 7.1 to stereo is unread. The folded songs sit at 0.26-0.80
  (median about 0.6) of a stereo song's RMS, `docs/formats/wwise.md`,
  "Eight-channel streams".
- Location 00 has no `Set_Music_Track_0__frontend` event in the bank: unplayed.
- The mix is a unity sum of stems and a scale-down when it would clip (chosen,
  not measured). The track property id 13 (-100 to 100) looks like a speaker
  position; how the original folds it to stereo is unread. A PS4 capture is the
  only arbiter.
- The 19-state and 9-state `Music_Track` switches may be `PlaylistId` 1 and 2;
  unchecked. The race plays the all-states branch in `PI_Music` order.
- Which of `Menus` and `Loading` is in force when, and whether the loop repeats
  by ranseq loop count or by the segment, is unread.
- Candidate follow-up: move `at3`, `at9`, `mp3`, `wem`, `music` and
  `catalogue::music` out of `oag-game` into an `oag-music` crate (no behaviour
  change, verified by the same race WAV hashes).

## Next Steps

1. ~~Choose a 7.1-to-stereo fold and add the 11 songs to the playlist.~~ Done
   (2026-11-23).
2. Read a track's base parameters (property 13, positioning) and check the fold
   and its level against a capture, if one ever exists; `fold_to_stereo` is the
   one function to replace.
3. Read the third switch group and settle `PlaylistId`.
