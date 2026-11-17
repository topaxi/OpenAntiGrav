---
status: measure
titles: [all]
updated: 2026-11-17
summary: "Zone-pair lanes took 28-107+ min each; plan: Zone save-state library, HD self-flying Zone, zone/score addresses, one zone-pair driver"
---

# A Zone compare pair should take minutes, not an hour

The maintainer's expectation (2026-11-17): "starting a zone race and grabbing a
screenshot would have been simple enough". Five capture lanes made one or two
Zone pairs each for Pure, Pulse PS2, HD, 2048 and Omega
(`docs/tools/compare-page.md`, the "Zone pairs" sections). Wall time per lane,
measured from each member's transcript (tool-call start to result, summed by
kind; "model" is the gap between one tool result and the next call):

| lane | wall | first pair | model | emulator | ours renders | gate | what the rest was |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Pure (PPSSPP) | 28 min | 13 min | 14 | 1 | 0.4 | 3 | script edit 3, a second state, docs |
| Pulse PS2 (PCSX2) | 29 min | ~25 min | 12 | 11 | 0.1 | 2 | menu walk 7 (old states refused), a missed paused grab redone |
| 2048 (Vita3K) | 60 min | ~35 min | 23 | 11 | 19 | 4 | Zone mode written (hull-matrix scan), camera variants, stage sweep, second boot |
| Omega (wipeout_pc) | 61 min | ~36 min | 19 | 1 | 13 | 5 | 25 min building a serve/control harness and pose read before the first grab |
| HD (RPCS3) | 107+ min | not yet | 29 | 75 | 0.1 | - | in Zone at 12 min; 90+ min failing to reach zone 5 (input-driven race, crashes, attract demo) |

The gates (2-5 min under the shared lock) and the build (1-4 min) are not the
problem. Model time (12-29 min a lane) scales with how much each lane had to
invent.

## Why

1. **Four of five pair scripts had no Zone mode** (only `psp-pulse-pair.py`
   and `rpcs3-hd-pair.py` did). Each lane wrote 135-300 lines: how to freeze
   the emulator, where the pose comes from (RAM row, hull-matrix scan, savestate
   row), how to pass zone and score. Now all five have one; the next pair on
   these titles does not pay this again.
2. **No mid-Zone save state existed**, so every lane walked menus from a cold
   boot and flew to zone 5. PCSX2 v2.9.168 refuses every `pulse-ps2` state made
   on v2.9.138, so even the existing PS2 states were lost. Pure and PS2 left
   reusable states behind (`data/saves/pure-psp/zone-pro-tozo-autopilot-*.ppst`,
   `data/saves/pulse-ps2/menu/main-menu-pcsx2-2.9.168.p2s`).
3. **Reaching zone 5 depends on the craft surviving 30-60 s.** Pulse PS2, 2048
   and Omega Zone races fly themselves; Pure needed its Autopilot armed by a
   debugger write; HD's Zone ship hits walls without input, which is where the
   HD lane's hour went. Omega on Tech de Ra stalls at zone 1 the same way.
4. **Zone and score are read off the HUD by eye** on every title but Pulse PSP:
   no RAM address is documented for Pure, PS2, HD, 2048 or Omega.
5. **Lanes did more than one screenshot**: second cold boots, two camera rigs,
   twelve pinned stages. Useful, but it is a measurement lane's work, not a
   pair's.

## Open

- HD: a way to reach zone 5+ without input (the Pure trick: arm the Autopilot
  by a debugger write; or a Zone counter poke), then one saved state.
- Zone and score RAM addresses for Pure, PS2, HD, 2048 and Omega.
- A Zone save-state library and its index; regenerating states after an
  emulator update.
- One shared driver instead of five per-emulator Zone modes.

## Next Steps

1. **Zone save-state library** (sonnet `oag-capture`, 1-2 h per title,
   confidence 60): for each title, one saved state per circuit at zone 5+ in
   `data/saves/<title>/zone/<circuit>-z<N>.<ext>` with a JSON sidecar (zone,
   score, emulator and version, how it was made). Never committed;
   `just sync-saves` carries them. `docs/tools/compare-page.md` names the
   emulator version each state needs.
2. **HD self-flying Zone** (opus `oag-re` on the HD/Fury executable, 2-4 h,
   confidence 40): find the Autopilot or AI hand-over for the player craft in
   Zone (Pure's lane armed it by a debugger write) or the zone counter, so a
   capture needs no driving. Then step 1 for HD.
3. **Zone and score addresses** (one `oag-re` or `oag-capture` lane per
   emulator family, 1-2 h each, confidence 50): the pair scripts read them
   instead of the eye; this also gives zone seconds, which every pair now lacks.
4. **One `scripts/zone-pair.py <title> --state <name>`** (sonnet `oag-wire`,
   half a day, confidence 60) over the five per-emulator backends: load state,
   freeze, grab, read pose and counters, render ours with `--hud-state` and
   `--zone-stage`, write the pair dir and the `zone.toml` entry. Target: one
   pair in under 5 minutes from a saved state.
5. **Capture-lane brief template** in `.claude/skills/oag-drive/` (lead, 15 min):
   one pair, one boot, the camera the script defaults to, a 30-minute time-box,
   a `phases.log`; repeatability, camera rigs and stage sweeps are their own
   measure lane.
