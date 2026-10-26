---
status: ready-to-wire
titles: [all]
updated: 2026-10-19
summary: "Skip the launch music fetch on a race_cache hit; re-time the web boot; Omega re-plans per track"
---

# Music runs off the frame thread; what is left

2026-10-19. No music read, listing or decode runs on the frame thread any more, and a
listing reads each track's head instead of the track. The paths, the probe and the Deck
numbers are in [`docs/architecture/music-threads.md`](../../docs/architecture/music-threads.md);
the ground truth is `crates/game/tests/music_resume_ground_truth.rs`.

## Open

- **A menu-launched race re-decodes a track it already holds.** `Session::launch_race`
  always spawns a `MusicFetchWorker`, so a second race on the same track decodes it again
  (571 to 614 ms on the Deck, on the worker). Off the frame thread; costs CPU beside the
  loading screen. Check `race_cache` before spawning. Measure:
  `./run.sh <label> 75 hdfury-ps3-eu-dec.iso --measure-race-load 2` on the Deck with
  `RUST_LOG=info,oag_sound=debug`, count the `read and decode on race-music` lines.
- **Omega's `load_entry` re-plans per track**, re-reading and parsing the Wwise bank on
  the worker each time. Keep the plan the way `Listings` keeps a listing.
- **Resuming a parked race from the menus was not run on the Deck.** By the code it is
  a `race_cache` hit with no work at all (the test's "a resume from the menus" step), and
  scripting HD's front end over ssh was not tried.
- **The browser was not re-measured.** The front-end music now decodes on a Web Worker;
  the music survey still runs inline there. `just web` and the boot timing in
  `docs/tools/web.md` ("Freezes after the threads").
- **The launcher pick's load frame** still opens the archives and the front end on the
  frame thread. Not music's, and the frame is marked a load stall.

## Next Steps

1. Skip the launch fetch on a `race_cache` hit (about 20 lines in
   `crates/game/src/main/session/load.rs` and `Audio::finish_race_music`), then re-run the
   Deck command above.
2. Re-time the web boot in Chromium against the 0.16 to 0.24 s the music decode used to
   cost there.
