# `Anim Transform` on PS2: one shared clock, not a per-object timer

Read 2026-09-03, chasing which clock feeds the PS2 boost plume's anchor
animation - previously an approximate fit to `Exhaust::plume_timer` with no
PS2 executable read behind it (`crates/game/src/race/scene/frame.rs`'s
boost-plume `write_node_anims` call). This settles that question. It does
**not** repeat the PSP class's full field map or channel
evaluators - [`anim-transform.md`](../psp-pulse-usa/anim-transform.md) already
did that work in detail and every structural finding below matches it
field-for-field, offsets aside. What is new here is PS2-specific: the
addresses, and confirmation that the mechanism the PSP page found is not a
PSP-only design.

## Names recovered

| Address | Name | Conf | Signature |
| --- | --- | ---: | --- |
| `0x001d0350` | `AnimTransform_Update` | 90 | `int (node *a0)` - the per-frame driver |
| `0x0023e098` | `fmodf` | 88 | libc, kept its real name per convention |
| `0x001cffa0` | `AnimTransform_Bind` | 85 | `int (node *a0, LoadContext *a1)` |
| `0x001cfed8` | `AnimTransform_Construct` | 75 | `node *(node *a0)` - per-instance constructor |
| `0x00123878` | `InGame_Construct` | 82 | stores the session object at `g_ingame` |
| `0x00123b98` | `InGame_Destruct` | 82 | zeroes `g_ingame` |

Two data globals are identified but **not yet renamed live** - the Ghidra
MCP bridge's `rename_data` has its own name-quality gate (a mandatory
Hungarian prefix after `g_`) that rejects this project's own
`g_snake_case` convention outright, the same convention every other entry in
[`names.tsv`](names.tsv) already uses. They are recorded here and in
`names.tsv` so `scripts/apply-ghidra-names.py` - which does not go through
that gate - can apply them:

