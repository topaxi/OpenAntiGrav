# The AI's six-stage plan (line-follower to a field of pilots) is complete; what remains unbuilt is the residual

Landed 2026-08-11 through 2026-08-17, all six stages (airbrakes, pilots/archetypes, craft awareness, aggression/provocation/ramming, opponent firing, TOML-authored pilots), each verified against the real disc and each isolated in the world-hash history. Design and full history are on [ai.md](../docs/gameplay/ai.md) - **do not requote from this file**. Still unbuilt, re-checked against the code 2026-08-17: reaction latency, pad greed and dodging fire (both need a fourth `Context` channel), and slipstreaming (no such force exists in `oag-physics`); adaptation between races is no longer blocked since per-opponent lap times landed. A ram shoves and nothing else - `resolve_craft_pairs` discards the contact and nothing arms `stun_timer` or the victim's provocation. Two traps from the 2026-08-12 benchmark work: (1) the index a rescue fires at is not the index the craft left at - the honest probe watches the craft cross 2x `max_half_width` off its line rather than reconstructing a subtracted offset; (2) `driver.index` and a `Spline` sample index are different spaces since `Race::ai_order` landed, identical on 9 of 12 circuits and not on `05_Track`, `14_Track` or `07_Track` - `Race::ai_sample` is the only bridge.

## Open

- Reaction latency is unbuilt
- Pad greed and dodging fire are unbuilt - both need a fourth `Context` channel
- Slipstreaming is unbuilt - no such force exists in `oag-physics`
- A ram only shoves: `resolve_craft_pairs` discards the contact, and nothing arms `stun_timer` or the victim's provocation

## Next Steps

- No next step named in the original record - read the prose above and decide one.
