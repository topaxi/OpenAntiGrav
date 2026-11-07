# 2048's `WeaponPads_TestCraft`: the Eliminator family stamps nothing

2026-11-07 (`eliminator-pads`). The 2048 counterpart of Pulse's `WeaponPads_TestCraft`
([`pads.md`](../psp-pulse-usa/pads.md)), read to answer one question: does the mode
that Pulse stamps `elimination_refresh_time` for do the same here? It does not.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x8121f860` | `WeaponPads_TestCraft` | 85 |
| `0x818a63ac` | `g_weapon_pad_refresh_time` (data, one float per speed class) | 90 |
| `0x818a63c0` | `g_weapon_pad_elimination_refresh_time` (data, one float per speed class) | 90 |

Found from the `<WeaponPad>` parse in `Handling_ReadGlobalSettings` (`0x811ace10`), which
stores `refresh_time` at `0x818a63ac + 4 * class` and `elimination_refresh_time` at
`0x818a63c0 + 4 * class` (class index `DAT_818a63d4`); the only code that reads either array
besides the parser is `0x8121f860`, which is also the function that plays `ActivateWPad`,
`WPickup_1`/`WPickup_2` and `WEAPONPICKUP`. Confidence 85, not 90: the shape is Pulse's
function line for line (same swept test, per-racer distance cache at `+0x184`, a `25.0`
teleport reject), but the decompile is a Thumb-2 NEON body with vector intrinsics and was
read, not run.

## What it does on a hit

```c
// after the swept test (FUN_813dbc14 is Pad_ContainsPoint) and the slot checks
if (GameMode_IsEliminatorFamily(&DAT_8153fc40) == 0) {       // 0x8100193c
    if (FUN_810006fc(&DAT_8153fc40, 2) == 0) {               // mode id 0x10..0x16, or mode flag 4
        if (hit) {
            if (DAT_8153fd24 == 8) pad[0x160] = g_weapon_pad_elimination_refresh_time[class];
            else                   pad[0x160] = g_weapon_pad_refresh_time[class];
            goto grant;
        }
    } else {
        // a 250 ms world-level debounce (world+0xde0 / +0xde4), not the pad's own timer
    }
} else {
    // Eliminator family: no stamp; grant straight away
    if (hit) goto grant;
}
```

`GameMode_IsEliminatorFamily` (`0x8100193c`, [`ai-controls.md`](ai-controls.md)) answers true
for mode `8` or `0x14`, or when the current mode object carries flag `1` or `4`. **Mode `8` is
in the family, so the branch that reads `elimination_refresh_time` (`DAT_8153fd24 == 8`) can
never run**: 2048 authors the attribute, parses it, and does not use it. In Eliminator a pad is
never stamped, so `WeaponPad`'s timer stays zero and the pad never greys.

Where Pulse's single test is `g_game_mode == 8` (`pads.md`), 2048 asks for the family first and
the family contains 8. Seen only in the decompile, not run: 2048 has no emulator route here
(Vita3K is not installed on this machine), so this is **checked, differs** from Pulse and
**ported** from the HD run below by lineage only.

## Against HD and Omega

HD's pad does not disarm in Eliminator either, measured on RPCS3: no stamp in 200 s of AI
crossings, and none with the player pinned on a pad with an empty slot while the pad's own
per-craft distance cache read `0.0` (inside); the same instrument stamps `0.75` in a Single Race.
Method and numbers: [`docs/gameplay/pickups.md`](../../../gameplay/pickups.md#eliminator-and-the-pads-per-title).
Omega's function has the same branch structure: [`../ps4-omega-eu/weapon-pads.md`](../ps4-omega-eu/weapon-pads.md).
