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

## Open

- How the `2048_Magnetic_Shield` program uses `ShieldColour` (alpha `0`, RGB `4.59`): unread. Recipe:
  `oag_rcs::gxp` over the material's `.gxp` programs for the constant named `ShieldColour` and its
  consumers; the 2048 shell is invisible if the program multiplies by `.a`.
- The two draw routines were read for constants, not traced instruction for instruction.
