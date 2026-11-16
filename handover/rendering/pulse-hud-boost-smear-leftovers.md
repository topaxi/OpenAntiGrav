---
status: re-first
titles: [pulse, pure]
updated: 2026-11-09
summary: "Pulse boost HUD: text direction byte unread, red shield bar in boost frames, PS2 glow, Pure unwired; stamp reads the HUD target"
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
  (hud-boost-smear.md, "The pickup icon at a boost"). No Model widget drew in any captured Pulse frame; the countdown's model and Pure's gauge models stay unmeasured.
- ~~Ready to wire: the Centred-widget copy direction~~: wired, `oag_hud::boost_smear` steps from the authored `x`/`y`
  (hud-boost-smear.md, "Ours now matches"); the pickup's copies step `(0, -1)`.
- Pure's `FUN_088c9ce8` has no copy loop (it is `Mode3D_DrawQueued`); Pure's real 2D copy loop and its count's source
  are unread, so `boost_smear` is `None` there.
- ~~The glow chain adds the bar's colour alone; the scene's colour under a half-covered bar copy is not mixed in.~~ Closed
  (`pulse-stamp-scene`): the stamp pass reads the HUD's target; the boosting pair's speed bar RMSE is `19.1 -> 14.7` and its fill
  reads `(241, 255, 254)` against the original's `(243, 255, 255)`. The shield bar's box reads worse (`120.6 -> 129.8`) because
  the cyan bar below the smeared copies glows more: the red-bar item below is now the largest gap there.
- A pair at the boost's first and last frames (`n = 0, 1`) and at its middle (`n = 10`) against the original; the
  maintainer's page should carry a boosting pair (below).
- The shield bar is red in the original's boost frames (`ShieldBar` vertex colour `ff0000ff` in the GE lists) and
  cyan in ours: a state, not read.

## Next Steps

1. Adopt a boosting pair on the compare page: the same command with `--run-ticks 100` (28, 45 and 70 read boost timer 0
   on this state; 100 reads 0.683 s) `--out <dir>` (the lead adds the manifest row; the pair reads the flare's timer for
   `--hud-boost-phase`).
2. Read the writer of the text direction byte, then drop the chosen direction.
3. ~~Give the 3D widgets their copies~~: nothing to give; the GE-list check is done (the icon is an Image, it smears, `n` copies).
