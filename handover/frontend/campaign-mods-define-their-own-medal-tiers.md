---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-13
summary: "Campaign mods declare their own medal tiers and draw their own medal atlas (landed); what is left: the HD-style target rows, other languages"
difficulty: easy (15)
---

# Campaign mods define their own medal tiers

Landed (2026-11-12): `[[medal-tier]]` in `schemas/v1/campaign.tosd` and `oag_mod::campaign`, the
tier list `oag_tables::race_campaign::MedalTiers` (`Medal` is a position in it, `Medal::Gold`
and the other two its standard positions), evaluation through `Launch::tier_targets`, records by
name (`best_tier`), Cell and Grid Selection swatches, points and text, the end-of-race award
line (`OAG_MEDAL_AWARD`), `[campaign-medals] tiers` naming any tier, and `example-campaign`'s
platinum, and a mod's own medal atlas (2026-11-13, `[campaign-medals] source = { mod = ... }`: Cell Selection, on the mod's own campaigns). Format: `docs/formats/custom-campaign.md`, "Medal tiers".

## Open

- Wipeout HD's Cell Selection (a mod campaign on an HD-dialect title) lists gold, silver and bronze
  in its three target rows and draws a tier beyond them as the standard tier below it; the Pulse
  screens list the best three tiers an event awards.
- A mod's tier words resolve through its `strings/english.toml` only.
- Whether a tier above gold counts toward a grid's gold count is implemented as "at least gold
  includes every better tier", **chosen, not measured**.

## Next Steps

1. ~~Draw a mod's own medal atlas (`[icons]`), rows named by `[campaign-medals] tiers`~~ Done 2026-11-13: `a_mod_atlas_draws_the_row_its_tier_names_in_the_block_of_the_difficulty` (`oag-ui-screens`), `a_mod_with_its_own_atlas_draws_it_on_its_own_campaigns_screens` and `a_mods_atlas_is_not_drawn_on_the_discs_campaign_or_another_mods` (`oag-game`); [mods.md](../../docs/tools/mods.md), "Campaign medals". Still open there: Grid Selection's tier hexagons and Pulse's target-row swatches keep the colour swatch (the HD atlas is the same); an SVG atlas is refused by name.
2. HD-dialect target rows by tier, as Pulse's.
3. The mod's `strings/<language>.toml` for tier words.
