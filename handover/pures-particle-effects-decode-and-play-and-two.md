# Pure's particle effects decode and play, and two of five names do not resolve

2026-08-12. `.pob` needed no change for Pure - `WO_SHIP_COLL_SPARK_DAMAGE`, `WO_ROCKET_EXPLO` and `WO_ROCKET_FLARE` all decode and play, which retires the "unknown, both" row on `docs/formats/pure-status.md`. **The content differs even where the name is shared**: `WO_ROCKET_EXPLO` is 7 emitters on Pulse and 5 on Pure. Open: `WO_ROCKET_EXPLO_TRACK` is on Pulse and not on Pure under that name - whether Pure spells it differently or ships no such effect is unread. `WO_SHIP_ENGINEFLARE` resolves on **neither** disc and never has, so it is not a Pure finding. And no Pure *trigger* has been read for any of them: these fire from Pulse's recovered triggers, which is fine for a race and is not a claim about what Pure does.

## Open

- `WO_ROCKET_EXPLO_TRACK` does not resolve on Pure under that name - whether it's spelled differently or absent is unread
- No Pure trigger has been read for any of the three effects - they currently fire off Pulse's recovered triggers
- `WO_SHIP_ENGINEFLARE` resolves on neither disc (not a Pure-specific finding)

## Next Steps

- No next step named in the original record - read the prose above and decide one.
