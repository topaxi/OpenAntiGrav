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

## Deep sweep (2026-08-05, third pass)

The two sweeps above (threshold 0.95, then 0.8) had already found everything a
plain threshold-based fuzzy match was going to find cheaply. After those, 81
of `psp-pulse-usa/names.tsv`'s 304 documented functions had an EU counterpart;
223 did not. This pass targets that remainder using a lower fuzzy threshold
plus verification techniques beyond a single score, per the task that opened
it.

**Method**: `bulk_fuzzy_match` at threshold 0.6 across all 10,703
`psp-pulse-usa` functions (11 paginated calls, `offset` 0 through 10000).
Cross-referenced the ~8,181 raw matches against the 223-row gap (198
functions, 25 data items - data was out of scope for this pass, see below).
146 of the 198 gap functions had a fuzzy candidate at threshold >= 0.6; 52 had
none at any threshold down to 0.6 and are left undocumented (candidates for a
future structural-reasoning pass, out of scope here since the task's
30-minute budget went to the functions the fuzzy matcher could at least
surface).

**Collision filter applied first, same discipline as the two prior sweeps**:
for every surviving candidate's EU target address, checked whether any
*other* source function (named or not) in the full ~8,181-row match set also
landed on that same target. 13 collided and were dropped without
individual verification, since a many-to-one match is the signature of a
collision-prone, generic function rather than a real correspondence:

| USA function | EU target | Other sources landing on the same target |
| --- | --- | --- |
| `Xml_GetAttributeValue` | `0x088046a0` | 401 other sources (a maximally generic small stub) |
| `Xml_DestroyElement` | `0x08804d28` | 30 other sources |
| `Xml_GetAttributeName` | `0x0881808c` | 17 other sources (same collision `Xml_GetAttributeName` hit in the 0.8 sweep) |
| `Options_ControlSchemeFlag` | `0x08806b80` | 14 other sources |
| `Zone_UpdateState` | `0x08822254` | 7 other sources |
| `atof` | `0x08826414` | 6 other sources |
| `Xml_AttributeNameIs`, `Xml_AttributeValueIs`, `Xml_ElementNameIs` | `0x08953204` | each other (same 3-way collision excluded in the full sweep) |
| `Wad_BuildCrcTable` | `0x0897065c` | 3 other sources |
| `Body_ClearAccumulators` | `0x0884e304` | 1 other source |
| `Options_SetControlScheme` | `0x08856c78` | 1 other source |
| `Pad_Bind` | `0x08925fd0` | 1 other source |

**Every one of the 133 remaining candidates was individually verified with
`diff_functions`** (dispatched across five parallel read-only verification
passes to fit the time budget, each independently checking its slice and
reporting body_equal/body_total plus a reason - none was renamed off score
alone). 132 confirmed clean; **one was rejected**:

- **`MoviePlayer_Shutdown` (USA `0x089145bc`) rejected against its top EU
  candidate `0x089140a8`.** `diff_functions` shows 167 vs 163 instructions
  (`added=8`, `removed=12`) with a genuine 4-instruction conditional block
  (`lui`/`lbu`/`bne` checking a byte at `+0x1d8` before calling a cleanup
  path) present in USA and **absent** in EU - not a relocated-operand
  artifact, an actual removed branch. This is a real logic/control-flow
  difference, so it was not applied. Worth revisiting independently (is the
  EU movie player missing a codec-cleanup step USA has, or is
  `0x089140a8` simply the wrong EU function for this one?) - out of scope to
  resolve here.

**Unlike the two higher-threshold sweeps, a low raw fuzzy score in this pass
usually was not a sign of weak evidence.** Many candidates that scored close
to the 0.6 floor came back from `diff_functions` with `body_equal` ==
`body_total` (fully identical instruction sequences) - the low score was
driven entirely by `calls_only_in_a`/`calls_only_in_b` divergence, i.e. the
callee inside the function resolving to an unnamed `FUN_` in the
still-mostly-undocumented EU binary rather than a real body difference. So
confidence here is set from the diff evidence (the fraction of the
instruction body that's an exact match, and whether every difference is a
`lui`/`addiu` immediate/`DATA_EXT` relocation) rather than the raw fuzzy
score, capped below the USA source's own confidence in every case:

- diff ratio (non-equal instructions / total) <= 2%, or a 100%-equal body
  with only callee-name differences: USA confidence minus 8
