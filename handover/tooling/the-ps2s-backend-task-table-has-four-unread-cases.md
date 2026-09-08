# The PS2's backend task table has four unread cases, and the 60 Hz answer's lifetime is unknown

Split out of the closed thread "The PS2 PAL/NTSC selector's ultimate trigger",
whose question is answered in
[refresh-mode.md](../../docs/ghidra/functions/ps2-pulse-eu/refresh-mode.md):
`g_refresh_mode` (`0x0027a85c`) is written only by `Video_SetRefreshMode`, which
is reached from cases 4 and 5 of `BackendController_RunTask`'s six-entry jump
table, and those two cases are the task names `Switch50` and `Switch60` that the
disc's own front-end XML authors on `RefreshTestScreen` and `RefreshTestFail`.

Two things that read left behind. Neither blocks anything: the 60 Hz cut is
already what `oag_pulse::movies::LOOSE_MOVIES` prefers, per the maintainer's
2026-09-08 directive, and the switch itself is deliberately not reimplemented.

## Open

- **Cases 0-3 of the task table are unread**: `Launch` (`0x00186f20`), `Kill`
  (`0x0018701c`), `Pause` (`0x001870e8`) and `Run` (`0x00187128`). Unlike
  `Switch50`/`Switch60`, none of the four appears anywhere in `WADS2.WAD`, so
  either they are driven from somewhere that is not the front-end screen tree or
  they are unused. Which of those two it is has not been checked - the archive
  was searched for the names, not for another route into the dispatcher.
- **Whether the player's 60 Hz answer is persisted, or asked on every boot.**
  The `Switch` menu on `SwitchRefreshMode` carries no `save` attribute, where
  the Aspect Ratio list carries `save="true"`, which suggests it is asked every
  boot. That is a suggestion from one absent attribute and is deliberately
  unscored on `refresh-mode.md`. It matters only if this project ever wants to
  reproduce the boot chain's shape, not its picture.
- **What a PAL raster actually does with the 448 lines.** Inherited unchanged
  from [aspect-ratio.md](../../docs/ps2/aspect-ratio.md)'s own Open section,
  and now with the numbers attached: `0x00216d70` adds `(+640, +52)` at 60 Hz
  and `(+680, +72)` at 50 Hz to the per-mode screen offsets.

## Next Steps

1. Search `WADS2.WAD` for another route into `BackendController` - an element
   that constructs one without a `task`, or a `task` value that is not one of
   the six - before assuming cases 0-3 are dead. `just wad extract` the archive
   and grep for `BackendController`, which is one command.
2. If they are reachable, read `0x00186f20`, `0x0018701c`, `0x001870e8` and
   `0x00187128`; case 2 and case 3 both touch `0x0027a5c0` and a `+0x2c` flag
   bit, so they are probably a pair.
3. Only if step 1 or 2 turns up something that needs it: a PCSX2 boot with
   `just pcsx2-read 0x0027a85c` before and after answering the 60 Hz question
   settles the persistence question directly. Note the disc is PAL-only, so the
   *default* is the 50 Hz side and the 60 Hz side is only reachable by answering
   yes.
