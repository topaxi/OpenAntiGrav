---
status: measure
titles: [pulse]
updated: 2026-10-30
summary: "Backdrop colour table wired (PICKUP_COLOURS, alpha 0x80); open: fade-in/hold/size pulse, Pure, 2048, Omega tables"
---

# Pulse's pickup backdrop colours are recovered: wire them

Evidence and confidence 92: [pickup-colour.md](../../docs/ghidra/functions/psp-pulse-usa/pickup-colour.md). The table at `0x08ab0844` holds a colour word per weapon, `0xAARRGGBB`, **translucent**.

## Open

Wired 2026-10-30 (`pulse-pickup-colour-wire`): `oag_pulse::hud::PICKUP_COLOURS` is the recovered table, alpha kept, Bomb/Mine blue, Disruptor `None`; PS2 inherits it from PSP; Pure keeps `HudBGColour`, HD its `0x5AFFFFFF`.

- Fade (decompile-only, unverified): the backdrop word is scaled, alpha included, by `clamp(T * 3, 0, 1)`, `T` = `status+0x44` growing at `dt * 1.2` and capped at 0.5, so full at about 0.28 s; the icon gates on `T >= 0.4`; after use the last colour holds while the row timer is above 0.1. See the page.
- The backdrop's size pulse (`+0x9c`/`+0xa0`) is read statically only.

## Next Steps

1. Verify the fade, hold and size pulse live, then wire them.
2. Check Pure's `PickupBackground` update for a table of its own.
3. Read 2048 and Omega's `Hud_UpdatePickup` for a per-weapon table.
