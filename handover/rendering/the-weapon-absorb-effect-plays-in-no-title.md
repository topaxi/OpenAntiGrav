---
status: ready-to-wire
titles: [all]
updated: 2026-11-09
summary: "Open: Pure templates held off, Pure ground ring, HD shell tint (nibble 7 read, decode alone worse), HD burst re-shot, PS2 bar"
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
  1. ~~The burst's shape~~ (landed): Pure's class-6 streak is Pulse's pool bar (`oag_title::Effects::pool_bars`,
     `psp-pure-usa/particle-bars.md`); wide soft purple shafts now, not 1 to 2 px white-cored lines. Still different: ours radiate where the
     original's rise mostly vertically, ours are more magenta, and ours lingers at 0.75 s (+7.7 against +3.1, the node stagger).
  2. The nose and wingtip bloom, the ground ellipse: `ring1` and `glow` have Pulse's draw (`FUN_08875138` and three more, static 85,
     `psp-pure-usa/particle-bars.md`, "The template draw"). `oag_title::Effects::templates` plays them (machinery landed, **no title lists it**): craft box at 0.5 s original
     +18.3, ours +11.3 muted and +15.2 on, but 0.75 s is +17.0 on against +3.1 (muted +7.7), so Pure keeps them off. The stagger is not the cause (a node delay
     that gates update and draw, as ours). Open: why one faint glow, and what draws the ground ellipse. PPSSPP 1.20.4 breakpoints armed after the ship-update loop
     deliver late; use the passive recipe on the particle-bars page (screenshot plus a RAM scan for template particles at `Ship_UpdateWeapons` stops).
  3. The HUD flash: `FUN_08925f4c` writes `*(_DAT_0005364c + 0x98) = 0xff`; what it draws is unread.
- **HD.** The state's player is the `Feisar_c1` concept hull; ours needs `--team feisar_c1 --variant concept1` to be the same ship.
  1. The shell's colour: pale lavender-white against the original's saturated blue (blue minus red at 1.0 s: original +9.8 to +10.8, ours +6.8).
     **Read, 2026-11-09 (`hd-absorb-tint-2`): unit 0's gamma nibble is 7** on all 14 shell draws (`0x60710101`, confidence 85, one boot), but **decoding the tap
     makes the 1.0 s pair worse** (blue minus red +4.4, luma +1.8 against +9.8 and +7.0), so it was not wired. Open: the pair's framing (ours' craft is about 1.5 times the
     original's width, so the craft box compares different pixels), which mesh Feisar_c1's `AbsorbEffect.vex` names (the original draws 6,692 triangles, the page reads 12,977),
     and the shell's alpha; `weapon-absorb-pairs.md`, "HD: the shell's unit 0 reads gamma nibble 7".
  2. The burst: "twice too strong" rests on one capture (+6.9 to +8.3 at 0.3 to 0.75 s); the first capture read the original at +13 to +21.
     **`renderer.md` item 4 was misread: HD's `psys_*` fragment programs carry no `/2`** (a blanket `/2` is wrong), and the absorb emitters'
     `kColourScale` word is 1.0. Needs a fresh original capture (three absorbs and a control on one boot) before any gain. Lead for the
     missile blast: the `+0xc84` word reads 0.5 on its mode-2 emitter and `oag_pob::Emitter::colour_scale` reads mode 5 only.
  3. The `DAMAGE` and `Absorb` labels beside the shield hex (absorb-only not isolated).
- **Pulse.** ~~The energy bar~~ (landed, `weapon-absorb-pairs.md`, "Pulse: the energy bar" and "Pulse: the stamp's bright pass reads the framebuffer"):
  the GE dump settled it, the fill stamps the glow mask (`0xff`, both blink halves) and there is no white layer, and the stamp pass reads the
  HUD target under the rectangle; ours reads 207 and 239 to 244 against 207 and 237 to 242, off-half RGB `(228, 242, 236)` against `(228, 242, 235)`
  (the original frames are gone: the original column is the earlier section's). The PS2 draw is undumped and its bloom chain has no stamp pass. HD, 2048 and Omega keep the flat white copy
  (`AbsorbGlow` is `None` for them; their draws are undumped).
  The first 0.3 s of the hull are one to two frames late and 20 % dim.
- **2048 and Omega: unpaired.** The shell fix is not checkable (neither draws a shell, their archives are not censused for an
  `AbsorbEffect` model, and their programs are GXP and PS4 bytecode that `ps3-microcode.py` does not read). 2048 has no recovered held-slot or absorb address (HD's are `0x000d9398` and
  `ship+0x5edc`); Omega has none either and its Racebox weapons row is not walked.
- **Not investigated:** AI absorbs, the PS2 port (VIF hull batches have no GE input coordinates to project), the Pulse overlay residuals
  (`LessEqual` for `EQUAL`, the per-mesh gate, the airbrake projection), the LeachBeam overlay (`leachbeam_surface.mip`), HD's cockpit shell
  (`vr_absorbinternal_cockpit`), and the sound gate (Pulse plays `ABSORB` only when the hull has collision-fx nodes, the port always).

## Next Steps

1. ~~Pulse bar: let the stamp pass read the HUD target's colour under the mask~~ (landed, `pulse-stamp-scene`; off half 223 to 226 -> 239 to 244).
2. ~~HD shell tint: read unit 0's gamma nibble~~ (read, 7; the decode alone moves 1.0 s the wrong way). Next: a pixel-mask or size-solved craft box for the pair, then the shell mesh and alpha reads (leads 1, 2 and 4 of the section above).
3. HD burst: re-shoot the original (three absorbs, one control, one boot) to settle +7.7 against +21.4 at 0.5 s; then read what writes `kColourScale` (`c467`) per psys draw. No `/2`.
4. Pure templates: the passive RAM scan on the particle-bars page to read how many template particles the original keeps at 0.5 and 0.75 s, then list `WO_WEAPON_ABSORB` in `Effects::templates` and re-pair.
5. 2048: read `Ship_PlayAbsorbFeedback` and the pickup slot in the 2048 eboot, then the HD recipe with a `/proc` write. Omega: walk the Racebox weapons row and read the same two on the eboot.
