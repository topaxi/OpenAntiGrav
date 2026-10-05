# Magstrip effects on Omega, HD/Fury and 2048: the arc wake, the POB, the sound, the rumble

2026-10-05, magfloor-omega-re lane, static reading of the PS4, PS3 and Vita
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

## Open

- **The over-the-strip predicate.** The virtual at `ship vtable + 200`, called at
  `0x017659bb` in `FUN_01762e80`, was not read. It is the mag-floor contact state; the
  PS3 logs it as the replicated field `m_overMagStrip`. Whoever wires anything starts here.
- **The blend state** of the arc batches (`FUN_012091a0(.., 1, 0, 1)`,
  `FUN_012091c0(.., 0, 0, 5)` into `DAT_020e2f88`). Do not guess additive.
- **Which `Locators.vex` entries carry `arc_anchor_point`**, and their offsets.
- **The sound law.** `FUN_012f8c70` starts `_magstrip01`; the only site that raises its
  trigger flag (`ship+0x5f48`) is the *deactivate* branch, which also stops the sound
  (`FUN_01312770`). No start-on-enter site and no second argument were recovered, so
  "loop while over the strip" is a hypothesis, not a law.
- **Which side a 2048 mode lands on** (Vita `FUN_81000930`, `FUN_810018d4` unidentified);
  and whether `DAT_01f998e8` (suppresses the wake on the local player in some mode)
  matters.
- **HD's own update and arc build** (`0x00109858`, `0x001095e0`, `0x00109720`) were not
  read; HD's vtable order differs from the PS4's, so the PS4 per-arc constants are
  single-source (65).
- **Whether the two ribbons are `trail-ribbon.md`'s class** (55).
- HD's `wo_magstrip_lightning.pob` has no referencing string. A built path is possible.

## Next Steps

1. (2 hours) Decompile `ship vtable+200` on PS4 (find the vtable from the `FUN_01309e90`
   constructor) and name the over-strip predicate; find who reads the HD
   `m_overMagStrip` field to confirm on PS3.
2. (1 hour) Read `FUN_012091a0` and `FUN_012091c0` for the blend, then the draw is a
   wiring job with the layout in the PS4 page ("The draw").
3. (ready to wire once 1 is done) `WO_MAGSTRIP_ZONE` and `WO_MAGSTRIP_SPARKS` from the
   2048 particle set: spawn at `ship+0x8860`'s anchor, enable while over the strip.
   Pick `ZONE` for Zone-style modes, `SPARKS` otherwise (the exact PS4 condition is on the PS4 page).
4. An `oag-wire` member implements only what lands above, with a test that fails when
   the law is dropped; nothing for the sound until the start site is found.
