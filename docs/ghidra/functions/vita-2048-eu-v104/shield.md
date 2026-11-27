# 2048's Shield pickup: the Omega code on ARM, and the `ShieldColour` encoding (4 x f16)

**Binary:** `vita-2048-eu-v104` `eboot.elf` (Ghidra `/2048/eboot-vita-2048-eu-v104.elf`, ARM Thumb-2,
image base `0x81000000`). Read 2026-11-03, lane `omega-2048-shield-read`: Ghidra decompile, **static
reading, no live capture**, capped at **85**. Checked against Omega's twin
([`../ps4-omega-eu/shield.md`](../ps4-omega-eu/shield.md)) function for function: same skeleton, same
constants, same mode gate. The values below are 2048's own and agree with Omega's.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x81208240` | `ShipShield_Construct` | 82 |
| `0x8120807c` | `ShipShield_BindColourParam` | 85 |
| `0x8120851c` | `ShipShield_Activate` | 85 |
| `0x81208b28` | `ShipShield_Update` | 85 |
| `0x81208d16` | `ShipShield_Hit` | 85 |
| `0x812089e4` | `ShipShield_DrawCockpit` | 70 |
| `0x812087bc` | `ShipShield_DrawShell` | 70 |
| `0x818ae574` | `g_shipshield_activation_colour` (data) | 85 |
| `0x818ae564` | `g_shipshield_hit_colour` (data) | 85 |

`FUN_81208d4c` writes the two colour blocks and also a run of unrelated hash constants; left as
`FUN_`. `FUN_81000930` is already named `GameMode_IsHdLineage` ([`ships-effects.md`](ships-effects.md)):
it returns `1` for mode `< 0x17`, the test this page calls "HD content".

## Found the same way

`search_strings`: `Backend/Weapons/Shield.cpp` `0x814a4108`, `vr_shield_cockpit.vex` `0x814a413c`,
`%s\%sshield.vex` `0x814a4154`, all referenced from `FUN_81208240` only. The `FE_TeamModel` fallback
is read at `0x814a4124`: `ship`.

## The law, against Omega

Object layout (Vita, 32-bit): live colour `+0xc0`, lerp target `+0xd0`, colour rate `+0xe0`, swell
`+0xe4` / target `+0xe8` / rate `+0xec`, active `+0xf4`, fading `+0xf5`, time `+0xf8`, cockpit model
`+0xfc`, shell model `+0x100`, bound target `+0x104..+0x110` (HD's `+0x104`, so 2048's layout is HD's;
Omega's is the same fields shifted).

- **`ShipShield_Construct`** (`0x81208240`): the cockpit path is `FUN_8120c3b4("vr_shield_cockpit.vex")`,
  `Data/Weapons/%s` when `FUN_81000930(.., -1) != 0` (mode `< 0x17`), else `Data/Weapons2048/%s`; the
  shell is `%s\%sshield.vex` with `*(*(craft+0x58e8)+0x1b0)` and `FE_TeamModel` / `"ship"`. Each model
  goes through the bind. At the end, **`if (shell_alpha < cockpit_alpha) target.a = cockpit_alpha`**
  (`param_1[0x44] = fVar6`), a missing model counting as `1.0`: Omega's target-alpha maximum, on both
  titles (and, by the HD lineage, probably on HD, unrecorded there).
- **`ShipShield_BindColourParam`** (`0x8120807c`): looks `~crc32("ShieldColour")` up in the model's
  parameter list (`FUN_812f3c4a`), reads **four IEEE half floats** from the value pointer, widens them
  (exponent `0` and exponent `0x1f` read as `0.0`, so a half subnormal is flushed to zero and Inf/NaN
  to zero), stores the four `f32` at `+0x104..+0x110`, then rebinds the parameter pointer of every
  mesh's entry to `shield+0xc0` (`FUN_812edc80`).
- **`ShipShield_Activate`** (`0x8120851c`): tints both models with the activation colour
  (`FUN_812885e0`, packed ARGB of `* 255`); lerp target `+0xd0..` = bound target; live `+0xc0..` =
  `g_shipshield_activation_colour` = `(0.547, 0.1, 0.1, 0.0)` (written by `FUN_81208d4c` as
  `0x3f0c0831, 0x3dcccccd, 0x3dcccccd, 0`); **rate `+0xe0` = `0.05` (`0x3d4ccccd`) when
  `FUN_81000930 == 0`, `0.15` (`0x3e19999a`) otherwise**; swell start `DAT_81520138` = `0x3f333333` =
  `0.7`, target `1.0`, rate `0.2` (`0x3e4ccccd`); active `1`, fading `0`, time `0`.
- **`ShipShield_Update`** (`0x81208b28`): `steps = (int)(dt * 59.999996)`, the same per-channel lerp
  and swell lerp, `n = sin(time)` (`SceLibm_4A496BC0`), `alpha = live.a * (n * 0.25 + 0.75)`, a camera
  check (`DAT_8154135c`, the craft's `+0x55e7` flag) choosing `FUN_812089e4` (cockpit: scale
  `(swell + 0.012 + n*0.012) * 1.8`) or `FUN_812087bc` (shell), `time += dt`, and the same
  `fading && live.a <= 0.1` switch-off.
- **`ShipShield_Hit`** (`0x81208d16`, a real function here, unlike Omega where it is inlined at three
  sites): `live = g_shipshield_hit_colour` `(1.0, 0.2, 0.0, 1.0)` (`0x3f800000, 0x3e4ccccd, 0,
  0x3f800000` at `0x818ae564`), `swell = 1.1` (`0x3f8ccccd`).

So the rate rule is: **HD content `0.15`, 2048 content `0.05`.** On both titles the two content
families are decided by the same mode id, and the 2048-content shell is a different shell (below).

## The `ShieldColour` encoding, per container

| Title | Value storage | Component order |
| --- | --- | --- |
| Vita 2048 | four IEEE `binary16` (half) at a pointer in the `0x18`-byte parameter entry (`+0x0c` offset, pool `0x2000` = half pool) | R, G, B, A |
| Omega | four `f32` at the entry's `+0x10` pointer (`0x28`-byte entries) | R, G, B, A |

Both are read by `oag_rcs::rcsmodel::psp2` (`Material::params`). **One reader caveat:** the reader
widens half subnormals (`Feisar_c1` red is `0x384e0000` as `f32`, a half subnormal); the game reads
`0.0`. Flush exponent-`0` halves to zero when wiring. The Omega values differ from the Vita's only by
the half quantisation (Omega `0.2652` against Vita `0.2651`, the same artist value).

## Per-team values on 2048

Base `data.psarc` (`PSP2/data.psarc`), then the v1.04 patch (`data2.psarc` replaces 12 `_n1`
directories); the **effective** value is the patch's. HD-content shells, `hdships/<Team>/shipshield`
(2048's own naming has capital letters):

| Team | RGBA (alpha 1) | Differs from HD? |
| --- | --- | --- |
| Feisar | (0.9585, 0.9956, 0.0000) | yes (HD blue) |
| Assegai | (0.2109, 0.6567, 1.0000) | no |
| Piranha | (1.0000, 1.0000, 0.0000) | yes (HD orange) |
| Qirex | (1.0000, 0.0417, 1.0000) | yes |
| Goteki | (0.7490, 0.2040, 0.7725) | no |
| EGX | (0.2196, 1.0000, 0.2196) | no |
| AG_Systems | (0.2651, 1.0000, 1.0000) | no |
| Triakis | (1.0000, 0.9326, 0.2793) | yes |
| Zone | (1.0000, 0.3022, 0.0000), alpha **0** | as HD DATA00 |
| Auricom | (0.1050, 0.1050, 1.0000) | yes |
| Harimau | (0.0000, 1.0000, 1.0000) | no |
| Icaras | (0.0000, 0.0314, 0.9961) | no |
| Mirage | (1.0000, 1.0000, 0.0000) | yes |
| Detonator | none (white default) | none |

`_c1` variants equal Omega's `_c1` list. `_n1` variants: the base package differs from Omega's
(`Assegai_n1` (0.7788, 0.7451, 0.1387), `EGX_n1` (0.6943, 0.0907, 0.7139), `Icaras_n1`
(0.7061, 0.1355, 0.7061), ...), but **the v1.04 patch's 12 `_n1` files equal Omega's `_n1`** (for
example `Assegai_n1` (1.0, 0.8184, 0.2035), `Icaras_n1` (0.9736, 0.2402, 1.0)); Omega shipped the
patched art.

2048-content shells: `ships/<team>2048/<1..4>/shipshield.rcsmodel` (ag_systems, auricom, feisar,
piranha, qirex; 20 files) all author **(0.0, 4.5938, 4.5938, 0.0)**, material
`2048_magnetic_shield.rcsmaterial`; so does `Data/Weapons2048/vr_shield_cockpit.rcsmodel`. RGB above
`1.0` (HDR cyan), alpha `0`; with the target-alpha maximum, a 2048-content shield has target alpha `0`
because no model authors another.
`Data/Weapons/vr_shield_cockpit.rcsmodel` (HD content) is (0.2651, 1, 1, 1), material
`hd_shipshield_old`.

## What it takes to build their shell (feeds the wire thread)

Not a missing format: `oag_mesh::mesh::rcs::psp2::build` builds every shield geometry on both
titles. Probed with the texture callback returning nothing (textures were therefore not exercised):

| File | Vita | Omega |
| --- | --- | --- |
| `hdships/<feisar>/shipshield.rcsmodel` | 3351 vertices, 2193 triangles, 1 submesh | same |
| `ships/feisar2048/1/shipshield.rcsmodel` | 1728 vertices, 2496 triangles, 1 submesh | same |
| `Data/Weapons/vr_shield_cockpit.rcsmodel` | 1152 vertices, 768 triangles | same |
| `Data/Weapons2048/vr_shield_cockpit.rcsmodel` | 280 vertices, 328 triangles, 2 submeshes | same |

What refuses today: `shield_model` in `crates/livery/src/shield.rs` always takes the PS3 route
(`mesh::rcs::build`) for a `.vex` with external geometry, with no `is_psp2` branch as the hull path in
`crates/livery/src/lib.rs` has; and `shield_colour()` parses with the PS3 `rcsmodel::Model::parse`,
which rejects the PS4/Vita container, so the colour is `None`. Both titles ship `shipshield.vex`
beside `shipshield.rcsmodel` in every directory above.

## Cross-title status

- Omega: **ported** (same code, same values; [`../ps4-omega-eu/shield.md`](../ps4-omega-eu/shield.md)).
- HD: **checked, differs**: palette (seven of thirteen teams), colour rate in 2048-content modes,
  the cockpit path by mode. Law otherwise identical.

## The HUD: `ManualShield` drains while the Shield runs

2026-11-27, lane `shield-countdown-titles`. Static reading of the disassembly,
no live frame of the original with a Shield up (Vita3K was in use).
**Confidence 80** on the crop, **70** on which branch an ordinary race takes.

`ManualShield` is a 72 x 134 image under `DamageBarParent` in
`data/xml/2048_hud/HUD_damage_indicator.xml`, the bright outline over the dim
one round the energy silhouette. `Hud_BindWidgets` (`0x81192cd0`) keeps it at
`hud+0x6c` and hides it.

**`Hud_UpdateWidgets` (`0x81195cdc`)**, every update:

- `0x81195e44`: `hud+0x10 & 0x8000` clear branches to `0x8119606c`.
- There `slot = *(ship+0x557c)`. With `slot+0x1bc > 0`,
  `f = min(slot+0x1b8 / slot+0x1bc, 1)`, else `0`.
- The widget is made visible, its height is `hud+0x1f4 * f`, its y
  `hud+0x200 + hud+0x1f4 * (1 - f) / 2` (the sprite is centred, so the bottom
  edge holds), its source height `hud+0x1fc * f` and its source v
  `hud+0x1f8 + hud+0x1fc * (1 - f)`. A bottom-anchored crop: it drains from the
  top.

**The `0x8000` gate.** The race managers `OR` their widget set into `hud+0x10`
after `Hud_Construct` (`0x81192940`) zeroes it, and none carries `0x8000`:
`0x200df` (`SpArcadeRaceManager_Construct`, `FUN_81160574`, `FUN_8116471c`,
`FUN_811853ec`), `0x10097` (`FUN_81161170`, `FUN_8117fb48`), `0x97`
(`FUN_81183e50`), `0x2` (`FUN_81161c74`, `FUN_81182148`, `FUN_8117c5ce`,
`FUN_81186584`), and `0xdf | 0x20000`, `0x20800` in the factory at
`0x812bc7b4`. With the bit set `FUN_81195620` would drain by the ship's
`ManualShieldStats` instead (`life_time`, `recharge_time`). A writer using
another instruction form is not excluded.

**The slot's two timers.** The Shield's start writes `+0x1b8` and `+0x1bc` to the
same duration, clears the held id (`+0x1f8 = -1`, so the weapon box is empty)
and sets `+0x1f4 & 0x10`; the per-slot update (`0x8120e906`) takes `dt` off
`+0x1b8`. The percentage under the silhouette is unchanged:
`Hud_UpdateEnergyBar` (`0x811957a2`) reads no shield timer.

**HD's countdown is in this executable and has nothing to draw on.**
`FUN_8115dfea` is [`Hud_SetShieldTimer`](../ps3-hdfury-eu/hud-readouts.md#the-shield-countdown)'s
twin and `FUN_81156220` prints `%2.1f` in `0xff30ff30`, both through
`DamageBarShieldBg`. No layout a 2048 race loads authors that widget or
`ShieldBarText`: the `2048_hud\` copy of `HUD_damage_indicator.xml` has neither
(only the root copy, which the demo roots include, does).

**Wired**: `oag_title::HudArt::shield_drain_widget`, drawn by
`oag_hud::dialect_2048::shield_drain` from `Readout::shield_drain_fraction`
(the running timer over its value when it was armed). Test:
`crates/raceplay/tests/vita_2048_hud_event_ground_truth.rs`.

**Omega**: **ported, inherited from 2048**. `data00.psarc`'s
`2048_hud\HUD_damage_indicator.xml` authors `ManualShield` at its own atlas's
rectangle and the executable holds the string (`0x018757ae`, no reference
found), so Omega's update was not read. **HD**: no `ManualShield`; it draws the
countdown instead.

**Open**: how long the pickup's Shield lasts in 2048's own modes. Under
`GameMode_IsHdLineage` (`0x81000930`) the start at `0x8120eaae` takes the time
from the weapon table's side (`0x8120eb70`); otherwise it takes the ship's
`ManualShieldStats` `life_time` (`stats+0x4cc`, `3` in all 20 base
`handlingstats.xml`), where the Race table's Shield authors `5`. Confidence 60,
not wired: it is simulation, and ours always takes the table's.

## Open

- How the `2048_Magnetic_Shield` program uses `ShieldColour` (alpha `0`, RGB `4.59`): unread. Recipe:
  `oag_rcs::gxp` over the material's `.gxp` programs for the constant named `ShieldColour` and its
  consumers; the 2048 shell is invisible if the program multiplies by `.a`.
- The two draw routines were read for constants, not traced instruction for instruction.


## The lattice scrolls

2026-11-26, lane `shield-anim`. Listed with `scripts/vita-usse.py disasm
<material> 1`, literal records dumped from the program header. Confidence **85**
(static; the same constants as the PS4 recompile of the same material, no live
frame). Three programs animate a shield:

- **`2048_shipshield_old`** (the HD-content shells, `hdships/<team>/shipshield`)
  and **`hd_shipshield_old`** (the cockpit sphere): the unlit fragment programs
  of the two are the same instruction for instruction. With `t` the `time`
  uniform and the literal records at words `10 = 0.8`, `11 = 0.2`, `12 = 3.0`
  (secondary program: `sa22 = sa22 * t` and `sa10 = sa21 * t`):
  `diffuse(u + 3 t, v)`; `noise = lightning(u2, 0.8 (v2 + 0.2 t)).a`;
  `lightning(uv2 + t + noise)`, **squared** (`pa2 = pa0 * pa0`) before
  `Lightning_RGB_Multiplier`. That is the PS4's law (`B = 0.8`, `C = 0.16`).
  The alpha tap stays at `(u, v)`; the vertex program sends the first pair of
  `Uv1_Uv2` to the diffuse and the second to the lightning, as on PS4.
- **`2048_magnetic_shield`** (2048's own craft, the player's default): the
  **vertex** program adds `time * Speed_multiplier` to `Uv1.u`
  (`sa32 = sa34 * sa35`, `add pa4.x, pa4.x, sa32.x`); the multiplier is the
  material's (`0x68292521`, `1.0`). Its pixel program has no clock.

The lit variants carry the same three literals at other word indices (the
compiler's order is not stable, as on PS4), so
`oag_rcs::gxp::lightning_rates` reads the unlit programs' records at the indices
disassembled above, not by data flow.

**Wired**: `psp2::shield_lightning` admits both lightning materials on the Vita
(previously PS4 only) and `hd_shipshield_old`, whose alpha now comes from the
diffuse's alpha as the shell's does; `shade.wesl` squares the lightning tap on
the gamma-2 scene only. The magnetic shell rides the plain scroll track
(`AnimTrack::Scroll`). Ground truth: `omega_2048_shield_colour_ground_truth`
and `game::shield_shell_scroll_ground_truth`. **Open:** the cockpit sphere's
blend stays the chosen alpha-over (its material has no HD equivalent in the
lineage blend table), the Vita lit variants and `constantAmbientColour` are
unread, and no Vita3K frame of a raised shield has judged the brightness.

## Wired (lane `omega-2048-shield-wire`)

- `oag_livery::shield` builds the shell through `psp2::build` and reads `ShieldColour` with `oag_rcs::rcsmodel::psp2` (a missing fourth component reads `0.0`, unread; the cockpit floor below makes it moot). Ground truth: `crates/raceplay/tests/omega_2048_shield_colour_ground_truth.rs` (every one of 13 teams' colours and a built shell).
- Target alpha is `max(shell.a, cockpit.a)` (`ShieldPalette::Omega`, `Palette::target_alpha_from_cockpit`, `Origin::Measured`); the cockpit is `Data\Weapons\vr_shield_cockpit` with alpha 1, so Omega's HD-content shells always target alpha 1.
- The colour rate is `Palette::colour_rate`; `0.05` (`CONTENT_2048_COLOUR_RATE`) exists but no palette carries it: every mode this project launches on Omega and 2048 is a named one (`< 0x17`), so the `0.15` branch is the one that runs. A mode `>= 0x17` (2048's own campaign events) would need the mode id threaded to the palette and `Data\Weapons2048\vr_shield_cockpit` as its cockpit.
- Render-only: `ShipShield` is not simulation state; no hash or lone-craft gate moves.

## The `2048_Magnetic_Shield` program (read 2026-11-03, PS4 pixel shader, confidence 70)

`ShieldColour` is a 12-byte (three-component) constant in the pixel program's cbuffer: only `.rgb` is loaded. Output RGB is `ShieldColour.rgb * Texture1 * attr0.w * attr2.x`, lerped toward fog; output alpha is `tex.r * attr2.x`, where `attr2.x` is the vertex stream's `VertexColour1.w`. So `ShieldColour.a` never reaches a pixel through this program, and the alpha `0` authored on these shells does not by itself hide them. Drawn on 2048 (HDR cyan `(0, 4.59, 4.59)`, floored to target alpha 1 by the cockpit rule): a cyan lattice around the hull. The Vita's lightning programs were read 2026-11-26 ("The lattice scrolls"); the magnetic pixel program's alpha-and-colour reading above is the PS4's.
