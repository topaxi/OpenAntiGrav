---
status: ready-to-wire
titles: [pulse]
updated: 2026-10-30
summary: "Pulse's pickup backdrop colour is a 14-row table, alpha 0x80; replace the measured PICKUP_COLOURS blends with it"
---

# Pulse's pickup backdrop colours are recovered: wire them

Evidence and confidence 92: [pickup-colour.md](../../docs/ghidra/functions/psp-pulse-usa/pickup-colour.md). The table at `0x08ab0844` holds a colour word per weapon, `0xAARRGGBB`, **translucent**.

## Open

Replace `oag_pulse::hud::PICKUP_COLOURS` (`crates/pulse/src/hud.rs`, the doc comment and the three `PINK`/`GREEN`/`BLUE` constants) with the authored words, keeping the positional `[Option<u32>; 14]` contract with `oag_hud`:

| Weapons | Word |
| --- | --- |
| Rocket, Missile, Quake, Cannon, Plasma, LeachBeam, Repulser, Shuriken | `0x80BE1E69` |
| Turbo, Shield, Autopilot | `0x801CBC1C` |
| Bomb, Mine | `0x802B1CBC` |

- Alpha is `0x80`, not opaque: `oag_hud` draws the backdrop with source-over alpha so the scene shows through. `Bomb`/`Mine` stop falling back to `PICKUP_BACKDROP_COLOUR` and the `Disruptor` slot stays `None` (Pure's).
- The icon stays `0xFFFFFFFF`.
- Fade in: the backdrop and icon words are scaled, alpha included, by `clamp(t * 3, 0, 1)` over the first third of a second after the grant (`t` = `status+0x44`). Optional, record it if wired.
- The backdrop's size pulse (`+0x9c`/`+0xa0`) is read statically only.
- Rewrite the long "Four frames" doc comment on `PICKUP_COLOURS` and the "category" paragraphs of `docs/gameplay/pickups.md` as a recovered law; retarget `crates/hud/src/tests.rs` (`PICKUP_COLOURS[Weapon::Turbo]`, the opaque-green and fallback tests), keeping a test that fails if the alpha or a grouping is dropped.
- Pure keeps `HudBGColour`; HD fixed `0x5AFFFFFF`; 2048/Omega unchecked.

## Next Steps

1. `oag-wire` lane: the edit above, then `just gate-affected`.
2. Check Pure's `PickupBackground` update for a table of its own.
3. Read 2048 and Omega's `Hud_UpdatePickup` for a per-weapon table.
