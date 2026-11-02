---
status: measure
titles: [all]
updated: 2026-11-02
summary: "Launch fetch skip and Omega bank-once landed; Deck count and the web boot re-time remain"
difficulty: easy (35)
---

# Music runs off the frame thread; what is left

2026-10-19. No music read, listing or decode runs on the frame thread any more, and a
listing reads each track's head instead of the track. The paths, the probe and the Deck
numbers are in [`docs/architecture/music-threads.md`](../../docs/architecture/music-threads.md);
the ground truth is `crates/game/tests/music_resume_ground_truth.rs`.

## Open

- **Deck count of the skipped fetch.** Landed 2026-11-02 (`Audio::race_music_fetch`,
  `docs/architecture/music-threads.md`, "Landed"): a second launch on the same track spawns
  no fetch. Not re-measured on the Deck: run
  `./run.sh <label> 75 hdfury-ps3-eu-dec.iso --measure-race-load 2` with
  `RUST_LOG=info,oag_sound=debug` and expect one `read and decode on race-music` line, not two.
- **Resuming a parked race from the menus was not run on the Deck.** By the code it is
  a `race_cache` hit with no work at all (the test's "a resume from the menus" step), and
  scripting HD's front end over ssh was not tried.
- **The browser was not re-measured.** The front-end music now decodes on a Web Worker;
  the music survey still runs inline there. `just web` and the boot timing in
  `docs/tools/web.md` ("Freezes after the threads").
- **The launcher pick's load frame** still opens the archives and the front end on the
  frame thread. Not music's, and the frame is marked a load stall.

## Next Steps

1. Re-run the Deck command above when the Deck is offered (see Open).
2. Re-time the web boot in Chromium against the 0.16 to 0.24 s the music decode used to
   cost there.