- diff ratio 2-8%, all relocation-only: USA confidence minus 11
- diff ratio 8-20%, all relocation-only: USA confidence minus 14
- diff ratio > 20% (one case, `Xml_AttributeAsBool`, a 5x-repeated relocated
  string-compare pattern that inflates the diff-line count without any logic
  change): USA confidence minus 18

19 of the 132 land below 70 and get `_q`. `Ship_UpdateSideshiftInput_q`
is a special case: the USA source itself carries a literal `_q` suffix in
its *name* (confidence 72, `engine.md`) rather than the usual bare-name
convention, so per this task's instructions the EU name carries that suffix
through exactly. Its formula confidence came out to 61 (a second `_q` gets
appended by `apply-ghidra-names.py`'s `< 70` rule, which would have produced
the ugly and misleading `..._q_q`), so it is pinned to exactly 70 - matching
how the USA page itself handles the same tension (72, no additional
suffix) - and the low-confidence signal already lives in the name.

No collisions arose among the 132 targets themselves or against the 81
already-documented in the prior two sweeps (checked directly against
`names.tsv`'s existing address set before applying).

| USA function (USA confidence) | EU confidence | EU address | Fuzzy score | Body-equal ratio |
| --- | --- | --- | --- | --- |
| `Wad_HashName` (97) | 85 | `0x089407e8` | 0.7566 | 101/105 |
| `Wad_Open` (93) | 85 | `0x08940cc4` | 0.6581 | 164/167 |
| `FogCube_Init` (92) | 84 | `0x08906278` | 0.7000 | 46/46 |
| `Xml_FirstChildElement` (90) | 82 | `0x08953528` | 1.0000 | 14/14 |
| `Wad_Read` (90) | 82 | `0x08941f2c` | 0.7429 | 205/205 |
| `AiTrack_LocatePosition` (90) | 82 | `0x0887ccd4` | 0.7000 | 68/68 |
| `atoi` (90) | 82 | `0x08971fa8` | 0.7000 | 8/8 |
| `Fog_Disable` (90) | 82 | `0x0891e490` | 0.7000 | 7/7 |
| `Input_BuildState` (90) | 82 | `0x0894e970` | 0.7000 | 165/165 |
| `Mesh_EmitDrawArray` (90) | 82 | `0x0890bce0` | 0.7000 | 52/52 |
| `Psys_RandFloatRange` (90) | 82 | `0x088f8714` | 0.7000 | 15/15 |
| `Psys_RandIntRange` (90) | 82 | `0x088f85bc` | 0.7000 | 23/23 |
| `Psys_RandSpread` (90) | 82 | `0x088f8750` | 0.7000 | 18/18 |
| `Ship_CastHoverProbes` (90) | 82 | `0x08849d60` | 0.7000 | 481/481 |
| `zlib_inflateInit_` (90) | 82 | `0x089540b4` | 0.7000 | 9/9 |
| `AiTrack_UpdateCursor` (90) | 82 | `0x0887e2c0` | 0.6842 | 461/465 |
| `Texture_SwizzleForGe` (90) | 82 | `0x08926884` | 0.6747 | 112/113 |
| `Vex_LoadModel` (90) | 82 | `0x0891267c` | 0.6654 | 692/704 |
| `Trail_DrawRibbon` (90) | 82 | `0x0892a7a4` | 0.6572 | 400/408 |
| `ParticleSystem_SpawnBurst` (90) | 82 | `0x088f5044` | 0.6532 | 67/68 |
| `Body_SetBoxInertia` (92) | 81 | `0x0884e038` | 0.7967 | 31/33 |
| `Xml_OpenFile` (92) | 81 | `0x089538b4` | 0.7172 | 77/79 |
| `Xml_NextAttribute` (92) | 81 | `0x089535e4` | 0.6827 | 108/111 |
| `Collision_RaycastWorld` (88) | 80 | `0x088169d0` | 0.7600 | 251/251 |
| `StateMachine_TransitionTo` (88) | 80 | `0x08891064` | 0.7200 | 131/131 |
| `Body_ApplyImpulseAtPoint` (88) | 80 | `0x0884d4d8` | 0.7000 | 125/125 |
| `MoviePlayer_DecodeVideoStep` (88) | 80 | `0x08913f78` | 0.7000 | 76/76 |
| `MoviePlayer_ParseHeader` (88) | 80 | `0x08913a8c` | 0.7000 | 85/85 |
| `Wad_ParseResidentDirectory` (88) | 80 | `0x08941350` | 0.7000 | 59/59 |
| `World_SetBodyBoxInertia` (88) | 80 | `0x0884e78c` | 0.7000 | 26/26 |
| `Xml_InitDocument` (88) | 80 | `0x089537b4` | 0.7000 | 24/24 |
| `MoviePlayer_Update` (88) | 80 | `0x08913594` | 0.6760 | 69/70 |
| `CollisionNode_ParseChunks` (88) | 80 | `0x08934524` | 0.6653 | 189/191 |
| `Trail_BakeVertexColours` (88) | 80 | `0x08929768` | 0.6629 | 216/220 |
| `Vfs_SplitDevicePath` (88) | 80 | `0x0893dde4` | 0.6521 | 53/54 |
| `Trail_ApplyPreset` (88) | 80 | `0x0892a200` | 0.6474 | 354/361 |
| `zlib_uncompress` (90) | 79 | `0x08956418` | 0.6500 | 58/60 |
| `Gfx_PresentFrame` (90) | 79 | `0x0891dc94` | 0.6469 | 100/105 |
| `Wad_MountArchive` (90) | 79 | `0x0894194c` | 0.6432 | 359/376 |
| `MoviePlayer_Create` (90) | 79 | `0x08914334` | 0.6381 | 250/264 |
| `ExhaustFlare_Draw` (90) | 79 | `0x089043b0` | 0.6313 | 118/122 |
| `Texture_BindEmbeddedData` (86) | 78 | `0x08927a04` | 0.7000 | 147/147 |
| `HandlingXml_ParseBackwardCamera` (92) | 78 | `0x08838950` | 0.6875 | 66/74 |
| `HandlingXml_ParseBonnetCamera` (92) | 78 | `0x08838828` | 0.6875 | 66/74 |
| `HandlingXml_ParseInternalCamera` (92) | 78 | `0x088386d0` | 0.6871 | 76/86 |
| `HandlingXml_ParseExternalCameraClose` (92) | 78 | `0x08838c3c` | 0.6850 | 99/113 |
| `HandlingXml_ParseExternalCameraFar` (92) | 78 | `0x08838a78` | 0.6850 | 99/113 |
| `Game_UpdateFrame` (92) | 78 | `0x08804978` | 0.6211 | 79/87 |
| `Collision_DispatchPair` (85) | 77 | `0x08816dbc` | 0.7600 | 96/96 |
| `ExhaustFlare_Submit` (85) | 77 | `0x0890428c` | 0.7343 | 72/73 |
| `Collision_StepNarrowphase` (85) | 77 | `0x088158d0` | 0.7273 | 151/151 |
| `Exhaust_Update` (85) | 77 | `0x08905230` | 0.7265 | 501/509 |
| `Body_StepWorld` (85) | 77 | `0x0884f598` | 0.7157 | 743/746 |
| `CollisionMesh_BuildSap` (85) | 77 | `0x08818450` | 0.7000 | 216/216 |
| `Fog_Apply` (85) | 77 | `0x0891e4f0` | 0.7000 | 43/43 |
| `MoviePlayer_DecodeAudioStep` (85) | 77 | `0x08913e50` | 0.7000 | 74/74 |
| `MoviePlayer_IsFinished` (85) | 77 | `0x08913830` | 0.7000 | 24/24 |
| `MoviePlayer_ReadThreadMain` (85) | 77 | `0x08914754` | 0.7000 | 53/53 |
| `ParticleSystem_ApplyBlendClass` (85) | 77 | `0x08916018` | 0.7000 | 53/53 |
| `Vex_RelocateNodeTree` (85) | 77 | `0x0891116c` | 0.7000 | 35/35 |
| `Wad_DecompressZlibMem` (85) | 77 | `0x08940a24` | 0.7000 | 10/10 |
| `Camera_UpdatePlayerView` (88) | 77 | `0x0883bf7c` | 0.6808 | 389/416 |
| `Ship_ApplyCollisionImpulse` (85) | 77 | `0x0883f124` | 0.6679 | 107/108 |
| `Input_ReadAnalog` (88) | 77 | `0x0894e67c` | 0.6660 | 128/131 |
| `Mesh_SetBatchDrawState` (85) | 77 | `0x0890d490` | 0.6630 | 110/112 |
| `ParticleSystem_EmitSphere` (85) | 77 | `0x088fccc0` | 0.6538 | 678/688 |
| `MoviePlayer_Open` (88) | 77 | `0x089133b8` | 0.6485 | 115/119 |
| `Lzss_ReadBits` (88) | 77 | `0x08941760` | 0.6290 | 116/122 |
| `Resource_LoadFile` (88) | 77 | `0x08942e0c` | 0.6229 | 213/221 |
| `Ship_InitCraft` (88) | 77 | `0x088491e0` | 0.6225 | 166/176 |
| `HandlingXml_ParseBrakes` (90) | 76 | `0x088394dc` | 0.7050 | 77/84 |
| `HandlingXml_ParseEngine` (90) | 76 | `0x0883930c` | 0.7009 | 105/116 |
| `HandlingXml_ParseAirbrake` (90) | 76 | `0x088398b4` | 0.6978 | 138/153 |
| `HandlingXml_ParseTurning` (90) | 76 | `0x08839790` | 0.6927 | 66/73 |
| `HandlingXml_ParsePhysical` (90) | 76 | `0x08838e00` | 0.6915 | 80/89 |
| `HandlingXml_ParsePitch` (90) | 76 | `0x0883962c` | 0.6915 | 80/89 |
| `HandlingXml_ParseAntigrav` (90) | 76 | `0x08839128` | 0.6896 | 108/121 |
| `Vex_RegisterClass` (90) | 76 | `0x08908838` | 0.6022 | 32/37 |
| `Xml_AttributeAsBool` (92) | 74 | `0x08953408` | 0.7969 | 27/42 |
| `MoviePlayer_GetCurrentTexture` (82) | 74 | `0x08913984` | 0.7000 | 37/37 |
| `ParticleSystem_Update` (85) | 74 | `0x088f551c` | 0.6858 | 485/496 |
| `ParticleSystem_InitParticle` (85) | 74 | `0x088f67ec` | 0.6847 | 683/713 |
| `ParticleSystem_UpdateParticles` (85) | 74 | `0x088f5cdc` | 0.6707 | 693/708 |
| `Xml_ReadBoostSettings` (88) | 74 | `0x08838f64` | 0.6683 | 92/113 |
| `ParticleSystem_UpdateEmission` (85) | 74 | `0x088f4904` | 0.6574 | 448/464 |
| `ParticleSystem_DrawParticle` (85) | 74 | `0x08918198` | 0.6434 | 326/335 |
| `Lzss_Decode` (85) | 74 | `0x089414b4` | 0.6433 | 164/170 |
| `powf` (85) | 74 | `0x0898568c` | 0.6357 | 54/56 |
| `InGame_Construct` (85) | 74 | `0x08812c68` | 0.6354 | 210/227 |
| `Ship_ApplySpeedupPad` (85) | 74 | `0x08848e28` | 0.6333 | 187/200 |
| `Wad_Unmount` (85) | 74 | `0x08940bb0` | 0.6306 | 65/67 |
| `Handling_ParseStats` (88) | 74 | `0x0883a1a0` | 0.6258 | 375/416 |
| `Mesh_CompileDisplayLists` (84) | 73 | `0x0890f5d4` | 0.6373 | 344/370 |
| `Ship_HoverTwoPoint` (80) | 72 | `0x0884a4e4` | 0.9714 | 521/526 |
| `Ship_HoverFourCorner` (80) | 72 | `0x0884ad1c` | 0.7193 | 730/735 |
| `ParticleSystem_ConeVelocity` (80) | 72 | `0x088fbcfc` | 0.7000 | 69/69 |
| `Trail_Draw` (80) | 72 | `0x0892a064` | 0.7000 | 7/7 |
| `ParticleSystem_DrawStreak` (80) | 72 | `0x089162fc` | 0.6773 | 311/312 |
| `Trail_BuildOffsetTable` (80) | 72 | `0x08929550` | 0.6532 | 98/99 |
| `Ship_UpdateCraft` (82) | 71 | `0x088494a4` | 0.8051 | 529/559 |
| `Xml_ParseIntHexOrDecimal` (82) | 71 | `0x08953128` | 0.7173 | 51/55 |
| `Utility_DialogLoop` (82) | 71 | `0x0893f618` | 0.6609 | 173/181 |
| `Ship_UpdatePitch` (82) | 71 | `0x08848b94` | 0.6443 | 46/47 |
| `ExhaustFlare_Init` (82) | 71 | `0x08904c88` | 0.6431 | 344/362 |
| `MoviePlayer_SoundThreadMain` (82) | 71 | `0x08914ab4` | 0.6379 | 86/89 |
| `Xml_ReadGlobalSettings` (85) | 71 | `0x0883a820` | 0.6340 | 358/437 |
| `Zone_UpdateRacing` (82) | 71 | `0x0882f248` | 0.6306 | 44/47 |
| `Sap_Init` (82) | 71 | `0x0882f7c0` | 0.6282 | 123/128 |
| `Ship_SetShield` (82) | 71 | `0x0883e5a4` | 0.6271 | 71/74 |
| `InGame_Destruct` (85) | 71 | `0x08812ff4` | 0.6074 | 78/88 |
| `Wad_ReadAsyncBegin` (78) | 70 | `0x08941124` | 0.7000 | 81/81 |
| `Race_CreateModeObject` (84) | 70 | `0x08821038` | 0.6208 | 569/619 |
| `Ship_UpdateSideshiftInput_q` (72) | 70 | `0x08846904` | 0.6666 | 714/767 |
| `Ship_DispatchCollisionFx` (80) | 69 | `0x0883dd40` | 0.6767 | 114/117 |
| `Sap_Insert` (80) | 69 | `0x0882f9ec` | 0.6370 | 87/89 |
| `Ship_UpdateHover` (82) | 68 | `0x08848598` | 0.6445 | 28/31 |
| `Ship_AddShield` (82) | 68 | `0x0883dc78` | 0.6250 | 23/26 |
| `Zone_UpdateResults` (82) | 68 | `0x0882f304` | 0.6094 | 57/63 |
| `StateMachine_FindState` (75) | 67 | `0x08890b0c` | 0.7000 | 101/101 |
| `Wad_ReadAsyncWait` (75) | 67 | `0x08941268` | 0.7000 | 58/58 |
| `ParticleSystem_EmitBox` (75) | 67 | `0x088fb9c0` | 0.6496 | 204/207 |
| `ParticleSystem_EmitPoint` (75) | 67 | `0x088fb3b0` | 0.6430 | 173/175 |
| `Gfx_FlushRenderManager` (78) | 67 | `0x0891de9c` | 0.6236 | 163/176 |
| `Body_ClearVelocity` (80) | 66 | `0x0884d8e8` | 0.7900 | 7/8 |
| `HandlingXml_ParseAirbrakeGraphics` (80) | 66 | `0x08839b18` | 0.7050 | 61/67 |
| `RaceMode_SetState` (80) | 66 | `0x0882721c` | 0.6244 | 58/65 |
| `Demo_UpdateAttractMode` (80) | 66 | `0x08818ee8` | 0.6077 | 210/231 |
| `ParticleSystem_OnParticleDeath` (75) | 64 | `0x088f4314` | 0.6225 | 208/217 |
| `RaceMode_PushState` (75) | 64 | `0x088279d4` | 0.6020 | 16/17 |
| `Hud_SetEnergyBar` (75) | 61 | `0x0881a0d8` | 0.6320 | 66/74 |
| `Gfx_BindTexture` (75) | 61 | `0x08927f3c` | 0.6135 | 52/57 |
| `Trail_Init` (75) | 61 | `0x08929d8c` | 0.6108 | 79/86 |

**Data items untouched in this pass**: 25 of the 223 gap rows are `data` kind
(`g_wad_crc_table`, `g_game`, `_ctype_`, and similar globals). `diff_functions`
and `find_similar_functions_fuzzy` are function-scoped tools; corroborating a
data address needs a different technique (matching via a rename-confirmed
function's xrefs into it), which the time budget for this pass did not reach.
Left for a future pass - see `HANDOVER.md`.

**52 gap functions had no fuzzy candidate at all down to threshold 0.6** and
are also left undocumented here - candidates for the structural/call-graph
reasoning approach (callers, callees, address position relative to
already-matched neighbours) the task allows for, not yet attempted.

Total documented for `psp-pulse-eu` after this pass: **213** (81 before, 132
added here). `psp-pulse-usa/names.tsv` documents 304; 91 functions plus 25
data items remain unmatched.
