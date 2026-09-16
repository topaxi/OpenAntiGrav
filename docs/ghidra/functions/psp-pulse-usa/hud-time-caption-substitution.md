# The HUD's mode-code caption substitution, read for one widget pair

**Binary:** `pulse-psp-usa` `BOOT.BIN`, image base `0x08804000`.

[`docs/ui/hud.md`](../../../ui/hud.md#and-one-structural-finding) found that a
frame reads `record` where `TimeTrial_HUD.xml` authors `TotalTimeTxt` with
`idstring="IG_HUD_TOTAL"` - code substitutes a different `IG_HUD_*` key into a
widget the layout already positioned, and left the mechanism unread. This page
reads it, for that one widget pair.

## The substitution table, confidence 84

`Hud_UpdateTimeCluster_q` (`0x0881c9d0`-`0x0881cdc3`) contains

```c
uVar2 = 0x275e04;                 // IG_HUD_TOTAL   (default)
if (iVar3 < 2) {
    if (-1 < iVar3) {
        if (iVar3 < 1) {
            uVar2 = 0x275e14;     // IG_HUD_BRONZE  (tier == 0)
        } else {
            uVar2 = 0x275e24;     // IG_HUD_SILVER  (tier == 1)
        }
    }
    // tier == -1 falls through: stays IG_HUD_TOTAL
} else if (iVar3 < 3) {
    uVar2 = 0x275e34;             // IG_HUD_GOLD    (tier == 2)
} else if (iVar3 < 4) {
    uVar2 = 0x275e40;             // IG_HUD_RECORD  (tier == 3)
}
// tier >= 4, tier != 5: stays IG_HUD_TOTAL
```

where `iVar3` is `*(int *)(*(int *)(param_1 + 0x3c) + 0x34)` - an ordinal read
off a race-context pointer the function is also using for other fields (see
below). The five literals are this database's pre-relocation immediates
(`docs/ghidra/workflow.md#why-a-psp-import-silently-loses-every-relocation`);
adding the image base gives, checked one for one against `search_strings`:

| Literal | `+ 0x08804000` | String at that address |
| --- | --- | --- |
| `0x275e04` | `0x08a79e04` | `IG_HUD_TOTAL` |
| `0x275e14` | `0x08a79e14` | `IG_HUD_BRONZE` |
| `0x275e24` | `0x08a79e24` | `IG_HUD_SILVER` |
| `0x275e34` | `0x08a79e34` | `IG_HUD_GOLD` |
| `0x275e40` | `0x08a79e40` | `IG_HUD_RECORD` |

All five match exactly, and the fifth is corroborated a second way: the
`addiu a0, a0, 0x5e40` at `0x0881cd50` that builds `0x275e40` (`lui` supplies
the high half `0x27`, shared with every other literal in the table) is the
**only** reference to `IG_HUD_RECORD`'s address anywhere in the binary -
`search_instructions` over all 525,049 disassembled instructions, both as
`addiu` and `ori`, found this one hit. `IG_HUD_TOTAL`'s own address has
**zero** code references at all, which is the other half of the same finding:
nothing ever loads it as a pointer, because nothing has to - it is only ever
the layout XML's own `idstring`, read the ordinary way, until this table
overrides it.

Score **84**, the ceiling for decompilation-only evidence with consistent call
sites (there is exactly one, and it is unambiguous): every literal in the
table round-trips to a real, distinct, correctly-named `IG_HUD_*` string, the
branch structure is total and mutually exclusive, and the one omitted key
(`tier == 5`) is visibly a *different* code path rather than a gap - see
below. Not runtime-verified, so it does not go higher.

The chosen key is looked up through the language plugin's string table
(`func_0x0008f8ec(_DAT_002ad160, key_address, 0)`, the same primitive
[hud.md](../../../ui/hud.md#localisation-keys) already identifies as the
`idstring` resolver) and written into the caption widget with
`func_0x000c8f6c`, **only when `iVar3` differs from a cached copy at
`param_1 + 0x400`** - so the lookup is a one-shot on a tier transition, not a
per-frame cost.

## What the tier itself is, is not read here - confidence 50, not renamed

`iVar3` selects the string; what makes it `0`..`3` is a different question,
and the fields involved are named provisionally at best:

- `iVar5 = *(int *)(*(int *)(param_1 + 0x3c) + 0x30)`: `-1` hides both the
  numeric widget (`param_1 + 0x200`) and its caption (`param_1 + 0x204`)
  (`&= 0xfffffffb` clears a visibility bit on each); anything else shows them.
  Whether this is a lap/race timer becoming valid, a placement, or something
  else is **not established** - below the confidence floor for a name.
- `param_1 + 0x34` (the tier itself) and `param_1 + 0x5a`, which gates the
  *other* caption pair this same function updates (`param_1 + 0x1f0` /
  `+ 0x1f4`, between `IG_HUD_CURRENT` at `0x08a79de4` and, unexpectedly,
  `MSC_RACE_ENDS` at `0x08a79df4` rather than a "best" caption) - both
  unread.
- `tier == 5` skips the whole caption block (it lives inside the `else` of
  `if (iVar3 == 5)`) and instead calls a different numeric formatter
  (`func_0x000157f0` instead of `func_0x000156e4`) on the same value. The
  caption is left exactly as it was the previous tick. What tier 5 means is
  unread; it is visibly *not* "no medal", since that is tier 4 and above via
  the fallthrough already in the table.

None of this is named. **Medal targets** (the "Deferred, and known" item in
[hud.md](../../../ui/hud.md#deferred-and-known)) is what would settle the
tier's meaning - it needs the same progression data this page does not have.

## `param_1` is the same struct `Hud_BindWidgets`'s dispatcher builds

Cross-checked, not assumed: `param_1 + 0x1f0`, `+ 0x1f4` (`+ 500` in the
decompilation), `+ 0x1f8`, `+ 0x1fc`, `+ 0x200` and `+ 0x204` are all read
here and are exactly the offsets `FUN_0881fbec` - the widget-bind function
[hud.md](../../../ui/hud.md#lap-counting-was-the-one-real-blocker) already
names as "the widget bind" - writes widget lookups into. So this function
consumes widgets that function found by name; which named widgets land at
`0x200`/`0x204` specifically was not chased (the two candidates are `TotalTime`
and `TotalTimeTxt`, since only they are known to co-occur in `TimeTrial_HUD.xml`
without a `Position` pair, but `FUN_0881fbec`'s own lookup calls were not
traced far enough to confirm the offsets by name rather than by position).

## No caller found, and that is the relocation bug again

`get_function_callers` and a `jal` operand search both return nothing for
`0x0881c9d0`, in a database where automatic xrefs are known to be empty end to
end - see
[workflow.md](../../workflow.md#why-a-psp-import-silently-loses-every-relocation).
`jal` targets do not need relocation to resolve (the 26-bit target is absolute
within the current 256 MiB segment and needs no `lui`/`addiu` split), so a
missing `jal` is a weaker signal than a missing data xref - but it is still
possible this function is reached only through a function-pointer table, the
same pattern `FUN_0881fbec`'s per-widget-kind dispatch already uses. Reimport
under the fix that page describes before spending more time on this by hand.

## What this does and does not settle for the implementation

**Settled:** the substitution is a real, single, table-driven mechanism keyed
on an ordinal state - not five independent code paths, not a per-mode
`match`. `oag_game::hud::draw::caption` resolving `TotalTimeTxt` straight off
the layout's own `idstring` (the `name => ... caption(label, strings)` catch-all
in [`draw.rs`](../../../../crates/game/src/hud/draw.rs)) is correct as far as
it goes: it is exactly what the original does for every tier this table maps
back to `IG_HUD_TOTAL`, i.e. most of the time.

**Not settled, and not implementable yet:** what drives the tier away from
that default. Wiring `IG_HUD_BRONZE`/`SILVER`/`GOLD`/`RECORD` in without a
real source for tier `0`-`3` would be inventing the medal/record evaluation
this page explicitly did not find - the stand-in this project's own rules
forbid. It waits on the same lap-timing and medal-progression data
[hud.md](../../../ui/hud.md#lap-counting-was-the-one-real-blocker) and its
"Medal targets" item already flag as absent.
