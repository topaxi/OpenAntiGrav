---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-06
summary: "One shared photo mode on all five titles: read done, each feature ranked by which title does it best, conflicts named"
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

Each feature: the best version, what it beats, and the score of the source read.

### Lane 2, the shared mode

| Feature | Take | Over | Why | Conf. |
| --- | --- | --- | --- | ---: |
| Freeze | Pause, plus a camera update | none, all alike | Same on Pulse and HD: no craft or race tick. State hash must not move | 90 |
| Camera list | `internal`, `close`, `track` (HD) | Pulse's five rows | HD `close` orbits on two free axes, so Pulse's `front` and `above` are positions within it | 90 |
| Orbit | HD: two axes, free, `0.05` a frame, `+-2 pi` wrap | Pulse pitch `[-1, 0.6]`, yaw only | Wider, and Pulse's clamp did not stop the eye going under the road anyway | 90 |
| Track collision | None | none, all alike | Neither Pulse nor HD has one. The eye may go under the road | 90 |
| Zoom, `close` | HD: right stick Y, field `[10, 100]`, starts 65 | Pulse: no zoom in `close` | | 90 top limit; 84 floor |
| Tilt | HD: L2, R2, `0.05` a frame | Pulse: none | | 90 |
| Pan | HD: left stick plus SQUARE, `+-5` | Pulse: none | Read, never moved live | 84 |
| Track camera | Field `[3, 40]`, starts 20, stick on the zoom axis (HD) | Pulse starts at 3 (4.3 degrees) | Same limits on Pulse, HD and 2048. HD's start frames the craft | 90 limits |
| Track field drawn | `z` itself | Pulse's `z + 0.06 (target - z)` | That ramp is a by-product of re-subjecting each frame, not a feature | chosen |
| Choose the craft | L1, R1, wrap, skip dead craft (HD, same law in 2048) | Pulse: player only in a race | | 84 |
| Reset | TRIANGLE, all of orbit, pan, tilt, zoom (HD) | Pulse: pitch and yaw only | | 84 |
| HUD | Row, default off | none, all alike | The same row on Pulse and HD | 90 |
| Hide the menu | Toggle help (HD, button index `0xe`) | Pulse: menu always up | Needed to look at the shot before taking it | 84 |
| Guard | No shot for `0.15` s after entry (HD, 2048) | Pulse: none | Stops the CROSS that opened the mode taking a shot | 84 |
| Entry | Pause row on every title | none | All four originals that have one use it | 90 |
| Shot flow | Press, result message, back with the camera kept (Pulse, HD) | 2048's preview step | Our render needs neither Pulse's 30 frames nor HD's transition: draw without UI | 90 |
| Count | One stat, `PRO_STATS_PHOTO` where a stats row exists | | Follow-up | |

Dropped on purpose: Pulse's rule that the stick is ignored while any digital button is held (it would
kill stick plus SQUARE). 2048's Glory Moment: needs a camera and a session.

### Lane 3, effects

| Feature | Take | Over | Why | Conf. |
| --- | --- | --- | --- | ---: |
| Exposure | HD's law, `(v - 33.3) * -0.03003`, opening at `33.3 - 33.3 g` | Omega's fixed `50%` | Opens on the picture already on screen. `g` is 0 on a title with no grade, so it opens at 33 | 90 |
| Saturation | HD's `v * 0.05`, opens 20% | Omega: removed | | 90 |
| Speed blur | HD, 0 to 100, step 5 | Omega: removed | | 84 |
| Depth of field | HD's focus curve `d` in 3 to 493, aperture, focus | 2048's touch law | Same formula in HD and 2048's own pad code, so it is the better-attested one | 84 |
| Effect choice | Omega's two lists, blur and depth independent | HD's single list | More shots possible, if the pipeline can run both | 80 |
| Motion blur | HD and 2048's track and ship | | Needs the conflict below settled | 84 |

### Conflicts to settle

1. **Finish button.** Pulse SELECT, HD and Omega SQUARE, 2048 a touch button. The draft says SELECT.
   SELECT stays, but `MSC_PHOTO_PRESS_SELECT` ships reading "OPTIONS" on Omega, and HD's button prompt
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
