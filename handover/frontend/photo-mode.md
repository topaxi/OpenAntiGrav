---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-06
summary: "The shared photo mode is wired on all five titles (docs/ui/photo-mode.md, What ours does); photo-effects through oag-post and the follow-ups are open"
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
- Depth of field on HD and 2048's two laws, and accumulated blur (7 and 120 frames), need `oag-post`.

## Next Steps

`photo-wire` landed: the one mode is wired on all five titles, and `docs/ui/photo-mode.md` ("What
ours does") says what it does, the bindings and the shot. The ranked table that chose its laws is
in that page's lineage and `git log`; what is left from it is here.

### Follow-ups to `photo-wire`

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

### `photo-effects`: through `oag-post`, after `photo-wire`

| Feature | Take | Over | Why | Conf. |
| --- | --- | --- | --- | ---: |
| Exposure | HD's law, `(v - 33.3) * -0.03003`, opening at `33.3 - 33.3 g` | Omega's fixed `50%` | Opens on the picture already on screen. `g` is 0 on a title with no grade, so it opens at 33 | 90 |
| Saturation | HD's `v * 0.05`, opens 20% | Omega: removed | | 90 |
| Speed blur | HD, 0 to 100, step 5 | Omega: removed | | 84 |
| Depth of field | HD's focus curve `d` in 3 to 493, aperture, focus | 2048's touch law | Same formula in HD and 2048's own pad code, so it is the better-attested one | 84 |
| Effect choice | Omega's two lists, blur and depth independent | HD's single list | More shots possible, if the pipeline can run both | 80 |
| Motion blur | HD and 2048's track and ship, shutter sets its strength | | Shutter is not an exposure control: `(v * 0.01)^2 * 0.25`, used only by motion blur. Needs the conflict below settled | 84 |

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
8. **Effect rows on a low tier.** An effect `oag-post` cannot do is left out and named, per the draft, so the
   one screen has rows that vary by build, not by title.
