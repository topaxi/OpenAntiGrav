# `WeaponType`'s own enum declaration settles `M_WEAPONAVAILABLEBITS`'s bit 4, and splits Mine from Bomb

Functions and data in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04),
image base `0x81000000`. **The names here are applied**, from [names.tsv](names.tsv).

`docs/formats/2048-campaign.md`'s "The weapon set gate" section decoded eight of
`M_WEAPONAVAILABLEBITS`'s bits from `SP.xml`'s own 20 `WeaponSetDefinition`
instances alone, at confidence 95, plus one joint pair (bits 8/9, `Mine`+`Bomb`
together) at confidence 75, and left bit 4 unpinned - suggestive of `Shield` but
below this project's 70 naming threshold. That prior pass found the field's own
*registration* call in `eboot.elf` by string cross-reference
(`m_weaponAvailableBits` at `0x814dcabc`) but no consumer, and did not chase
`WeaponType`'s own declaration. This pass does.

## `WeaponType_RegisterEnum` - `0x8101291a`

**Confidence: 90.**

```
void WeaponType_RegisterEnum(void)

{
  FUN_812da48a(&DAT_815776e0,"WeaponType",WeaponType_RegisterValues,0);
  SceLibc_EDC939E1(&DAT_815776e0,0x812da561,&DAT_819e77a4);
  return;
}
```

Found from the second of two `"WeaponType"` string hits in the binary
(`search_strings`): `0x8141c8d4` (this function's own literal) and
`0x814dcad4` (the field-type string `m_weaponAvailableBits`'s own registration
cites, see below). `FUN_812da48a` is mjolnir's enum-type registrar (by shape:
name string, callback, flags) - registers the type name `"WeaponType"` and
hands it a callback, `WeaponType_RegisterValues`, that populates the enum's
own name/value table. Not read past this - `SceLibc_EDC939E1`'s role and
`DAT_819e77a4`'s callback are unidentified, below the renaming floor.

## `WeaponType_RegisterValues` - `0x8100e650`

**Confidence: 95.**

```
void WeaponType_RegisterValues(void)

{
  FUN_812da67c(&DAT_815776e0);
  FUN_812dafe6(&DAT_815776e0,"ROCKET",0);
  FUN_812dafe6(&DAT_815776e0,"MISSILE",1);
  FUN_812dafe6(&DAT_815776e0,"PLASMA",7);
  FUN_812dafe6(&DAT_815776e0,"CANNON",5);
  FUN_812dafe6(&DAT_815776e0,"QUAKE",2);
  FUN_812dafe6(&DAT_815776e0,"LEACHBEAM",10);
  FUN_812dafe6(&DAT_815776e0,"TURBO",3);
  FUN_812dafe6(&DAT_815776e0,"AUTOPILOT",6);
  FUN_812dafe6(&DAT_815776e0,"SHIELD",4);
  FUN_812dafe6(&DAT_815776e0,&DAT_8141c32c,8);
  FUN_812dafe6(&DAT_815776e0,&DAT_8141c334,9);
  return;
}
```

`FUN_812da67c` clears the enum's own value table before population (shape
matches other mjolnir reflection resets in this binary), then eleven calls to
`FUN_812dafe6(table, name, value)` register one `(name, ordinal)` pair each.
`DAT_8141c32c` and `DAT_8141c334` are C-string literals read directly off
memory (`inspect_memory_content`), not inlined by Ghidra's string search
because they are two five-byte, null-padded strings back to back rather than
one contiguous run: `"BOMB\0"` at `0x8141c32c` and `"MINE\0"` at `0x8141c334`.

This **is** `WeaponType`'s own declaration: the eleven `(name, ordinal)` pairs
are the enum's complete membership, straight from the executable, independent
of `SP.xml`'s authored data entirely.

## Why this settles the three open questions

