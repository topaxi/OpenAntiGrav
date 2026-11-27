# ADR-0098: A mod carries fallback entries, and a backported mode ships as an official mod

## Status

Accepted. Extends [ADR-0061](0061-two-asset-layers-around-the-disc.md) (the player's override
in front of the disc, this project's embedded fallback behind it) with a third loose layer, and
applies [ADR-0083](0083-official-mods-are-a-public-folder-and-the-fingerprint-counts-race-changers.md)
(official mods are a public folder; the fingerprint counts race changers) to it.

## Context

Detonator is backported to Wipeout Pulse, which ships no such mode
([detonator-pulse.md](../../gameplay/detonator-pulse.md)). Everything of it that is a file
(six XML tables and HUD layouts, two craft hulls of our own) lived in the embedded fallback, so
every build raced it and nobody could turn it off. The maintainer asked for it to move into
the official mods, so a player can turn it off and the engine can carry the backport without
owning its data.

Until now a mod could add rows, pages, strings, tracks, campaigns, pilots, effects, texture
packs and simulation modules, but no asset file by its path: a mod had no place in the
archives' search order.

## Decision

1. **A mod may carry fallback entries.** Every file under `fallback/<title-slug>/` of an enabled
   mod (`oag_source::layers::slug`, such as `wipeout-pulse`) is an entry named by its path under
   that folder, the layout the player's `overrides/<title>/` has. It is read in place from a
   folder, a `.zip` or a `.oagmod`. No manifest key names it; the folder is the convention.
2. **Its place in the search order** is behind the disc and its DLC packs and ahead of the
   embedded fallback: override, then update, bulk, companion and extra archives, then DLC,
   then enabled mods in search order, then the embedded fallback
   (`oag_assets::Archives::with_mods`, `layers::Kind::Mod`). Like the fallback it answers only a
   name no disc or DLC holds, so it cannot replace disc data; a player's override still wins
   over everything. The load report names each entry a mod answered.
3. **How every open reaches it.** The composition root knows which mods are on
   (`oag_game::mods::Mods`); it registers their entries once at start and again whenever a mod
   is switched (`oag_source::layers::register_mods`). Every open that mounts the player's
   layers reads them through `oag_source::layers::Layering::player`; the explicit
   `open_source_in`/`SourcePair::open_in` take a `Layering`, so a test registers or passes
   what it needs and the benchmark's `Layering::none` mounts neither override nor mods. This
   was chosen over threading the mods through the twenty-odd call sites of `open_source`,
   which mostly open a title for a front-end, sound or preview read and would each have had to
   learn about mods; the override search path was already process state read the same way.
4. **A mode that loads data of its own is enabled by its data.** It is offered only where the
   title names that data and every entry resolves in the title's archives
   (`oag_title::Title::detonator_data`, `oag_ui::menu::mode_offered`): on HD, 2048 and Omega the
   disc holds Detonator's, on Pulse an enabled mod does. The MODE row, the multiplayer and LAN
   resupply (which cap the same list), the Ship Select preview keyed on the saved mode and the
   race load all apply the one rule; a race asked for by flag, a stale setting or a campaign
   cell is refused with a `warn` naming the missing entries, never raced half-built.
5. **A backported mode ships as an official mod.** Pulse Detonator is `oag.detonator-pulse`
   (`assets/mods/official/detonator-pulse/`), on by default. The engine keeps the law and the
   title data that names the entries; it names no mod.
6. **The fingerprint counts it, unless the mod says its entries are single-player.** Fallback
   entries can hold a race's own tables, so a mod with any counts as a race changer in the LAN
   and online fingerprint (ADR-0083). A mod may declare `[mod] network-races = false`
   (absent: `true`, which keeps that rule): then no LAN or online race mounts its fallback
   entries (`oag_source::layers::Layering::for_network`, applied where a race's options are
   the network's, `oag_raceplay::Options::network`) and they do not count. The key governs the
   fallback entries alone: a mod's rows, pages, strings, effects, sounds and texture pack are not
   in the fingerprint anyway, and its tracks, campaigns and simulation module count as before.
   `oag.detonator-pulse` sets it, since Detonator is single player: two players, one with the
   mod on and one with it off, race each other. This answers the maintainer's question of
   whether a mod could declare that it does not load for an online race.

## Consequences

- With the mod off or absent, Pulse has no Detonator: no MODE value, nothing loaded, and
  `--mode detonator` on Pulse refuses. HD, 2048 and Omega are unaffected.
- **Packaged builds do not copy the official mods yet, and web and phone builds have none**
  ([mods.md](../../tools/mods.md#official-mods-assetsmodsofficial)), so Pulse Detonator is
  absent there until they do.
- A test of a Pulse Detonator race registers the official mod's layer itself, and fails under
  `OAG_REQUIRE_GAME_DATA=1` when the official-mods submodule is missing.
- There is no `overrides/`-style mod folder that wins over the disc; a mod that must replace
  disc data still has no way to, by design.
