# ADR-0089: A mod may ship turned off

## Status

Proposed (2026-11-03). Supersedes the "installed later is on" clause of
[ADR-0078](0078-a-mod-is-turned-off-by-id-live-and-the-built-in-never.md) and the "enabled
unless `[mods] disabled` names it" clause of
[ADR-0083](0083-official-mods-are-a-public-folder-and-the-fingerprint-counts-race-changers.md).

## Context

ADR-0078 stores the mods the player turned off, so every mod found is on until named. A mod
whose effect a player should opt into (a weapon that changes every race it is on for) needs
to ship off and be turned on from the MODS page.

## Decision

1. The manifest has `default-enabled` (boolean, default `true`). `false` ships the mod off.
2. `settings::Mods` keeps `disabled` and gains `enabled` (the mods turned on against a
   default-off manifest). `mods::switch` writes the one set that matters for the mod's
   default, and the load reads `disabled` plus every default-off mod not in `enabled`.
3. A switch that equals the manifest default is stored in neither set, so a mod follows a
   later change of its own default.
4. Everything else in ADR-0078 stands: the built-in is never turned off, a switch applies
   live, and an off mod adds nothing.

## Consequences

- A mod with `default-enabled = false` is off in a fresh profile and on the MODS page as OFF.
- The state of one mod is read from two sets, which the resolver hides; a hand-edited file
  naming a mod in both sets reads as off.
- A mod's own default changing later moves every player who never touched the switch.
