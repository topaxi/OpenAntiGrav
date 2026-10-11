# Asset layers: what is left after the fallback and the override landed

2026-10-11: both layers exist (`oag_assets::layers`, `oag_source::layers`, ADR-0061) and
Pilot Assist's indicator on Pulse and Pure is their first consumer. Decision, mechanism,
census and wiring: [`docs/architecture/asset-layers.md`](../../docs/architecture/asset-layers.md).

## Open

- **A mod must be recorded.** The replay's state hashes already turn a modded table into a
  reported desync, but the replay header does not say that mods were active, and a record or
  a ghost lap set on a modded race is stored like any other. Decide: a `mods` flag in
  `oag_replay::Header`, and whether records refuse a modded race.
- **Override forms beyond a loose name.** No hash-addressed override (`read_hash` skips both
  layers, so an unmined entry cannot be replaced) and no whole-archive mount. A PS2 model's
  texture set, found by directory position, does not follow a model override.
- **Today's one-off stand-ins** load by their own paths, not through `Archives`: the
  PromptFont glyphs (`assets/ui/prompts`) and the cursors (`assets/cursors`). Decide whether
  they move into the fallback layer; the layer would then report them.
- **The optional second step**: when the source title's own disc is also present (2048 when
  Pulse borrows a 2048 element), read the real asset from it before ours.
- **Omega's indicator** applies and is not wired (see `handover/gameplay/pilot-assist.md`).
- **A sample override to ship in the docs** (a directory layout a player can copy) is only
  described, not provided; the disc-backed test builds one in a temp directory.

## Next Steps

1. Header flag for active mods, and the records decision. 1-2 hours.
2. Move the PromptFont glyphs and cursors into the layer if the maintainer wants one report
   line for every stand-in. 1 hour.
