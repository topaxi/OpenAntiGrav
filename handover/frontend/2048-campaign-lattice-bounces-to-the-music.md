---
status: re-first
titles: [2048, omega]
updated: 2026-11-12
summary: "2048 campaign map: the lattice waves to the music, hidden prototype events stay still and are revealed by a click on their spot; both unread"
categories: [audio]
difficulty: hard (30)
---

# 2048's campaign lattice bounces to the music

Reported by the maintainer from play (2026-11-12), not yet read off the disc or binary:

- The campaign map's lattice (`Canvas3D`, `docs/ghidra/functions/vita-2048-eu-v104/canvas3d-lattice.md`)
  moves in a wave or bounce driven by the front-end music, by beat or spectrum.
- A hidden event (the fifteen `m_options & 0x10` prototype challenges, five "P Ship" and ten
  "S Phantom") does **not** bounce. Its stillness is how a player spots it before clicking its spot to
  find it (the cursor that reaches the spot is wired; the reveal is step 0 below).

Nothing here is drawn today: our lattice is a static baked picture.

## Next Steps

0. **The reveal (maintainer chose RE first, 2026-11-12).** The right-stick cursor and mouse click
   already reach any lattice spot (`oag_ui::frontend::campaign_map`, `hit_at`); a click on a hidden
   event's spot does nothing yet. Read: what a tap on a hidden event's spot does (the progress word it
   raises; a sweep for writers of offset `+0x2d8` found none), the hit area (the event's own cells?),
   what makes a hidden event revealable (`m_RankRequired` 10-50, player rank unread), and the found
   tile's shape (hexagon or triangle). Then wire: a persisted "found" set in the profile, the found
   tile drawn, pad navigation reaching found events.
   Leads (from `canvas3d-lattice.md`, unread): `Canvas3D_UpdateCellHeights` `0x81071794` (confidence 60)
   is the likeliest home of the height wave and a per-cell "static" flag; its caller is `Canvas3D_Update`
   `0x81072220` (70). Ghidra program `/2048/eboot-vita-2048-eu-v104.elf`.
1. RE: find what drives the cells' height. Start at the canvas vertex program's height input (the cell
   alpha "height" `canvas3d-lattice.md` names) and the per-frame writer of the cell buffer; look for an
   audio analysis call (beat detector, FFT or level meter on the music voice) feeding it. Record the
   law (amplitude, falloff, wave speed, which band), with addresses and a confidence.
2. Read how a hidden event's cells are excluded from the motion (a per-cell flag, or the event's cells
   simply never written).
3. Measure against the running original in Vita3K: a short capture of the lattice moving with the
   music, to check the law.
4. Wire it: the music path (`oag-music`/`oag-sound`) exposes a level or spectrum value per frame to the
   front end (an output, never `World` state, ADR-0018); the canvas draw stops being one baked picture
   for the moving cells. Hidden events stay still.
5. Omega: check whether Omega's 2048 front end ships the same canvas animation; record it on the doc page.

## Open

- Whether the motion follows the beat, a spectrum band or overall level is unknown.
