---
status: ready-to-wire
titles: [hd, 2048, omega, pulse]
updated: 2026-11-07
summary: "HD, 2048 and Omega never stamp a weapon pad in Eliminator; ours greys them for 0.15 s. Pulse (0.05 s) is right"
---

# Eliminator weapon pads: three titles never disarm, ours does

**Measured, 2026-11-07 (`eliminator-pads`).** The evidence, numbers and method are
[`docs/gameplay/pickups.md`](../../docs/gameplay/pickups.md#eliminator-and-the-pads-per-title); the 2048 and
Omega functions are `weapon-pads.md` in their `docs/ghidra/functions/` directories. The
short form: Pulse stamps `elimination_refresh_time` (`0.05` s, 2 grey frames at 60 Hz, run on PPSSPP).
HD, 2048 and Omega author `0.15` and **never stamp in Eliminator**: HD run on RPCS3 (no stamp in
five Eliminator runs, the same instrument stamps `0.75` in a Single Race), 2048 and Omega read
(the Eliminator family skips the stamp, so the `elimination_refresh_time` branch is dead).

## Open

- **Ours is wrong on three titles.** `crates/raceplay/src/load/global.rs` (`weapon_pad_refresh`,
  the `Mode::Eliminator` arm) takes `elimination_refresh_time` on every title that authors it, so
  an HD, 2048 or Omega pad greys for 10 ticks after a hit; the original's never greys.
- HD's own stamp function is not located (the `.opd`/TOC resolution hides its readers of the
  `refresh_time` table; `elimination_refresh_time` at `0x00781530` resolves only to a libm
  function). Confidence 75 for HD rests on the run, not on a read.
- Omega live is not checkable here (shadPS4 has no memory readout; the native `wipeout_pc` port's
  drivers are untracked): the command is `scripts/rpcs3-hd-weapon-pad-poll.py`'s loop against that
  process's `/proc/<pid>/mem`, reading the pad object's `+0x140`.
- One frame of draw order: ours greys the stamp tick, Pulse's original greys the tick after
  (3 and 33 grey frames against 2 and 32).

## Next Steps

1. **Ready to wire.** Add a `Title` data axis (ADR-0058; do not branch on `title.name`), for
   example `weapons.pads_stamp_in_eliminator: bool`, `true` for Pulse, `false` for HD, Fury, 2048
   and Omega, beside `Weapons::elimination` in `crates/title/src/weapons.rs`. In `load/global.rs`,
   for `Mode::Eliminator` on a title where it is `false`, set the pad refresh to `0.0`: a pad is then
   never stamped, never greys (`weapon_pad_refresh_left` stays `0.0`) and a craft with an empty slot
   is handed a pickup on every entry edge, which is the original's "no stamp, grant straight away".
   Keep Pulse at `0.05`. The loader report line says which case a run took.
2. Test: a ground-truth row per title (Pulse `0.05`, the other three `0.0`), and an Eliminator
   race on HD where a craft crosses one pad twice in under `0.15` s and is handed a pickup both
   times once its slot is empty. Update `docs/gameplay/weapon-lineage.md`'s pad row in the same change.
3. Fury: HD's disc and file, so the HD value applies; no separate row.
