---
status: ready-to-wire
titles: [all]
updated: 2026-11-09
summary: "Open: Pure burst and rings, HD shell tint (gamma nibble unread), HD burst needs a re-shot original (no /2), Pulse bar glow"
---

# The weapon-absorb effect: built on every title, what a player still sees differently

The user, who plays the originals, reported on 2026-09-17 that the absorb effect did not play in any title. It does
now: the burst on every title (`oag_raceplay::absorb`, `Race::play_absorb_feedback`), Pulse's hull overlay
(`oag_fx::hull_overlay`), HD's absorb shell (`oag_fx::absorb_shell`), Pulse's HUD flash. The first matched pairs of a
real absorb on the player's craft were taken on 2026-11-09 (Pulse, Pure, HD); method, tables, frames and the ranked
differences are in [`weapon-absorb-pairs.md`](../../docs/rendering/weapon-absorb-pairs.md). The older frames this
thread cited are gone.

## Open

Re-paired on a second machine on 2026-11-09 (`absorb-fx-wire`); numbers and commands are in
[`weapon-absorb-pairs.md`](../../docs/rendering/weapon-absorb-pairs.md), "Fixes and re-pairs on a second machine".

- **Landed.**
  - Pure draws the hull overlay (`FUN_0886e954`, Pulse's law; craft-box luma at 0.5 s: original +12.9 and +14.5, ours +6.6 before and +14.3 after).
  - HD's shell is halved by the program's `/2` and reads the craft's scene block (shell alone at 1.0 s: original +5.8, ours +13.1 before and +5.8 after).
- **Pure.**
  1. The burst's shape: the original's streaks are wide, soft, dim purple; ours are thin hard white-cored lines (same total light).
     The draw is in `crates/fx/src/psys`, held by another lane.
  2. The ground-ring halos at the wingtips: Pure's `WO_WEAPON_ABSORB.POB` has one streak emitter and no `rings` emitter, so the
     source of the rings is unread (not the POB; candidates: another effect `FUN_08925e20`'s `ShipCollisionFx_Trigger` kind 1 starts, or the
     bloom of the streaks).
  3. The HUD flash: `FUN_08925f4c` writes `*(_DAT_0005364c + 0x98) = 0xff`; what it draws is unread.
- **HD.** The state's player is the `Feisar_c1` concept hull; ours needs `--team feisar_c1 --variant concept1` to be the same ship.
  1. The shell's colour: pale lavender-white against the original's saturated blue (blue minus red at 1.0 s: original +10.8, ours +6.7).
     The lead is unit 0's gamma nibble on the shell's draw (7 would make the tap `enc(dec(t) * 0.5)`, as the flame and the bomb-blast fireball read);
     unread, read recipe in `weapon-absorb-pairs.md`, "HD burst and shell tint".
  2. The burst: "twice too strong" rests on one capture (+6.9 to +8.3 at 0.3 to 0.75 s); the first capture read the original at +13 to +21.
     **`renderer.md` item 4 was misread: HD's `psys_*` fragment programs carry no `/2`** (a blanket `/2` is wrong), and the absorb emitters'
     `kColourScale` word is 1.0. Needs a fresh original capture (three absorbs and a control on one boot) before any gain. Lead for the
     missile blast: the `+0xc84` word reads 0.5 on its mode-2 emitter and `oag_pob::Emitter::colour_scale` reads mode 5 only.
  3. The `DAMAGE` and `Absorb` labels beside the shield hex (absorb-only not isolated).
- **Pulse.** The energy bar: the original alternates bright cyan (luma 207) and white (239), both bloomed, 7 to 8 frames each; ours alternates
  normal (163) and flat white (232), no halo. Not attempted in the last lane (budget; `time-trial-platinum-rush-white-venom-grid.ppst` exists and the same driver should reach it). Confirm at the draw call first (a GE
  dump of the bar's draw: does it stamp the glow mask for the whole window), then wire; the mechanism (`+0xf4 = 0xff`) is confidence 60.
  The first 0.3 s of the hull are one to two frames late and 20 % dim.
- **2048 and Omega: unpaired.** The shell fix is not checkable (neither draws a shell, their archives are not censused for an
  `AbsorbEffect` model, and their programs are GXP and PS4 bytecode that `ps3-microcode.py` does not read). 2048 has no recovered held-slot or absorb address (HD's are `0x000d9398` and
  `ship+0x5edc`); Omega has none either and its Racebox weapons row is not walked.
- **Not investigated:** AI absorbs, the PS2 port (VIF hull batches have no GE input coordinates to project), the Pulse overlay residuals
  (`LessEqual` for `EQUAL`, the per-mesh gate, the airbrake projection), the LeachBeam overlay (`leachbeam_surface.mip`), HD's cockpit shell
  (`vr_absorbinternal_cockpit`), and the sound gate (Pulse plays `ABSORB` only when the hull has collision-fx nodes, the port always).

## Next Steps

1. Pulse bar: GE-dump the bar's draw inside the window, then bloom it through the window and draw its off phase bright (rows 248 to 258 against 207 and 239).
2. HD shell tint: read unit 0's gamma nibble on the shell's draw (`rsx-draw-list.py --samplers`); if 7, decode the tap on HD's encoded store as the flame does, then re-pair at 0.5 s and 1.0 s.
3. HD burst: re-shoot the original (three absorbs, one control, one boot) to settle +7.7 against +21.4 at 0.5 s; then read what writes `kColourScale` (`c467`) per psys draw. No `/2`.
4. Pure rings: find what draws them (`ShipCollisionFx_Trigger` kind 1's other effects, or the bloom).
5. 2048: read `Ship_PlayAbsorbFeedback` and the pickup slot in the 2048 eboot, then the HD recipe with a `/proc` write. Omega: walk the Racebox weapons row and read the same two on the eboot.
