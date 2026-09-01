# Pure's particle effects decode and play, and two of five names do not resolve

2026-08-12. `.pob` needed no change for Pure - `WO_SHIP_COLL_SPARK_DAMAGE`, `WO_ROCKET_EXPLO` and `WO_ROCKET_FLARE` all decode and play, which retires the "unknown, both" row on `docs/formats/pure-status.md`. **The content differs even where the name is shared**: `WO_ROCKET_EXPLO` is 7 emitters on Pulse and 5 on Pure. `WO_SHIP_ENGINEFLARE` resolves on **neither** disc and never has, so it is not a Pure finding. And no Pure *trigger* has been read for any of them: these fire from Pulse's recovered triggers, which is fine for a race and is not a claim about what Pure does.

2026-09-01: `WO_ROCKET_EXPLO_TRACK` is read - it ships on Pulse and not on Pure, and not under a different name either. `strings` on Pure's `BOOT.BIN` (both USA and EU) lists every other rocket/explosion `.pob` path as a literal, including `WO_ROCKET_EXPLO` and `WO_ROCKET_FLARE`, and never a track-hit variant; six plausible alternate spellings all miss against Pure USA's `Data.wad` directory by hash. Confidence 78 - see `docs/formats/pure-status.md`. Still open: whether Pure's rocket draws `WO_ROCKET_EXPLO` on both craft and track hits, or nothing on a track hit, needs Pure's own rocket-collision code read, not just its string table.

## Open

- No Pure trigger has been read for any of the three effects - they currently fire off Pulse's recovered triggers
- `WO_SHIP_ENGINEFLARE` resolves on neither disc (not a Pure-specific finding)
- Whether Pure's rocket fires `WO_ROCKET_EXPLO` or nothing on a track hit (string table only shows the name is absent, not what the code does)

## Next Steps

- Read Pure's rocket-collision code (the counterpart of Pulse's `Rocket_Update`/`Rocket_HitCraft_q`) to settle the track-hit question and, separately, to find where any of the three effects are actually triggered on Pure rather than borrowed from Pulse
