# HD's own collision-fx dispatch, read for the first time - and it is richer than Pulse's

2026-08-31. The open thread on HD's particle effects left
`WO_SHIP_COLL_SPARK_DAMAGE`'s owner at confidence 55 - "that a looping effect
needs one is the flag's own contract, that the owner is *wall contact* is
inference... what would settle it is HD's own dispatch, which nothing has
looked at." This reads that dispatch. **It does not confirm the wall-contact
guess as stated** - it finds a real three-function call chain and a
previously undocumented effect name, and the picture that comes back is more
specific than "wall contact": HD splits weapon damage from generic contact in
a way Pulse's own recovered dispatch does not obviously mirror.

## The chain

```
0x00875800 (reaction dispatch table, entry 11 of >=12, {func,toc} pairs)
  -> FUN_0010f730 @ 0x0010f730           ("a reaction handler", not renamed - see below)
       -> Ship_DispatchCollisionFx @ 0x000d14c8   (nearest-of-ten locator pick)
            -> ShipCollisionFx_Trigger @ 0x002d9730  (kind-switched particle spawn)
```

**2026-09-15: both functions below are corroborated by an independent
reading on a second binary and confidence raised past the `_q` threshold -
see "Corroborated on a second architecture" below.** The per-function
sections keep their original reasoning; the corroboration is additive.

### `Ship_DispatchCollisionFx` (`0x000d14c8`), confidence 78

Takes `(craft, contactPoint, kind)`. Reads ten pointer slots at
`craft+0x79d0..+0x79f4` - the same locator-array shape `hd-status.md` already
measured from the *data* side (`locators.vex` carries exactly ten `Ship
Collision Fx` nodes on `data/ships/detonator`) and the same count
`hd-status.md` predicted from it: "ten is also the number
`Ship_DispatchCollisionFx` picks the nearest of." This is that prediction
read from the executable rather than inferred from the node count. For each
non-null slot it calls `FUN_002d91e8(locator, contactPoint)` - confirmed by
raw disassembly to be squared Euclidean distance (`vectorSubtractFloatingPoint`
then a self dot-product) - and keeps the minimum, exactly Pulse's
nearest-of-N selection. The winner and the caller's `kind` are then passed to
`ShipCollisionFx_Trigger`.

Raw disassembly at the call site (`0x000d1714`, `bl 0x002d9730`):
`r3 = lwzx r3,r8,r11` (the winning locator, indexed by `iVar1*4+0x79d0`),
`r4 = rldicl r4,r27,0,0x20` where `r27` was set from the function's own third
parameter at entry (`or r27,r5,r5`, `0x000d1500`) - i.e. **`kind` is forwarded
unchanged from caller to callee**, confirmed at the register level rather than
trusted from the decompiler's parameter reordering (which for this function
renders the args in a confusing order - see the pitfall this avoided, below).

**Confidence 78 (was 68).** The behavioural match to Pulse's
`Ship_DispatchCollisionFx` is exact and independently corroborated by a
doc page written before this one existed, but nothing inside this function
names itself - the identification was, and still is, entirely structural.
Raised past the `_q` threshold 2026-09-15 by a second, independent
structural match on `ps4-omega-eu` - see "Corroborated on a second
architecture" below; still capped below 85 since neither reading is
runtime-verified.

### `ShipCollisionFx_Trigger` (`0x002d9730`), confidence 76

Takes `(locatorOrObject, kind, ...)` and gates the whole body on a flag byte
(`*(object+0xe4)+0x5f42 == 0`) before switching on `kind`:

- **`kind == 0`**: no-op, returns immediately.
- **`kind == 1`**: the "real" spawn path. Builds a locator-relative transform
  (a long run of `vectorMultiplyAddFloatingPoint` composing what reads as a
  3x4 or 4x4 matrix), allocates a `0x180`-byte object of type `0x21e|0x21e`-tagged
  via `FUN_006762b8`, and names it from a pointer read at TOC displacement
  `0x6a94` - resolved (TOC `0x008ad4d8`, the ordinary default TOC; this
  function needed no per-function TOC correction) to `0x008b3f6c`, which holds
  `0x007a1c68`, which is the string **`WO_SHIP_SPARK_DAMAGE_WEAPON`** -
  **not previously recorded anywhere in this repository.** It is not in
  `crates/game/tests/psys_inventory_ground_truth.rs`'s HD notes, not in
  `hd-status.md`, not in the picked handover thread. Whether it is a `.pob`
  the disc actually ships (as opposed to a name only the executable
  references) is unchecked here - see Open.
