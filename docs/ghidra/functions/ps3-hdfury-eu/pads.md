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

## What is still open

**Where `this+0xf0` / `this+0x1b0` go after this function writes them is not
traced this pass.** Neither field is `GpuVertex::colour` itself (that is
baked per-vertex in the `.rcsmodel` chunk, flat `0,0,0` on every pad chunk
measured, per `docs/rendering/pads.md`) - if this cycle reaches the screen at
all, it has to be through a parameter patch (the `patch fslot -> const@slot`
mechanism `docs/ghidra/functions/ps3-hdfury-eu/renderer.md` already
documents) or a per-instance vertex-colour override written after `Mesh_Bind`,
neither of which was searched for this pass. **This is the next concrete
step**, not the vtable diff (done) or the colour-cycle discovery (done): find
what reads `this+0xf0` or `this+0x1b0` on a `WeaponPad_Importer` object and
whether that consumer is a shader constant patch (which would make this the
runtime mechanism the material-record's static `W_Cycle` colour rides on top
of) or dead code that never reaches the renderer (which would make this a
second, independently-established "authored but unwired" gap, the same shape
`docs/rendering/pads.md`'s own emissive-mask finding already is).

The `.bss` global at `0x00aec2c0` itself still has no found writer - a
whole-image literal scan for it as a 4-byte value
(`scripts/ghidra/PadImporterVtableDiff.java`'s `findLiteralConsumers`) finds
exactly one TOC slot referencing it (`0x008b431c`) and five functions that
*read* through that slot (the two `Pad_Importer` constructors, this update
function, and two more in the `0x002e0xxx`/`0x006b5xxx` range) - none of them
write to the sixteen bytes at `0x00aec2c0` itself, only read them. Its
initial (likely zero-filled, `.bss`) value versus whatever non-zero neutral
colour it is expected to hold was not resolved this pass.

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
