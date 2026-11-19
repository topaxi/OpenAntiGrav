# Omega's Zone HUD gate at the start

**Binary:** `ps4-omega-eu` `eboot.bin`. Read statically, 2026-11-19, lane `zone-hud-gate`. This is the
Omega side of [HD's page](../ps3-hdfury-eu/hud-start-gate.md), which carries the live evidence.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x012b3400` | `SPZoneRaceManager_Update`: slot 3 of vtable `0x019148a8`, which `SPZoneRaceManager_Construct` (`0x012b4960`) stores | 80 |

The function switches on the phase:

- **Cases 0 and 1** write `*(DAT_019ffc90 + 0x189) = 0`.
- **Case 2** writes 1.

`+0x189` is three bytes before `+0x18c`, the player index the same function compares when it builds the
`HUD->`/`HUD1->` names. On HD, `+0x115` sits three bytes before `+0x118` in the same way. So this is HD's
whole-HUD draw gate at 8-byte-pointer offsets: a Zone race draws no HUD until the race is released.

Static only. Two sites agree, and HD's law was measured live.
