# HD's `Pad_Importer` family: the vtable diff, and the weapon pad's own colour cycle

2026-09-17. Answers the HD pads handover thread's Next Steps item 2 ("diff the
`Pad_Importer` vtables"): HD keeps Pulse's whole runtime colour-cycle
mechanism for a weapon pad, reimplemented rather than dropped. See
[`docs/rendering/pads.md`](../../../rendering/pads.md) for the asset-side
half (the material's own authored colour, settled separately by commit
`91071e60`) and psp-pulse-usa's
[`pads.md`](../psp-pulse-usa/pads.md) for `WeaponPad_UpdateRefreshTimer`'s
original, this page's counterpart.

## The vtable diff, done mechanically rather than assumed

`WeaponPad_Importer`'s object takes its vtable, `WeaponPad_Importer_Vtable`
(`0x0086a730`), from there; `SpeedupPad_Importer`'s vtable,
`SpeedupPad_Importer_Vtable` (`0x0086a780`), sits right after it - both confirmed as real, distinct
17-slot tables by reading raw words `0x0086a700..0x0086a800`: each table is
17 pointers followed by three zero words (Ghidra's own auto-generated
`PTR_PTR_0086a730` / `PTR_PTR_0086a780` symbols land exactly on each table's
first slot, and a third table ends at `0x0086a72c` with the same
three-zero-word tail, so the 17+3 shape is structural, not a guess). Every
slot value is an OPD (function descriptor) address, not a code address
directly - resolved through the `.opd.FUN_<hex>`-named symbol Ghidra's own
importer already places on it, per `docs/reverse-engineering/toolchain.md`'s
warning about PS3's two-level function pointers.

**The earlier "shares fourteen, differs at slots 0, 3 and 5" claim in the
handover thread does not match a direct read of all 17 slots, and is
corrected here rather than repeated.** The actual diff, resolved to function
addresses:

| Slot | `WeaponPad_Importer` | `SpeedupPad_Importer` | Differs |
| --- | --- | --- | --- |
| 0 | `0x002bf238` | `0x002e0648` | yes |
| 1 | `0x00323518` | `0x00323518` | |
| 2 | `0x00323520` | `0x00323520` | |
| 3 | `0x002e02b8` | `0x002dff40` | yes |
| 4 | `0x00327040` | `0x00327040` | |
| 5 | `0x002be9c0` | `0x002e0020` | yes |
| 6 | `0x00327228` | `0x00327228` | |
| 7 | `0x00323528` | `0x00323528` | |
| 8 | `0x00323530` | `0x00323530` | |
| 9 | `0x006af998` | `0x006af998` | |
| 10 | `0x003237e8` | `0x003237e8` | |
| 11 | `0x00323540` | `0x00323540` | |
| 12 | `0x006b57d8` | `0x006b59f8` | yes |
| 13 | `0x006b5818` | `0x006b5f78` | yes |
| 14 | `0x006b5870` | `0x006b5fd8` | yes |
| 15 | `0x006b4dd8` | `0x002e0590` | yes |
| 16 | `0x002be978` | `0x006af9a0` | yes |

Nine of seventeen match exactly (base-class behaviour, shared vtable
targets); eight differ, not three. Slots 12-16 all decompile as C++
destructor/clone boilerplate (paired "set vtable, call base dtor" shapes at
12-13, and a "clone into a freshly allocated 0x200/0x140-byte object" shape at
14) - structurally uninteresting for gameplay and not investigated further
this pass. Slot 0 and slot 5 both differ in a way that looks meaningful (very
different decompiled bodies, not just a destructor variant) but their role is
not identified this pass either - recorded here so nobody re-diffs the table
from scratch, not as a closed question.

**Confidence 90** for the table itself (direct memory reads plus Ghidra's own
OPD symbol resolution, reproducible from `scripts/ghidra/PadImporterVtableDiff2.java`);
the per-slot role identifications below are scored individually.

## `WeaponPad_UpdateRefreshTimer` (`0x002e02b8`), confidence 85

Slot 3 is the per-frame update - the decompiled signature takes a `double`
(the frame delta, passed in a floating register) and an `int` (the pad
object, `this`) exactly the way `Pad_Importer`'s per-frame callback is called
elsewhere in this codebase. Read in full
(`scripts/ghidra/PadImporterDecompile.java`), it is PSP's
`WeaponPad_UpdateRefreshTimer` (`0x0892c034`, `psp-pulse-usa/pads.md`)
reimplemented, not dropped:

