---
status: re-first
titles: [pulse, hd]
updated: 2026-10-26
summary: "Pulse's pre-race voice-over is unplayed; HD's song start and bar tint are read, not watched"
---

# Now-playing display: all five titles wired, what is left

2026-10-26, lane `hud-now-playing`. The song's `Artist - name` scrolls through a bar on the race
HUD on Pulse, HD/Fury and Omega (authored) and on Pure and 2048 (borrowed, chosen not measured).
How and what differs: `docs/formats/now-playing.md`. Evidence:
`docs/ghidra/functions/psp-pulse-usa/now-playing.md`,
`docs/ghidra/functions/ps3-hdfury-eu/now-playing.md`.

## Open

- Pulse's fly-over plays a pre-race voice-over through the music player
  (`MusicPlayer_StartPreRaceDialogue`, `Data\Sound\PreRaceDialogue\%d\...`); it is not played
  here, so the fly-over is silent (`docs/gameplay/race-intro.md`).
- The playlist resumes mid-track at the green light after a visit to the menus; the originals
  start the next song (**deliberate: the maintainer's resume decision**,
  `docs/architecture/race-music-resume.md`). A resume is not a new song, so the marquee depends on
  whether the race before it showed one.
- The race song's start at the release (`docs/formats/now-playing.md`, "When the song starts")
  is watched on PPSSPP for Pulse's display only (the silence before it was not heard, the capture
  being muted); HD's is read off the executable, not watched; Pure, Omega and 2048 inherit.
- Pulse's `Dynamic HUD` list also has `FE_CONCAVE` and `FE_FLAT`, which flag the HUD's two panel
  groups (bit 1 of `+0xd0` on `hud+0x274`/`+0x278`); what they draw is undecoded, so the DYNAMIC
  HUD row offers `default` and `off` only (`docs/ghidra/functions/psp-pulse-usa/now-playing.md`).
  Whether the PS2's Options screen carries the same list is unread (its archive has no names).
- HD's black bar tint and its per-update step are read from code, not confirmed on a frame of
  the original (an RPCS3 capture; Pulse's step is watched).
- Omega's executable has not been read for the display; HD's law stands in.
- A PS2 boot of Pulse shows nothing: the PS2 archive has no names.
- Pure's borrowed bar sits over the craft's exhaust on the chase view; 2048's bar and text size
  are picked, not measured.

## Next Steps

- Watch HD's song start, bar tint and update step on a muted RPCS3 capture, to move its start
  rule and its bar from read to watched.
