---
status: ready-to-wire
titles: [all]
updated: 2026-11-28
summary: "Override forms beyond a loose name; records and ghosts of a modded race await the maintainer's call"
size: S (45)
difficulty: medium (35)
---

# Asset layers: what is left after the fallback and the override landed

2026-10-11: both layers exist (`oag_assets::layers`, `oag_source::layers`, ADR-0061) and
Pilot Assist's indicator on Pulse and Pure is their first consumer. Decision, mechanism,
census and wiring: [`docs/architecture/asset-layers.md`](../../docs/architecture/asset-layers.md).

## Open

- **Records and ghosts of a modded race (maintainer's call).** Replay and ghost headers list the
  active mods (`docs/formats/replay-file.md`, "Active mods"); `records.toml` does not, and a ghost
  set with a data mod on replaces the vanilla one. Options: (a) tag only, add the list to the
  record row; (b) keep apart per race-changing mod set; (c) refuse. Recommended: (a) for data
  mods, which cannot change what a disc race reads, and (b) for simulation modules, which
  `docs/architecture/wasm-extensions/README.md` 4.4 already prescribes (a separate modded table)
  and nothing builds: their results file into the normal rows today (read in the code, not run:
  the finish and exit captures gate on demo, seed, pose and `--give` alone). Whether a mod's pilots change
  an opponent field enough to matter for a record was not checked.
- **The mods refusal is English only** on the REPLAYS row and in the viewer's notice.
- **Override forms beyond a loose name.** No hash-addressed override (`read_hash` skips both
  layers, so an unmined entry cannot be replaced) and no whole-archive mount. A PS2 model's
  texture set, found by directory position, does not follow a model override.
  By reading, not run: Pulse PSP's `stats.xml` readers open `FEData.wad` themselves
  (`oag_raceplay` `load::track_stats::read`, the campaign's AI skill scale in
  `app/session/campaign.rs`), so a `stats.xml` override is not seen there.
- **Today's one-off stand-ins** load by their own paths, not through `Archives`: the
  PromptFont glyphs (`assets/ui/prompts`) and the cursors (`assets/cursors`). Decide whether
  they move into the fallback layer; the layer would then report them.
- **The optional second step**: when the source title's own disc is also present (2048 when
  Pulse borrows a 2048 element), read the real asset from it before ours.
- **Omega's indicator** applies and is not wired (see `handover/gameplay/pilot-assist.md`).
- **A sample override to ship in the docs** (a directory layout a player can copy) is only
  described, not provided; the disc-backed test builds one in a temp directory.

## Next Steps

1. The records and ghosts decision above, then build it: tag only 1 hour, a separate table with a
   RECORDS page switch half a day.
2. Move the PromptFont glyphs and cursors into the layer if the maintainer wants one report
   line for every stand-in. 1 hour.
