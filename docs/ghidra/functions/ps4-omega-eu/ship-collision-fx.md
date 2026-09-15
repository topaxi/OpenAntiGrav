# Ship collision-fx dispatch, read a second time on a second architecture

2026-09-15. `ps3-hdfury-eu/ship-collision-fx.md` read HD/Fury's own
collision-spark dispatch chain (`Ship_DispatchCollisionFx_q`,
`ShipCollisionFx_Trigger_q`, both below 70 confidence, entirely structural -
"nothing inside this function names itself") and left one question open:
which function names the plain `WO_SHIP_COLL_SPARK_DAMAGE`/
`WO_SHIP_SPARK_DAMAGE_LEACHBEAM` variants, since the PPC64 read's one located
call site only reaches `kind` values 0-3 and none of those branches name
them. This page is the same chain read on `eboot.bin` (x86-64, no custom
Ghidra processor module needed - Ghidra's decompiler is at its most mature
here of any binary in this project), found by searching for the three
`WO_SHIP_*` strings the HD page names and following their cross-references
rather than by any structural resemblance search - **the two are independent
readings that happen to agree**, which is what makes this real corroboration
rather than one confirming itself.

## `ShipCollisionFx_Trigger` - `0x01723ae0`

**Confidence: 76.**

Takes `(craft, locator, kind, transform, ...)` and switches on `kind`. All
three of HD's `ShipCollisionFx_Trigger_q` string names are present verbatim
and referenced from this one function, nowhere else:

| `kind` | Names | Matches HD's reading |
| --- | --- | --- |
| 0, 3 | (no-op, shared branch) | Yes - HD's `kind == 0`: no-op; PS4 additionally folds `kind == 3` into the same no-op path, which HD's own four-case read did not find (see below) |
| 1 | `WO_SHIP_SPARK_DAMAGE_WEAPON` | Yes - HD's `kind == 1` "real" spawn path |
| 2 | `WO_SHIP_SPARK_NODAMAGE_ZONE` (masked) / `WO_SHIP_COLL_SPARK_NODAMAGE` (unmasked) | Yes - HD's `kind == 2`, same mask-gated pair of names |

Structural match beyond the names: the same locator-relative transform
build, the same flag-byte gate before the switch body runs at all. This is
independent decompilation on an unrelated ISA (x86-64 vs PowerPC64) reaching
the same three-way kind mapping HD's own read found - the "second binary"
the [confidence rubric](../../../reverse-engineering/confidence-rubric.md)
weighs above a second reading of the first. Not runtime-verified (capped
below 85); no string inside the function names *this* function, so the
identification is still entirely behavioural, same limit HD's own page
noted for its half.

**What this reading adds, not just confirms**: the switch has at least two
more cases HD's read never located - `case 4` (falls straight to the same
return as 0/3, an equivalent no-op) and `case 5`, which spawns a completely
unrelated effect named `WO_FORCE_FIELD`, sharing nothing with the collision-
spark family beyond the same dispatch function. **Two live, unconfirmed
hypotheses for why, neither checked yet:**

1. Studio Liverpool's PS4 remaster folds logic 2048 already has (2048 has
   its own force-field mechanic) into a dispatcher HD/Fury's own PS3 build
   kept narrower - genuinely newer/combined behaviour on this build.
2. `ships-effects.md`'s own `MagstripWake_Construct` finding already
   recorded that this binary inlines setup HD/Vita call out to separately;
   the same compiler behaviour could be merging a sibling dispatcher's body
   into this one's switch rather than this build's game logic actually
   having more cases. `FUN_01330a90` (below, a second, much larger caller of
   this function - 0x223d bytes, not decompiled this pass) is the natural
   place to check that against.

Neither hypothesis is chased here - flagging the divergence and both
readings of it is what's known; resolving it needs the work this page
explicitly leaves open.

## `Ship_DispatchCollisionFx` - `0x012f7500`

**Confidence: 78.**

Loops a locator array, breaking at exactly **ten** slots
(`9 < (long)(uVar12 + 1)` breaks the loop) - the same ten-slot cap HD's own
`Ship_DispatchCollisionFx_q` reads at `craft+0x79d0..+0x79f4`, which itself
matches `hd-status.md`'s independently-measured ten `Ship Collision Fx`
locator nodes on `data/ships/detonator`'s `.vex`. Computes squared Euclidean
distance per slot (subtract, square each component, horizontal sum - the
same shape HD's `FUN_002d91e8` used, though as AVX shuffle/multiply
sequences here rather than PPC64's `vectorSubtractFloatingPoint`), keeps the
minimum, and calls `ShipCollisionFx_Trigger` with the winning locator at the
end. **Three-way agreement**: PPC64 decompile, x86-64 decompile, and the
disc's own shipped `.vex` locator count all land on the same number ten,
none derived from either of the other two.

One divergence from HD's read, not yet resolved: HD found `kind` forwarded
**unchanged** from an outer caller that hardcoded the literal `1` at its own
call site (weapon damage). This PS4 call site instead **computes** `kind`
itself - `2`, `3`, or `5` depending on a nearby float/int test - never `0`
or `1`. That means this specific call site is not the weapon-hit path HD
traced; it is a sibling caller (plausibly the zone/force-field path
`ShipCollisionFx_Trigger`'s own new case 5 spawns into). `Ship_
DispatchCollisionFx` has a second, unexamined caller too
(`FUN_01330a90`, 0x223d bytes) - a candidate for the weapon-hit path HD's
`FUN_0010f730` reaction-table entry represents, not checked this pass.

## What this resolves on the HD side, and what it doesn't

**Resolves**: HD's own "either this function is not the sole owner of every
`WO_SHIP_*SPARK*` variant, or the plain-damage variant is reached through a
different caller of this same function" - it's the first branch. On this
binary, `WO_SHIP_COLL_SPARK_DAMAGE` and `WO_SHIP_COLL_SPARK_DAMAGE_ZONE` (a
fourth variant name, not previously recorded anywhere in this repository -
the zone-mode counterpart to the no-damage pair `ShipCollisionFx_Trigger`
already names) are each referenced from `FUN_01309e90` and `FUN_0130e540`
respectively, and `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` from `FUN_016f0f20` -
three functions distinct from `ShipCollisionFx_Trigger`. **None of the three
are named here** - their cross-reference location is verified fact (the
disassembly says so directly), but their behaviour is not decompiled or
read, so per the confidence rubric's own floor they stay `FUN_`. Whether HD
has the same three-way split under its own three sibling functions, or
merges some of them, is unread on the PPC64 side too.

**Doesn't resolve**: what `FUN_0010f730`'s reaction-table entry (HD, below
50 confidence) actually is - see `Ship_DispatchCollisionFx`'s own second
caller above, an unread lead rather than an answer.

## See also

- [`ps3-hdfury-eu/ship-collision-fx.md`](../ps3-hdfury-eu/ship-collision-fx.md) -
  the PPC64 reading this page corroborates and extends; its own confidence
  is raised and its Open section updated in the same change as this page
- [`README.md`](README.md) - the lineage question this finding is a second,
  independent data point for
