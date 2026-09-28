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

## What this does not settle

This function only proves the fields exist, their offsets, and their sizes -
it says nothing about *who reads them at runtime*. The player-craft
override (`m_pPlayerShipModelData`) is confirmed working end to end through
`oag_2048::campaign::craft::forced_craft`/`race::load_event`, verified live
against the real package. **Which native screen reads
`m_bPrevent*Ships`, and how, is not found.** `Frontend/Screens/
TeamSelection_Screen.cpp`'s own constructor (`0x8113b4dc`) and every method on
its vtable reachable from `0x81510928` (`0x8113b668`, `0x8113b77e`,
`0x8113bec8`) were decompiled looking for a byte load at `GameModeBase+0x80`
through `+0x83` or a word load at `+0x3c`; none of the three touches those
offsets. The two screens that redirect into Team Selection
(`0x811410e6`, `0x81143efc`, both naming `"TeamSelectRedirectPlayer1"` as
their own tick target) are track/course-carousel screens transitioning *into*
Team Selection, not the enforcement site either. The actual read is somewhere
else on `TeamSelection_Screen`'s own remaining vtable slots (not all were
decompiled this pass) or in a different class entirely - left open rather
than guessed at, confidence under 50 for any specific site.

## See also

- [`docs/formats/2048-campaign.md`](../../../formats/2048-campaign.md)'s own
  "Craft choice" section - the player-facing finding this evidence supports.