- A refresh timer at `this+0x160` counts down by the frame delta each tick,
  floored at a constant (`DAT_008b42f8`).
- **While cooling down** (timer above the floor): packs `0xff3f3f3f` - alpha
  `0xff`, RGB `0x3f3f3f` - into `this+0xf0`, the *exact* flat grey PSP's own
  version packs into `pad+0x6c` while cooling down. It also copies a 16-byte
  vector from the address held at TOC slot `0x008b431c` (value `0x00aec2c0`,
  the same `.bss` global the handover thread already named as
  `WeaponPad_Importer`'s `this+0x1b0` target with no found writer) into
  `this+0x1b0`.
- **Once the timer reaches the floor**: cross-fades through a keyframe table
  (`PTR_DAT_008b4320`, 12 bytes per entry) indexed `0..5` (`if (iVar9 != 5)
  iVar15 = iVar9 + 1`, a 6-entry wraparound) - the same size as PSP's own
  6-entry table at `0x08ac00c8`. The three interpolated components are
  packed into `this+0xf0` as a byte-per-channel RGBA word (`0xff000000 |
  b<<0x10 | g | r<<8`, alpha forced opaque) **and** written as a float
  vector to `this+0x1b0` via `vectorMultiplyAddFloatingPoint` - the same two
  fields the cooling-down branch above writes, so `this+0xf0` and
  `this+0x1b0` are two encodings (packed bytes, float vector) of the *same*
  cycling colour, not a colour plus something spatial.

**This reopens two claims the handover thread and `docs/rendering/pads.md`
both currently state as closed, and both need correcting there, not just
here:**

1. "Pulse's `Weapon Pad` cycle... stays exactly as recovered on PSP/PS2 and
   no longer runs on HD" is wrong for the *mechanism* - HD's own
   `WeaponPad_Importer` runs a structurally identical grey-while-cooling /
   keyframe-cross-fade-when-armed cycle, at the same magic grey constant
   (`0x3f3f3f`) and the same table size (6 entries). What is still true is
   narrower: neither this function nor anything else found so far writes
   into a `.rcsmaterial` parameter or a shader constant slot - `this+0xf0`
   and `this+0x1b0` are fields on the pad's own C++ object, not a material
   record.
2. "Do not add a cooldown grey by analogy with Pulse - the two titles' pads
   are already established to differ in mechanism" is the wrong prior: they
   do not differ in mechanism, they differ only in *what texture that
   mechanism was ever meant to recolour* (PSP's `weapon_under.tga` is neutral
   by design for exactly this; HD's `ds_weaponup_cs.gtf` is already painted).
3. `WeaponPad_Importer`'s constructor storing a 128-bit vector at `this+0x1b0`
   (already established, loaded through the same TOC slot `0x008b431c`) is
   **not** "the same offset PSP's `Pad_Bind` writes its box minimum to" in
   the sense of holding spatial data on HD - it is the colour-cycle vector,
   initialised to the cooling-down neutral value at construction time.

`SpeedupPad_Importer`'s own slot 3 (`0x002dff40`) is a different, unrelated
one-argument accessor (no frame-delta parameter at all, returns a stored byte
flag) - confirming, independently of the material-record finding, that a
speed pad's own per-frame update carries no colour logic of any kind.

## Answered 2026-10-08 (`hd-weapon-pads`): the cycle reaches the light bars' constant

**The red the maintainer sees on Vineta K is this function's cycle.** On RPCS3,
one RSX frame of `01_vineta_k` (`data/scratch/hd-weapon-pads/cap1`, craft at
`-804.2,-144.0,262.5`), the weapon-pad fragment programs' inline constant at
code slot `0x3a` (`MAD H2.xyz, R0.wwww, C, H0`, the `_ne`-alpha-gated term)
reads `{1.16612, 0.0326835, 0, 1}` on one pad (draws 108 and, in the
alpha-tested pass, 233) and `{1.65581, 0.0407428, 0, 1}` on another (draw 109);
the speed pads on the same frame (draws 110, 144, 145) read the material's own
authored `{0, 0.768628, 0.992157, 0}`. The weapon pad's material authors that
same cyan for `W_Cycle`, so **the engine overwrites the constant per pad, per
frame**, and `docs/rendering/pads.md`'s "red on four circuits" table was a
reading of the authored value only.

The numbers are this page's function and nothing else (confidence **88**: two
pads fit one law to 0.01 %, and the static initialiser's constants were also
read live):

- the pad's `this+0x1c0` is a keyframe index, `this+0x1c4` a position in
  `[0, 1)` advancing by `dt * DAT_008b4324` = **3.0** per second while the pad is
  ready; the colour is `lerp(key[i], key[i+1], t)` over **six keyframes at
  `0x008c26a0`** (12 bytes each, floats): `(512, 16, 0)`, `(256, 0, 0)`,
  `(64, 0, 0)`, then the same three again, wrapping;
- the blend is multiplied by the vector at **`0x00aec2d0`**, which the static
  initialiser `FUN_002e0450` writes as `(DAT_008b4330, DAT_008b4330,
  DAT_008b4330, 1.0)` with `DAT_008b4330 = 0x3b808081 = 1/255` (read live:
  `0.003921569`), and the result lands in `this+0x1b0`;
- while the pad cools down, nothing advances and `this+0x1b0` is the vector at
  **`0x00aec2c0`**, which the same initialiser writes as `(DAT_008b4334,
  0, DAT_008b4338, 1.0)` = **`(0.025, 0, 0.01, 1)`** (read live on RPCS3:
  exactly that), a near-black dull red. The packed `0xff3f3f3f` grey in
  `this+0xf0` is a second encoding of the cooldown and the cycle colour that
  nothing in the capture shows reaching a bar.

So a ready pad's bars run a 1 s loop (the table repeats after three keys, at
three keys a second) of red `2.008, 0.063, 0`, `1.004, 0, 0`, `0.251, 0, 0`,
through an unclamped half-float register. Fit of the two constants: green alone
fixes the position (`2.5208` and `0.35071` keys), and red, the third lane and
the fourth then agree.

