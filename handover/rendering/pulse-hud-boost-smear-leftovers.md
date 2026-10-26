---
status: ready-to-wire
titles: [pulse, pure]
updated: 2026-10-21
summary: "Pulse boost HUD: 3D widget copy loop, text copy direction table by widget+0x172, PS2 glow and Pure unwired"
categories: [rendering]
---

# Pulse's boost HUD: the copies and the glow are wired; the 3D widgets, the PS2 glow and Pure are not

2026-10-21, `pulse-hud-boost-glow`. The HUD "glow on a speed pad" is the HUD's own multi-copy draw and a glow
stamp under the speed bar's fill, driven by the exhaust flare's 0.8 s boost timer
([hud-boost-smear.md](../../docs/ghidra/functions/psp-pulse-usa/hud-boost-smear.md)). Wired for Pulse PSP
(and the copies for the PS2 through the shared `oag_pulse::hud::ART`).

## Open

- The 3D widgets (`Mode3D_DrawQueued`, `0x088b8548`) carry the same copy loop (`lui 0x44e2` at `0x088b8748`) and
  are drawn once in ours.
- A text widget's direction is the line from the screen's centre; the original indexes a 32-entry table
  (`0x08ab1c78`/`0x08ab1cf8`) by the byte at `widget+0x172`, whose writer was not read. The text copies' gates
  (HUD shake exactly zero, `flags & 4` clear, global `0x08ab0818` non-zero) are read, not modelled.
- Pulse PS2: the copies draw through the shared art, the glow stamp does not (`Ps2Bloom` has no stamp pass);
  nothing was compared against a PS2 boost frame.
- Pure has one widget draw with the same copy loop (`FUN_088c9ce8`); its `Hud_Update` and the count's source are
  unread, so `boost_smear` is `None` there.
- The glow chain adds the bar's colour alone; the scene's colour under a half-covered bar copy is not mixed in.
- A pair at the boost's first and last frames (`n = 0, 1`) and at its middle (`n = 10`) against the original; the
  maintainer's page should carry a boosting pair (below).
- The shield bar is red in the original's boost frames (`ShieldBar` vertex colour `ff0000ff` in the GE lists) and
  cyan in ours: a state, not read.

## Next Steps

1. Adopt a boosting pair on the compare page: `scripts/psp-pulse-pair.py --field --layout forward --team Assegai
   --track 16_Track --state data/saves/pulse-psp/race-talons-venom-alongside.ppst --run-ticks 28 --out <dir>`
   (the lead adds the manifest row; `--run-ticks` runs the original that many ticks past the load, and the pair
   reads the flare's timer for `--hud-boost-phase`).
2. Read the writer of the text direction byte, then drop the chosen direction.
3. Give the 3D widgets their copies.
