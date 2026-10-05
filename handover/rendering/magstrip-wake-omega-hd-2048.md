# Magstrip effects on Omega, HD/Fury and 2048: the arc wake, the POB, the sound, the rumble

2026-10-05, magfloor-omega-re lane (shape) and magstrip-omega-law lane (the law), static reading of the PS4, PS3 and Vita
executables plus an asset census. Evidence and addresses:
[ps4-omega-eu/ships-effects.md](../../docs/ghidra/functions/ps4-omega-eu/ships-effects.md)
("2026-10-05"), [vita-2048-eu-v104/ships-effects.md](../../docs/ghidra/functions/vita-2048-eu-v104/ships-effects.md),
[ps3-hdfury-eu/magstrip-wake.md](../../docs/ghidra/functions/ps3-hdfury-eu/magstrip-wake.md).
Pulse's own mechanism (two `.vex` meshes) is a different thread:
[magfloor-fx-drawn-on-pulse-runtime-check-and-pure-open.md](magfloor-fx-drawn-on-pulse-runtime-check-and-pure-open.md).
Nothing here was run live; no PS4 or Vita emulator is in the toolchain.

## What the originals do

**Two different effects, split by mode, never both on one ship (Omega 80, Vita 65).**

1. **The procedural arc wake (`MagstripWake`)** - HD-lineage content. One object per
   ship, built when `DAT_01f999e4 < 0x17` (Omega) or `FUN_81000930(..) != 0` (Vita).
   Nine arc slots, each a six-segment strip sampled from the 8x8 atlas
   `HD_electric_arc_8x8` plus a contact quad from `HD_ElectricArc_Contact`, fed from
   the ship's `arc_anchor_point` locator, walking the AI track spline, plus two
   speed-driven ribbons offset `+/- 0.28 * clamp(speed - 40, 0, 120)`. It is active while
   the ship is over a magstrip (a virtual on the ship, `vtable+200`).
2. **A `.POB` particle effect** - 2048-lineage content (`mode >= 0x17` on Omega).
   `WO_MAGSTRIP_ZONE` in Zone-style conditions, `WO_MAGSTRIP_SPARKS` otherwise.
   Both ship in the Omega archives under `Data/particles2048/` (`data00.psarc`,
   `data05.psarc`) and are named by the Vita executable. The existing `.pob` player
   (`oag_fx::psys`) plays any `.POB` by name; this half needs a trigger, not a decoder.
3. **A looping sound, cue `_magstrip01`** (`~magstrip01` in HD's `shiphd.bnk`, with a
   `### Magstrip` label), in a group `MagStrip_Player` or `MagStrip_NPC` on Omega.
4. **Rumble:** `enter_mag_rumble.xml`, `travel_mag_rumble.xml`, `exit_mag_rumble.xml`
   ship in the HD `DATA02.PSARC` and the Omega `data00.psarc`. Their reader was not
   searched for.

## Asset census (paths, not bytes)

| What | HD (PS3) | Omega (PS4) | 2048 (Vita) |
| --- | --- | --- | --- |
| arc atlas | `/data/tex/hd_electric_arc_8x8.gtf` (349,696 B) | `data/Tex/HD_electric_arc_8x8.gnf` (`data08`), `Data/particles/Tex/` and `Data/particles2048/Tex/` copies (`data00`) | `data/Tex/HD_electric_arc_8x8.gxt` named, archive not listed |
| contact quad | `/data/tex/hd_electricarc_contact.gtf` (5,632 B) | `data/Tex/HD_ElectricArc_Contact.gnf` | `.gxt` named |
| `_NonAnchored` atlas, `electricity_1x4` | in `DATA02`, **no EBOOT string names them** | `data/Tex/HD_electric_arc_8x8_NonAnchored.gnf`, `HD_electricity_1x4.gnf` | not named |
| `WO_MAGSTRIP_ZONE/SPARKS.POB` | **absent** | `Data/particles2048/` only | named in code, archive not listed |
| `wo_magstrip_lightning.pob` | `/data/psys/` (3,360 B), **unreferenced by any EBOOT string** | not found | not found |
| shaders | `MagStripArc_vp/_fp` named in the EBOOT | named in the EBOOT | named in the EBOOT |
| sound cue | `~magstrip01` in `/data/sound/shiphd.bnk` | `_magstrip01` named (bank not extracted) | `~magstrip01` named |
| rumble | three `*_mag_rumble.xml` in `DATA02` | three in `data00` | not searched |

