# Now-playing display: all five titles wired, what is left

2026-10-26, lane `hud-now-playing`. The song's `Artist - name` scrolls through a bar on the race
HUD on Pulse, HD/Fury and Omega (authored) and on Pure and 2048 (borrowed, chosen not measured).
How and what differs: `docs/formats/now-playing.md`. Evidence:
`docs/ghidra/functions/psp-pulse-usa/now-playing.md`,
`docs/ghidra/functions/ps3-hdfury-eu/now-playing.md`.

## Open

- The race music starts at load; the originals start it, and the display, at the green light.
  The display waits for the light, the sound does not.
- No `Dynamic HUD` setting to hide the display (Pulse and HD gate on it), and no marquee on a
  split screen.
- The 60 Hz step and HD's black bar tint are read from code, not confirmed on a frame of the
  original (a PPSSPP capture for Pulse, RPCS3 for HD).
- Omega's executable has not been read for the display; HD's law stands in.
- A PS2 boot of Pulse shows nothing: the PS2 archive has no names.
- Pure's borrowed bar sits over the craft's exhaust on the chase view; 2048's bar and text size
  are picked, not measured.

## Next Steps

- Move the race music's start to the green light on the titles that show the display, then
  drop `advance_now_playing`'s wait.
- Confirm Pulse's timing on PPSSPP and HD's bar on RPCS3 with a muted capture.
