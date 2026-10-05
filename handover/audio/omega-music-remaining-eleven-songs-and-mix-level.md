# Omega music plays 17 of 29 songs; eight-channel songs and the mix level are open

2026-10-05. A race plays the `PI_Music` songs whose segment is stereo stems, and
the front end plays its loop. Evidence is in `docs/formats/wwise.md`, "Music".

## Open

- 11 songs are one eight-channel ATRAC9 file; `atrac9dec` refuses them, so they
  are listed (`omega::Skip::Channels`) and not played. FFmpeg decodes them.
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

1. Decode the eight-channel files (FFmpeg out of process, or a fork of the
   decoder), mix them down, and add the 11 songs to the playlist.
2. Read a track's base parameters (property 13, positioning) and check the fold
   against a capture, if one ever exists.
3. Read the third switch group and settle `PlaylistId`.
