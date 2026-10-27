---
status: re-first
titles: [2048]
updated: 2026-10-27
summary: "2048 lattice wired on title and game mode; open: other pages' blurred backdrop, composite constants, scroll writer"
---

# 2048's menu backdrop lattice: wired on the title and game mode pages

Wired 2026-10-27 (lane `lattice-final`): `oag_ui::frontend::canvas_lattice` bakes one 960 x 544 picture off
`canvasTexture.gxt` with the tilted camera matrix and the live translation, run through the game's composite
square-root encode, and the title screen and game mode page draw it over the white clear. Against the Vita3K
captures: edge shift (0, 0), edge NCC 0.95, the five empty boxes at 0.32 to 0.40 mean grey difference (white:
1.7 to 2.4), whole frame 5.39 to 3.71 (title) and 7.68 to 6.27 (game mode). Law, passes and tables:
[`canvas3d-lattice.md`](../../docs/ghidra/functions/vita-2048-eu-v104/canvas3d-lattice.md), "Wired"; the
RenderDoc method: [`vita3k-capture.md`](../../docs/reverse-engineering/vita3k-capture.md).

## Open

- **The other pages' blurred backdrop** (home, team, extras, options, profile). The frame has a bloom chain
  (240 x 136 downsample and two blurs) whose output is black at the title; on those pages it may carry the
  backdrop, or the composite may mix differently. One RenderDoc capture per page answers it (method in
  `vita3k-capture.md`); unread.
- **The composite's identity and constants.** The front end's composite samples `mainTex` and `bloomTex` and
  the measured transfer is `sqrt` plus a 0.25 px offset; which program (`wo_composite_notonemap_fp` or the
  front-end branch of `PostFx_RenderChain`) and its uniforms sit behind a buffer pointer. Read them from guest
  memory (`vita3k-mem.py`) to turn the measured law into a read one, and check whether the in-race composite
  (`exposure.md`) and this one are the same program with different inputs.
- **Who writes the (-3000, -1500) scroll.** Candidates `FUN_810efc4a` (`0x810effcc`) and `FUN_810efff6`
  (`0x810f032a`), the campaign map's centring of the first event (confidence 55). Our phase is the live
  translation row, a constant; deriving it from the page's own node positions needs the call pinned.
- Who sets bit `0x80` at `canvas+0x30` (the page set is read off the captures); painted cells (the campaign
  map, `canvasTex` alpha and `canvasTex2` shifts).
- **Omega**: `canvasTexture.gnf` ships and the reset angles are the same; whether Omega's pages draw a
  `Canvas3D`, and through which encode, is unread (Omega's front end is HD's).

## Next Steps

1. RenderDoc the 2048 home page (same recipe, one capture): is the blurred backdrop the canvas through the
   bloom chain? If so it is this composite with a nonzero bloom, and a draw from the disc's data.
2. Read the composite's uniform block from guest memory at the title and name the program.
3. Pin the scroll writer, then derive the phase from the page's nodes.