| Address | Name | Conf | Meaning |
| --- | --- | ---: | --- |
| `0x0027ddbc` | `g_ingame` | 85 | the session object; two writers, both above |
| `0x00299b10` | `g_anim_transform_vtable` | 80 | the class vtable, `{offset, method}` pairs, same shape as PSP's |
| `0x002e3640` | `g_ingame_fallback` | 70 | read whenever `g_ingame` is null; no writer traced (matches the PSP page's own unresolved fallback) |

## How the class was reached

Not through a registration site this time - through the three attribute
strings the PSP page already named. `search_strings` for `LoopEnd` in
`SCES_547.48` returns four hits in four different functions; one of them,
`AnimTransform_Bind` (`0x001cffa0`), also reads `AnimEnd` and `FixedFrames`
right after it, at `strcasecmp`-guarded offsets into the same attribute-list
walk the PSP page documents (`h + *(ushort*)(h+6)`, `strcmp`-per-entry,
`+2` stride field). Two things confirm this is the *same* class as PSP's,
not a lookalike with the same three attribute names:

1. **The rotation/scale disable checks.** `AnimTransform_Bind` zeroes the
   rotation-key count when three `s16`s at the rotation-key-values pointer
   are all `0` (a null quaternion) and the scale-key count when three `s16`s
   at the scale-key-values pointer are all `0x100` (a unit scale, `1/256`
   fixed point). Both checks are specific to a class with exactly a
   translation/rotation/scale triple, and both match the PSP binder's own
   checks instruction for instruction, just at different node offsets.
2. **The literal string `"file"`.** `AnimTransform_Construct` (`0x001cfed8`)
   sets `node+0x28 = &"file"` (`0x002b7aa8`) and `node+0x38 =
   &g_anim_transform_vtable`. PSP's `AnimTransform_Register` sets the
   *class descriptor's* `+0x28` to the same literal `"file"` - not a common
   word to share by accident in the same field slot of the same kind of
   struct, across two independently compiled builds of the same engine.

One difference from PSP worth carrying: PSP's binder uses `strcmp`, and a
`LoopEnd` typo'd as `Loopend` in three of PSP's own nodes does not match and
so does not loop. **PS2's binder uses `strcasecmp`** - confirmed at all four
`LoopEnd`-adjacent call sites, including `AnimTransform_Bind`'s own. If the
same `Loopend` typo exists in the PS2 data (not checked here), it loops on
PS2 and not on PSP. Worth checking if a PS2/PSP scenery-animation diff ever
turns up unexplained.

`AnimTransform_Bind` stores `LoopEnd` (converted to seconds, `value *
seconds_per_key`) at `node+0x104` - the field `AnimTransform_Update` reads
as its wrap period, matching PSP's `node+0x54` role exactly, just at a
different offset (PS2's node header runs wider, consistent with the
0x10-aligned VU-friendly layout `AnimTransform_Bind`'s other field
reads suggest throughout).

## The clock: this settles the boost-plume question

`AnimTransform_Update` (`0x001d0350`) is the per-frame driver, found by
walking backward from `AnimTransform_Bind` through the function immediately
before it in the same source-file address range - the same adjacency
`anim-transform.md` used to locate PSP's `Update` next to its `Evaluate` and
`Bind`. Decompiled whole:

```c
int AnimTransform_Update(node *n) {                       // 0x001d0350
    clock = g_ingame ? g_ingame : g_ingame_fallback;
    if (clock == NULL) return 1;
    float dt = clock->0x40 - n->0x130;    // elapsed since this node last ran
    if (n->0x10c != 0) {                  // optional rate multiplier
        float denom = payload->0x30;
        if (denom == 0) goto check_pause;
        float rate = n->0x10c / denom;
        if (rate < 1.0f) rate = 1.0f;
        dt *= rate;
    }
check_pause:
    float t;
    if (n->0x134 == 0) { n->0xc0 += dt; t = n->0xc0; }   // +0x134 is the pause flag
    else                { t = n->0xc0; }
    if (n->0x104 <= t) { n->0xc0 = fmodf(t, n->0x104); } // <- the loop, LoopEnd at +0x104
    Node_MarkDirty(n, 0x1000);
    n->0x130 = clock->0x40;
    return 1;
}
```

This is `AnimTransform_Update`'s PSP counterpart translated field for field:
same fallback-then-primary clock read, same optional rate multiplier gated
on two non-zero checks, same pause-flag gate, same `fmodf`-against-`LoopEnd`
wrap, same "cache the clock's time for next `dt`" tail write. The clock
itself is `g_ingame`, found the same way PSP's was: `InGame_Construct`
(`0x00123878`) is the only writer that stores a non-zero value
(`g_ingame = param_1`, the object under construction) and `InGame_Destruct`
(`0x00123b98`) is the only writer that zeroes it (`g_ingame = 0`) - exactly
PSP's two-writer shape for `g_ingame`. `g_ingame` has 25 other read sites
across the binary (WAD loading, movie playback, camera, collision FX and
more), the same "one clock every in-race subsystem hangs off" shape the PSP
page found.

**So the node clock feeding every `Anim Transform` node on PS2, including the
boost plume's anchors, is one shared per-race clock - not a per-object
reveal timer.** This is not a PS2-specific reading standing alone: it is the
same mechanism `crates/game/src/race/scene/frame.rs` already assumes for
every *other* `Anim Transform` node in the scene ("the scenery: both
animation mechanisms off the one clock", driving `write_node_anims` with the
frame's own `seconds = race.world.tick as f32 / 60.0`). The boost plume's
anchors were the one outlier still wired to `Exhaust::plume_timer` instead -
an approximate fit made with, at the time, no PS2 executable read behind it.
There is no reason to expect the plume's `Anim Transform` nodes to be
special: they are ordinary nodes of this same class, loaded through the same
generic `Vex`/scene-graph machinery as every world mesh, and the engine has
no code path that walks them differently depending on which `.vex` file they
came from.

Confidence **90** on the clock finding itself (`AnimTransform_Update`'s
algorithm is read in full, `fmodf` is a named library call, `g_ingame`'s two
writers are both found and match PSP's shape exactly). The class identity is
**85** - not the maximum in the rubric, because there is no found class-tag
registration comparison the way PSP's `AnimTransform_ClassTag` gave one; the
"file" string match plus the disable-check idiom match is corroboration by
behavior rather than by an identity token.

## What is still open

- **The class registration site itself.** No `Vex_RegisterClass(..., 0x3c0)`
  call was found for PS2 - `AnimTransform_Construct`'s two callers
  (`0x0014b700`, `0x00268dd8`) both look like per-instance object
  constructors reaching into the node system, not the one-time class
  registrar PSP's `AnimTransform_Register` is. If a registration site turns
  up, it would raise the class-identity confidence above 85.
- **`AnimTransform_SetAnimTime`'s equivalent, at vtable slot `+0x80`.**
  Inferred by analogy to PSP's `g_anim_transform_vtable+0x80` (the pointer
  sitting at `g_anim_transform_vtable+0x80` in this table, `0x001d0318`,
  has no Ghidra function covering it) but not independently confirmed by
  decompiling it.
- **`g_ingame_fallback`'s writer.** Not traced, same as PSP's
  `g_anim_clock_fallback`.
- **Whether any PS2 node hits the `Loopend`-typo divergence** from
  `strcasecmp` versus PSP's `strcmp` - not checked against the disc's own
  data.