**A pad's starting key is its heap address modulo six.** The constructor
`FUN_002e0168` stores `this - 6 * (this / 6)` in `this+0x1c0` (the
`mulhwu`/`rlwinm` pair at `0x002e01dc`-`0x002e01f0`) and zero in `this+0x1c4`
and `this+0x160`. The two pads of the capture sit in different spans of the
table for this reason. Nothing in this project has the original's addresses, so
our start key (pad index modulo six) is **chosen, not measured**.

Names recovered here (`names.tsv`):

| Address | Name | Confidence | What |
| --- | --- | --- | --- |
| `0x008c26a0` | `WeaponPad_CycleKeyframes` | 88 | the six 12-byte keyframes |
| `0x00aec2c0` | `WeaponPad_CoolingGlowVector` | 90 | `(0.025, 0, 0.01, 1)`, read live |
| `0x00aec2d0` | `WeaponPad_GlowScaleVector` | 90 | `(1/255, 1/255, 1/255, 1)`, read live |
| `0x002e0450` | `WeaponPad_InitStatics` | 75 | the static initialiser that writes both vectors (and the `WeaponPad_Importer` vtable stores); the decompiled `FUN_002e0450` body |

**What is not read**: the code that carries `this+0x1b0` into the program's
patched constant (the material parameter `W_Cycle`, `0xce5c4410`). It is not
found by xref (no `lis 0xce5c` immediate anywhere in the image); the claim
that `this+0x1b0` is the source rests on the two constants matching the
function's own arithmetic, not on a traced copy. Whether the cooling vector
reaches a bar as the program's constant was not captured (no pad was caught
cooling).

## Reproducing this

```sh
GHIDRA_INSTALL_DIR=/opt/ghidra /opt/ghidra/support/analyzeHeadless \
  <path-to-checkout> OpenAntiGrav/ps3-hdfury-eu -process EBOOT.elf -noanalysis \
  -scriptPath scripts/ghidra \
  -postScript PadImporterVtableDiff2.java /tmp/vtable-diff.txt
```

`PadImporterVtableDiff.java` (the literal-scan pass), `PadImporterVtableRaw.java`
(the raw-word dump that found the 17+3 table shape) and
`PadImporterDecompile.java` (batch-decompiles every differing slot for both
classes) are the other three read-only scripts this page's findings come
from, all under `scripts/ghidra/`.
