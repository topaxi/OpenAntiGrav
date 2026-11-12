---
status: ready-to-wire
titles: [2048, omega]
updated: 2026-11-12
summary: "2048 map: music wave wired (.fft band 1, radius 7); hidden-event reveal wired (rank gate, flat spot, click/Confirm, saved find); unlock-on-race-end and live checks open"
categories: [audio]
difficulty: hard (30)
---

# 2048's campaign lattice bounces to the music

Read 2026-11-12 (lane `w2048-lattice-reveal-re`), static only: every law with its address and
confidence is on `docs/ghidra/functions/vita-2048-eu-v104/canvas-music-wave.md`. In short:

- **The wave** is not a live analysis. Every music track ships a `*_stereo.fft` sidecar (`AFEX`, 12
  bands, 30 rows a second, 13 of 13 files checked). The map samples band 1 (the bass) at the playing
  position, keeps a 30-row history, and each cell within **7 units of the current event's hexagon
  centre** follows that band delayed by `30 d^2 / 49` updates and scaled by `600 f`
  (`f = (49 - d^2) / 49`), then decays at 230 alpha a second and eases into the texture at 15 % a
  frame. Painted tiles (priority 6 or 7) and anything farther than 7 units do not move.
- **A hidden prototype event** (`m_options & 0x10`) waits at progress 0 (plain ground, moves with the
  wave) until the player's XP rank reaches its `m_RankRequired`; then `Campaign_UnlockEvents` sets
  progress 1: the button is state 3, **draws nothing**, and its six cells are averaged every frame
  (they rise and fall as one flat hexagon while the ground around them ripples). A tap on any of its
  six cells sets progress 2: it becomes the ordinary grey state-2 hexagon prism.
- **Omega**: checked, differs (the analyser code ships, no `.fft` file does; Omega's music is Wwise).

## Next Steps

The wave landed (`docs/ghidra/functions/vita-2048-eu-v104/canvas3d-lattice.md`, "Wired: the music
wave"): the `.fft` reader, the music-position output, the law (`canvas_wave`), a generic
`Draw::Triangles` and the CPU vertex program (`canvas_mesh`). Steps 2 to 4 need the XP model.

1. ~~The wave.~~ Open edges: the analyser's automatic gain (`MusicAnalysis_Update`, peak-hold unread),
   `Wave::flatten` is called with no groups until a hidden event can be at progress 1, and no live
   Vita3K confirmation of the amplitude.
2. **XP total and rank.** Nothing in this engine keeps 2048's XP
   (`docs/ghidra/functions/vita-2048-eu-v104/xp-award.md`). Minimum: a per-profile XP total, the
   awards the campaign pays, the rank from `XPRanks.xml`. A subsystem of its own; it may deserve its
   own thread.
3. **The unlock rule** (`Campaign_UnlockEvents` `0x812b5b6c`, `GameModeBase_IsUnlockable` `0x812b3a76`):
   on map open and at race end, every progress-0 event that is unlockable (XP rank at least
   `m_RankRequired`, `m_pEventRequired` passed) goes to 1 if `m_options & 0x18`, else to 2.
4. **The reveal** (`Canvas3D_TapCell` `0x81071de4`): a click on any of a progress-1 event's six cells
   sets it to 2, persisted in the profile; it then draws as the grey state-2 tile and joins pad
   navigation. A click on ground (no owning button) is the ground bump (gains and caps on the page).
   The original's touch is a Vita touch panel (which one is open); a mouse click is our choice.
5. A regression test per law: the `.fft` header and size identity on the disc's 13 files; the wave's
   delay and fall-off on a synthetic band-1 step; the unlock rule's 1-versus-2 split; the tap.

## Open

- **Persistence of the progress word**: `GameModeBase_SetProgress` (`0x812b3f90`) writes it, the save
  serialiser is unread (recipe on the page).
- **Live confirmation**: nothing here was watched running. Best check: Vita3K at the open map, read
  `canvas+0x5ed40..+0x5ed54` and the cell alphas round the current event over ten stops. A state-3
  button needs a save at XP rank 10 or more (420,000 XP); poking `profile+0x3257c` in a private copy is
  the short way.
- **Ask the maintainer**: does a hidden spot stay completely motionless, or rise and fall as one flat
  hexagon? The code says the second (confidence 55 that this is what a player reads as "still").
- The ripple branch of `Canvas3D_UpdateCellHeights` (`+0x5ed24`) has no writer that turns it on.
- The analyser's peak-hold detail in `MusicAnalysis_Update` (`0x8125f460`, confidence 60).
