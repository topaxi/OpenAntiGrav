---
status: measure
titles: [hd]
updated: 2026-11-05
summary: "HD Fury's updates change audio mix values and three string IDs; decide whether the engine reads an installed chain"
categories: [audio, tooling]
---

# The update chain's audio config and strings are not loaded

`docs/formats/hd-updates.md` has the package and archive table. Size: an hour to diff, a short
lane to load (confidence 50).

## Open

- `data07` carries `globalaudioconfig.xml` (`user1` `1.1` to `1.2` and more) and eight per-environment
  `audioconfig.xml`, zones included. `oag_sound::hd_mix` reads the disc's. Which is right for a
  player who has updated: the update's, by any reading of "what the original does".
- `oag_title::ArchiveCandidates::patch` stays empty for HD. Disc players can only have these
  through an RPCS3 install tree, and only if the chain is complete.

## Next Steps

1. Diff `data07` audio files against the disc in full and quantify audible change. If it is only
   `user1`, close this thread with a note.
2. If it matters: an archive-candidate route for `data07`/`data08` from a `dev_hdd0` tree, plus a
   loader report line when the chain is partial (02.51 alone has neither archive).
3. Say nothing about the 3D rows: stereo is not planned.
