# `Anim Transform` (`0x3c0`): the class that moves scenery

The scene-graph node whose translation, rotation and scale are each a keyframe
track. Read whole on 2026-08-18, from the class registration down to the three
channel evaluators, and ported in the same change - the format side is
[`vex.md`](../../../formats/vex.md), the renderer side and the counts are
[`scenery-animation.md`](../../../rendering/scenery-animation.md).

Distinct from the per-material texture transform in
[`texture-animation.md`](texture-animation.md), which slides a *coordinate*
under fixed geometry. This one moves the geometry.

## Names recovered

| Address | Name | Conf | Signature |
| --- | --- | ---: | --- |
| `0x0890009c` | `AnimTransform_Register` | 90 | `void (void)` |
| `0x088fe5a4` | `AnimTransform_Bind` | 90 | `int (node *a0, LoadContext *a1)` |
| `0x088fe400` | `AnimTransform_Evaluate` | 90 | `void (node *a0)` |
| `0x088fe0a8` | `AnimTransform_Update` | 90 | `int (node *a0)` |
| `0x088fe564` | `AnimTransform_SetAnimTime` | 88 | `void (node *a0, float seconds in f12)` |
| `0x088fed44` | `AnimTransform_EvalTranslation` | 90 | `void (float t in f12, node *, mat4 *, u32 frame, char step)` |
| `0x088ff1c8` | `AnimTransform_EvalRotation` | 90 | as above |
| `0x088ffc08` | `AnimTransform_EvalScale` | 90 | as above |
| `0x089100a0` | `Mesh_Register` | 88 | `void (void)` |
| `0x08a6bb84` | `AnimTransform_ClassTag` | 88 | `void *(void)` |
| `0x08a6bd48` | `Mesh_ClassTag` | 88 | `void *(void)` |
| `0x08ad0fb4` | `g_anim_transform_vtable` | 85 | data |
| `0x08ab0818` | `g_ingame` | 85 | data - the in-game object, whose `+0x40` is the animation clock |
| `0x08b620f8` | `g_anim_transform_desc` | 85 | data - the `desc` local `AnimTransform_Register` builds and passes to `Vex_RegisterClass`; see below |
| `0x08813328` | `InGame_Update` | 85 | `int (float dt in f12, InGame *this in a0)` - advances `this->0x40` once its own boot sequence (`this->0x3c` < 3) has run |
| `0x08ac7ab0` | `g_ingame_vtable` | 80 | data - `InGame`'s own class vtable, three of its slots corroborated (`InGame_Update`, `InGame_UpdatePauseInput`, `InGame_Destruct`) |

**A trap first, because every address above depends on it.** This program's
`jal` targets and `lui`/`addiu` immediates are **link-time**, not runtime: the
ELF is relocatable and Ghidra rebased the sections without rewriting the
immediates. Confirmed two independent ways before anything here was believed:
`jal 0x0010d4fc` inside `Node_SetAnimTimeTree` is that function's own recursive
call (`0x089114fc`), and `jal 0x00104eb8` is the already-named
`Vex_RegisterClass` (`0x08908eb8`). A reader who takes the decompiler's
`func_0x...` at face value will chase addresses that do not exist.