Other bank names matching the filter (`MAGPULSE*` in `weapons_det.bnk`, `c_ElecArc[A-D]`
in `shiphd.bnk`) are weapon and collision cues, not the magstrip loop; who plays
`c_ElecArc*` was not read. The HD track-floor art (`ds_magstrip_*`, `ds_mag_wave_c`)
is the static floor, already handled in `docs/formats/rcsmaterial.md`.

Not done: the Vita PSARCs and the Omega sound banks were not listed or extracted.
`data/scratch/magfloor-omega-re/` holds the HD and Omega path lists this table came from.

## The law, ready to wire (magstrip-omega-law lane, 2026-10-05)

Evidence in the Omega page's "2026-10-05, magstrip-omega-law lane" section. All static, no
live PS4, so these are 80-90 reads, not captures.

- **Wiring target: HD/Fury first.** Omega racing is out of scope (CLAUDE.md) and HD has no
  mode split: it builds the arc wake only, never the POB. **Precondition, unverified:** the
  HD race path must produce `mag_contact`. The pieces exist - HD's `.vex` version 6 declares
  `Mag Floor Collision` (`0x3e6`, 14 objects on Talon's Junction) and
  `oag_gameplay::collision` maps it to `Surface::MagFloor` - but nobody has watched an HD
  race report `mag_contact.is_some()` over a strip, and `maglock::probe` also needs a
  `track_sample` (the AI spline). Check that first with a headless HD race over a strip.
- **Contact predicate (85).** Over a magstrip = the ship's surface probe hit a triangle of
  surface type `3` this tick (`3` is the HD-lineage `Mag Floor Collision` class byte, the
  same table `oag_vex::kdcol::class_of` reads). In our physics that is `ShipState::mag_contact.is_some()`
  (`oag_physics::maglock::probe`, the same `Surface::MagFloor` literal `3` as Pulse's
  `craft+0x240`). Use the **instantaneous** contact, not the mag-lock blend (the blend ramps
  and lingers). Edges: rising = `contact && !prev`, falling = `!contact && prev`. The
  activation is also vetoed by `ship+0x71f5 & 0x10`, and the original freezes the flag
  (no edges) while `controller+0x2d8 == 0` or `+0x2c5 & 4`; none of the three is identified,
  so a port omits them and says so.
- **Which effect (80).** `mode < 0x17` Omega-style (HD-lineage modes) builds the procedural
  arc wake; the 2048-lineage modes play `WO_MAGSTRIP_ZONE` / `WO_MAGSTRIP_SPARKS` instead,
  never both. A title with no 2048 modes (Pulse's own two-`.vex` mechanism is a different
  thread) wires only the arc wake.
- **Blend of both arc batches (85): additive RGB.** `out.rgb = src.rgb + dst.rgb` (factors
  ONE/ONE, **not** SRC_ALPHA), alpha channel `dst.a * (1 - src.a)`. Vertex alpha is fixed
  `0xb2` and is not a blend factor; whether `MagStripArc_fp` uses it is unread, so alpha
  handling and the draw's depth test/write (second state word is zero, meaning undecoded)
  are **chosen, not measured**: label them so.
- **Anchor (90).** One `arc_anchor_point` node per hull in `Locators.vex`, class `110`,
  centreline, model space, translation in row 3. HD and Omega agree on all 38 hulls; the
  per-hull values are in the Omega page and printed by
  `magstrip_anchor_ground_truth` (`#[ignore]`d). `world = ship_model * anchor`.
- **Sound (80 shape, 55 arguments).** Arming flag, initially armed. Over the strip and armed:
  start cue `_magstrip01` (HD `~magstrip01`, `shiphd.bnk`) once in group `MagStrip_Player`
  (local ship) or `MagStrip_NPC`, anchored to the ship transform; disarm. On the falling edge:
  stop the instance and group, re-arm. The cue's loop flag is bank-side and not read; the
  `[300.0, 50.0]` pair on the group and the `ship+0x648b ^ 1` argument (probably a
  paused flag) are unresolved.
- **Rumble (70), local player only.** Rising edge: `Enter_Mag_Rumble.xml`, then hold
  `Travel_Mag_Rumble.xml` as a handle while over the strip; falling edge: queue
  `Exit_Mag_Rumble.xml` and finish the held handle. Whether our front end has a rumble layer
  at all is not checked.

## Open

- **Which probe is the fifth `FUN_012582a0` call** (segment endpoints `local_db8`/`local_dc8`
  in `FUN_0131b510`); until read, "same probe as Pulse's mag-floor probe" is a reading (85
  on the literal, 65 on the index).
- ~~The arc's per-arc constants~~ - read 2026-10-05 (magstrip-hd-measure): HD's own pool (`0x002bbd60`, `0x002bb530`, `0x002bc7b0`) agrees with the PS4 on life, reach, speed blend, scale, spread, shed radius and smoothing; the five jitter scales are `0.4 + 0.6 sin(k pi/4)` (read live, two boots, 95); **HD's brightness (`0.125..0.2`), contact brightness (`0.05..0.1`) and vertex alpha (`0.3` body, `0.25` contact, float) differ from the PS4 reading our code carries (`0.7`, `0.7`, `0xb2`) and are not wired**: with them `INTENSITY` would have to be re-chosen and the fragment gain is unread. See the HD page's last section.
- **The `MagStripArc_fp` fragment program** (compiled into the EBOOT, no shader file on the disc) and the draw's depth state; HD names no `kIntensity`, so `INTENSITY = 3.0` has no counterpart to read and stays chosen; the **loop flag of
  `~magstrip01`**; `ship+0x648b`, `ship+0x71f5 & 0x10`, `DAT_01f998e8`.
- ~~Which side a 2048 mode lands on~~ - settled 2026-10-05 (magstrip-2048-pob, 70): every named mode is `< 0x17`, so the arc wake; see the Vita `ships-effects.md`.
- **The HD writer of the replicated `m_overMagStrip` bit** (bit 9 of the network flag word;
  `ShipNet_CheckSendState`, `0x00337d78`, reads it) was not located on the PS3.
- **Whether the two ribbons are `trail-ribbon.md`'s class** (55).
- HD's `wo_magstrip_lightning.pob` has no referencing string. A built path is possible.

## Next Steps

1. ~~The arc wake on the HD-lineage path~~ - landed 2026-10-05 (magstrip-wire-hd): predicate off the blend, arming law,
   `~magstrip01`, additive draw, `magstrip_wire_hd_ground_truth`. Still open from it: the two speed ribbons are not drawn;
   `INTENSITY`, jitter scales and the end-point walk are chosen; `~magstrip01` is a 35-leaf tree the reader flattens
   (leaf choice unresolved); rumble edges not wired (no rumble layer checked); the Omega `WEAPON_MODELS` row waits for Omega racing.
2. ~~`WO_MAGSTRIP_ZONE` / `WO_MAGSTRIP_SPARKS` for the 2048 modes~~ - landed 2026-10-05 (magstrip-2048-pob) as the *arc wake*, not the POB: the Vita's
   `GameMode_IsHdLineage` is true (`id < 0x17`) for every mode the executable names, Zone included, so no 2048 mode we have plays the `.POB` (both parse;
   they are the CRC-id-mode side, unreached). 2048 now builds the HD arc wake off its own `.gxt` pair; `magstrip_wire_2048_ground_truth` pins it on `tower`
   (Altima has no surface-3 triangle). Open from it: the hum is silent (the Vita `shipHD.bnk` resolves no cue by name); the arc reads faintly on 2048's
   washed-out floor; and ~~`EFFECT_DIR` is the unreached side~~ **checked 2026-10-05: 2048's own events (SP/MP `m_name`) are CRC ids >= 0x17, so they read `Particles2048` (right) and, by the same predicate, take the `WO_MAGSTRIP_*` POB branch, not the arc wake - the arc wake we draw is the HD-lineage named-mode side, which no campaign event reaches (conf 70, arbiter: Vita3K read of `0x8153fd24`)**. Next: play the POB for 2048 events and decide which side `--mode` races stand for.
3. ~~Read HD's arc build~~ - done 2026-10-05 (magstrip-hd-measure): jitter scales and paired draws wired (`oag_fx::magstrip::JITTER_SCALE`); the HD brightness/alpha law is documented, not wired. Next (2 hours, `oag-re`): decode `MagStripArc_fp` (find the program blob via TOC slot `0x008b389c`) to learn what the float vertex colour is multiplied by - that is the only route to a measured `INTENSITY`, and to why the original's arcs look several times wider (fragment gain or HD bloom). Then wire HD's brightness ramp (slots start at zero, no spawn write), the `0.3`/`0.25` alphas and the two ribbons (`0x00109858` is HD's ribbon update).