- **`kind == 2`**: the no-damage path, and it repeats the same
  `< 0x16` / `& 0x206040` game-state mask `FUN_002d9270` uses to choose
  between a plain and a `_ZONE` resource set (see that function's own
  reading, in the picked thread's evidence). Under the mask it names the
  spawn from TOC `+0x6aa0` (`0x008b3f78`, holding `0x007a1ca8`,
  `"WO_SHIP_SPARK_NODAMAGE_ZONE"`); otherwise from `+0x6a9c` (`0x008b3f74`,
  holding `0x007a1c88`, `"WO_SHIP_COLL_SPARK_NODAMAGE"` - the bare name,
  confirmed to be the same string `docs/formats/hd-status.md` and the picked
  thread already know as an asset on the disc).
- **`kind == 3`**: a teardown/release call (`FUN_00281cb8(x, 1)`), gated on a
  flag bit (`object+0x34 & 0x1000`) and only if a stashed handle
  (`in_stack_00000094`) is non-null. Read as the release half of the
  attach/detach pair `oag_game::race`'s own `sparks_attached` doc comment
  already predicted a looping effect needs.

**What this does not show.** No branch here names `WO_SHIP_COLL_SPARK_DAMAGE`
(the plain, non-weapon damage variant) or `WO_SHIP_SPARK_DAMAGE_LEACHBEAM`.
Only four `kind` values exist (0-3) and all four are read above in full - this
is not a partial read stopping short of a fifth case. Either this function is
not the sole owner of every `WO_SHIP_*SPARK*` variant (a sibling function
plays the plain-damage one), or the plain-damage variant is reached through a
different caller of this same function with a `kind` this pass's one found
call site never supplies. Both are open - see below.

**Confidence 76 (was 65).** Four literal string names pin the variant
selection across two of four branches (as strong a signal as Pulse's own
`ShipCollisionFx_Trigger` read at 85), the switch is branch-clear, and the
flag-gated no-op/spawn/spawn/teardown shape matches what
`oag_game::race::Race::sparks_attached`'s doc comment already needed to
exist. The kind-to-Pulse-variant mapping still isn't a one-to-one match to
Pulse's own three kinds, and `WO_SHIP_COLL_SPARK_DAMAGE` itself is still
absent from all four branches read here - that gap is not closed, just now
independently explained (see below), which is why this moves into the
70-84 band rather than higher.

### Corroborated on a second architecture

**2026-09-15.** [`ps4-omega-eu/ship-collision-fx.md`](../ps4-omega-eu/ship-collision-fx.md)
read the same chain independently on `eboot.bin` (x86-64) - found by
searching for these three `WO_SHIP_*` string names and following their
cross-references, not by looking for a structural match to this page. It
landed on the same three-way `kind` mapping (0/off, 1/weapon, 2/zone-masked
no-damage) and the same ten-slot locator cap this page's own two functions
read. Two independent decompilations of two unrelated ISAs reaching the
same kind mapping is the "second binary" the confidence rubric weighs above
a second reading of the first - hence both functions' scores above.

**It also resolves this page's own open question about the missing
variants**, in the "sibling function" direction rather than the "different
caller, different kind" one: on the PS4 binary, `WO_SHIP_COLL_SPARK_DAMAGE`
(plus a fourth variant this page never found, `WO_SHIP_COLL_SPARK_DAMAGE_ZONE`)
and `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` are each referenced from a different
function, not from `ShipCollisionFx_Trigger`'s own PS4 counterpart. Whether
HD's own binary has the matching three sibling functions is not checked -
the PS4 finding is real and independently verified on *that* binary, but is
being used here as a strong lead for this one, not substituted for it.

**One divergence, deliberately not resolved here**: the PS4 switch has at
least two `kind` values (4, 5) this page's read never found, one of them
(`kind == 5`) spawning an unrelated effect (`WO_FORCE_FIELD`) with nothing
to do with collision sparks. Two explanations are open and neither is
checked: this build could genuinely combine behaviour HD/Fury and 2048 keep
separate (Omega is a remaster of both), or the PS4 binary could simply
inline a sibling dispatcher's body into the same switch more aggressively
than this compiler does - `ps4-omega-eu/ships-effects.md`'s own
`MagstripWake_Construct` finding already recorded that binary inlining setup
this one calls out to separately. Either way it doesn't change the kind
0-3 mapping both binaries agree on above.

### The dispatch-table entry, `0x0010f730` - `Cannon_ApplyCraftHit`, confidence 72

**2026-09-24: it is the Cannon's craft-hit handler.** The earlier read
stopped at the spark call. The block above it, re-read for the Pulse
hit-spark question, writes the struck craft's pending-hit channel exactly
as Pulse's `Cannon_MarkCraftHit`/`Cannon_ApplyCraftDamage` do
([`psp-pulse-usa/cannon-quake-leachbeam.md`](../psp-pulse-usa/cannon-quake-leachbeam.md)):

```c
if (round->flags /* +0x40 */ & 1) {                       // after `|= 0x24`
    victim->attacker /* +0x13c */ = shooter;
    victim->by_rival /* +0x124 */ = FUN_0012d7e0(...);
    victim->kind     /* +0x138 */ = 3;                   // the Cannon's own tag
    victim->pending  /* +0x120 */ += stats->0x90;        // damage per bullet, by position
    victim->slowdown /* +0x130 */ += stats->0x94;        // slowdown time, by position
}
Ship_DispatchCollisionFx(victim_entity->0xf0, contact, 1); // WO_SHIP_SPARK_DAMAGE_WEAPON
```

So **on HD the Cannon throws its own spark on a craft hit, from the weapon's
side**: `WO_SHIP_SPARK_DAMAGE_WEAPON` on the locator nearest the contact.
On Pulse the Cannon spawns nothing on a craft, and the struck hull sparks
from `Ship_Damage` instead
([`psp-pulse-usa/shield.md`](../psp-pulse-usa/shield.md), "`Ship_Damage`'s
weapon branch throws the hit sparks"). The spark call is not inside the
`& 1` gate. `Ship_DispatchCollisionFx` has no other direct caller.
Confidence **72**: the tag literal `3` and the three field offsets match
Pulse's Cannon field for field. Two things are not read: which stats fields
`+0x90`/`+0x94` are, and whether HD's own `Ship_Damage` also sparks.

What follows is the 2026-08-31 reading, kept as written:

**Confidence below 50, per `CLAUDE.md`'s own rule - left as `FUN_0010f730`.**

It is entry 11 (0-indexed, byte offset `0x58`) of a table of >=12 raw
`{func, toc}` descriptor pairs starting `0x00875800` - the same shape as the
already-named `g_CraftVtable`/`g_DebrisVtable` (`physics.md`), i.e. a
reaction/handler table selected by some contact or material-pair type this
pass did not identify. Its own body runs an indirect ("virtual") call through
an object's own vtable slot `+0x44` to get a contact result into two stack
buffers, ORs `0x24` into a flags word at `object+0x40` - the same
flag-writing shape Pulse's own contact-response code uses
(`docs/ghidra/functions/psp-pulse-usa/contact-response.md`'s `0x20`/`0x400020`
bits) - and then unconditionally calls `Ship_DispatchCollisionFx` with
`kind` **hardcoded to the literal `1`** at this call site (confirmed in
disassembly, not decompiler paraphrase).

That hardcoded literal is structurally identical to Pulse's own "`kind` is
hardcoded `0` at that call site" wall-contact reaction
(`contact-response.md`) - a reaction table entry that always fires one fixed
kind is exactly the shape a per-material collision reaction takes in this
codebase. **But `kind == 1` is the branch that names
`WO_SHIP_SPARK_DAMAGE_WEAPON`**, a weapon-sounding name, which is the reason
this entry is not named `Ship_WallContactReaction` or similar on the
strength of the structural match alone - doing so would be exactly the kind
of plausible-but-unverified name `CLAUDE.md` and `vex-classes.md`'s own
"framing trap" section warn against. What table index 11 actually represents
- wall, weapon impact, or something this pass has not distinguished - is
undetermined. See Open.

## The reusable technique, and why it was necessary

Every step above needed the whole-image literal-address technique
`toolchain.md`'s PS3-traps section already describes (`zone-effectsettings-loader.md`'s
"nineteenth pass") - `get_xrefs_to` on every string and every dispatch-table
address in this chain came back **empty**, consistently, because each
consumer reaches its target through its own per-function TOC rather than the
one Ghidra defaults to. What is new here is doing that search **with a
script instead of by hand**: for a target address, walk every function's
`r2` value (read from the program's own context register - reliable, because
`AssignPs3R2FromOpd.java` already populated it during import, confirmed by a
before-writing sanity check that reproduced `zone-effectsettings-loader.md`'s
already-known `Scene_PrepareFrame @ 0x003aaf8c: lwz r9,-0x61f8(r2)` finding
exactly, with zero other hits), compute the byte displacement to the target,
and keep only functions whose own disassembly contains a `d(r2)`-addressed
instruction at that exact displacement. This is mechanical, sound (validated
against a known answer before being trusted on an unknown one), and turns a
technique that previously took nineteen manual passes into a few seconds per
target - see `run_script_inline`'s script body, not committed anywhere but
reproducible from this description. Worth reaching for first on any future
"this string/global has no xrefs" dead end on this binary, before assuming
the consumer doesn't exist.

One target searched this way came back **empty** and stayed empty even after
widening to the dispatch table's own base address (`0x00875800`) - the
reaction table is reached through at least one more hop (very likely a
runtime object field holding the table pointer, populated once at
construction, itself reached through machinery this pass did not trace) than
the single-TOC-slot pattern the rest of this chain used. That is why
`FUN_0010f730`'s own caller/index is open rather than read.

## Open

- **What names the plain `WO_SHIP_COLL_SPARK_DAMAGE` and
  `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` variants on *this* binary is still
  unfound**, though the shape of the answer is no longer a mystery -
  `ps4-omega-eu/ship-collision-fx.md` found the equivalent names owned by
  three sibling functions there, and HD/Fury very plausibly has the same
  shape. `ShipCollisionFx_Trigger`'s four `kind` values (0-3, all read) do
  not cover them here either way. Finding this binary's own three sibling
  functions - not just assuming the PS4 answer transfers - is the actual
  next step.
- **`WO_SHIP_SPARK_DAMAGE_WEAPON` is a newly found name**, referenced only
  from `ShipCollisionFx_Trigger`'s `kind == 1` branch. Not cross-checked
  against the disc's own `.pob` inventory (`psys_inventory_ground_truth.rs`'s
  `hd` module) - do that before deciding whether it needs a
  `NO_TRIGGER_RECOVERED`-style entry or is already covered under a name
  variant this repo already lists.
