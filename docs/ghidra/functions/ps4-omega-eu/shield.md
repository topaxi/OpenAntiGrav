# Omega's Shield pickup: HD's law, a mode-gated colour rate, and the `ShieldColour` value decoded

**Binary:** `ps4-omega-eu` base `eboot.bin` (Ghidra `/omega/eboot-ps4-omega-eu.bin`, x86-64, 8-byte
pointers, image base `0x01000000`). Read 2026-11-03, lane `omega-2048-shield-read`: Ghidra decompile,
**static reading, no live capture** (Omega has no memory readout), so every claim here is capped at
**85** (HD's own page, [`shield.md`](../ps3-hdfury-eu/shield.md), is capped at 88 with a live leg).
The 1.07 patch `eboot.bin` was not searched for these functions.

**Verdict.** Omega's `Shield.cpp` is HD's `Shield.cpp` constant for constant, with one law difference
HD does not have: the colour-lerp rate depends on the game mode. The per-team `ShieldColour` is the
material parameter of the shell's `shipshield.rcsmodel`, **four little-endian `f32`**, and every team's
value is read below. Omega's palette is **not** HD's: the inheritance currently wired
(`ShieldPalette::Hd` for the colour set, HD's per-team table) is wrong for the colours that differ
(table below).

| Address (base) | Name | Confidence |
| --- | --- | --- |
| `0x0136f5f0` | `ShipShield_Construct` | 82 |
| `0x0136fda0` | `ShipShield_BindColourParam` | 85 |
| `0x0136fed0` | `ShipShield_Activate` | 85 |
| `0x0136e880` | `ShipShield_Update` | 85 |
| `0x0136ef30` | `ShipShield_DrawCockpit` | 65 |
| `0x01a15700` | `g_shipshield_activation_colour` (data) | 85 |
| `0x01a15710` | `g_shipshield_hit_colour` (data) | 85 |

`FUN_0136f3f0` writes the two colour blocks above, but in the same function body also a run of
unrelated 32-bit constants (the tail of a CRC table); it is left as `FUN_` rather than named for half
of what it does.

## How the class was found

`search_strings "Shield"` finds `C:\WOPS4\Wipeout\Code\Backend\Weapons\Shield.cpp` at `0x01816852`,
`%s\%sshield.vex` at `0x01816883` and `vr_shield_cockpit.vex` at `0x01816aa7`; all three are
referenced only from `FUN_0136f5f0`, the same way HD's page found its constructor. The `ShieldColour`
string itself is **not** in Omega's executable: the constructor and the bind hash it at build time
(the inlined CRC loop writes `-0x550acee7` = `0xaaf53119` = `~crc32("ShieldColour")`, the value HD's
page and `oag_fx::absorb_shell::SHIELD_COLOUR` carry).

## `ShipShield_Construct` (`0x0136f5f0`, 82)

Same skeleton as HD's `ShipShield_Construct` (`0x00126418`): tag `Shield.cpp` at object `+0x58`,
craft at `+0xb0`, then two model loads.

- **Cockpit sphere.** `snprintf("Data/Weapons/%s" or "Data/Weapons2048/%s", "vr_shield_cockpit.vex")`;
  `Data/Weapons/...` when `DAT_01f999e4 < 0x17` (the mode id, [`weapons.md`](weapons.md)), otherwise
  `Data/Weapons2048/...`. Stored at `+0xc0`. HD has one fixed path, so this is new.
- **Shell.** `snprintf("%s\\%sshield.vex", *(*(craft+0x6910)+0x220), team_model)` where `team_model`
  is the `FE_TeamModel` config value or the literal `"ship"` (read at `0x0183bbe8`: `ship`). Stored at
  `+0xc8`.
- Each model is bound with `FUN_0136fda0` (below).
- **Target alpha is a maximum.** After both binds: `if (shell_alpha < cockpit_alpha) target.a =
  cockpit_alpha` (`+0xdc`), where a model that failed to load counts as `1.0`. Each bind overwrites all
  of `+0xd0..+0xdc`, so the target RGB is the **shell's** (it loads last) and the alpha is the larger of
  the two. Consequences for the values below: in HD-content modes (`< 0x17`) the cockpit
  `Data/Weapons/vr_shield_cockpit` carries alpha `1`, so every HD-content shield targets alpha `1`
  even for Zone, whose own shell authors alpha `0`; in 2048-content modes both models author alpha
  `0`, so the target alpha is `0`.
  **HD's page does not record this compare** (it says only that the shell's value is the one left in
  `object+0x104`); the same compare is in Vita's constructor (see the 2048 page), so HD very likely has
  it too. Open on HD, not re-derived here.
- Defaults before any bind: `+0xd0..+0xdc` = `(1,1,1,1)` (`_DAT_017ecbd0`), `+0x80..+0x8c` =
  `(1,1,1,1)` (`_DAT_018078a0`). A shell with no `ShieldColour` therefore flies with white
  (`Detonator`'s `shipshield` and `VR_FURY_n1`'s carry none).

## `ShipShield_BindColourParam` (`0x0136fda0`, 85)

HD's `0x00125690`, inlined differently. For model slot `m` it walks the model's meshes
(`mesh+0x48` entry count, `mesh+0x50` entries), each entry `0x28` bytes: `+0x00` u32 hash, `+0x04` u16
kind (`1` = a uniform), `+0x10` pointer to the value. The first entry with hash `0xaaf53119` and kind
`1`:

1. copies **16 bytes** (four `f32`) from the value pointer to the shield's `+0xd0..+0xdf` (the target);
2. in a second walk, replaces the entry's value pointer with `shield+0x80` (the live colour), setting
   bit `0` of the byte at `entry+7` when `entry+0x1d & 0x20`.

So the live colour is bound as the shader's `ShieldColour` instance parameter on both models, exactly
as on HD. **The encoding on Omega is plain `f32`.**

## `ShipShield_Activate` (`0x0136fed0`, 85)

Takes the shield only (no colour argument, as HD's corrected page says). In order:

- both models get a per-submesh tint (`submesh+0xc4`) of the activation colour times 255, packed
  `R | G<<8 | B<<16 | A<<24` (HD's `FUN_002c32c8` equivalent);
- `+0x90..+0x9c` (the lerp target `Update` reads) `= +0xd0..+0xdc` (the target bound at construction);
- `+0x80..+0x8c` (live colour) `= g_shipshield_activation_colour` = **`(0.547, 0.1, 0.1, 0.0)`**
  (`0x01a15700`: floats `0x3f0c0831`, `0x3dcccccd`, `0x3dcccccd`, `0`; read statically);
- **`+0xa0` (colour rate) `= 0.15` (`0x3e19999a`) when `DAT_01f999e4 < 0x17`, else `0.05`
  (`0x3d4ccccd`).** HD's constant is `0.15`; the `0.05` is Omega's (and Vita's) 2048-content rate;
- `+0xa4` (swell) `= 0.7` (`0x3f333333`), `+0xa8` (swell target) `= 1.0`, `+0xac` (swell rate)
  `= 0.2` (`0x3e4ccccd`); `+0xb8` (active) `= 1`, `+0xb9` (fading) `= 0`, `+0xbc` (time) `= 0`.

HD's live read had the swell starting near `0.70`; here the constant is read.

Timing, from the per-step rate: the lerp `c += (target - c) * rate` runs once per 1/60 s step, so at
60 Hz the colour has covered 99 % of the distance after `ln(0.01)/ln(1-rate)` steps: **28 steps
(0.47 s) at `0.15`, 90 steps (1.5 s) at `0.05`**. HD's page quotes "about 2 s" off an RPCS3 run; the
step count is `int(dt * 59.999996)` per frame, so a frame slower than 1/60 s can run fewer steps than
elapsed time implies and that figure is emulator-rate dependent. Do not carry "2 s" over.

## `ShipShield_Update` (`0x0136e880`, 85)

`(dt, shield)`. Active flag `+0xb8`.

```text
steps = (int)(dt * 59.999996)
repeat steps: live.rgba += (target(+0x90) - live) * rate(+0xa0)      // whole vec4
repeat steps: swell(+0xa4) += (swell_target(+0xa8) - swell) * swell_rate(+0xac)
n     = _FSin(time(+0xbc))                                             // plain sin
alpha = live.a * (n * 0.25 + 0.75)
tint  = pack(live.rgb, alpha) * 255 on each model's submeshes (+0xc4)
draw  -> FUN_0136ef30 (cockpit/shell scale = (n * 0.012 + 0.012 + swell) * 1.8)
time += dt
if (!(0.1 < live.a) && fading(+0xb9)) active = fading = 0, hide both models
```

This **reads** the two items HD's page could only argue structurally: the flicker is a bare `sin`
(`_FSin`, one argument) and the alpha term is exactly `n * 0.25 + 0.75`. They corroborate HD's
`ALPHA_FLICKER`/`ALPHA_MID` and `flicker = sin` by lineage; they are not an HD measurement. The
fade-out threshold `0.1` and the swell numbers `0.012`, `1.8` also match.

The live **raw** colour (`+0x80`) is what the shader's `ShieldColour` parameter reads; the tint
channel carries `alpha * (sin*0.25+0.75)`. Two channels, as in HD's Update.

`FUN_0136ef30` (`ShipShield_DrawCockpit_q`, 65): carries the `0.012` / `1.8` scale and the model
transform; named at 65 on the constants alone because which of the two models it draws was not
traced, and the Vita twin (`FUN_812089e4`) is the one that reads the cockpit slot.

## `ShipShield_Hit` is inlined, not a function

Three sites write the hit colour and the swell `1.1`, each guarded on `shield+0xb8` (active):
`FUN_012f2be0` (`0x012f2f1a` region, the contact path), `FUN_012eaf20` (`lVar26 + 0x80 =
_DAT_01a15710`, `+0xa4 = 0x3f8ccccd`) and `FUN_01762e80`. They store `g_shipshield_hit_colour`
(`0x01a15710`) = **`(1.0, 0.2, 0.0, 1.0)`** (`0x3f800000`, `0x3e4ccccd`, `0`, `0x3f800000`) at `+0x80`
and `1.1` at `+0xa4`. None of the three is a Shield function, so none is named.

## The `ShieldColour` value, per team (4 x f32, little endian)

Read with `oag_rcs::rcsmodel::psp2` (the Vita container with 8-byte pointers) over every
`*shield*.rcsmodel` in `data00`-`data04` and the patch's `data05`/`data07`-`data09`: for each model
material, the parameter whose hash is `0xaaf53119`. All 12 base teams carry one. (The reader's
parameter pool cuts the last component off a value that ends exactly at the pool end; the game copies
16 bytes regardless. That affects only the 2048 magnetic shield below, whose fourth component is
read as `0` on Vita.)

HD-content shells, `Data/art/published/hdships/<team>/shipshield.rcsmodel` (the base ship; `_c1`/`_n1`
variants author their own values, 24 files, listed in the third column's note):

| Team | Omega `ShieldColour` RGBA | HD (for comparison) | Same as HD? |
| --- | --- | --- | --- |
| feisar | (0.9584, 0.9956, 0.0000, 1) | (0.0000, 0.0314, 0.9961, 1) | no |
| assegai | (0.2110, 0.6567, 1.0000, 1) | (0.2110, 0.6567, 1.0000, 1) | yes |
| piranha | (1.0000, 1.0000, 0.0000, 1) | (1.0000, 0.5333, 0.0000, 1) | no |
| qirex | (1.0000, 0.0417, 1.0000, 1) | (0.7294, 0.1843, 0.5490, 1) | no |
| goteki | (0.7490, 0.2039, 0.7725, 1) | (0.7490, 0.2039, 0.7725, 1) | yes |
| egx | (0.2196, 1.0000, 0.2196, 1) | (0.2196, 1.0000, 0.2196, 1) | yes |
| ag_systems | (0.2652, 1.0000, 1.0000, 1) | (0.2652, 1.0000, 1.0000, 1) | yes |
| triakis | (1.0000, 0.9327, 0.2793, 1) | (0.4078, 0.3804, 0.1137, 1) | no |
| zone | (1.0000, 0.3022, 0.0000, alpha unread) | (0.4078, 0.3804, 0.1137, 1) | no (HD's DATA00 copy matches) |
| auricom | (0.1050, 0.1050, 1.0000, 1) | (1.0000, 1.0000, 1.0000, 1) | no |
| harimau | (0.0000, 1.0000, 1.0000, 1) | (0.0000, 1.0000, 1.0000, 1) | yes |
| icaras | (0.0000, 0.0314, 0.9961, 1) | (0.0000, 0.0314, 0.9961, 1) | yes |
| mirage | (1.0000, 1.0000, 0.0000, 1) | (1.0000, 0.1142, 1.0000, 1) | no |
| tigron | (1.0000, 0.0417, 1.0000, 1) | not in HD | - |
| tigron_x | (0.0000, 0.3927, 1.0000, 1) | not in HD | - |
| detonator | none (default white) | none | - |

`_c1` / `_n1` variants (Omega `data01`/`data03`): feisar_c1/_n1 (0.0, 1.0, 0.6577), assegai_c1/_n1
(1.0, 0.8185, 0.2035), egx_c1/_n1 (1.0, 0.1594, 0.9965), icaras_c1/_n1 (0.9736, 0.2403, 1.0),
mirage_c1/_n1 (0.0, 1.0, 0.0938), goteki_c1/_n1 and piranha_c1/_n1 (1.0, 1.0, 0.0), triakis_c1/_n1
(1.0, 0.7831, 0.2280), auricom, harimau, ag_systems, qirex variants equal their base.

2048-content shells, `Data/art/published/ships/<team>2048/<1..4>/shipshield.rcsmodel` (ag_systems,
auricom, feisar, piranha, qirex x 4) and `ships/VR_2048/<1..4>`: **all carry
(0.0000, 4.5948, 4.5948, alpha not read by the reader)**, material `2048_Magnetic_Shield`. The
cockpit `Data/weapons2048/vr_shield_cockpit.rcsmodel` carries the same. The values are above `1.0`
(HDR cyan). Alpha is `0` on Vita (the pool is intact there, see the 2048 page).

Patch 1.07 (`data08`) overrides: `Data/weapons/vr_shield_cockpit.rcsmodel` becomes
(1.0, 0.0417, 1.0, 1) (the base is (0.2652, 1, 1, 1)), and `HDShips/VR_HD`, `HDShips/VR_FURY` shells
are (1.0, 0.0417, 1.0, 1). The cockpit's RGB never reaches the screen (the shell binds last), only its
alpha does.

## What the 2048 and HD checks say

- 2048: **ported (same code, same values)**. See [`../vita-2048-eu-v104/shield.md`](../vita-2048-eu-v104/shield.md).
- HD: **checked, differs** on seven of the thirteen teams' colours (table), and on the colour rate
  in 2048-content modes. The activation colour, hit colour, swell numbers, flicker, `0.1` fade-out
  and `59.999996` step count agree.

## Open

- The `2048_Magnetic_Shield` program's use of `ShieldColour` (alpha `0`, RGB above `1`) is unread on
  both titles: whether the program multiplies by `.a` decides if a 2048 shell is visible. Recipe:
  `cargo run -p oag-rcs --example gcn_dump -- <data00.psarc> data/art/published/ships/materials/2048_Magnetic_Shield.rcsmaterial`
  (its `.gcn` programs list the `ShieldColour` constant and what reads it).
- Whether `ship_dir` reaches `hdships/<team>_c1` or `ships/<team>2048/<n>` for each variant in a race
  is unmeasured here, as on HD.
- `0x0136ef30`'s model slot, the second draw routine, and the Omega 1.07 addresses.


## Wired (lane `omega-2048-shield-wire`)

- `oag_livery::shield` builds the shell through `psp2::build` and reads `ShieldColour` with `oag_rcs::rcsmodel::psp2` (a missing fourth component reads `0.0`, unread; the cockpit floor below makes it moot). Ground truth: `crates/raceplay/tests/omega_2048_shield_colour_ground_truth.rs` (every one of 13 teams' colours and a built shell).
- Target alpha is `max(shell.a, cockpit.a)` (`ShieldPalette::Omega`, `Palette::target_alpha_from_cockpit`, `Origin::Measured`); the cockpit is `Data\Weapons\vr_shield_cockpit` with alpha 1, so Omega's HD-content shells always target alpha 1.
- The colour rate is `Palette::colour_rate`; `0.05` (`CONTENT_2048_COLOUR_RATE`) exists but no palette carries it: every mode this project launches on Omega and 2048 is a named one (`< 0x17`), so the `0.15` branch is the one that runs. A mode `>= 0x17` (2048's own campaign events) would need the mode id threaded to the palette and `Data\Weapons2048\vr_shield_cockpit` as its cockpit.
- Render-only: `ShipShield` is not simulation state; no hash or lone-craft gate moves.

## The `2048_Magnetic_Shield` program (read 2026-11-03, PS4 pixel shader, confidence 70)

`ShieldColour` is a 12-byte (three-component) constant in the pixel program's cbuffer: only `.rgb` is loaded. Output RGB is `ShieldColour.rgb * Texture1 * attr0.w * attr2.x`, lerped toward fog; output alpha is `tex.r * attr2.x`, where `attr2.x` is the vertex stream's `VertexColour1.w`. So `ShieldColour.a` never reaches a pixel through this program, and the alpha `0` authored on these shells does not by itself hide them. Drawn on 2048 (HDR cyan `(0, 4.59, 4.59)`, floored to target alpha 1 by the cockpit rule): a cyan lattice around the hull. Not read on the Vita `.gxp` side (`checked, applies, not wired` for the Vita program itself: the same material name and the same constant name).

## The two shell programs (PS4 GCN, read 2026-11-03, confidence 70)

Per-team shells (`ships/materials/2048_shipshield_old`, Omega's `hdships/<team>` shell except Zone's own `zonebattle_shield`, state word `0x3d`, blended, culling) and the 2048-content shells (`2048_Magnetic_Shield`) run different pixel programs:

- **`2048_shipshield_old`** (read as above, then in full 2026-11-03 off the unlit pixel variant `@0xee4`, 89 instructions, and checked against the other six variants): alpha is `ShieldColour.a * diffuse.a(uv1) * VertexColour1.a` (`image_sample ... dmask:0x8`, unscrolled). Colour is `ShieldColour.rgb * constantAmbientColour * diffuse.rgb(u1 + 3 * time, v1)` (a second diffuse tap, `v_madmk_f32 .., 3.0, ..`, **the scroll is on the diffuse colour, not on the lightning**) **plus** `Lightning_RGB_Multiplier * lightning.rgb(uv2 + time + noise)`, where `noise = lightning.a(u2, 0.8 * v2 + 0.16 * time)` (the lightning picture's alpha, `cf_plasma`). The lightning term is **not tinted** by `ShieldColour`. Fog lerps the result as in every program. Lit variants add `liveLighting0` and `shCoeffs`; their three literals are the same. `3.0`, `0.8` and `0.16` are literals of the program, not parameters; the weight `Lightning_RGB_Multiplier` is the material's (Assegai authors 3, Icaras 1; per team in the `.rcsmaterial` instance). Reproduce: `cargo run -p oag-rcs --example gcn_dump -- data/extracted/ps4/omega-eu/uroot/data00.psarc data/art/published/ships/materials/2048_shipshield_old.rcsmaterial`. Where the compiler put the three literals differs across the seven Lightning pixel variants (their order is not stable); `oag_rcs::gcn::lightning_rates` finds each by data flow (a `v_madmk_f32` whose result addresses a diffuse sample is `A`, one that addresses a lightning sample is `B` and its addend was last written by the `v_mul_f32` that carries `C`), pinned by `gcn_ground_truth::the_shipshield_programs_carry_the_lightning_rates_in_every_variant`.
- **`2048_Magnetic_Shield`**: red channel as alpha, three-component `ShieldColour`.

**Vertex program (`@0xa84`):** inputs `position`, `VertexColour1`, `Uv1_Uv2` (four halves, hash `0x1aefe524`); `param1` is the first pair, `param2` the second, `param3` the vertex alpha, `param0.w` the fog factor. The shell's declaration names `Uv1` and `Uv2` outright, **both at the offset of the second pair (24)**; the first pair (offset 20, a 0..1 unique unwrap) is what the program's `Uv2` taps. **Measured by picture, not by the declaration:** read at offset 20, the lightning draws coherent arcs across the hull; the reverse (the vertex program's literal reading, first pair as the diffuse) draws a faint smear. The existing reader's diffuse (offset 24, tiled) is unchanged. Confidence 60 for the pair assignment; the mismatch with the vertex program's operand order is unexplained.

Wired (lane `omega-shield-lightning`): the alpha channel of the first (`slots::alpha_channel(3)`), HD's `SRC_ALPHA`/`ONE` for the first material by equivalence with `hexagonalshield_rich` (`lineage_blend::EQUIVALENT`, census pinned in `hd_lineage_blend_ground_truth.rs`), and the Lightning layer: `psp2::shield_lightning` plans one glow-table entry (`PROGRAM_SHIELD_LIGHTNING`, tint `(weight, A, C)`, offset `B`), binds `cf_plasma` third and the first pair as `texcoord2`; `shade.wesl` scrolls the diffuse at `A * time`, adds the lightning after the `ShieldColour` multiply. Checks: `render::omega_shield_lightning_lit_path` (the scroll follows `A`, the weight scales the term), `raceplay::omega_2048_shield_colour_ground_truth` (every Omega team but Zone carries the entry, the picture and the program's literals; the 2048 magnetic shield carries none).

**Not wired / open:** `constantAmbientColour` (the program multiplies the diffuse colour by it; the scene's value is not read here, so the multiplier is the implicit 1), the lit variants' `liveLighting0` and `shCoeffs` terms (the shell takes the unlit reading), and Zone's `zonebattle_shield` program (unread; no lightning is drawn for it). The picture reads as blue-white crackle over the hull with a faint hexagon lattice, not HD's glassy hexagon shell; the dimness of the lattice is the unread ambient factor until a live Omega capture says otherwise (wipeout_pc is the Omega race original, `docs/reverse-engineering/`).

**2048/Omega cross-check:** the Vita 2048 shell is the magnetic shield (`2048_Magnetic_Shield` family; `vita-2048-eu-v104/shield.md`); no `2048_shipshield_old` or `LightningTexture` sampler was found in the extracted Vita files, and the builder gates the layer on the PS4 container. Checked, differs (2048 has no such shell); the Vita program of `2048_shipshield_old` is not checkable from this extraction.

The builder's default for every Vita/PS4 model reads the **red** channel as alpha; for the shield it is overridden (above). Other materials: unexamined beyond the census in the lane report.
