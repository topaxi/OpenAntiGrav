# Pointer cursors

The mouse pointer drawn over each title's screens, one SVG per title plus
this build's own for the disc chooser. **Every one of these is ours.** No
title in this lineage was authored for a mouse, so there is no cursor on any
disc to recover; each is drawn in the title's own menu palette so it reads
as belonging to the screen it is over, and nothing about it is a claim about
the original.

| File | Where it is drawn | Palette it borrows |
| --- | --- | --- |
| `launcher.svg` | the disc chooser, before a title is known | `assets/icons/64x64.svg`: `#F8F8F8` hull, `#06ABE9` streak, `#3A4A60` slate |
| `pulse.svg` | Wipeout Pulse's front end and menus | `FEGlobals->TextColor` `#88D6E8`, the measured selected ink `#34ACC2`, the 45-degree cuts of `Pulse_20.fnt` |
| `pure.svg` | Wipeout Pure's | the measured selected ink `#16AED1` and `MenuHighLightArrowColor` `#ED4796`, the pink of its own `ArrowSelect` |
| `hd.svg` | Wipeout HD / Fury's | `HD_Blue` `#8AC0CA`, `HD_Grey` `#666666`, `HD_White`, and the cut-and-landing corner of `Block_Render`'s tab |
| `2048.svg` | Wipeout 2048's placeholder screen | its flat orange-on-white front end, `#F26522` and `#111111` |

All five share one geometry: a 24x32 box with the hotspot at the top-left
corner `(0, 0)`, which is where `oag_game::cursor` anchors the sprite. They
are rasterised at boot through the same `resvg` path the window icon takes,
and drawn as a sprite by the frame loop rather than handed to the window
system - see `oag_game::cursor` for why.
