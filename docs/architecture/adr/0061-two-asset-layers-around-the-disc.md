# ADR-0061: Two asset layers around the disc: a player override and a project fallback

## Status

Accepted. Extends [ADR-0021](0021-region-independent-dlc.md), which set how DLC
packs layer behind a source. The mechanism, the census and the per-title wiring
are on [`asset-layers.md`](../asset-layers.md).

## Context

[`oag_assets::Archives`](../../../crates/assets/src/source.rs) looks a name up
across a title's archives in a fixed order, first hit wins: update archives, the
bulk archive, the companion archive, extra archives, then DLC packs, last "so the
disc always wins a collision". Two things want a place in that order that no
archive can fill:

1. **A backported feature has no art on the older discs.** Pilot Assist is
   backported to every title; its HUD indicator exists on HD and later discs
   only. Pulse and Pure need something to draw, and "never invent what the
   assets already author" says that something may substitute for a *missing*
   asset and must never override data the disc has.
2. **A player may want to replace a disc entry**, which is the whole of modding.

## Decision

`Archives` gains two layers of loose entries (a name maps to a file or to bytes;
no archive format is involved):

- **Fallback, after the DLC packs.** This project's own art and layouts, carried
  in the binary from `assets/fallback/`. It answers only a name no archive holds,
  so it cannot shadow a disc or DLC entry **by construction**, and a test proves
  it on real Pulse and Pure discs.
- **Override, before the update archives.** Files the player supplies under
  `overrides/<title>/`, replacing the disc's entry of the same name. It is user
  content: never shipped, never committed, empty by default.

Entries are named by their path, matched case-insensitively with either
separator and without a leading one, so a WAD's `Data\XML\Foo.xml` and a PSARC's
`/data/xml/foo.xml` reach one file. A loose entry has no name hash and no
directory position, so the hash reads and the neighbour reads skip both layers,
the way a PSARC already refuses them.

**Every answer a layer gives is reported**, in the loader report, one line per
name: an override says it replaced the disc's own, a fallback says it is a
substitute and **chosen, not measured**. A disc read is the ordinary case and is
not reported. With neither layer mounted every lookup is the one it always was.

**Fallback art is ours, simple, and labelled.** It is geometry drawn by a script
(`scripts/gen-fallback-assist-art.py`), stored in a plain-text alpha-mask format
(`OAGMASK`, `oag_texture::mask`) so its originality is visible in a diff, and
nothing extracted from a disc may be placed in `assets/fallback/`
(`just audit-leakage` covers the tree). Its layout is a fragment at **the entry
name the source title uses** (`Data\XML\HUD_assist_indicator.xml`), so on HD, where
the disc answers the same name, the ordering is exercised on real data.

**The first consumer is Pilot Assist's HUD indicator on Pulse and Pure.** The
census found no indicator on any pre-HD disc. `oag_title::hud::AssistIndicator`
moves out of `RuntimeHud` into `HudArt`, with a `fragment` field naming where a
layout that does not embed the widgets gets them: giving Pulse a `RuntimeHud` to
carry one indicator would have switched on HD's shield and arc rules, which no
Pulse measurement backs.

## Consequences

- A player can mod any entry the loader reads by name, and the report shows it.
  A modded table changes the simulation. The replay file's initial and interval
  state hashes already make that a reported desync rather than silent divergence
  ([ADR-0055](0055-replays-are-inputs-and-a-ghost-is-poses.md)); an explicit
  "mods were active" flag in the header is open.
- Mounting happens once, in `oag_source::title::open_source`, so every caller
  that opens a source as a title gets both layers. The one-off stand-ins that load
  by their own paths (PromptFont glyphs, cursors) stay where they are.
- A second consumer of the fallback is a new `HudArt`/`Title` field and a file
  under `assets/fallback/`, not a new mechanism.
