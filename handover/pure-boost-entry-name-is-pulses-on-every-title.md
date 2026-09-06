# `boost_entry_name` composes Pulse's spelling on every title, and Pure has no plume to compose

2026-09-06. Split out of `handover/wipeout-pure-races-on-pulses-physics-and-what.md` because it is `crates/game` work and that thread's recovery half was `crates/formats`/`crates/pure`.

**The measurement is done.** `docs/ghidra/functions/psp-pure-usa/ship-models.md` establishes at confidence 93 that **Wipeout Pure ships no boost-plume asset at all** - four probes across three independent sources, each with its Pulse positive control beside it:

- No path template. Pulse composes its plume from `%s\%sboost.vex` (`0x08a84ccc`, `psp-pulse-usa`). Pure's five per-craft templates sit contiguously at `0x08a7a5d4`..`0x08a7a61c` and none is a plume; the whole 3.6 MiB executable holds two strings matching `boost` case-insensitively and neither is a path.
- No archive entry. `<location>\shipboost.vex` resolves against none of the eleven `<PI_Team>` locations the disc declares, on either pressing, while the same probe resolves on Pulse.
- No anchor. Pulse's hulls carry `boost_flare` and `boost_flare1`; no Pure hull carries a node naming `boost`.

Pinned by `crates/pure/tests/ship_models_ground_truth.rs::pure_ships_no_boost_plume_asset_and_pulse_does`, which asserts the Pulse half in the same run so the negative is a measurement rather than a name spelled wrong.

## What the engine does today

`oag_game::race::assets::boost_entry_name` (`crates/game/src/race/assets.rs:483`) returns `ships::entry_name_in(paths.dir, team, oag_pulse::race::ships::BOOST)` outside Zone mode - **`oag_pulse`'s constant, unconditionally, on every mounted title**. So a Pure race asks the archive for `Data\Ships\<Team>\shipboost.vex`, gets nothing, and `Loaded::boost_model`'s `None` path reports a missing boost model on every single load. That report was *correct* while the name was believed unrecovered. It is now noise: there is no file to look for, and the loader saying otherwise on every Pure race is the kind of standing false alarm that trains a reader to skim the report.

`ship_entry_name` and `shield` already take the title axis properly (`oag_title::race::ShipPaths`, and `assets.rs`'s own comment on the two shield names) - so the shape to copy is beside this function, not new.

## Open

- **Whether the boost is visually inert on Pure is not established, and this thread must not decide it.** The asset is absent; the effect need not be. Pure carries `Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip` and `Engine_noise.mip` exactly as Pulse does, plus a Pure-only `vr_engine_noise.mip`, so a boost that brightens or widens the existing `engine_flare` billboard in code would leave nothing for any of the probes above to find. **Drawing nothing extra is the honest state**; drawing an invented intensification is not, and the project rule against a plausible-looking stand-in applies directly here.
- Whether HD and 2048 want the same axis. HD's plume is a subtree of `engineflare.vex` (`docs/rendering/trail-ribbon.md`, "HD's flare is a model, not a sprite - and the plume is inside it"), which is a third shape again - two titles with no `shipboost.vex` and two different reasons.

## Next Steps

1. Give the boost model the same title axis the hull has: a `boost` field on `oag_title::race::ShipPaths` (or the nearest existing seat) that Pulse fills with `"shipboost"` and Pure fills with `None`, and make `boost_entry_name` return `Option<String>`. Do **not** move `oag_pulse::race::ships::BOOST` - it is Pulse's own measured constant and stays there.
2. Stop the load report naming a Pure boost model as missing. The distinction to keep visible in the report is *"this title ships none"* against *"this title ships one and it did not resolve"* - the second is still a real fault worth shouting about.
3. Assert it: a Pure race's load report should contain no missing-boost line, and a Pulse race's should still find its plume. `crates/game/tests/boost_plume_ground_truth.rs` already exercises the Pulse half.
