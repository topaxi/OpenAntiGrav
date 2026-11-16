---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-16
summary: "Race PNGs carry their race, --from-png remakes them; open: HUD row, serial, headless camera, DLC, finish and watched shots, modded tune bytes"
size: S (70)
difficulty: easy (80)
categories: [tooling, rendering]
---

# A screenshot names its race and `--from-png` remakes it; four gaps remain

Landed: [the format page](../../docs/formats/shot-png.md). Photo mode, `--race --screenshot` and a
drive script's `screenshot` write the `iTXt` `oag` text and the `oaGs` chunk; `--png-info` prints
the text and `--from-png` re-simulates, checks the hash and draws the frame.

Landed 2026-11-16 (lane `png-replay`): a shot opens in the replay viewer, paused on its tick, by
`--replay-png FILE` or a drop on the window over the main menu or an attract demo, refusals shown
over the screen ([replays](../../docs/ui/replays.md#a-screenshot-opens-the-viewer)); a `--race`
shot's header now names the opponent driver and the circuit, which the viewer needs.

## Open

- **Viewer follow-ups.** A drop is not reported by winit's Wayland backend (X11, Windows and macOS
  only); a shot from another title is refused with its name rather than switching disc (the
  session boots one title); a shot of a race flown with DLC packs mounted is opened with the packs
  this machine has mounted, and a mismatch shows as the tick-zero refusal rather than naming the
  pack (the same gap as the DLC item below); a `--race` shot's `[setup]` still lacks the team and
  hull variant the loader chose (the rebuild chooses the same default).

- **The HUD row is not in the text.** A photo taken with its HUD off (the default) is re-rendered
  with the HUD by `--from-png`. Add `shot.hud` to `Meta` (photo mode knows it) and a capture option
  that hides the HUD.
- **A headless screenshot has no `serial`.** `Loaded` carries the platform and no serial;
  `boot::Shell::serial` is only on the session's path.
- **DLC packs are not identified.** A race with a DLC pack mounted rebuilds to another hash
  without it (the first test run desynced at tick 60 until `data/dlc` was passed). The text lists
  the enabled mods but not the packs. ~~A simulation mod makes the race unrecordable~~ (landed
  2026-11-16, lane `shot-mods`): a modded race records, the header's `mods` list names each module
  with its SHA-256 and `--from-png` refuses a missing or other build by name; a headless
  screenshot's text now carries `[[mods]]` (id, version, sha256).
- **Modded replays and shots, still open** (README 4.2, 4.4): the tune bytes (no `setup.tune`
  exists), the lobby fingerprint's SHA-256, the cross-engine conformance test (a replay across
  Cranelift, Pulley and the browser is unproven), host-local faults still enter the hash (a
  recording whose module faulted on a pool or mmap failure desyncs elsewhere, reported by hash),
  manifest tables a race reads (pickup odds) are covered by the mod's `version` and not hashed, a
  ghost loaded with `--ghost` is still raced in a modded race, and the REPLAYS list's playback
  attaches the mods that are on and refuses by name a file whose modules are missing.
- **A headless screenshot's text lacks `[camera]`**; it has `[render]`. A drive
  `screenshot` outside photo mode has no `[camera]` either (take the eye and fov from
  `camera_position` and `camera_fov_degrees`, the look direction from the view matrix).
- **A Race End Photo shot has no stream** (the recording stops at the finish): cut it to the
  finish and re-simulate the ticks after it with no input, or keep the reason in the text.
- **A shot taken while watching a replay has no stream of its own**: its race is the replay's.
  The viewer holds the replay; cut it at the playhead the way a rewind does.

## Next Steps

1. `shot.hud` and the capture switch (an afternoon).
2. List the mounted DLC packs by name in `[title]`, and have `--from-png` warn when one is
   missing (an hour).
