# `/psp-pulse-eu/BOOT.BIN` - corroborated names

`psp-pulse-eu` is not a reverse-engineering target of record - `psp-pulse-usa`
keeps that role, see [source-images.md](../../../reverse-engineering/source-images.md#pulse-psp-euchd---wipeout-pulse-psp-euaustralia).
These names are not independently derived: they are carried over from the
already-documented USA function at a matching address, found by cross-binary
fuzzy matching (`find_similar_functions_fuzzy`, threshold 0.8) and checked
individually with `diff_functions` before naming anything.

Every diff below is between the cited USA function and its EU match. In each
case the only differences are `DATA_EXT`/immediate-value operands - the
addresses a rebuilt binary relocates data and literal pools to - not any
change in instruction sequence or control flow. That is the basis for the
confidence figures here: real, but capped below what the USA page itself
claims, because this page's evidence is "this matches a name someone else
verified", not a fresh derivation on this binary.

## Functions

### `Xml_AttributeAsFloat` -> `0x08953274`

USA: `Xml_AttributeAsFloat` at `0x0895379c`, confidence 95, see
[xml-reader.md](../psp-pulse-usa/xml-reader.md). Fuzzy match score 0.9906,
51/51 instructions equal after realigning the one `DATA_EXT` operand.

### `Body_Integrate` -> `0x0884e0bc`

USA: `Body_Integrate` at `0x0884e230`, confidence 88, see
[rigid-body.md](../psp-pulse-usa/rigid-body.md). Fuzzy match score 0.9624,
145/146 instructions equal; the one difference is an immediate operand.

### `strcasecmp` -> `0x08972b34`

USA: `strcasecmp` at `0x089732d8`, confidence 92, see
[xml-reader.md](../psp-pulse-usa/xml-reader.md). Fuzzy match score 0.9093,
44/46 instructions equal; the differences are the literal-pool address the
function loads from.

### `strlen` -> `0x08972cf8`

USA: `strlen` at `0x0897349c`, confidence 95, see
[wad-subsystem.md](../psp-pulse-usa/wad-subsystem.md). Fuzzy match score 1.0,
byte-for-byte identical instruction sequence (a trivial libc routine, so this
is the weakest evidence of the four despite the perfect score - see below).

### `Gu_Fog` -> `0x08811658`

USA: `Gu_Fog` at `0x08811748`, confidence 95, see
[fog.md](../psp-pulse-usa/fog.md). Fuzzy match score 0.81, 28/33 instructions
equal; weaker than the others above, hence the `_q` suffix and the lower
confidence figure in `names.tsv`.

## What did not match

`Wad_HashName`, `Wad_Open`, and `Wad_MountArchive` - all high-confidence,
frequently-checked USA functions - returned zero matches at threshold 0.8.
Not investigated further: the WAD subsystem is large and reference-heavy, so
either the fuzzy hash is more sensitive to relocated literal pools there, or
the EU build's WAD code genuinely diverges more than the functions above.
Worth revisiting with a lower threshold if the WAD subsystem becomes an EU
corroboration target in its own right; out of scope for this sweep.

## Full sweep (2026-08-05)

Ran `bulk_fuzzy_match` across every one of `psp-pulse-usa`'s 10,703 functions
at threshold 0.95 (22 paginated calls, `offset` 0 through 10500), then
cross-referenced the results against every row `psp-pulse-usa/names.tsv`
documents - not a hand-picked sample this time, the full set. Every hit below
was individually re-verified with `diff_functions`; in every case the only
differences are `DATA_EXT`/immediate-operand shifts from the rebuilt binary's
relocated literal pools, never a change in instruction sequence or control
flow.

**Deliberately excluded, not missed**: several USA names collapsed onto the
*same* EU target as other, unrelated USA functions (`Xml_AttributeNameIs`,
`Xml_AttributeValueIs` and `Xml_ElementNameIs` all matched `0x08953204`;
`Xml_GetAttributeName` matched `0x0881808c`, a target dozens of unrelated
tiny stubs also matched). A many-to-one match is the signature of a
collision-prone, too-generic function, not a real correspondence - none of
those four were applied.

| USA function (confidence) | EU address | Fuzzy score |
| --- | --- | --- |
| `Collision_GetContact` (90) | `0x08815b2c` | 1.0 |
| `CollisionMesh_AvgVertexScalar` (84) | `0x0881826c` | 1.0 |
| `Collider_SetBoxDimensions` (90) | `0x08818864` | 1.0 |
| `Collider_SetBoxTransform` (90) | `0x08818874` | 1.0 |
| `Collider_BoxSamplePoints` (90) | `0x08818910` | 1.0 |
| `Collision_SegmentTriangle` (90) | `0x08818aec` | 1.0 |
| `Collision_SegmentHitsTriangle` (88) | `0x08818c68` | 1.0 |
| `SapAxis_Update` (82) | `0x08833b94` | 1.0 |
| `Ship_ThrustInput` (55, `_q`) | `0x0883d700` | 1.0 |
| `Ship_UpdateSteering` (84) | `0x08848614` | 0.975 |
| `Ship_UpdateBrakes` (82) | `0x08848864` | 1.0 |
| `Ship_ApplyLateralGrip` (80) | `0x08848a04` | 0.9686 |
| `Ship_ApplyWeathervaneTorque` (74) | `0x08848c50` | 1.0 |
| `Ship_ApplyQuadraticDrag` (80) | `0x08848cb4` | 1.0 |
| `Ship_ApplyAngularDamping` (80) | `0x08848d5c` | 1.0 |
| `Ship_ApplyRollingResistance` (76) | `0x08848dd8` | 1.0 |
| `Ship_UpdateEngine` (84) | `0x0884c454` | 0.9627 |
| `Body_AddForceWorld` (82) | `0x0884d354` | 1.0 |
| `Body_AddTorqueLocal` (74) | `0x0884d448` | 1.0 |
| `Body_AddTorqueWorld` (74) | `0x0884d490` | 1.0 |
| `Body_SetPosition` (90) | `0x0884d6cc` | 1.0 |
| `Body_SetMass` (88) | `0x0884d6dc` | 1.0 |
| `Body_Translate` (92) | `0x0884d884` | 1.0 |
| `Ship_SetColliderFriction` (85) | `0x0884d908` | 1.0 |
| `Body_RecordContact` (85) | `0x0884da80` | 1.0 |
| `Body_SetBoxDimensions` (80) | `0x0884db58` | 1.0 |
| `Camera_SetMode` (82) | `0x08880580` | 1.0 |
| `Pad_ContainsPoint` (85) | `0x08886518` | 1.0 |
| `ParticleSystem_DeriveScaledParams` (90) | `0x088f4290` | 1.0 |
| `Exhaust_UpdateEngineSound` (80) | `0x08904674` | 0.9518 |
| `FogCube_Sample` (92) | `0x08906330` | 1.0 |
| `Gfx_Enqueue` (65, `_q`) | `0x0891de38` | 1.0 |
| `Trail_PushPoint` (85) | `0x08929434` | 1.0 |
| `Wad_Close` (85) | `0x08940f60` | 1.0 |
| `Lzss_InitFromFile` (80) | `0x0894143c` | 1.0 |
| `Lzss_InitFromMemory` (85) | `0x08941478` | 1.0 |
| `Input_IsPressed` (92) | `0x0894ec78` | 1.0 |
| `Input_ConsumePress` (70) | `0x0894ecd8` | 1.0 |
| `Xml_InitElement` (85) | `0x089534ec` | 1.0 |
| `Xml_BeginAttributes` (90) | `0x08953598` | 1.0 |
| `Xml_SetElementRange` (82) | `0x089537a8` | 1.0 |
| `Xml_CloseDocument` (78) | `0x08953a98` | 1.0 |
| `strrchr` (90) | `0x08972fcc` | 1.0 |
| `Math_TransformVec4` (90) | `0x08985854` | 1.0 |

`Ship_ThrustInput` and `Gfx_Enqueue` are the two rows above with confidence
in parentheses marked `_q`: both matched their USA counterpart at a
confidence already below 70 there, so the corroboration keeps the same
qualifier rather than inflating it.

## Lower-threshold sweep (2026-08-05, second pass)

The full sweep above used `bulk_fuzzy_match` at threshold 0.95, chosen to be
conservative. That threshold is too conservative for this pair specifically:
`psp-pulse-eu` and `psp-pulse-usa` are the same build, region-shifted, so code
doesn't diverge the way it does between Pulse and Pure - only literal pools
and relocated data addresses move. A bigger function accumulates more
relocated-address diff lines and drops below 0.95 even when it is otherwise
100% the same logic.

Reran `bulk_fuzzy_match` at threshold 0.8 across all 10,703 `psp-pulse-usa`
functions (11 paginated calls, `offset` 0 through 10000, fetched directly
over the bridge's Unix socket to keep the ~2,157 raw matches machine-parsable
rather than transcribed by hand). Cross-referenced the results against every
row `psp-pulse-usa/names.tsv` documents - the full 304-row set, not a sample.

**Drift check applied before anything else**: for each fuzzy match whose
`source_name` isn't a bare `FUN_`, the sweep only kept it if that name's
*address* in the raw match also matched the address `psp-pulse-usa/names.tsv`
records for that exact name. One stray hit turned up: a live-database
function at `0x0884de5c` is named `Body_Integrate` in the open Ghidra project,
but `names.tsv` documents `Body_Integrate` at `0x0884e230` and calls
`0x0884de5c` by a different name (`Body_Init`, confidence 85, see
[rigid-body.md](../psp-pulse-usa/rigid-body.md)). That's undocumented drift in
the live `psp-pulse-usa` database, not a real correspondence - the address
mismatch excluded it automatically, and it should be recorded in
`HANDOVER.md` as a drift issue for `psp-pulse-usa` (out of scope to fix here).

**Collision check**: for every surviving candidate's EU target address, the
full ~2,157-row match set was checked for any *other* source function (named
or not) also landing on that same target. Any collision was dropped, per the
signature already established above (`Xml_AttributeNameIs`, `Xml_GetAttributeName`,
etc.) - a many-to-one match means the target is generic/collision-prone, not a
real one-to-one correspondence. No collisions survived into the table below;
they were filtered before individual verification started.

32 candidates survived both filters and were each individually re-verified
with `diff_functions`. Every one comes back with the only differences being
`DATA_EXT`/relocated-immediate operands or a called function's relocated
address - never an added/removed instruction, never a changed control-flow
edge. Several show a repeating pattern worth naming explicitly, since it
explains both the low fuzzy score and why it's still safe to apply:

- **GE state-cache literal pool** (`Gu_ColorMaterial`, `Gu_TexScale`,
  `Gu_TexWrap`, `Gu_Ambient`, `Gu_StencilFunc`, `Gu_StencilOp`, `Gu_DepthFunc`,
  `Gu_DepthMask`, `Gu_BlendFunc`): each is a tiny (9-22 instruction) function
  that loads a fixed base pointer via `lui rX,IMM:2222` in USA vs.
  `lui rX,IMM:0` in EU, then indexes off it. Same three-instruction swap in
  every one of them - a single relocated data segment, not nine independent
  edits. This is the exact same pattern the first sweep already saw and
  accepted for `Gu_Fog` (confidence 68, `_q`); these nine get the same
  treatment for the same reason: real but weak evidence, so `_q` and a
  confidence in the low-to-mid 60s.
- **Dispatch/jump-table base pointer** (`Vex_FindClassDescriptor`,
  `Input_ParseButtonName`): larger functions (212 and 216 instructions) built
  from dozens of near-identical `lui rX,IMM:2230` / `addiu rX,rX,offset`
  pairs indexing a table - each pair relocates by the same offset. High
  `body_added`/`body_removed` counts here are an artefact of table size, not
  logic divergence; still `_q`'d because the raw fuzzy score and body-equal
  ratio are both in the weak range this page treats conservatively.
- **Callee-only difference, 100% instruction-identical body**
  (`Collision_BoxAgainstMesh`, `Collision_RaycastMesh`, `Trail_Update`,
  `Wad_DecompressLzssMem`, `Wad_DecompressLzssStream`, `Xml_GetRootElement`,
  `Xml_NextElement`): `body_equal` equals the full instruction count with
  zero added/removed lines in every one of these - the low ~0.8 fuzzy score
  is driven entirely by the callee inside the function having moved to a
  different EU address (visible only in `calls_only_in_a`/`calls_only_in_b`,
  not in the body diff). Confidence set a little below the "clean 1.0-score"
  tier above to reflect the lower raw score, but not `_q`'d given the
  instruction sequence is verbatim identical.

Everything else in the table is a straightforward relocated-literal-pool
match at a middling-to-high fuzzy score, same as the majority of the first
sweep.

| USA function (USA confidence) | EU confidence | EU address | Fuzzy score | Body-equal ratio |
| --- | --- | --- | --- | --- |
| `Body_AddForceAtPoint` (88) | 85 | `0x0884d39c` | 1.0 | 43/43 |
| `Body_SyncBoxCollider` (85) | 85 | `0x0884d9ac` | 1.0 | 53/53 |
| `Collision_AddContact` (86) | 85 | `0x08816774` | 1.0 | 146/146 |
| `World_SetBodyMass` (88) | 85 | `0x0884e73c` | 1.0 | 20/20 |
| `Ship_UpdateAirbrakes` (88) | 85 | `0x0884c830` | 0.9906 | 435/436 |
| `Ship_UpdateMagLock` (88) | 85 | `0x0884b898` | 0.9766 | 748/751 |
| `Fog_FindVolume` (88) | 83 | `0x088875c0` | 0.9604 | 153/156 |
| `Body_ResolveContact` (88) | 82 | `0x0884e7f4` | 0.8308 | 358/360 |
| `Options_LoadDefaultControlMapping` (88) | 79 | `0x088365f8` | 0.9157 | 41/43 |
| `Sap_Update` (82) | 78 | `0x088300ac` | 0.8967 | 50/52 |
| `Ship_UpdateStartBoost` (85) | 77 | `0x0883fc9c` | 0.9056 | 86/94 |
| `Ship_Shield` (78) | 73 | `0x0883e53c` | 0.9217 | 24/26 |
| `Xml_GetRootElement` (90) | 80 | `0x08953a50` | 0.8 | 18/18 |
| `Xml_NextElement` (92) | 80 | `0x08953b7c` | 0.8 | 229/229 |
| `Collision_BoxAgainstMesh` (88) | 80 | `0x08815be4` | 0.8 | 486/486 |
| `Collision_RaycastMesh` (88) | 80 | `0x0881637c` | 0.8 | 254/254 |
| `Wad_DecompressLzssMem` (85) | 78 | `0x089409d8` | 0.8 | 19/19 |
| `Wad_DecompressLzssStream` (85) | 78 | `0x0894098c` | 0.8 | 19/19 |
| `Trail_Update` (80) | 78 | `0x08929f2c` | 0.8 | 78/78 |
| `Gu_BlendFunc` (90, `_q`) | 66 | `0x0881188c` | 0.8329 | 19/22 |
| `Vex_ResolveClassId` (75, already `_q` on USA) | 65 | `0x08936d8c` | 0.8838 | 24/30 |
| `Gu_TexScale` (88, `_q`) | 65 | `0x088109c4` | 0.826 | 14/17 |
| `Gu_Ambient` (90, `_q`) | 65 | `0x0881116c` | 0.826 | 13/16 |
| `Input_ParseButtonName` (88, `_q`) | 65 | `0x0894ed4c` | 0.8425 | 164/216 |
| `Vex_FindClassDescriptor` (82, `_q`) | 64 | `0x089084e8` | 0.8404 | 160/212 |
| `Gu_StencilFunc` (88, `_q`) | 64 | `0x08811824` | 0.826 | 10/13 |
| `Gu_StencilOp` (88, `_q`) | 64 | `0x08811858` | 0.826 | 10/13 |
| `Options_ButtonForAction` (90, `_q`) | 67 | `0x08836588` | 0.8665 | 9/11 |
| `Gu_TexWrap` (88, `_q`) | 63 | `0x08811394` | 0.82 | 8/11 |
| `Gu_ColorMaterial` (85, `_q`) | 63 | `0x08810970` | 0.82 | 7/10 |
| `Gu_DepthFunc` (85, `_q`) | 61 | `0x08811760` | 0.81 | 6/9 |
| `Gu_DepthMask` (85, `_q`) | 61 | `0x08811784` | 0.81 | 6/9 |

Twelve of the thirteen rows marked `_q` above (`Gu_ColorMaterial`,
`Gu_TexScale`, `Gu_TexWrap`, `Gu_Ambient`, `Gu_StencilFunc`, `Gu_StencilOp`,
`Gu_DepthFunc`, `Gu_DepthMask`, `Gu_BlendFunc`, `Options_ButtonForAction`,
`Input_ParseButtonName`, `Vex_FindClassDescriptor`) land below 70 confidence
on this page purely from the weaker match evidence (low fuzzy score and/or
body-equal ratio), even though their USA sources sit at 82 or above - the
same reasoning `Gu_Fog` set precedent for in the first sweep. `Vex_ResolveClassId`
is the thirteenth: its USA confidence (75) is already above 70 there, but the
EU match evidence here is weak enough (0.8838 score, 80% body-equal on a
30-instruction function) to earn its own `_q` independently.

Total documented for `psp-pulse-eu` after this pass: 81 (49 before, 32 added
here). `psp-pulse-usa/names.tsv` documents 304.
