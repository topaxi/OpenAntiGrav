---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-08
summary: "Photo mode with picture controls, rewind, motion blur and accumulated depth of field runs; split screen, touch and follow-ups are open"
---

# Photo mode

The read is `docs/ui/photo-mode.md`; each binary's evidence is
`docs/ghidra/functions/<binary>/photo-mode.md`. Those pages record what each original does.

**Maintainer's decision (2026-11-06): one implementation for all titles, the best of all of them, not
faithful per title.** On by default everywhere, Pure included, and a saved shot always takes full
render resolution. So every row below is a choice for the shared mode, and a value from one title
used on another is chosen, not measured. Originals' 480x272, JPEG and per-title row sets are fact, not
limits.

## Open

- Audio on HD: not measured; recipe with addresses on the HD page. Pulse stops its engine note,
  cues and music callback, measured.
- SELECT from Pulse's `Race End Photo` and SQUARE from HD's: read, not driven. From a finish the
  camera stays the spectator's and the subject stays the chosen craft (`PhotoMode_Update`, circle branch).
- HD pan (`+0x2a8`, `+0x2ac`, `+-5`): read, never moved, because the virtual pad's square and triangle
  reach the game swapped. Recipe on the HD page.
- HD tiled capture `0x008c1f08`: reads 1, no writer searched.
- Omega's update `FUN_0155e450`: unread, with its recipe. Not needed for the shared mode.
- 2048: who writes `this+0x10c = 4` (track camera in the touch mode).
- Pulse PS2: the screens' XML was not read; no save path in its binary.
- Accumulated depth of field and motion blur (7 and 120 frames, shutter `(v * 0.01)^2 * 0.25`): how a craft's motion gets into a frozen race was never read, and the simulation must not tick. The aperture target `0x003bad38` (one float) has no reader chased; the pass's aperture reach is chosen.

## Next Steps

`photo-wire` landed: the one mode is wired on all five titles, and `docs/ui/photo-mode.md` ("What
ours does") says what it does, the bindings and the shot. The ranked table that chose its laws is
in that page's lineage and `git log`; what is left from it is here.

### Follow-ups to `photo-wire`

- **Entry moved (2026-11-07):** on Pulse the PHOTO MODE row is the pause screen's (`handover/frontend/pause-menu.md`); the other titles keep the main page's.
- **Split screen.** Left out on purpose (the mode frames one picture and one pad drives it); HD and
  Omega offer it from either pad. A cut from the best version, not a neutral default.
- **The photo counter stat** (`PRO_STATS_PHOTO` where a stats row exists).
- **Touch.** 2048's gestures (drag, two-finger roll and zoom) and a one-finger layout for a phone:
  the chips and a click work, the stick-and-modifier bindings do not reach a touchscreen.
- **Saving on a page or a phone.** The row is left out there: neither has a directory the player
  reaches. A download on the web, the photo library on a phone.
- **HD's `close` rest offset** was never read; Pulse's measured 12 behind, 3 above is used on every
  title (chosen). HD's separate tilt triggers and right-stick zoom are folded onto the one stick.
- **The shot at a render scale below 1.0** is the presentation target, upscaled from the scene at its configured render size (dynamic resolution is held at that size while the mode is open); a shot at the scene's own pixels, before the upscaler, would be a second target.
- **The track lens** is only where a circuit authors a `Camera` node and the title builds the
  spectator director (Pulse off a disc today); HD's, 2048's and Omega's nodes are not read into
  it, so those titles cycle `internal` and `close`.
- **Mods.** `photo_mode` is a new `menu.toml` action name; the mod author skill lists actions only
  as "a closed set", so it needed no change.

### `photo-effects`: landed

Exposure, saturation, speed blur and depth of field run through `oag_post::photo_fx` and the zoom ring on all
five titles, with HD's laws and independent blur and depth rows (`docs/ui/photo-mode.md`, "Picture
controls", says what is chosen). What is left from that table:

- ~~**Motion blur and its shutter**~~ and ~~**accumulated depth of field**~~ landed with the rewind
  (`photo-rewind`, ADR-0087 stage 6; `docs/ui/photo-mode.md`, "Accumulated shots and rewind"): HD and 2048.
  Open from it: **Omega's `Photo Use MB`** (its frame counts and shutter are unread: `photo_shot` is `None`),
  **HD's ship blur factor** (2048's `0.05` is used, chosen), and 2048's own depth of field law.
- ~~**Leaving photo mode with CIRCLE changes the race that goes on.**~~ Fixed (`replay-photo-switch`): the frame loop read the
  devices before the mode spent its presses, so the leave tick ran `Race::tick` with CIRCLE as an edge; the leave tick is
  now the mode's. `leaving_photo_mode_with_circle_hands_the_race_no_press` (tapped and held, Single Race and Time Trial).
- **Rewind from `Race End Photo`** and in a split: not offered (the end screens keep a board a rewind would
  lose).
- **HD's aperture reader** (`0x003bad38`) and its exposure shader: unread, so the aperture reach and the
  exposure curve are chosen.
- **The glow Pulse and Pure add over the HUD** is not graded or blurred.
- **GL** was not run.

### Conflicts to settle

1. **Finish button.** Pulse SELECT, HD and Omega SQUARE, 2048 a touch button. The draft says SELECT.
   SELECT wins, because SQUARE is the pan modifier inside the mode, but `MSC_PHOTO_PRESS_SELECT` ships reading "OPTIONS" on Omega, and HD's button prompt
   glyph is square: use one project string.
2. **Split screen.** Pulse none, HD and Omega yes, from either pad. The draft leaves it out for now.
   That is a cut from the best version, not a neutral default: name it as a follow-up.
3. **Which pad drives it.** Follows from 2: HD picks the paused pad.
4. **Strings.** Pulse `ER_PHOTO_*`, HD `MSC_PHOTO_*`, Pure none, and Pulse has no pan, tilt or exposure
   text. One screen needs one id per row, with `Title` data mapping to a disc string where one exists.
5. **Motion blur on a frozen race.** HD and 2048 accumulate 120 frames while the simulation is paused,
   from a camera or craft delta job. How the craft's motion gets into it was not read. Our sim must not
   tick, so this needs a design of its own or it stays out, named.
6. **`close` base offset.** HD's rest position of the eye was not read. Pulse's measured `12` behind, `3`
   above is the fallback, chosen on the other titles.
7. **One stick.** Rotate, pan and zoom assume two sticks. A keyboard or a one-stick device needs its own
   binding; 2048's touch gestures (drag, two-finger roll and zoom) are the phone follow-up.
8. **Effect rows on a low tier.** Every effect the pass runs is on every title; none is left out on a tier. A
   pass that fails to build is reported and the frame draws without it.