- **Whether this binary's own switch has the PS4 build's extra `kind`
  values (4, a no-op; 5, `WO_FORCE_FIELD`, unrelated to collision sparks)**
  is unchecked - not found in this pass's read, but this pass did not
  specifically look past `kind == 3` either. If HD's own switch turns out
  narrower, that would favour the "PS4 inlines more aggressively" reading
  over "PS4 combines HD+2048 logic" - see `ps4-omega-eu/ship-collision-fx.md`'s
  own open pair of hypotheses, neither resolved yet.
- **HD may have a shield-state damage smoke that Pulse lacks.** The EBOOT
  names `WO_DAMAGE_MILD`, `WO_DAMAGE_MODERATE` and `WO_DAMAGE_CRITICAL`
  (`0x0079be40`..`0x0079be68`, path strings at `0x007a1a20`..`0x007a1a68`).
  Pulse has no such names, and a Pulse craft at low shield draws nothing.
  Their consumer is unread: the only xref is a data pointer at `0x008b2fa0`,
  so the TOC-displacement search above is the way in. Also unread: whether
  HD's `Ship_Damage` throws hull sparks the way Pulse's does.
- **Index 11 is now identified as the Cannon's craft hit** (2026-09-24,
  above); the table's key is still unread.
- **What `0x00875800`'s reaction table is keyed by, and what index 11
  represents**, is undetermined - the table itself is reached through at
  least one hop this pass's literal-address search technique could not close
  in one step (see above). Whether index 11 is wall contact, weapon impact,
  or something this pass has not distinguished is the actual open question
  the picked thread's confidence-55 bullet asked about, and it is **still
  open**, now with a concrete function (`FUN_0010f730`) and a concrete blocker
  (the table's own base is a second hop away) rather than no dispatch read at
  all.
- The other 10+ entries of the `0x00875800` table are unread; at least one
  (`FUN_0010f390`, entry 1) does something unrelated to particle effects
  (queues something into a small array), confirming the table is a general
  reaction dispatch and not specific to collision fx.

## History

- 2026-09-24: `FUN_0010f730` renamed `Cannon_ApplyCraftHit` (72): it writes the
  Cannon's pending-hit channel, tag `3`, before its spark call.
- 2026-08-31: `Ship_DispatchCollisionFx_q` 68, `ShipCollisionFx_Trigger_q`
  65 - entirely structural, single binary, `_q` suffixed per the rubric.
- 2026-09-15: raised to 78 and 76 respectively, `_q` dropped - an
  independent reading on `ps4-omega-eu` (x86-64) reached the same kind
  mapping without looking for a structural match to this page first. See
  "Corroborated on a second architecture" above.

## See also

- [`memory.md`](memory.md) - the two-TOC defect this whole chain had to route
  around
- [`vex-classes.md`](vex-classes.md) - an unrelated table (`g_VexClassTable`)
  sitting nearby in the same data segment; ruled out as a dead end for this
  question early in this pass, kept here as a note so a future reader does
  not re-walk it expecting a trigger
- `docs/ghidra/functions/psp-pulse-usa/contact-response.md` - Pulse's own
  `ShipCollisionFx_Trigger`/`Ship_DispatchCollisionFx`, the shape this page
  compares HD against throughout
- [`ps4-omega-eu/ship-collision-fx.md`](../ps4-omega-eu/ship-collision-fx.md) -
  the second, independent reading that raised this page's confidence
