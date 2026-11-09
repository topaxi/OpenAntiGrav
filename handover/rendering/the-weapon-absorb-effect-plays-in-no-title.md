---
status: ready-to-wire
titles: [all]
updated: 2026-11-09
summary: "Absorb differences measured on Pulse, Pure, HD: Pulse bar glow, Pure hull overlay and crackle burst, HD shell tint and tail. 2048, Omega unpaired"
---

# The weapon-absorb effect: built on every title, what a player still sees differently

The user, who plays the originals, reported on 2026-09-17 that the absorb effect did not play in any title. It does
now: the burst on every title (`oag_raceplay::absorb`, `Race::play_absorb_feedback`), Pulse's hull overlay
(`oag_fx::hull_overlay`), HD's absorb shell (`oag_fx::absorb_shell`), Pulse's HUD flash. The first matched pairs of a
real absorb on the player's craft were taken on 2026-11-09 (Pulse, Pure, HD); method, tables, frames and the ranked
differences are in [`weapon-absorb-pairs.md`](../../docs/rendering/weapon-absorb-pairs.md). The older frames this
thread cited are gone.

## Open

A wire lane can take these in order, per title. Each has its number in the page above.

- **Pulse.** The hull overlay, the burst and the ground ring match (peak +41 against +43, within the original's own
  run spread). Left:
  1. The energy bar. During the window the original's fill alternates bright cyan (luma 207) and white (239, both with
     a bloom halo), 7 to 8 frames each; ours alternates normal (163) and flat white (232), no halo. Period and window
     are right. The bar needs to enter the glow mask for the whole window, and its off phase is the bloomed bar.
     Mechanism (`+0xf4 = 0xff` feeds the glow mask) is confidence 60, read from frames; confirm at the draw call first.
  2. The first 0.3 s are one to two frames late and 20 % dim on the hull.
  - Closed here: "short crackles against long streaks" is not reproduced at the chase view.
- **Pure.** Nothing of the hull overlay is drawn: `oag_title::Effects` for Pure is the all-unread table.
  1. The hull flash: pale pink-white over the whole hull at its 0.5 s peak (+17 against ours +7, tint -6 against
     0). The disc ships `absorb_surface.mip`; the Pure draw call and its gate are unread (static first: the absorb path
     is `Ship_UpdateWeapons` `0x0892935c` -> `FUN_08925e20`, the held id is at weapon record `+0x1d8`).
  2. The burst: ours draws 30 to 60 px star spikes, the original short crackles and a bright bloom at nose and wings.
  3. A purple ground ring at 0.33 s to 0.5 s.
  4. The HUD flash: `FUN_08925f4c` writes `*(_DAT_0005364c + 0x98) = 0xff` on an absorb, Pulse's `+0xf4`
     counterpart; what it draws is unread, the original's frames show orange bars at the top centre.
- **HD.** The shell and its timing are right (live fade matches, both sides decay alike after 1.0 s); its strength and
  colour are not. The controls agree in exposure (mean luma 91.9 on both), so this is the shell, not the pair.
  1. The shell is about 2.7 times too bright and too white: shell alone +17.7 luma on ours against +6.5 on the original
     (RGB gain `(+16, +18, +24)` against `(+4, +6, +14)`); the original keeps a blue hull with the livery showing.
     Check `ShieldColour` and the shell's blend first.
  2. The burst is about half as strong (about +7 against +15 once the shell is taken out): the wingtip balls are larger
     and brighter in the original and light the floor from 0.3 s to 0.75 s.
  3. The `DAMAGE` and `Absorb` labels beside the shield hex (absorb-only not isolated).
- **2048 and Omega: unpaired.** 2048 has no recovered held-slot or absorb address (HD's are `0x000d9398` and
  `ship+0x5edc`); Omega has none either, and its Racebox weapons row is not walked. Both take HD's burst (ported); neither draws the shell (their effects tables are the unread one) and whether
  their archives ship an `AbsorbEffect` model was not censused; the HD items above are not checkable until a live frame
  exists. Details and what would unblock each are in the page's last section.
- **Not investigated:** AI absorbs (an AI craft's hull lights on Pulse; the path is the same call), the PS2 port (VIF
  hull batches have no GE input coordinates to project), the Pulse overlay residuals (`LessEqual` for `EQUAL`, the
  per-mesh gate, the airbrake projection), the LeachBeam overlay (`leachbeam_surface.mip`, fade source read statically,
  not wired), HD's cockpit shell (`vr_absorbinternal_cockpit`, placed at `(0, 0, 7)`), and the sound gate (Pulse plays
  `ABSORB` only when the hull has collision-fx nodes, the port always).

## Next Steps

1. Pulse bar: bloom the energy bar through the window and draw its off phase bright; compare the bar's rows 248 to 258
   against 207 and 239.
2. Pure hull overlay: read the draw of `absorb_surface.mip` on Pure (the Pulse overlay's path is
   `HullOverlay_Submit`), fill `hull_overlay` in Pure's `Effects`, then re-pair at 0.5 s (target +17).
3. HD shell: cut its strength to the original's (about 0.4x at full fade) and move its tint to blue-lavender; re-pair, then
   the burst ball size.
4. 2048: read `Ship_PlayAbsorbFeedback` and the pickup slot in the 2048 eboot, then the HD recipe with a `/proc` write.
   Omega: walk the Racebox weapons row and read the same two on the eboot.
