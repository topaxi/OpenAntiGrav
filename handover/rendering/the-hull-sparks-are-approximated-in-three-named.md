# The hull sparks are approximated in three named ways

`ShipCollisionFx_Trigger` (`0x089246b4`) is found, renamed and documented, and `sparks.rs` implements the blend-split pipeline pair, the streak geometry and the locator anchoring. Still approximated, each with a doc note: colour endpoints instead of the 256-entry tables (ADR-0006; runtime disc-load is the recorded follow-up), a cone aimed along the contact normal instead of the authored emitter-node frame, and ~~the streak end-cap stretch (untraced resource field)~~ - **that one is recovered**: `ParticleSystem_InitParticleFields` (`0x088f79b4`) writes a literal `1.0f` and there is no resource field behind it (confidence 88). **A side-by-side `just play` wall hit against the original is the open visual check.**

## Open

- Colour endpoints stand in for the 256-entry tables (ADR-0006)
- The cone is aimed along the contact normal instead of the authored emitter-node frame

## Next Steps

- Implement runtime disc-load of the 256-entry colour tables (the recorded ADR-0006 follow-up)
- Run a side-by-side `just play` wall hit against the original as the open visual check