**The eight bits `SP.xml` alone already pinned at confidence 95 (data-only)
match this table's ordinals exactly**, with zero exceptions: `ROCKET`=0,
`MISSILE`=1, `QUAKE`=2, `TURBO`=3, `CANNON`=5, `AUTOPILOT`=6, `PLASMA`=7,
`LEACHBEAM`=10 - the same eight bits, same values, cross-checked by an
entirely independent source (the enum's own declaration versus 20 authored
instances). That agreement is what licenses reading the remaining three
ordinals - `SHIELD`=4, `BOMB`=8, `MINE`=9 - as bit assignments too, at the same
confidence tier as the eight already pinned:

- **Bit 4 is `Shield` - confidence 95.** Resolves the prior pass's suggestive-
  but-unpinned reading (`Shield` was the one of 2048's eleven real pickups the
  other eight bits + mines pair didn't already account for) to a direct read
  of the enum's own name.
- **Bits 8 and 9 split - confidence 90, not 75.** `BOMB`=8, `MINE`=9
  individually, not a joint pair. Scored one tier below the other nine
  because the shipped `SP.xml` data never actually sets one without the
  other across all 20 instances - so the split is read directly off the
  enum, but nothing in the authored data has yet exercised it independently.
- **Transposition trap**: this order is the *opposite* of
  [`pickup-icon-uv-table.md`](pickup-icon-uv-table.md)'s `Hud_UpdatePickupIcon`
  held-weapon id table, where id `8` is `FE_MINES` and id `9` is `FE_BOMB`.
  That table is a different, unrelated 2048 enum (the held-weapon id, not
  `WeaponType`) - the two orderings already disagreed from position 5 onward
  (`WeaponType` bit 5 is `Cannon`, held-id 5 is `Shield`) before this pass,
  and continue to disagree at 8/9. `MINES_BITS`'s prior doc comment cited the
  held-id table's order in passing; that citation is corrected in the same
  change that adds this page.

## The field's own type link

`FUN_812b4f00` (the `m_weaponAvailableBits` field registration the prior pass
already found by string xref) declares the field with `"WeaponType"` as its
type string:

```
FUN_812dab08(&DAT_819658e8,4,0,"m_weaponAvailableBits","WeaponType",1,0,0,0);
```

The `"WeaponType"` argument is `0x814dcad4`, next to the field-name string
`m_weaponAvailableBits` itself (`0x814dcabc`) - this is the mjolnir reflection
metadata that binds `WeaponSetDefinition::M_WEAPONAVAILABLEBITS` to the
`WeaponType` enum as its element type, i.e. a bitmask over `WeaponType`'s own
ordinals. This is the link this project's methodology asked for -
`WeaponType`'s own declaration/reflection table, not a runtime bit-test
consumer - and it is enough: reflection-declared field/enum pairs are how
mjolnir enforces the bitmask shape at all, so this is the authoritative
source for "which bit is which weapon," stronger than a single consumer call
site would be.

**No runtime consumer of `m_weaponAvailableBits` (the pad spawn or pickup
gate that tests a bit against a candidate weapon) was found in this pass.**
`search_functions` for `Weapon` returns nothing in this binary - no function
carries that word yet. Not chased further: the enum declaration above already
clears this project's 70-confidence threshold on its own, so a consumer is
no longer required to enforce the bits; it would only corroborate.

## `"NoQuake"` (`32`) stays unexplained

`WeaponType`'s own declaration does not bear on `SP.xml`'s `"NoQuake"`
instance carrying the same value (`32`) as `"Cannons Only"` - that is a
authored-data anomaly, not a bit-decode question, and no consumer was found
to check it against. Left recorded as an anomaly in
`docs/formats/2048-campaign.md`, not resolved here.

## See also

- [`docs/formats/2048-campaign.md`](../../../formats/2048-campaign.md) - "The
  weapon set gate" section, updated in the same change with this page's
  findings folded in.
- [pickup-icon-uv-table.md](pickup-icon-uv-table.md) - the *other* 2048
  weapon-id ordering (`Hud_UpdatePickupIcon`'s held-weapon id), and why its
  8/9 order is the opposite of `WeaponType`'s.
