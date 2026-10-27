---
status: ready-to-wire
titles: [hd, pulse]
updated: 2026-10-12
summary: "Emulator states: RPCS3 only for HD EU; wire a restored state into Session; PPSSPP guard; replace sleeps"
difficulty: easy (35)
---

# Emulator tooling: attach mode and RPCS3 states landed, five loose ends

`docs/reverse-engineering/emulator-recipes.md` has the tools and the measurements
(watchdog, `emu-run.py`, `rpcs3-drive.py serve` with attach, `psp-state.py`, and the RPCS3
save states: `emu-restore-state.sh` restores a drivable race in about 10 s, `emu-save-state.sh` takes one).

## Open

- RPCS3 states exist for HD EU only (Talon's grid, Racebox Single Race with weapons, Time Trial start,
  a mid-race pose; `data/saves/hd-fury/`). Other circuits, teams, the US disc (serial in
  `--serial`) and Fury's campaign grids are untaken; `emu-save-state.sh` takes any.
- A restored state has no `TTY.log` screen lines, so an attached script's first step (`wait_for_screen_pressing("Main Menu")`)
  finds `?`. Workaround: `cross` into the race, then `rpcs3-drive.py restart`. Not wired into
  `Session` itself (a `Session.after_restore()` that does both would let scripts take `OAG_RPCS3_STATE=<name>`).
- PPSSPP scripts (`psp-*.py`) have no `Session` guard; only `emu-run.py`'s silence
  watchdog covers them. Pure and the EU discs have no measured state point.
- Attract mode: parked in a race is safe (5 min). Parked on `Main Menu` the demo starts
  at about a minute; the walk now handles it, but an idle `serve` at `Main Menu` for
  hours was not tested.
- Scripts other than weapon, height, trace, whiteout, lightpoll, mem-poll and xfade-probe still
  `sleep(20)`/`sleep(70)`; the race has no `TTY.log` signal for the end of the countdown.

- 2026-10-12 (`hd-leach-2`), four traps on one machine, none reproduced twice on purpose:
  `emu-restore-state.sh` with `hd-racebox-talons-venom-weapons-grid` hung at RPCS3's state
  loader (log stopped at 0.57 s, CPU idle afterwards; the `serve` stage watchdog ended it at
  240 s), so every boot was a fresh `serve` plus the Racebox route by `rpcs3-drive.py press`;
  an attached `Session`'s walk dropped every press on `Main Menu` and `Single Player` while
  `rpcs3-drive.py press` on the same live session landed each one; a client killed while the
  target was paused (an `emu-run` stall kill) wedged the stub for every later client until a
  fresh `serve` (twice; a third such kill did not); and a weapon fired right after a
  `rpcs3_place` teleport ends at once (the LeachBeam: state 6 on the next update, eight of
  eight), so give the game a few seconds after a placement.

## Next Steps

1. Wire a restored state into `Session` (`OAG_RPCS3_STATE=<name>` on attach: cross, wait, `restart_race`).
2. Wire `psp-drive.py` onto `emu_guard.Guard` (its own stages: `menu`, `restart`, `drive`).
3. Convert the remaining sleeps to `wait_for_load` / `settle_menu`.
