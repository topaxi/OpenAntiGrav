---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-12
summary: "A campaign mod declares its own ordered medal tiers (platinum, diamond) instead of the fixed gold/silver/bronze"
difficulty: medium (40)
---

# Campaign mods define their own medal tiers

Asked for by the maintainer (2026-11-12), as the step after the `[campaign-medals]` mod element
(HD medals on Pulse's campaign grid), which keys its art by tier name so that more tiers need no
second schema change. This thread is that second step; it waits for `[campaign-medals]` to land.

## What is fixed today

A medal is the three-tier `Medal { Gold, Silver, Bronze }` (`crates/tables/src/race_campaign.rs`),
and every consumer assumes it:

- `schemas/v1/campaign.tosd`: `[types.targets]` and `[types.medals]` name `gold`, `silver`,
  `bronze` (and the per-rung `easy`/`medium`/`hard` target triples);
  `[types.requirement.medal]` allows `any`, `bronze`, `silver`, `gold`.
- Saved records (`oag_game::records::CampaignRecord`) store the medal as that enum.
- `Medal::points()`, the Cell and Grid Selection swatches and text (`IG_HUD_GOLD`/`SILVER`/`BRONZE`),
  the end-of-race medal line.

## Next Steps

1. `campaign.tosd`: an optional ordered `[[medal-tier]]` list (name, points, colour, string id), best
   first. Absent: gold, silver, bronze as today, so every disc campaign and existing mod is unchanged.
   Targets and requirements name tiers from that list; the loader refuses an unknown name by name.
2. Replace the fixed enum on the campaign path by a tier index resolved against the campaign's list;
   keep the disc titles' behaviour byte-identical (a test per title's campaign screen).
3. Records store the tier by name; an old record's `Gold`/`Silver`/`Bronze` reads as those names.
4. `[campaign-medals] tiers` (the mod element) accepts the campaign's tier names; a tier with no art
   draws its declared colour as a swatch.
5. Same change: `assets/mods/examples/.claude/skills/oag-mod-author/SKILL.md`, `docs/tools/mods.md`,
   `just check-schemas`, and `example-campaign` (or a new example) gaining one extra tier.

## Open

- Whether a tier above gold counts toward a grid's gold count (`Grid_CountMedalsAtLeast`) is a design
  choice for the mod's campaign: propose "at least gold" includes every better tier.
