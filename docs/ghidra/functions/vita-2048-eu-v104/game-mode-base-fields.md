# `GameModeBase`'s own field table, and the craft-restriction fields it names

Function in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. **The name here is applied**, from [names.tsv](names.tsv).
Found while chasing the user's own play observation that 2048 restricts craft
selection on some events - `WOShipCreatorParams`
(`M_PPlayerShipCreatorParams`/`M_PGridShipCreatorParams`) was the field
`docs/formats/2048-campaign.md` expected to carry this and is authored on zero
of `SP.xml`'s 141 events; this function is where the real mechanism's own
field names and byte offsets come from.

## `GameModeBase_RegisterFields` - `0x812b0e2a`

**Confidence: 90.** Every field name and its own offset/size/count triple is
literal in the decompile - a straight sequence of calls to a reflection
registrar (`FUN_812dab08(&DAT_81965950, size, offset, "field_name",
type_name_or_type_id, count, ...)`) - not inferred from usage. Not renamed
higher than 90 because the registrar's own remaining parameters (the trailing
zeros/`bVar1` flags) are not decoded, and this is one constructor function
observed, not cross-checked against a second title's own `GameModeBase`.

Registers `GameModeBase`'s own size (`0x328` bytes) and every field
`docs/formats/2048-campaign.md`'s own census already names by its `SP.xml`
tag (`M_TRACKDEF` at `+0x104`, `M_PNEXTEVENT` at `+0x2cc`, and so on) plus
several this project had not read the byte offset of before this pass:

| Field | Offset | Size | Count | What it is |
| --- | --- | --- | --- | --- |
| `m_pPlayerShipModelData` | `0x3c` | `0x158` | 1 | `M_PPLAYERSHIPMODELDATA` - forces the player's own craft, see below |
| `m_pGridShipModelData` | `0x44` | `0x158` | 7 | `M_PGRIDSHIPMODELDATA` - the AI grid's own craft, one slot per opponent |
| `m_bPreventCombatShips` | `0x80` | 1 | 1 | `M_BPREVENTCOMBATSHIPS` |
| `m_bPreventAgilityShips` | `0x81` | 1 | 1 | `M_BPREVENTAGILITYSHIPS` |
| `m_bPreventSpeedShips` | `0x82` | 1 | 1 | `M_BPREVENTSPEEDSHIPS` |
| `m_bPreventProtoShips` | `0x83` | 1 | 1 | `M_BPREVENTPROTOSHIPS` |
| `m_pPlayerShipCreatorParams` | `0x40` | 8 | 1 | `M_PPLAYERSHIPCREATORPARAMS` - a pointer, never authored (see below) |
| `m_pGridShipCreatorParams` | `0x60` | 8 | 3 | `M_PGRIDSHIPCREATORPARAMS` - three pointers, never authored |

**The prevent-flags' own offset order (`0x80`..`0x83`, combat/agility/speed/
proto) is the evidence `oag_2048::campaign::craft::variant_index` cites for
mapping `WOShipModelData`'s `M_LIVERY` strings onto
`oag_2048::race::SHIP_TYPES`' own slot order** (`fighter`/`agility`/`speed`/
`prototype`) - both orderings land on the same four-way axis in the same
sequence, corroborating the mapping from a second, independent source rather
than the field census alone.

**`m_pPlayerShipCreatorParams`/`m_pGridShipCreatorParams` are pointers
(`WOShipCreatorParams*`, size 8 including whatever padding the count carries)
sitting right beside `m_pPlayerShipModelData`/`m_pGridShipModelData` in the
same struct** - two parallel mechanisms the original clearly supports, of
which `SP.xml` only ever authors one. `WOShipCreatorParams`'s own shape is
still unread; nothing in this function decodes it, only registers where a
pointer to one would go.

## `GameModeBase_IsShipTypeAllowed` - `0x812b41da`

