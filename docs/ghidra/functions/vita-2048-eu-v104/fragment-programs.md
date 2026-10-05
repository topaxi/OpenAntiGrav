# Where 2048 creates fragment programs, and the one blend table found (2026-10-05, `omega-2048-materials`)

Static read of `/2048/eboot-vita-2048-eu-v104.elf` (ARM Thumb-2). The question: does the executable
carry a per-material blend equation, the thing Wipeout 2048's `.rcsmodel` header lacks (the state word at
`+0x12` carries the mode in its low two bits and no factor pair)? On the Vita the blend is not stored in the
`.gxp`: it is a `SceGxmBlendInfo` handed to `sceGxmShaderPatcherCreateFragmentProgram` at runtime.

## `0x812f6bee`, the only caller of `sceGxmShaderPatcherCreateFragmentProgram`

The import is NID `0x4ED2E49D` (`SceGxm_4ED2E49D`, thunk `0x813f2390`). It has two references: a data
reference at `0x813f5a10` (the import table) and **one call, at `0x812f6c9e`, inside `FUN_812f6bee`**.

```text
FUN_812f6bee(program_id, msaa_mode, sample_count, blend_info*)
  look the program up by id in a 256-bucket table (DAT_81981208)
  key = hash(msaa_mode, sample_count) then hash(.., 4 bytes of blend_info, default DAT_8198162c = zeros)
  walk the program's cache list; on a hit return the cached fragment program
  on a miss: SceGxm_4ED2E49D(patcher, program, msaa, count, blend_info or NULL, 0, &out), cache it
```

So it is a cache keyed on the blend info's four bytes, and `blend_info` is the caller's: NULL means no
blending (the all-zero default). Confidence 80 (single import caller, the key and the cache are direct
reads). Name: `Gxm_CreateFragmentProgramCached`.

## Its 22 callers are not the model-material pass

`get_function_callers` finds 22. Every one builds a fixed set of programs at start-up for its own
class: a sprite or UI batcher (`in_position`/`in_colour`/`in_uv`, `kColourScale`, `kDiffuseTexture`,
`kDispersion`), post-process passes, debug primitives (`FUN_812f7dcc`, which draws with `sceGxmDraw`
directly), the ten-variant families at `0x810733dc` and `0x812f1b08` (`msaa` 1 or 5, three sample
counts, with and without a local blend info). None passes a blend info read out of a material or
out of a state word. The same finding the earlier mesh-path reads reached from the draw side
([2048-rcsmodel.md](../../../formats/2048-rcsmodel.md): all eleven `sceGxmDraw` callers are not the
mesh path): **the model-material pass was not located**, so no per-material equation was recovered.

## One blend table that is readable: `FUN_81289bbc`

`FUN_81289bbc` builds five fragment programs from one program (`+0x10/+0x14` to `+0x30/+0x34`): no
blend, then four blend infos. It belongs to the sprite batcher built by `FUN_81289d0e` (caller
`FUN_812a60a0`), not to models. Its four-byte blend infos, read as `SceGxmBlendInfo` (`colorMask`,
then bit-fields `colorFunc:4 alphaFunc:4`, `colorSrc:4 colorDst:4`, `alphaSrc:4 alphaDst:4`, low
nibble first - the vitasdk header's layout, not read off this binary):

| variant | bytes | reads as |
| --- | --- | --- |
| 1 | `0f 11 54 10` | add, `SRC_ALPHA` / `ONE_MINUS_SRC_ALPHA` (alpha-over), alpha `ZERO` / `ONE` |
| 2 | `0f 11 14 10` | add, `SRC_ALPHA` / `ONE` (additive) |
| 3 | `0f 11 54 10` | the same as variant 1 |
| 4 | `0f 33 14 10` | reverse-subtract, `SRC_ALPHA` / `ONE` (subtractive) |

Confidence 62: the byte values are direct, the bit-field order is an assumption about the SDK
header. Not renamed (below 70 would be `_q` and it is a UI batcher this lane does not need). It
shows the engine's own vocabulary: alpha-over, `SRC_ALPHA`/`ONE` additive and a subtractive mode, the
same two equations HD authors per material.

| address | name | conf |
| --- | --- | --- |
| `0x812f6bee` | `Gxm_CreateFragmentProgramCached` | 80 |
