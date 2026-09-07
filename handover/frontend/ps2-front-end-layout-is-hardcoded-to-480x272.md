# PS2 front-end layout is hardcoded to 480x272

**Closed 2026-08-09, not the way this row proposed.** `SCREEN` did not become per-source; `frontend::Space` carries a grid **and** the display aspect (24% apart on the PS2, more than one `(f32, f32)` holds), and the rects/pillarboxes route through it. `SCREEN`'s readers are where 480x272 is right whatever the disc: the loading screen, the HUD bounds check, `race::AUTHORED_ASPECT`. **That 24% was 7% until 2026-08-23**: the display aspect was 4:3 off the television and it is 480/272 - the port stretched the PSP's art by `(640/480, 448/272)`, as `FUN_001e9370` does. `docs/ps2/aspect-ratio.md` also retires the movies' declared 4:3. **Still open**: (a) the original draws a screen *title* in the `title` role (`Pulse_14.fnt`, 17px) against rows in `menu` (22px); this build draws both in the row face at the title scale - needs the atlas keyed by face, as `hud::Font`'s unhonoured variants do; (b) `widthlimited="true"` is ignored, so `Show Logo`'s PS2 legal line runs off the edge now the screen is wide - it always overflowed, the letterbox hid it.

## Open

- Screen titles are drawn in the row (`menu`) face at title scale instead of the original's separate `title` face
- `widthlimited="true"` is ignored, so `Show Logo`'s PS2 legal line runs off the edge on the wide screen

## Next Steps

- Key the font atlas by face, as `hud::Font`'s unhonoured variants already do, to separate the `title` and `menu` roles
- Honour `widthlimited="true"` so the legal line stops overflowing
