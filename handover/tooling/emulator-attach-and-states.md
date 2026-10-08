# Emulator tooling: attach mode landed, five loose ends

`docs/reverse-engineering/emulator-recipes.md` has the tools and the measurements
(watchdog, `emu-run.py`, `rpcs3-drive.py serve` with attach, `psp-state.py`).

## Open

- RPCS3 `SaveState` writes a file in 1 of 6 attempts; the failures log "User selected
  savestate in home menu" and nothing else. Whether the GDB stub matters is not
  established, and what makes the one success different is unknown.
- A loaded RPCS3 state had a dead virtual pad and the stub off (one load each). Not
  known whether a pad rescan or a different input config fixes it.
- PPSSPP scripts (`psp-*.py`) have no `Session` guard; only `emu-run.py`'s silence
  watchdog covers them. Pure and the EU discs have no measured state point.
- Attract mode: parked in a race is safe (5 min). Parked on `Main Menu` the demo starts
  at about a minute; the walk now handles it, but an idle `serve` at `Main Menu` for
  hours was not tested.
- Scripts other than weapon, height, trace, whiteout, lightpoll, mem-poll and xfade-probe still
  `sleep(20)`/`sleep(70)`; the race has no `TTY.log` signal for the end of the countdown.

## Next Steps

1. Try `serve` plus a `SaveState` loop (up to 3 tries from fresh `serve`) and log what differs
   on the one that writes.
2. Wire `psp-drive.py` onto `emu_guard.Guard` (its own stages: `menu`, `restart`, `drive`).
3. Convert the remaining sleeps to `wait_for_load` / `settle_menu`.