**Confidence: 85.** `bool GameModeBase_IsShipTypeAllowed(GameModeBase *event,
char *livery)` - the enforcement site [`GameModeBase_RegisterFields`'s own
"What this does not settle" section (below, until this pass) named as
unfound. All four flags clear returns `true` unconditionally (any livery,
including one that matches none of the four strings, is allowed with no
restriction authored); otherwise `"combat"`/`"agility"`/`"speed"` each return
their own prevent-flag's negation, and a livery matching none of the four
strings at all (a guest, HD-roster team's own [`WOShipModelData_LiveryLabelId`],
`0x8105b6c2` below, resolves those to `"fe_hd_livery_normal"`/
`"fe_fury_livery_normal"`) falls through to the same unconditional `true`.
Not renamed higher than 85: the decompile is literal, but this was read, not
watched running against a real restricted event this pass.

**`"prototype"` is not a fifth, independent class.** `m_bPreventProtoShips`
set refuses it outright; otherwise the function looks up the *querying
event's own player team's* prototype `WOShipModelData` in a global registry
(`DAT_81967280`, iterated through `FUN_812d9344`/`FUN_812d9354`) and re-tests
its `M_PROTOTYPELIVERY` field (`WOShipModelData+0x25`, immediately after
`M_TEAM`+`0x5` and `M_LIVERY`+`0x15` in the same 0x10-byte-stride layout) -
looping back through the same four-way check with that string instead.
Measured against the real `SP.xml`: every one of the five teams' own
`*_Proto` instances carries exactly one of `"Agility"`/`"Combat"`/`"Speed"`
(never `"Prototype"` itself) - `AG_System_Proto`→Agility,
`Auricom_Proto`→Combat, `Feisar_Proto`→Speed, `Piranha_Proto`→Speed,
`Qirex_Proto`→Combat (`crates/tools/examples/ship_creator_scan.rs`'s own
per-field dump). `oag_2048::campaign::craft::refused_craft`/`class_allowed`
implement this exact recursion, matched case-insensitively since
`M_PROTOTYPELIVERY` is title-cased where every other caller of this same axis
spells it lower-case.

## `WOShipModelData_LiveryLabelId` - `0x8105b6c2`

**Confidence: 90.** `char *WOShipModelData_LiveryLabelId(char *livery)` -
`"speed"`→`"FE_SHIP_SPEED"`, `"combat"`→`"FE_SHIP_COMBAT"`,
`"agility"`→`"FE_SHIP_AGILITY"`, `"prototype"`→`"FE_SHIP_PROTO"`, a guest
team (checked through `FUN_8100170a`/`FUN_81001774` against the HD/Fury
unlock tables) → `"fe_hd_livery_normal"`/`"fe_fury_livery_normal"`, anything
else → `""`. Literal string comparisons and returns; already corroborated
independently by `oag_2048::race::SHIP_TYPE_LABELS`' own doc comment
(`FE_SHIP_COMBAT`→`"FIGHTER"` etc., confidence 92, read off
`english/entries.xml`). Called from [`CampaignEventCard_Draw`](campaign-event-card.md)
to draw a forced craft's own class name and from [`PostRace_ShipCategoryTip`](campaign-event-card.md)
for its own tip text.

## What this does not settle

This function only proves the fields exist, their offsets, and their sizes -
it says nothing about *who reads them at runtime*; `GameModeBase_IsShipTypeAllowed`
above answers that for the four prevent-flags specifically. The player-craft
override (`m_pPlayerShipModelData`) is confirmed working end to end through
`oag_2048::campaign::craft::forced_craft`/`race::load_event`, verified live
against the real package. **2026-09-28: superseded** - the paragraph
previously here said the restriction's own enforcement site was unfound.
`GameModeBase_IsShipTypeAllowed` is that site; see
[campaign-event-card.md](campaign-event-card.md) for the screen that calls
it (not `TeamSelection_Screen.cpp`, which the search that wrote the original
paragraph had focused on) and `docs/formats/2048-campaign.md`'s "Craft
choice" section for the player-facing finding.

## See also

- [`docs/formats/2048-campaign.md`](../../../formats/2048-campaign.md)'s own
  "Craft choice" section - the player-facing finding this evidence supports.
- [campaign-event-card.md](campaign-event-card.md) - the screen that calls
  `GameModeBase_IsShipTypeAllowed`, and what it does with the answer.
