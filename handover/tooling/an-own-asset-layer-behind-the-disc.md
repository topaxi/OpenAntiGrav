# Two asset layers around the disc: a fallback for backported features, an override for mods

2026-10-10, maintainer decision: **not queued yet; start after Sunday
2026-10-11 18:00** (the usage reset). Propose it to the maintainer as a drive lane
then.

## The idea

`oag_assets::source::Archives` (`crates/assets/src/source.rs`, around line 202)
already searches a title's archives in a fixed order, first hit wins: `patch`,
`data`, `fe`, `extra`, then the DLC `packs`, last "so the disc always wins a
collision". Add **two** layers (maintainer, 2026-10-10):

- **Fallback, after `packs`**: files this project ships itself, so a lookup the
  disc and its DLC cannot answer falls through to our own asset automatically.
- **Override, before `patch`**: files the *player* supplies, searched first so
  they replace the disc's own entry by name. The main use case is modding.
  User content, never shipped or committed: it lives in a player directory
  beside `images/` (the way `data/dlc` does), not in the repository.

The fallback fits the "never invent what the assets already author" rule
(CLAUDE.md): a stand-in may substitute for a missing asset and never override
data the disc has, and a last-place layer cannot override anything by
construction. The override layer is the player's own explicit choice, not this
project authoring anything, so it does not conflict with the rule; it must be
empty by default and every hit reported.

## First use case: Pilot Assist's HUD indicators on Pulse and Pure

Pilot Assist (the `pilot-assist` lane, 2026-10-10) is backported to every title,
on the 2048/Omega three-level model. Its HUD indicators exist on HD and later
discs only, so Pulse and Pure have no art for them. The maintainer believes no
pre-HD disc authors such an indicator; check that first (a Pulse/Pure HUD
sprite census, `oag-hud`'s `sprite` sheets), and if it holds, Pulse and Pure
draw their indicators from the own-asset layer.

## Open

- Whether any Pulse/Pure HUD art could serve as the indicator (unchecked).
- Entry naming: our file sits at the entry name the feature asks for (for a
  backported element, the name the source title uses).
- The loader report must say when the layer answered ("using this project's
  substitute for X"), so a missing asset never goes silent.
- `just audit-leakage` must keep the fallback layer free of anything derived from a disc.
- Override layer: where the folder lives (per title, beside `images/`), how an
  entry is named (WAD/PSARC path, or hash for hashed WADs), whether a mod can
  ship a whole archive or loose files, and keeping overrides out of anything
  hashed for determinism (a modded table changes the simulation: replays and
  ghosts must record that mods were active, or refuse them).
- The loader report names every override hit too.
- Today's one-off stand-ins load by their own paths, not through `Archives`:
  the PromptFont glyphs (`assets/ui/prompts`) and the cursors
  (`assets/cursors`). Decide whether they move into the layer.
- Optional second step: if the source title's own disc is also present (2048
  when Pulse borrows a 2048 element), read the real asset from it before ours.

## Next Steps

1. After 2026-10-11 18:00, propose the lane (sonnet, about 3-4 h): both
   layers, the loader report line, tests that the disc beats the fallback and
   an override beats the disc, then Pilot Assist's Pulse/Pure indicators as the
   fallback's first consumer and one sample override as the mod path's proof.
2. Delete this file and its `HANDOVER.md` index line once it lands.
