# The hull sparks are approximated in two remaining ways

`ShipCollisionFx_Trigger` (`0x089246b4`) is found, renamed and documented, and `sparks.rs`/`psys.rs` implement the blend-split pipeline pair, the streak geometry and the locator anchoring. Of the three approximations this thread originally named, two are now closed and this file's own title is stale by one: ~~the streak end-cap stretch (untraced resource field)~~ is recovered (`ParticleSystem_InitParticleFields`, `0x088f79b4`, writes a literal `1.0f`, confidence 88), and **the colour-endpoints approximation is also closed** - it landed on 2026-08-12 (`7fce3d90`, "render: implement dynamic .pob particle system parsing") along with the rest of the generic `.pob` parser, but this thread and `docs/formats/pob.md` both kept describing the pre-2026-08-12 state until 2026-09-10. `crates/vex/src/pob.rs` reads all 256 entries of every emitter's own colour table off the disc (`+0xc4`), `oag_render::psys::EmitterSpec::from_record` normalises all 256 by `ColourScale`, and `crates/render/tests/psys_ground_truth.rs` now pins that against the disc - `the_collision_spark_palette_is_read_from_disc_not_two_endpoints` plus two neighbours showing what the retired two-endpoint version got wrong: the smoke ramp is not linear (three quarters of its `RandomEntry` population spawn some shade of grey ember, not the uniform mix two endpoints imply) and the bright spark's hue is not monotonic (it brightens toward yellow partway through its life before dimming, which no two-point gradient can reproduce). See `docs/formats/pob.md`'s "The collision-spark file is a four-emitter tree" section for the write-up.

Still open: the cone is aimed along the contact normal instead of the authored emitter-node frame, and **a side-by-side `just play` wall hit against the original is the open visual check** - this machine has no working GUI window presentation, so that check has still not been run by anyone.

## Open

- The cone is aimed along the contact normal instead of the authored emitter-node frame

## Next Steps

- Run a side-by-side `just play` wall hit against the original as the open visual check