**It is not always `+0x08804000`.** A `jal` target always is - a call target
is always in `.text`. A `lui`/`addiu` or `lui`/`lw` pair can resolve into
either of this binary's two `PT_LOAD` segments, and each has its own base:
`+0x08804000` for segment 0 (`.text` through `.data`), `+0x08ad9798` for
segment 1 (`.cplinit`/`.linkonce.d`/`.ctors`/`.bss`). `g_anim_transform_desc`
above is the second case in this same function: its `lui a0,0x9` / `addiu
s0,a0,-0x76a0` at `0x089000a0` computes to `0x0888c960` under the `.text`
base - which lands inside an unrelated function, `FUN_0888c408` - and to
`0x08b620f8`, in `.bss`, under the segment-1 base instead. Full derivation:
[the second relocation base](#the-second-relocation-base-is-found-it-is-per-segment-not-per-image),
found chasing the same problem in `AnimTransform_Update`'s clock fallback.

## How the class was reached

Not from the class-name table - through its own registration site, which is the
same route [`pads.md`](pads.md) used for the two pad classes:

```c
void AnimTransform_Register(void) {                     // 0x0890009c
    Vex_RegisterClass(&g_anim_transform_desc, 0x3c0);    // 0x08b620f8, 0x3c0 = Anim Transform
    g_anim_transform_desc->0x38 = g_anim_transform_vtable; // 0x08ad0fb4
    g_anim_transform_desc->0x28 = "file";                // 0x08a84bc0
    g_anim_transform_desc->0x04 = AnimTransform_ClassTag(); // 0x08a6bb84
    ...
}
```

The `li a1, 0x3c0` in the `jal`'s delay slot at `0x089000b8` is the only one in
the image, so this is the registration and there is no second candidate.

`g_anim_transform_vtable` is an array of `{u32 offset, u32 method}` pairs. Slot
`+0x80` holds `{0, 0x000fa564}` = `AnimTransform_SetAnimTime`, which is what
`Node_SetAnimTimeTree` (`0x089114fc`) dispatches. **Corroborated against a
known-good neighbour rather than assumed**: Mesh's vtable at the same `+0x80`
holds `0x0010a240` = `Mesh_SetAnimTime` (`0x0890e240`), already named at 78 in
[`texture-animation.md`](texture-animation.md).

### This corrects `texture-animation.md`'s reading of `Node_SetAnimTimeTree`

That page records the walker as dispatching "for nodes of three classes (Mesh's
class-identity `0x08a6bb84` plus two unidentified)". **`0x08a6bb84` is `Anim
Transform`'s tag, not Mesh's.** Mesh's is `0x08a6bd48`, and it is the walker's
*second* branch. Both are pinned by their own registration functions, each
storing the tag it calls into its descriptor's `+0x04`:
`AnimTransform_Register` calls `0x08a6bb84`, and `Mesh_Register`
(`0x089100a0`, the only `li a1, 0x125` in the image) calls `0x08a6bd48`.

The third branch, `0x08a6bd18`, is `PsysNode_Tag` - `ParticleSystem`'s identity
tag, identified 2026-10-02 in [placed-particle-systems.md](placed-particle-systems.md).
The text that follows predates that: it was still unidentified - it is called from 40
sites including `Missile_Init`, `Rocket_Init` and `ShipCollisionFx_Trigger`,
which is the shape of a base class rather than a leaf. **Not named**, per the
[rubric](../../../reverse-engineering/confidence-rubric.md)'s rule for a reading
below 50.

Nothing built on the old reading breaks: the walker's three branches all
dispatch the same vtable slot, so the *behaviour* documented there is unchanged.
What changes is which class each branch names - and it matters, because it is
why `Node_SetAnimTimeTree` exists at all: it feeds the texture-transform clock
and this class's clock through one tree walk.

The class-tag functions are the same three-instruction idiom `pads.md` records -
`lui v0, hi; jr ra; addiu v0, v0, lo`, returning **their own address** as a
unique token. They sit in a contiguous table of 12-byte thunks.

## The payload, and how the field map was pinned

`AnimTransform_Bind` (`0x088fe5a4`) stores the payload at `node+0x50` and
relocates **six** pointer fields in place by adding the payload base - at
`+0x08`, `+0x0c`, `+0x1c`, `+0x2c`, `+0x38` and `+0x40`. That is the field map's
skeleton: six arrays, four of them found by which evaluator dereferences which.

```text
+0x00 u16    unused; 0 on all 393 nodes
+0x02 u16    translation key count
+0x04 u16    rotation key count
+0x06 u16    scale key count
+0x08 u32    rotation key times      -> u16 frames
+0x0c u32    translation key times   -> u16 frames
+0x10 f32[3] translation base
+0x1c u32    rotation key values     -> (s16, s16, s16) per key
+0x20 f32[3] translation quantum, per axis
+0x2c u32    translation key values  -> (s16, s16, s16) per key
+0x30 f32    rate-multiplier denominator; see AnimTransform_Update
+0x34 u32    flags: bit0 picks the translation evaluator, bit1 the rotation one
+0x38 u32    scale key times         -> u16 frames
+0x3c f32    seconds per key unit
+0x40 u32    scale key values        -> (s16, s16, s16) per key, 1/256
```

**The map is falsifiable and was falsified against, not merely fitted.** A key
count of `0` still stores one key, and under the map above the six arrays tile
`[0x50, payload_len)` **exactly on all 393 nodes of all twelve circuits** - one
wrong offset or stride breaks the tiling on the first file.
`crates/render/tests/scenery_animation_ground_truth.rs` re-runs it.

Two of the counts are pinned a second way, by the binder's own disable checks:
it reads three `s16` at `+0x1c` and switches the `+0x04` channel off when they
are all `0` (a null quaternion), and three at `+0x40` switching `+0x06` off when
they are all `0x100` (a unit scale). Those defaults only make sense for rotation
and for scale respectively.

`+0x34` is **zero on all 393**, so the two alternate evaluators it selects -
`0x088ff6ec` for rotation and `0x088fe934` for translation - were never entered
and are **not read**. A non-zero value there means this decode does not apply.

## The three channels

`AnimTransform_Evaluate` (`0x088fe400`) converts the node's clock to key units
and calls the three in a fixed order, then caches the time at `node+0x60`:

```c
float t = node->0x40 / payload->0x3c;   // seconds / seconds-per-key = frames
int frame = (int)modff(t, &whole);
mat4 *m = FUN_08945220(node);
(payload->0x34 & 2) ? EvalRotation_8bit(...) : AnimTransform_EvalRotation(...);
(payload->0x34 & 1) ? EvalTranslation_alt(...) : AnimTransform_EvalTranslation(...);
AnimTransform_EvalScale(...);
node->0x60 = t;
```

So **rotation writes the basis, translation writes row 3, scale then multiplies
rows 0 to 2** - the composition is scale, then rotate, then translate.

All three pick a key the same way, and it is the same clamp-and-lerp
`TexAnim_EvalKeyframes` (`0x08927034`) makes for the texture block: hold the
first key below `times[0]`, hold the last at or past `times[count-1]`, otherwise
blend from the preceding key. The `step` argument snaps to the preceding key
instead.

- **Translation** (`0x088fed44`): `pos = value * quantum + base`, one `vmul_q`
  against `payload+0x20` and one `vadd_q` against `payload+0x10`, written to the
  matrix's `+0x30` row with `+0x3c` forced to `1.0`. A count of `0` writes a
  constant row instead.
- **Rotation** (`0x088ff1c8`): each `s16` is scaled by `3.051851e-05` = 1/32767
  and `w` is reconstructed as `sqrt(1 - x^2 - y^2 - z^2)` - a **compressed unit
  quaternion**, three components stored. Between keys it slerps
  (`0x08986060`) and renormalises through `vdot_q`/`vrsq_s`/`vscl_q`, then
  converts to a matrix. A count of `0` copies a constant identity basis.
- **Scale** (`0x088ffc08`): lerped raw, then scaled by `0x3b800000` = **1/256**
  - the same fixed point the texture-transform block uses - and multiplied into
  matrix rows 0, 1 and 2. A count of `0` leaves the basis alone.

## The clock, and what `LoopEnd` actually does

`AnimTransform_Update` (`0x088fe0a8`) is the per-frame driver, and it is where
the loop lives - not in the setter and not in the evaluator, which is why a
first pass through those two left the wrap unaccounted for:

```c
int AnimTransform_Update(node *n) {
    clock = g_anim_clock ? g_anim_clock : g_anim_clock_fallback;
    float dt    = clock->0x40 - n->0x80;   // elapsed since this node last ran
    float loop  = n->0x54;                 // LoopEnd, in seconds
    float t     = n->0x40;

    if (n->0x5c != 0 && payload->0x30 != 0) {       // optional rate multiplier
        float rate = n->0x5c / payload->0x30;
        if (rate < 1.0f) rate = 1.0f;
        dt *= rate;
    }
    if (n->0x84 == 0) { t += dt; n->0x40 = t; }     // +0x84 is a pause flag
    if (loop <= t) n->0x40 = fmodf(t, loop);        // <- the loop
    Node_MarkDirty(n, 0x1000);
    n->0x80 = clock->0x40;
    return 1;
}
```

**So `LoopEnd` is the wrap period, applied as `fmodf`, and that is read rather
than inferred - confidence 90.** An earlier revision of this page put the wrap
*site* at 55 and said only the attribute was evidenced; that caveat is
withdrawn. `oag_vex::vex::AnimTransform::sample` wraps by
`loop_seconds` and matches, with one difference worth stating: the original
integrates `dt` into a per-node clock while this project evaluates
`race_seconds % loop` directly. The two agree while a node starts at zero and is
never paused, which is every node on the disc - `+0x84` is not written by
anything read here.

**This also retires "`+0x30` is read by nothing".** It is the denominator of the
rate multiplier above, against a per-node `+0x5c`. Both guards are `!= 0`, and
nothing read so far writes `+0x5c`, so the multiplier is inert on this disc -
which is consistent with `+0x30` being `0.0` on the median node. What `+0x5c` is
and who sets it is **not traced**; the field's *role* is now evidenced at 85,
its *units* at 0.

### There is one animation clock, not a clock per model

`AnimTransform_Update` takes its `dt` from `g_ingame->0x40`, and
`AnimTransform_SetAnimTime` copies the same field into `node+0x80`. So does
`Mesh_SetAnimTime` (`0x0890e240`) - the two functions are the same shape
instruction for instruction, differing only in where they store it
(`node+0x80` against `mesh+0x194`).

**`g_ingame` has exactly two writers, and both are already named**:
`InGame_Construct` stores the object at `0x08812ddc` and `InGame_Destruct`
zeroes it at `0x088131b0`. Its readers are the animation paths above plus
`Texture_BindCausticFrame` and `ShipCollisionFx_Trigger` - the shape of the
session object every in-race subsystem hangs its clock off.

That answers a question [`texture-animation.md`](texture-animation.md) has
carried as open since the texture-transform work - "the general rule for which
clock a model gets was not traced". **The rule is that there is no per-model
rule**: both mechanisms read one clock off one object. Its census saw eleven
world meshes on the race clock and one on `4.80`; the odd one out is a model
whose `+0x40` was written by something other than the tree walk, not a model on
a different clock source. Confidence **85** - the writers and readers are read.

A third reader turned up while chasing the advancer below, at
`0890cefc`/`0890cf4c` - same fallback-then-primary pattern, feeding a `dt`
into an object with its own cache at `+0x194` and pause flag at `+0xc0` (the
`Mesh_SetAnimTime` field shapes). Ghidra has no function covering that
address (`get_function_by_address` returns nothing for it), so it is not
named - one more instance of the decompiler gaps this binary already has open
elsewhere, not investigated further here.

**Measured live 2026-10-01** (`psp-weapon-pair.py`, every frame row carries it):
`g_ingame->0x40` read 49.52 s in the start-line countdown of one run and 90.676 s at GO of
another, advancing 0.0166-0.0168 per frame - it counts from the session's start, not the
race's, so the phase of any animated node at a given race tick is not fixed. The laid Bomb
and Mine play their own `Anim Transform`s off it (`mine.md`, 2026-10-01 second pass).

#### What advances `g_ingame->0x40`

Found: **`InGame_Update`** (`0x08813328`), dispatched through the object's own
class vtable (`g_ingame_vtable`, `0x08ac7ab0` - the same `+0x38` slot pattern
`AnimTransform_Register` uses for `g_anim_transform_vtable`, confirmed by three
of its slots resolving to already-named functions: `InGame_UpdatePauseInput`
and `InGame_Destruct` sit at two other slots in the same table). Its own
disassembly:

```c
int InGame_Update(float dt in f12, InGame *this in a0) {  // 0x08813328
    if (this->0x3c < 3) {
        if (this->0x3c == 1) { /* one large one-time setup pass */ }
        // this->0x3c == 0 or == 2: nothing, just count
    } else {
        this->0x40 = this->0x40 + dt;   // <- the advancer
    }
    this->0x3c += 1;
    return 1;
}
```

`this->0x3c` is a call counter, not a game-time phase: the first three calls
are consumed bringing the race up (call 0 and call 2 do nothing but count,
call 1 does one large allocation-and-registration pass for the race's HUD and
managers - the block already partly read as a plain decompile in the earlier
draft of this investigation), and only from the fourth call onward does the
function do anything to `+0x40` at all - a straight `+= dt` with no clamping,
looping or pausing of its own (the `+0x84` pause flag other readers test is a
per-*node* flag, not read here). This matches the field being zero-initialized
in `InGame_Construct` and never written except here and by construction/
destruction. Confidence **85**: the field offsets match `InGame_Construct`'s
own initialization exactly (`+0x3c`, `+0x40`), the dispatch site is
corroborated by two independently-named sibling slots in the same table, and
the write instruction sequence (`lwc1`/`add.s`/`swc1` on `+0x40`, `f12` as the
first float argument) is unambiguous. What is not traced: who calls
`InGame_Update` itself (presumably a generic per-frame Update dispatch through
the same `+0x38` vtable slot other classes may also populate), so the actual
frame-to-frame `dt` value's own provenance is one level further out than this
page goes.

#### The second relocation base is found: it is per-segment, not per-image

Both functions fall back to a second global when `g_ingame` is null:
`lui a0,0x6` / `lw a0,-0x7fe8(a0)` at `0x088fe0f4`, naive value `0x00058018`.
Adding the `.text` image base `0x08804000` lands at `0x0885c018` - inside a
real function (`FUN_0885bf84`, confirmed by disassembling it: dense, legitimate
`lv.q`/`sv.q` VFPU code, not a literal pool) - so that base is wrong for this
one, not merely coincidental.

This binary's ELF has **two** `PT_LOAD` program headers, and each gets its own
base once the image loads: segment 0 (`VirtAddr 0x00000000`, covering `.text`
through `.data`) gets `+0x08804000`, and segment 1 (`VirtAddr 0x002d5798`,
covering `.cplinit`/`.linkonce.d`/`.ctors`/`.bss`) gets
`+0x08804000 + 0x002d5798 = 0x08ad9798`. Applying the segment-1 base here:
`0x08ad9798 + 0x00058018 = 0x08b317b0` - which lands in `.bss`, and is exactly
`texture-animation.md`'s `DAT_08b317b0`, the address an earlier import had by
hand and this page previously could not reproduce.

Confirmed a second way, against the raw `.rel.text` relocation records rather
than the address arithmetic alone: the two-entry `R_MIPS_HI16`/`R_MIPS_LO16`
pair for this global carries `r_info = 0x00010005` / `0x00010006` - bits 16-23
set to `1` - while `g_ingame`'s pair (`0x088fe0b8`/`0x088fe0bc`, the confirmed
segment-0 case) carries `0x00000005`/`0x00000006`, bits 16-23 `0`. Three more
confirmed segment-0 globals on this same page's family
(`0x08ab0838`/`PickupBackground`, `0x08ab2120`, `0x08ab10e8`, all cited from
[zone-mode.md](zone-mode.md)) carry the same `0` in that byte. So bits 16-23 of
`r_info` is the segment selector, `0` or `1` matching the program header index,
and the two observed bytes (`0x00` and `0x01`) are the only two segments this
binary has - there is no third base to find. Confidence **90**: exact
arithmetic reproducing an independently-recorded address, corroborated by the
relocation record itself on both the positive (segment 1) and negative
(segment 0) cases.

**The rule generalises: any `.cplinit`/`.linkonce.d`/`.ctors`/`.bss` global in
this binary needs `+0x08ad9798`, not `+0x08804000`.** HANDOVER.md's relocation
trap previously said "apply the workaround unconditionally... there is no
exempt region" for the single `+0x08804000` case - that is now corrected there.
The class descriptors mentioned by an earlier draft of this caveat as sharing
"the same problem" were not re-checked in this pass; if one is later found in
`.bss` or `.cplinit`, this is the fix to apply.

**Corrected 2026-09-05: `+0x08ad9798` is not the whole rule either - the correct
addend is per-relocation-entry, not per-segment, and "there is no third base to
find" does not hold.** Found chasing `DAT_08b32428` (the WeaponStats file
selector - see [missile.md](missile.md)'s write-site citation, which turned out not
to be `$gp`-relative after all despite the addressing-mode name it was chased
under): three naive values in one weapon-code function -
`0x00057b40`, `0x00058fd0`, `0x00058fd8` - all land in `.bss`, but need
`+0x08ad9450` (not `+0x08ad9798`) to reproduce three independently-known
addresses (`DAT_08b30f90` from [pads.md](pads.md), and `DAT_08b32420`/
`DAT_08b32428` from [missile.md](missile.md)), each confirmed by an
instruction-level read matching that page's own description, not by arithmetic
alone. `0x00058018` (this page's own confirmed case, needing `+0x08ad9798`) sits
*inside* that same naive range (`0x00057b40`-`0x00058fd8`) yet needs a different
addend - so the two cannot both be explained by "which of the two `PT_LOAD`
segments". The addend most likely tracks which object file's local, zero-based
`.sbss`/`.bss`/`.scommon` numbering the linker merged into the final section,
which the segment's `VirtAddr` alone cannot recover. **Practical rule, replacing
the segment lookup:** never compute a naive-value correction from a segment
header. Instead, find one instruction near the naive value whose target is
already independently documented (a load/store at a known field offset, a
string reference, a value some other page measured live), solve for the addend
from that single instruction, and only then apply it to nearby naive values in
the *same function* - not the same segment. The `.rel.text` `r_info`
segment-index check used above is still valid evidence for the *specific*
relocation entries it was run against; it just does not license generalising
to "this segment" the way it looks like it should.

**Retracted 2026-09-05, later the same day: there is no third base, and the
per-segment rule was right after all.** The correction above rests on three
naive values, `0x00057b40`, `0x00058fd0` and `0x00058fd8`. **None of the three
exists in this binary.** Replaying the PRX relocation sections directly
(`scripts/psp-relocate.py`, which walks `SHT_PRXRELOC` rather than decoding
instruction immediates by hand) gives the record count for each naive value:

| Naive value | Relocation records | Resolves to |
| --- | ---: | --- |
| `0x000577f8` | 931 | `0x08b30f90` |
| `0x00057b40` | **0** | - |
| `0x00058018` | 120 | `0x08b317b0` |
| `0x00058c88` | 147 | `0x08b32420` |
| `0x00058fd0` | **0** | - |
| `0x00058fd8` | **0** | - |

Every real value resolves under the **segment-1 base `+0x08ad9798`** with no
exception, reproducing `DAT_08b30f90`, this page's own `0x08b317b0`, and
`DAT_08b32420` together. And each of the three phantom values is exactly
`+0x348` from a real one - `0x57b40 - 0x577f8`, `0x58fd0 - 0x58c88` and
`0x58fd8 - 0x58c90` are all `0x348`. So a single constant slip of `+0x348` was
made while decoding the immediates by hand, and then cancelled by inventing a
base `0x348` lower (`0x08ad9798 - 0x348 = 0x08ad9450`). Two errors that
multiply out to the right answers, which is why the three cross-checks all
appeared to confirm it.

The practical rule this section proposed - "solve for the addend from one
documented instruction, then apply it only within the same function" - should
**not** be followed. It generalises a hand-arithmetic slip into a theory about
linker `.sbss` numbering, and it would keep producing a bespoke constant per
function forever. Read the relocation records instead; they say which of the
two segment bases each entry uses, and there are only ever two. The whole model
is validated against five independently-recorded addresses at once in
[workflow.md](../../workflow.md).

## The node attributes, and what `+0x0e` is

`AnimTransform_Bind` looks up three named attributes on the **node header**,
each with a `strcmp` against a literal:

```c
h = node->header;
if (*(short *)(h + 0x0e) == 0) -> no attributes
a = h + *(ushort *)(h + 6);
while (*(short *)(a + 2) != 0) {
    if (strcmp(a + 4, name) == 0) { value = *(float *)(a + *(byte *)(a + 1)); break; }
    a += *(ushort *)(a + 2);
}
```

| String | At | Stored | Meaning |
| --- | --- | --- | --- |
| `LoopEnd` | `0x08a84bc8` | `node+0x54`, seconds | the loop period, in key units |
| `AnimEnd` | `0x08a84bd0` | `node+0x58`, seconds | not traced further |
| `FixedFrames` | `0x08a84bd8` | `node+0x78`, bool | snap to the key instead of blending |

Both time attributes default to `6000.0 / seconds_per_key` key units, which at
1/60 is **6,000 seconds** - long enough that nothing on the disc reaches it,
which is what makes "no `LoopEnd`" mean "does not loop".

**This settles the `unk_0x0e` field of `oag_vex::vex::Node`.** Its doc
recorded the meaning as unestablished, with the observation that where it is set
"the values are small and round (24, 36, 48, 56, 68, 368) and look like byte
counts". They are byte counts, of exactly this attribute list.

Measured over three circuits' `Anim Transform` nodes: `LoopEnd` 162 times,
`FixedFrames` 16, and **`Loopend` three times** - a case typo in the artists'
own data that the engine's `strcmp` does not match, so those three nodes do not
loop in the original either. Reproducing that is a decision, not an oversight;
matching them case-insensitively would be a departure.

## A mesh's texture time: seeded per spawn, so object age (2026-10-05)

Read for the three effects whose tracks look like object-age shapes (`explosion_hemisphere`,
`Bomb_Shockwave`, `pulse_repulsorwave`), then measured live. The clock above is not the only
phase a mesh can have: **what `Mesh_SetAnimTime` seeds decides it.**

```c
void Mesh_SetAnimTime(float t, Mesh *m) {                 // 0x0890e240
    m->0x40  = t;                                          // the texture time
    m->0x194 = (g_ingame ? g_ingame : fallback)->0x40;     // cache: the clock now
}
// the per-frame update, no Ghidra function covers it (0x0890cef8..0x0890cf80):
dt = clock->0x40 - m->0x194;
if (m->0xc0 == 0) { m->0x40 += dt; }                       // 0xc0 is the pause byte
m->0x194 = clock->0x40;
```

A model that is never seeded has `+0x40 = +0x194 = 0`, so its first update adds the whole session
clock: the **race clock** (the laid Mine and Bomb, the scenery). A model seeded with
`Node_SetAnimTimeTree(0.0)` at construction has `+0x194` set to the clock *then*, so `+0x40` counts
**the object's age at rate 1**: `BombBlast_Construct` (`0x08872078`, at `0x08872268` for the
hemisphere and `0x08872470` for the shockwave), `Repulser_Init` (`0x08875210`, at `0x08875324`)
and `ShipShockwave_Construct` (`0x0885ecf0`, at `0x0885ee44`, `f12 = 0.0`). None calls
`Node_SetAnimTimeTree` again (the Plasma's `PlasmaBlast_Update` does, with `age * rate`), so no rate
other than 1 and no reset.

**Measured on PPSSPP** (v1.20.4, software renderer, 2026-10-05, two boots; a conditional breakpoint on
`Mesh_UpdateTextureTransforms` `0x0890e160`, `a0 = mesh`, logging `mesh+0x40`, `mesh+0x194` and
`g_ingame->0x40`; `scripts/psp-weapon-pair.py --probe meshtex`):

| Effect | Spawn clock (= detonation or fire clock) | `+0x40` at first sight | Slope against the clock | Residual |
| --- | --- | --- | --- | --- |
| Bomb hemisphere and shockwave, boot 1, moving craft | 60.889 | `0.000` | `1.0000` over 84 samples | `0.0000` |
| Same, stationary craft | 294.072 | `0.567-0.818` (blast out of view until then) | `1.0000` | `0.0000` |
| Same, boot 2 | 75.125 | `0.567-0.734` | `1.0000` | `0.0000` |
| Repulser field model, boot 1 | 341.338 | `0.000` | `1.0000` over 95 samples | `0.0000` |
| Same, boot 2 | 168.616 | `0.000` | `1.0000` over 95 samples | `0.0000` |

The two falsifiers both came out for age: detonations at race clocks 60.9, 294.1 and 75.1 (mesh time read) each
start from `0`; a third probe read the blast object's own age (`+0xd0`), which starts at `0.0` and ticks 0.0166 per frame (clock 157.5), but not a mesh time (a race clock would carry the session's tens of seconds), and the three meshes of one
blast, first seen 0.15 s apart, share one spawn clock equal to the detonation's own, so the time counts
from spawn, not from first sight. The updates stop at 1.535 s (hemisphere hide, 1.55 s) and 1.569 s (the
repulser's `blast_time + wave_time`), once the meshes are not drawn. The wrap period is each block's own
`+0x2c`, applied by `TexAnim_UpdateTransform`'s `fmodf` (`texture-animation.md`), which the renderer's
sampler already does.

Confidence **88** for the Bomb and the Repulser (the ship explosion's shockwave is **80**: the seed is read, nothing was captured of it): both seeds read at the instruction, the update read at the instruction, and five
captures of those two over two boots agree to four decimals. Not 90+ because the update has no Ghidra function, so its
caller and the class slot it sits in were not traced. Unnamed for the same reason (no function to name).

## What is still open

- **`node+0x5c`**, the rate multiplier's numerator. Nothing read writes it, so
  the multiplier never fires here, and the units of `payload+0x30` stay unknown
  with it.
- **`node+0x84`**, the pause flag `AnimTransform_Update` tests. Never seen set.
- **`AnimEnd`** is read into `node+0x58` and nothing was traced reading it back.
  `LoopEnd` is the field that governs playback; this one may be authoring-time
  only.
- **The two alternate evaluators**, `0x088ff6ec` and `0x088fe934`, unreachable
  on this disc.
- **The third `Node_SetAnimTimeTree` class**, `0x08a6bd18`.
- **Who calls `InGame_Update`.** The write site for `g_ingame->0x40` is found
  (above); the driver that calls it once per frame with a `dt`, presumably a
  generic per-frame dispatch through the same class-vtable slot other classes
  may also populate, is not traced.
- **The third reader at `0890cefc`/`0890cf4c`**, found chasing the advancer
  above, has no Ghidra function covering it and is not named.
