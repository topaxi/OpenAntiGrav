---
categories: [rendering]
---

# HD's boost and damage zoom-streak ring is ported; its remaining triggers are open

2026-10-09, lane `hd-zoom-ring`, closing the 2026-10-08 `hd-motion-blur` thread. Evidence and law:
[funklayer-zoom.md](../../docs/ghidra/functions/ps3-hdfury-eu/funklayer-zoom.md). HD has no motion
blur; `FunkLayerZoom` is a recursive zoom (the history draw crops and re-blends the previous
quarter-resolution scene, weight `0.25 E`, crop `0.05 E`) shown through a 24-quad ring. It is
`oag_post::hd_zoom`, HD's numbers are `oag_hd::race::ZOOM_RING`, and the player's speed pad and
Turbo fire it. It is not a blur, so it has no motion blur setting value.

## Open

- **What fires `EngineFlare_TriggerZoomGlow` beyond a pad and a Turbo.** The start boost did not
  fire it in three starts, but none of them was shown to have had a start boost; a barrel roll is
  untested. The caller itself is unnamed: a write watch on `0x00ad7880` needs the PPU interpreter,
  and the two bounded runs under it (restored with `emu-restore-state.sh --interpreter`) took no
  hit on `0x0029ef40` at about 20 % speed.
- **`P` has no trigger in our race.** The pass draws it and the tint is the live one, but nothing
  relates `Ship_ApplyDamage`'s `damage` (`P = 1.8 * damage`) to a hit of ours.
- **An opponent's boost** does not fire it, as in the original.
- The history block's gate byte (`FunkLayer + 0xa87`) and the ring colour's other branch
  (`FunkLayer + 0x50` above zero) are unread; the history runs every tick here.
- Radial bloom (`zone_1` only) and the seven DoF programs: not read.
- Omega: checked, differs, by strings only; its shader packs are not unpacked.

## Next Steps

1. Show whether the original grants a start boost in the captured runs (the speed at `GO`), then
   re-run the onset watch across a start with and without a good launch.
2. Name the caller: restore a state with the interpreter and cross a pad within its slow run, or
   break on `0x0029ef40` with the patched RPCS3's `Z2` build.
3. Find the unit of `Ship_ApplyDamage`'s `damage` against our shield pool, then wire `P`.
4. Pulse and the other titles: `do-the-originals-have-motion-blur.md` keeps Pulse PSP/PS2, Pure,
   2048 and Omega.
