---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-06
summary: "Photo mode read on all five titles, Pulse and HD measured live; Pulse's rows are ready to wire, Pure takes them as a port"
---

# Photo mode

The read is `docs/ui/photo-mode.md`; each binary's evidence is
`docs/ghidra/functions/<binary>/photo-mode.md`. Maintainer's decisions (2026-11-06): every title gets the
mode on by default, a title without one takes Pulse's as a port, a saved shot always takes full
render resolution. So 480x272 and JPEG quality 90 are recorded fact, not a limit.

## Open

- Audio during photo mode: not measured on either title, both emulators ran muted. Recipe on
  the Pulse EU page.
- SELECT from Pulse's `Race End Photo` and SQUARE from HD's: read, not driven. From a finish the
  camera stays the spectator's and the subject stays the chosen craft (`PhotoMode_Update`, circle branch).
- HD pan (`+0x2a8`, `+0x2ac`, `+-5`): read, never moved on the virtual pad. Index 7 is
  `FUN_00677308`. Recipe on the HD page.
- HD tiled capture `0x008c1f08`: reads 1, no writer searched.
- Omega's update `FUN_0155e450`: unread, with its recipe. Its camera limits are assumed HD's until then.
- 2048: who writes `this+0x10c = 4` (track camera in the touch mode).
- Pulse PS2: the screens' XML was not read; no save path in its binary.
- Depth of field on HD and 2048's two laws, and accumulated blur (7 and 120 frames), need `oag-post`.

## Next Steps

**Lane 2, Pulse.** All of it is on the Pulse page, confidence 90:

1. Rows off `InGame Photo` in `InGame_Definition.xml`, `Photo Mode Top Level` for the title bar. Strings on disc.
2. Entry: `EXT_PHOTO` on the four single-player pause screens only; absent in multiplayer, so leave
   it out of a split on Pulse. SELECT on `Race End Photo` replaces the swallow in `Session::tick_endrace`.
3. The race does not tick and only the spectator camera runs; the same as pause, so a state hash must
   not move. Re-subject to the player each frame from a race; keep the chosen craft from a finish.
4. Camera: modes 1, 4, 3, 2, 7 already exist in `oag_render::camera` for the finish. Pitch `[-1, 0.6]`,
   yaw free, both `0.05` per frame on the stick, TRIANGLE resets, and only with no digital button
   held. Track: field `z` in `[3, 40]`, L and R `+-5`, stick X additive, drawn at
   `z + 0.06 * (target - z)`. No track collision: neither title has one.
5. HUD default off; the row draws the HUD under the photo screen.
6. Shot: 30 frames on an empty screen, then save, with result dialog back to `InGame Photo`.
   Counter: `PRO_STATS_PHOTO`, `MSC_DATA_PHO`, `TKR_PHOTOS`.

**Lane 3.** Pure: Pulse's rows as a port, chosen, not measured. HD, Omega: their rows, in
`photo-mode.md`, with `Exposure` opening at `33.3 - 33.3 g` and `Saturation` at 20%; leave out what
`oag-post` cannot do and name it. HD offers split screen; SQUARE at the finish. 2048: the touch
screen, with its own depth of field law. The Glory Moment stays out.
