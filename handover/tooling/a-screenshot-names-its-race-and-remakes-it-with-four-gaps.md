---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-16
summary: "Race PNGs carry their race and --from-png remakes them; open: HUD row, headless serial, DLC identity, watched-replay shots"
size: S (70)
difficulty: easy (80)
categories: [tooling, rendering]
---

# A screenshot names its race and `--from-png` remakes it; four gaps remain

Landed: [the format page](../../docs/formats/shot-png.md). Photo mode, `--race --screenshot` and a
drive script's `screenshot` write the `iTXt` `oag` text and the `oaGs` chunk; `--png-info` prints
the text and `--from-png` re-simulates, checks the hash and draws the frame.

## Open

- **The HUD row is not in the text.** A photo taken with its HUD off (the default) is re-rendered
  with the HUD by `--from-png`. Add `shot.hud` to `Meta` (photo mode knows it) and a capture option
  that hides the HUD.
- **A headless screenshot has no `serial`.** `Loaded` carries the platform and no serial;
  `boot::Shell::serial` is only on the session's path.
- **DLC and mods are not identified.** A race with a DLC pack mounted rebuilds to another hash
  without it (the first test run desynced at tick 60 until `data/dlc` was passed). The text lists
  the enabled mods but not the packs; a simulation mod makes the race unrecordable, so the shot
  has no stream.
- **A shot taken while watching a replay has no stream of its own**: its race is the replay's.
  The viewer holds the replay; cut it at the playhead the way a rewind does.

## Next Steps

1. `shot.hud` and the capture switch (an afternoon).
2. List the mounted DLC packs by name in `[title]`, and have `--from-png` warn when one is
   missing (an hour).
