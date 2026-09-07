# Pure's particle effects decode and play, and two of five names do not resolve

2026-08-12. `.pob` needed no change for Pure - `WO_SHIP_COLL_SPARK_DAMAGE`, `WO_ROCKET_EXPLO` and `WO_ROCKET_FLARE` all decode and play, which retires the "unknown, both" row on `docs/formats/pure-status.md`. **The content differs even where the name is shared**: `WO_ROCKET_EXPLO` is 7 emitters on Pulse and 5 on Pure. `WO_SHIP_ENGINEFLARE` resolves on **neither** disc and never has, so it is not a Pure finding. And no Pure *trigger* has been read for any of them: these fire from Pulse's recovered triggers, which is fine for a race and is not a claim about what Pure does.

2026-09-01: both open questions below are now closed. `WO_ROCKET_EXPLO_TRACK` ships on Pulse and not on Pure, not under a different name either - confirmed by `strings` on both Pure pressings' executables, corroborated by six missed alternate-spelling hashes against Pure's `Data.wad`. And Pure's own trigger code is now read for all three effects: `Rocket_Init`, `Rocket_SpawnCraftExplosion` and `ShipCollisionFx_Trigger`
(`docs/ghidra/functions/psp-pure-usa/rocket-and-collision-fx.md`) fire `WO_ROCKET_FLARE`, `WO_ROCKET_EXPLO` and `WO_SHIP_COLL_SPARK_DAMAGE`/`_NODAMAGE`/`WO_WEAPON_ABSORB` directly, matching Pulse's own trigger structure closely enough to carry Pulse's function names. The rocket's track-hit case turned out to be the interesting one: Pure's `Rocket_Update` doesn't fire a spelling variant of `WO_ROCKET_EXPLO_TRACK` at all - it fires `WO_TRACK_ROCK_DEBRIS`, a name `docs/formats/pob.md` had catalogued as PS2-Pulse-only until this pass found it in Pure's PSP `Data.wad` too, decoding cleanly to a 3-emitter debris burst.

## Open

- No caller has been found for any of the four functions in `rocket-and-collision-fx.md` - the same `jal`-relocation wart that broke the string lookups breaks `get_function_callers` too, so each reading is a name-and-tag match, not a call-graph-verified one. Worth another hour if higher confidence is wanted, not blocking.
- Whether Pure has an equivalent to Pulse's `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` branch (`kind==1` on Pulse's `ShipCollisionFx_Trigger`) is unread - Pure's version only showed two `kind`-like branches in this pass.

## Next Steps

- If picked up again: try resolving a caller for `Rocket_Update` or `ShipCollisionFx_Trigger` via `search_byte_patterns` on the `jal`'s raw encoded bytes rather than the operand text, in case that survives the relocation wart where the printed operand doesn't.
