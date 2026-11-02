---
status: ready-to-wire
titles: [pulse, pure]
updated: 2026-11-02
summary: "Pulse boost HUD: text copy direction by widget+0x172, per-widget Image shift vector (pickup backdrop measured (0,-i)), PS2 glow and Pure unwired"
categories: [rendering]
difficulty: hard (30)
---

# Pulse's boost HUD: the copies and the glow are wired; the 3D widgets, the PS2 glow and Pure are not

2026-10-21, `pulse-hud-boost-glow`. The HUD "glow on a speed pad" is the HUD's own multi-copy draw and a glow
stamp under the speed bar's fill, driven by the exhaust flare's 0.8 s boost timer
([hud-boost-smear.md](../../docs/ghidra/functions/psp-pulse-usa/hud-boost-smear.md)). Wired for Pulse PSP
(and the copies for the PS2 through the shared `oag_pulse::hud::ART`).

## Open

- ~~The 3D widgets' copy loop~~: false, `0x088b8748` is `Gu_SetOffset(OriginX + 1808, OriginY + 1912)`, no loop
  (hud-boost-smear.md, "`Mode3D_DrawQueued` has no copy loop"). ~~Whether a `Model` widget's own draw smears~~:
  Pulse's pickup icon is not a Model widget; it is 13 `Image` quads and a boost GE list shows each drawn `n` times
  (hud-boost-smear.md, "The pickup icon at a boost"). Pulse's race HUD has no Model widget to ask about; Pure's do.
- The pickup backdrop's measured copy shift is `(0, -i)` px with alpha `0x80 / n`, not the centre-line vector `boost_smear`
  uses for every Image (sideways 0 or 1 px off); one widget read, so the general direction rule is unknown. Read the
  `+0x114/+0x118` writer, or sample more widgets from a boost dump's `OFFSETX/OFFSETY` words, then give `boost_smear` a
  per-widget vector.
- ~~Pure's `FUN_088c9ce8` copy loop~~: false, it is Pure's `Mode3D_DrawQueued` (`0x088c9dec` is `Gu_SetOffset(OriginX + 1808, OriginY + 1912)`, no loop; psp-pure-usa/hud-boost-smear.md). Pure's real 2D copy loop (`Image_Draw`/`Text_Draw` twins) is unsearched.
- A text widget's direction is the line from the screen's centre; the original indexes a 32-entry table
  (`0x08ab1c78`/`0x08ab1cf8`) by the byte at `widget+0x172`, whose writer was not read. The text copies' gates
  (HUD shake exactly zero, `flags & 4` clear, global `0x08ab0818` non-zero) are read, not modelled.
- Pulse PS2: the copies draw through the shared art, the glow stamp does not (`Ps2Bloom` has no stamp pass);
  nothing was compared against a PS2 boost frame.
- Pure's `FUN_088c9ce8` has no copy loop (it is `Mode3D_DrawQueued`); Pure's real 2D copy loop and its count's source
  are unread, so `boost_smear` is `None` there.
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
3. ~~Give the 3D widgets their copies~~: nothing to give; the GE-list check is done (the icon is an Image, it smears, `n` copies).
