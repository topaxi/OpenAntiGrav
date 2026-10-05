# `MagstripWake` in Wipeout HD / Fury (PS3 `EBOOT.elf`)

Functions in `EBOOT.elf` (Wipeout HD Fury, `BCES-00664`, EU), PowerPC64, image
base `0`. The full object description, evidence and caveats are on
[`ps4-omega-eu/ships-effects.md`](../ps4-omega-eu/ships-effects.md) (section
"2026-10-05"); this page is the HD half, written 2026-10-05 by the
magfloor-omega-re lane, static reading only (no RPCS3 run). Addresses are
the code addresses Ghidra shows as `.opd` entries (`_opd_FUN_0010a0c0` is the code
at `0x0010a0c0`).

**The question the brief asked: does HD have the class? Yes (confidence 85).**
`MagstripWake.cpp` (`0x00782fa0`), `"MagstripWake"` (`0x00782f90`),
`"arc_anchor_point"` (`0x007834a0`, `0x00782fb8`),
`"**** WARNING **** : Ship has no arc_anchor_point locator"` (`0x00782fd0`),
`MagStripArc_vp`/`MagStripArc_fp` (`0x007a0d18`, `0x007a0d28`),
`Data/Tex/HD_electric_arc_8x8.gtf` (`0x007a0d38`), `Data/Tex/HD_ElectricArc_Contact.gtf`
(`0x007a0d70`) and `"Send due m_overMagStrip change - now %i\n"` (`0x007a9320`) are
all present. This is the class Omega and 2048 carry forward, not a newer one.

## Constructors - `0x0010a0c0` and `0x00109e40`

`MagstripWake_Construct` is `0x0010a0c0`; `MagstripWake_ConstructAlt` is `0x00109e40`.
They are byte-for-byte the same body (a complete and a base-object constructor
of the same class, a compiler pair), and both: call a base constructor, store the vtable
(`PTR_PTR_008a98d0`, the table at `0x00863868`), store `PTR_s_MagstripWake_cpp_008a98d8`
at `param_1[0xc]`, resolve `"arc_anchor_point"` against the ship (`FUN_006762e8` on the
ship's locator set, then a walk up the parent chain), allocate two `0x200`-byte objects
(the two ribbons), a `0x5e0` arc pool, OR `6` into the flags word and increment the
live-instance counter (a global block at `+0xb80`). **85** for `0x0010a0c0`: the same tag
string, the same `arc_anchor_point` lookup, the same `| 6` and counter increment as the
PS4 and Vita constructors. **75** for `0x00109e40` (identical body, but which one the
ship constructor calls was not checked).

## Vtable `0x00863868` (slot, function descriptor, code)

Slots 3, 5, 7, 12 and 13 hold `.opd` descriptors in the `0x0087540x` run; the others
are base class (`0x00885axx`/`0x00885cxx`). Descriptors at `0x00875408` ->
`0x00109028`, `0x00875428` -> `0x00109350`, `0x00875438` -> `0x001095e0`, `0x00875440`
-> `0x00109720`, `0x00875448` -> `0x00109858`.

- **Slot 5, `0x00109028` = `MagstripWake_EnqueueRender`, conf. 78.** Appends
  `{param_1, 0x4d000000 | (inst+0x11c & 0xfffff)}` to the list at `*(PTR_DAT_008a98c0) + 0x630`
  (count at `+0x44b0`): the same body as the PS4's `0x012e2720`. This is the
  cross-binary agreement the PS4 name rests on, and `renderer.md` already
  attributes this site to `MagstripWake.cpp` through the TOC slot `008a98c0`.
- **Slot 3, `0x00109350`** is *not* the PS4 update. It draws the two ribbons
  (`FUN_002aa1c8` on `inst+0x50` and `inst+0x54`) and calls `FUN_002bbd60` with the
  speed read at `ship_body+0x4c4` (the PS4's `+0x4b8`) and `ship+0x7820`/`+0x7830`
  (the ribbons' lateral frame) - which is the tail of the PS4's draw `0x012e2770`.
  So HD's slot order is not the PS4's and **slot index is not evidence across the
  two builds**; only slot 5 is matched. The arc update and arc quad build are
  presumably `0x00109858` (slot 7) and `0x001095e0`/`0x00109720` (slots 13, 12);
  **none of those were read**, and they stay unnamed (below 50).
- `0x001092d0`/`0x00109198`/`0x00109568` etc. in the descriptor run are other
  vtable-adjacent code and were not read.

## What HD does with the arc textures and the sound

- **Textures** `Data/Tex/HD_electric_arc_8x8.gtf` and `HD_ElectricArc_Contact.gtf`
  ship in `DATA02.PSARC` (`/data/tex/hd_electric_arc_8x8.gtf`, 349,696 bytes;
  `/data/tex/hd_electricarc_contact.gtf`, 5,632 bytes), plus siblings
  `hd_electric_arc_8x8_nonanchored.gtf` (349,696), `psys/tex/hd_electric_arc_8x8.gtf`
  and `hd_electricity_1x4.gtf` (699,264). **No literal in the EBOOT names the
  `_nonanchored` or `electricity_1x4` files** (strings search), so they are unreferenced
  by name from the executable (they may be reached by a built path; not checked).
- **No `WO_MAGSTRIP_*` string exists in the HD EBOOT**, and no `MagStrip_Player`/`_NPC`.
  HD ships a different particle file instead: `/data/psys/wo_magstrip_lightning.pob`
  (3,360 bytes, `DATA02`) which **no string in the EBOOT references**. It, and its textures
  `psys/tex/pulse_mag_sprite1_orange_rings.gtf` and `weapons/textures/mag_lightning2_add_glow.gtf`,
  are found-but-unwired as far as static strings go (the name could be built by a format
  string; not searched).
- **Sound:** `~magstrip01` is a cue in `shiphd.bnk` (`DATA01`, `/data/sound/shiphd.bnk`, at
  file offset `0x19790`, beside `~jet..` cues; a `### Magstrip` section label follows at
  `0x198a8`). Its two string literals in the EBOOT (`0x00781080`, `0x00781dd0`) have no direct code
  references (they are TOC-addressed); the start site was **not located**.
- **Rumble:** `enter_mag_rumble.xml`, `travel_mag_rumble.xml`, `exit_mag_rumble.xml`
  (`/data/xml/rumble/`, 356, 548, 356 bytes) ship for the magstrip pad; the same three
  ship in the Omega `data00.psarc`. Who plays them was not searched.
