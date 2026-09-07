# `/psp-pulse-eu/BOOT.BIN` - corroborated names

**Reversed 2026-08-05.** Everything below this notice was written under the
opposite policy - `psp-pulse-eu` treated as pure corroboration, `psp-pulse-usa`
as the sole target of record - and that framing is now **wrong going
forward**. As of this date, **`psp-pulse-eu` is the preferred first target
for new investigation; `psp-pulse-usa` (and `ps2-pulse-eu` where relevant)
are the cross-verification sources.** Rationale: the EU release is the more
complete artefact of the two. Pulse PSP got DLC content only on the EU disc -
the USA release never had any - and Pulse PS2 was a **EU-only release with no
USA disc at all**, so `ps2-pulse-eu` is already this project's sole PS2
reference by necessity; preferring `psp-pulse-eu` on PSP too keeps the two
platforms' primary targets in the same region rather than split. The four sweeps recorded below are kept
verbatim as history, not deleted or rewritten: they document a real, useful
method (cross-binary fuzzy matching plus `diff_functions` verification) that
remains valid for corroborating a name across binaries in *either* direction,
and the 268 names they landed are unaffected - carried-over evidence doesn't
stop being evidence because the preferred search order changed. New pages
under this directory going forward may carry independently-derived findings
at the normal confidence rubric, not just capped carryovers; see
`HANDOVER.md`'s dated correction to the "Final Ghidra project layout" note
for why.

The rest of this page, unmodified below, is the record of the four sweeps
that ran under the old policy.

---

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


## Structural sweep (2026-08-05, fourth pass)

The deep sweep above closed 132 of the 223-function/data gap using
`bulk_fuzzy_match` at threshold 0.6. That left 91: 66 functions with no
usable fuzzy candidate or a rejected/collision-prone one, and 25 `data`-kind
globals the fuzzy/diff tooling doesn't address at all. This pass targets
both, using call-graph and positional corroboration instead of a fuzzy
score, per the task that opened it.

**Reconciling the count first**: of the 66 leftover functions, 52 genuinely
had zero fuzzy candidate at threshold >= 0.6; the other 14 are the
already-accounted-for exclusions from the deep sweep (13 collision-prone,
1 rejected - `MoviePlayer_Shutdown`) and are not reattempted here. This pass
covers the 52, plus the 25 data items.

### Method: functions

For each of the 52, checked `find_similar_functions_fuzzy` at threshold 0.5
first (lower than the deep sweep's 0.6 floor, since this set had already
failed 0.6). Where that still returned nothing or only tied/generic
candidates, used structural reasoning instead:

- **Address adjacency to an already-matched neighbor.** `Game_RenderFrame`
  (USA `0x08804ad4`) immediately follows the already-matched `Game_UpdateFrame`
  (USA `0x08804978` -> EU `0x08804978`, a *zero-offset* match - EU's function
  body occupies the identical address range as USA's). Checking EU
  `0x08804ad4` directly found a real function there, confirmed by
  `diff_functions`. The same held for `Game_Bootstrap`, which precedes
  `Game_MainLoop` in both binaries.
- **Callers'/callees' own diffs surfacing the missing name.** `diff_functions`
  on an already-accepted pair reports `calls_only_in_a`/`calls_only_in_b` for
  every callee that isn't itself named yet in EU - and when the USA side of
  that list is a function from this 52-item gap, the EU side (still a bare
  `FUN_` address) is a strong positional candidate for it, since both lists
  preserve calling order. This is how `Gu_Start` (via `Game_RenderFrame`'s
  diff), `SystemRoot_Create` (via `Game_MainLoop`'s diff), `Gu_Disable`/
  `Gu_Enable` (via `Fog_Disable`/`Fog_Apply`'s diffs from the previous pass),
  `Gu_SetMatrix` (via `ExhaustFlare_Draw`'s diff) and `strtol` (via `atoi`'s
  diff) were found - in every case a 1:1 correspondence between one
  unresolved USA callee and one unresolved EU callee at the same call site.
- **Fixed local offset between two already-matched neighbors, applied to a
  third.** Within a tight subsystem cluster (e.g. `CollisionMesh_BuildSap`'s
  USA-EU address delta applied to the adjacent, still-unmatched
  `CollisionMesh_Init`), the guessed EU address was checked against the
  actual EU function table with `get_function_by_address` before accepting
  it - if it didn't land exactly on a function's entry point, the guess was
  discarded rather than diffed against a truncated/wrong function. This
  found `FogCube_Construct`, `Texture_LoadEngineFlare`, `CollisionMesh_Init`
  and `Texture_LoadEngineNoise`; the same technique also produced a
  candidate for `Loading_ThreadMain`, which `diff_functions` then rejected
  (see below) - landing on a function entry point confirms the address
  arithmetic, not that it's the *right* function.

**Every candidate from every technique still went through `diff_functions`
before being trusted** - corroboration lowers the bar for which candidates
are worth checking, it doesn't replace the check. Two candidates failed it:

- **`Options_LoadControlMapping`** (USA `0x08836a48`) against its lone fuzzy
  candidate (EU `0x08836914`, score 0.516): real divergence, not relocation.
  USA has an extra 3-`jal`-call block with a conditional branch that EU's
  version doesn't have, and a later branch does inline stores in EU where
  USA calls a function; instruction count differs (118 vs 111) with the gap
  concentrated in genuinely different code, not `lui`/`addiu` pairs.
  Rejected.
- **`Loading_ThreadMain`** (USA `0x0890b52c`) against the offset-interpolated
  candidate (EU `0x0890aef0`): 411 vs 489 instructions, `body_added=188`,
  `body_removed=110` - EU carries a large block (roughly 30 extra
  instructions repeated three times, an unrolled variant of a loop USA
  expresses more compactly) that has no USA counterpart. This is a real
  structural difference far beyond anything the relocation pattern produces
  elsewhere in this project, so the candidate is rejected. `Loading_ThreadMain`
  is left undocumented; the true EU counterpart, if findable, needs a
  different technique than local-offset interpolation.

**Six more were excluded before ever reaching `diff_functions`**, because
their own fuzzy candidate list showed the same three-way (or worse) exact
score tie the project has already established as the signature of a
too-generic function, not a real one-to-one correspondence: `Gfx_ViewDepth`
(three candidates tied at 0.938 out of 1094 total matches), `Xml_AttributeAsFloatLibc`
and `Xml_AttributeAsOwnedString` (tied at 0.55 out of 260/573 matches), and
`strtod` (also tied three-way at 0.51, with no already-matched caller to
corroborate through - its only caller, `atof`, is itself one of the deep
sweep's collision exclusions). `Gu_SetMatrix` looked like a fifth case
(three-way tie at 0.51) but was recovered separately through the
callers'-diff technique above, so it is *not* in this excluded set - see
the accepted table below. All other four are left undocumented.

### Confidence: functions

Since this pass's evidence is call-graph/positional corroboration plus a
`diff_functions` check - not a fuzzy score - confidence is set from the
diff's `body_equal`/`body_total` ratio, using a steeper discount than the
fuzzy-based deep sweep to reflect the different (indirect) evidence source,
capped below the USA source's own confidence throughout:

- diff ratio <= 2%: USA confidence minus 11
- diff ratio 2-8%: USA confidence minus 14
- diff ratio 8-20%: USA confidence minus 17
- diff ratio > 20%: USA confidence minus 21

46 candidates were evaluated this way; 45 were renamed (24 of them below 70, getting `_q`) and one, `Race_ResetCraftBoosts_q`, computed below the 50 floor and was left un-renamed (below).

**`Race_ResetCraftBoosts_q` computed to 46 (below the 50-confidence floor)
and is deliberately left un-renamed** despite a clean diff (76/80 body-equal,
all relocation): its USA source itself sits at only 60 confidence (already
`_q`-suffixed there), and this pass's discount for an 8% diff ratio (-14)
pushes any reasonable evidence-based number under 50. Per project rule,
under 50 means don't rename, write the hypothesis down instead - which is
what this is: `0x08827080` in EU is a strong candidate for
`Race_ResetCraftBoosts_q` (USA `0x088271b4`) if anyone wants to revisit it
with independent evidence, but the two sources of uncertainty (a
low-confidence USA identification, corroborated only indirectly) compound
rather than cancel.

| USA function (USA confidence) | EU confidence | EU address | Body-equal ratio |
| --- | --- | --- | --- |
| `Game_MainLoop` (95) | 78 | `0x08807204` | 224/274 |
| `Movie_ParseAttributes` (95) | 78 | `0x088b9d84` | 203/247 |
| `Sap_QueryAabb` (88) | 77 | `0x088303a0` | 146/148 |
| `Body_SetOrientation` (88) | 77 | `0x0884d6f4` | 84/85 |
| `Gu_DrawArray` (92) | 75 | `0x08810da8` | 43/52 |
| `Body_Init` (85) | 74 | `0x0884dce8` | 208/212 |
| `FogCube_RegisterClass` (95) | 74 | `0x089064e0` | 14/23 |
| `Game_Bootstrap` (90) | 73 | `0x08807198` | 22/27 |
| `Gu_CallList` (90) | 73 | `0x088104a8` | 38/45 |
| `Gu_Enable` (90) | 73 | `0x08810cc8` | 20/22 |
| `Gu_Disable` (90) | 73 | `0x08810d20` | 21/23 |
| `Xml_AttributeAsString` (90) | 73 | `0x089533b4` | 9/10 |
| `Game_RenderFrame` (88) | 71 | `0x08804ad4` | 37/44 |
| `Gu_Start` (88) | 71 | `0x0881009c` | 126/154 |
| `Gu_SetMatrix` (88) | 71 | `0x08810d7c` | 10/11 |
| `Gu_TexOffset` (88) | 71 | `0x08811540` | 14/17 |
| `ExhaustFlare_OnSpeedupPad` (88) | 71 | `0x08904890` | 43/49 |
| `Trail_BuildStateList` (85) | 71 | `0x089291d4` | 60/63 |
| `Xml_AttributeAsInt` (88) | 71 | `0x08953350` | 10/11 |
| `Xml_InternString` (85) | 71 | `0x08953aec` | 34/36 |
| `Ship_LoadModel` (87) | 70 | `0x08843108` | 412/469 |
| `ExhaustFlare_BuildDisplayList` (80) | 69 | `0x08904598` | 54/55 |
| `Collision_RegisterNodeClasses` (90) | 69 | `0x08934820` | 29/56 |
| `CollisionMesh_Init` (85) | 68 | `0x088182f8` | 16/18 |
| `World_LoadTrack` (85) | 68 | `0x088835f0` | 1087/1228 |
| `Texture_LoadEngineFlare` (85) | 68 | `0x08904b90` | 55/62 |
| `FogCube_Construct` (85) | 68 | `0x089060a0` | 22/26 |
| `ShipCollisionFx_Trigger` (85) | 68 | `0x08924190` | 264/289 |
| `Texture_LoadEngineNoise` (85) | 68 | `0x0892a080` | 84/96 |
| `Wad_DecompressZlibStream` (85) | 68 | `0x08940a4c` | 66/73 |
| `Xml_AttributeAsIntHex` (85) | 68 | `0x089533dc` | 10/11 |
| `strtol` (85) | 68 | `0x08978afc` | 10/11 |
| `Zone_Create` (84) | 67 | `0x0882edb0` | 187/204 |
| `Zone_Update` (84) | 67 | `0x0882f498` | 139/157 |
| `Ship_LoadHandlingStats` (84) | 67 | `0x088c23fc` | 44/50 |
| `World_CollectNodeLists` (80) | 66 | `0x08887830` | 87/89 |
| `Options_BuildControlSchemeRows` (82) | 65 | `0x0889b290` | 108/131 |
| `InGame_UpdatePauseInput` (85) | 64 | `0x08813154` | 44/57 |
| `ParticleSystem_CacheModeFlags` (80) | 63 | `0x088f474c` | 92/110 |
| `Trail_InitPreset` (80) | 63 | `0x08929b2c` | 46/52 |
| `SystemRoot_Create` (78) | 61 | `0x0894f20c` | 145/167 |
| `ExhaustFlare_Destroy` (75) | 58 | `0x08903ec0` | 33/40 |
| `Pad_SweptTest` (65) | 54 | `0x088866c8` | 162/162 |
| `Pads_TestCraft` (65) | 54 | `0x08886fa0` | 78/78 |
| `ParticleSystem_AimedVelocity` (65) | 54 | `0x088fbe10` | 105/105 |
| `Race_ResetCraftBoosts_q` (60) | 46 | `0x08827080` | 76/80 |

### Method: data

The 25 `data`-kind gap rows are globals (`g_wad_crc_table`, `g_game`,
`_ctype_`, and similar). `diff_functions`/`find_similar_functions_fuzzy`
are function-scoped tools, so these needed a different technique: find the
USA global's referencing functions with `get_xrefs_to`, check whether any of
those functions already has a confirmed EU match, then `decompile_function`
the EU counterpart and read the actual relocated address or offset directly
out of the decompiler output (Ghidra resolves the `lui`/`addiu` pair to a
concrete `DAT_xxxxxxxx`/`_DAT_xxxxxxxx` symbol name in pseudo-C, which
`diff_functions`' raw disassembly view normalizes away as `DATA_EXT`).

**This surfaced a significant, previously-undocumented fact about the EU
binary's memory layout**: EU's `.data`/`.rodata`/BSS segments are *not*
rebased to the `0x088xxxxx`+ range the way the `.text`/code segment is.
`Wad_HashName`'s EU disassembly computes `g_wad_crc_table`'s address as
`lui s1,0x2` / `addiu s1,s1,0x2bac` = `0x00022bac` - compare USA's
`lui s1,0x8b0` / `addiu s1,s1,-0x4004` = `0x08afbffc`, in the normal
high-address data region. Every other data address resolved in this pass
also landed in a low, non-`0x088`-prefixed region (`0x0002xxxx` to
`0x002xxxxx`, one at `0x0008b000`), which is internally consistent across
independently-decompiled functions - not a fluke of one lookup. This makes
sense given `HANDOVER.md`'s existing note that EU's `BOOT.BIN` is genuinely
smaller than USA's (3,844,732 vs 3,854,564 bytes): a smaller `.data`/`.rodata`
region shifts where the loader/ELF places BSS, and unlike the code segment
(which both binaries pin to a fixed base), these sections apparently aren't
pinned the same way. **This is why the earlier fuzzy/diff-based sweeps never
needed to know this**: `diff_functions` normalizes every data reference to
the placeholder `DATA_EXT` regardless of its actual value, so a
function-vs-function comparison never surfaces the underlying address at
all - only `rename_data`/`decompile_function` on the data side exposes it.
Recorded in `HANDOVER.md` as a finding for future data-symbol work on this
binary, not something this pass tries to fully map.

**`rename_data` was tried first and rejected every name** with
`missing_hungarian_prefix` - the bridge polices global-variable names against
a Hungarian-notation convention this project doesn't use (see
`scripts/apply-ghidra-names.py`'s own comment on this exact issue, already
documented from applying `psp-pulse-usa`'s names). `create_label` is the
tool `apply-ghidra-names.py` uses for `data`-kind rows for exactly this
reason, and it accepted every name unchanged; used here directly, then
verified by re-decompiling the referencing function and confirming the
symbolic name appears in place of the `DAT_`/`_DAT_` address.

Ten of the 25 were resolved with enough confidence to apply; the other 15
were not reached (either their referencing functions have a large,
ambiguous fan-out of xrefs that makes picking the right symbol
error-prone without more time, or - `_ctype_` - `get_xrefs_to` found no
references to the USA address at all, and it wasn't investigated further):

- **`g_wad_crc_table`** (EU `0x00022bac`): `Wad_HashName`'s EU disassembly
  computes this address directly (see above); `get_xrefs_to` on it in EU
  independently confirms 3 `DATA` reads inside `Wad_HashName` plus a `WRITE`
  from the still-unnamed CRC-table constructor - the same reference shape
  USA has.
- **`g_vex_class_table`** (EU `0x002adaf0`): appears identically in *two*
  independently-decompiled EU functions, `Vex_RegisterClass`
  (`piVar2 = (int *)&DAT_002adaf0; iVar1 = _DAT_002adaf0;`) and
  `Vex_FindClassDescriptor` (the same linked-list walk). Two-way
  corroboration.
- **`g_wad_device_vtable`** (EU `0x002ccf4c`): the same constant
  (`*(undefined4 *)(param_1 + 0x30) = 0x2ccf4c;`) appears in both
  `Wad_Unmount` and `Wad_MountArchive`'s EU decompiles, matching USA's
  vtable-pointer-store pattern in both functions. Two-way corroboration.
- **`g_sap_clamp_min`/`g_sap_clamp_max`** (EU `0x002ac3d0`/`0x002ac3e0`):
  `Sap_Insert`'s EU decompile clamps a point against a three-float vector at
  `...d0/d4/d8` (min bound) then a second at `...e0` (max bound) - a
  `0x10`-byte gap matching USA's `0x08ab0c50`/`0x08ab0c60` gap exactly.
- **`g_force_fixed_30`/`g_force_fixed_30_hide_hud`/`g_force_fixed_60`** (EU
  `0x002b920c`/`0x002b9218`/`0x002b9219`): `Game_UpdateFrame`'s EU decompile
  has the identical three-flag frame-timestep decision USA's page documents
  - checks `hide_hud` first, then `fixed_60`, falling through to a
  `fixed_30` check later in the same function - matched by logic shape, not
  just address proximity. `hide_hud` and `fixed_60` are adjacent bytes in
  both binaries.
- **`g_engine_flare_texture`** (EU `0x000874b8`): `Texture_LoadEngineFlare`'s
  EU decompile writes the loaded texture handle to this address in the same
  position USA's version writes to `g_engine_flare_texture`; single-function
  evidence, not cross-corroborated.
- **`g_decompress_staging_buffer`** (EU `0x0008b000`): `Lzss_Decode`'s EU
  decompile uses this literal address as a fixed staging buffer three times
  within the function, matching USA's `g_decompress_staging_buffer` usage
  shape; internally consistent across three sites within one function, not
  cross-checked against a second function.

| USA global (USA confidence) | EU confidence | EU address | Evidence |
| --- | --- | --- | --- |
| `g_wad_crc_table` (95) | 78 | `0x00022bac` | Direct address computation + xref count match |
| `g_vex_class_table` (92) | 78 | `0x002adaf0` | Two independently-decompiled referencing functions agree |
| `g_wad_device_vtable` (90) | 76 | `0x002ccf4c` | Two independently-decompiled referencing functions agree |
| `g_sap_clamp_min` (75, `_q`) | 62 | `0x002ac3d0` | Vec3 pattern + exact `0x10`-byte gap to clamp_max, matching USA |
| `g_sap_clamp_max` (75, `_q`) | 62 | `0x002ac3e0` | Same as above |
| `g_force_fixed_30` (88, `_q`) | 68 | `0x002b920c` | Matches USA's three-flag decision logic shape |
| `g_force_fixed_30_hide_hud` (88) | 70 | `0x002b9218` | Matches USA's decision logic; adjacent byte to `g_force_fixed_60` |
| `g_force_fixed_60` (88) | 70 | `0x002b9219` | Matches USA's decision logic; adjacent byte to `g_force_fixed_30_hide_hud` |
| `g_engine_flare_texture` (88, `_q`) | 68 | `0x000874b8` | Single-function write-site match |
| `g_decompress_staging_buffer` (88, `_q`) | 65 | `0x0008b000` | Single-function, 3 internally-consistent usage sites |

### What did not match

`MoviePlayer_Shutdown` (USA `0x089145bc`), rejected in the deep sweep above
against EU `0x089140a8`: confirmed here as a genuine logic divergence, not a
wrong-candidate mismatch worth re-attempting. `diff_functions` shows a real
4-instruction conditional block (`lui`/`lbu`/`bne` testing a byte at
`param+0x1d8` before a cleanup call) present in USA and absent in EU -
`body_added`/`body_removed` (8/12) don't balance the way a pure relocation
diff's always does. A future sweep should not re-run this pair expecting a
different result without new evidence (e.g. a different EU candidate
address entirely).

`Options_LoadControlMapping` and `Loading_ThreadMain` (this pass, above),
and five too-generic exclusions (`Gfx_ViewDepth`, `strtod`,
`Xml_AttributeAsFloatLibc`, `Xml_AttributeAsOwnedString`, plus
`Race_ResetCraftBoosts_q` left un-renamed for falling under the confidence
floor) remain unmatched; so do 15 of the 25 data items, `_ctype_` among them
(`get_xrefs_to` found nothing referencing it in USA, so there was no
referencing-function trail to follow in EU at all).

**Superseded in part, 2026-09-07**: three of this pass's own exclusions were
independently resolved by
[`exact-hash-transfer.md`](exact-hash-transfer.md)'s exact opcode-hash match,
a stronger (uniqueness-on-both-sides, zero-diff-body) criterion than the tied
fuzzy scores that excluded them here - `strtod`, `Xml_AttributeAsFloatLibc`
and `Xml_AttributeAsOwnedString` all now carry applied EU names. A fourth,
`Race_ResetCraftBoosts` (this page's `Race_ResetCraftBoosts_q`, below-floor
and left un-renamed at the time), matched **the exact EU address this page's
own fourth pass already named as its best candidate** (`0x08827080`,
"a strong candidate... if anyone wants to revisit it with independent
evidence") - the exact-hash pass is that independent evidence, arriving at
the same address by an unrelated technique on a since-reimported database,
and it is now applied at confidence 55. `Body_ClearAccumulators` (excluded
above as a 1-collision drop under the fuzzy sweep) and `Xml_AttributeValueIs`
(excluded as part of a 3-way fuzzy tie) were resolved the same way - a fuzzy
collision does not imply a hash collision, since the two techniques fail on
different axes. See `exact-hash-transfer.md` for the method and confidence
rule; none of this page's own tables are edited to match, per this project's
"record contradictions rather than quietly picking a side" rule - the tables
below are left exactly as this pass wrote them, a record of what fuzzy
matching alone could and could not do.

**Also recorded here, a genuine unresolved naming disagreement, not a
database defect**: exact-hash cross-referencing (2026-09-07) found that
`psp-pulse-usa`'s `Mesh_SetBatchLighting` (`0x0890d3ac`, confidence 82,
[`mesh-draw.md`](../psp-pulse-usa/mesh-draw.md)) and `psp-pulse-eu`'s
`Mesh_ApplyMaterialLighting` (`0x0890cea8`, confidence 78,
[`lighting.md`](lighting.md)) are **the same function** - identical
normalized opcode hash, and both pages' independently-decompiled bodies show
the same `Gu_Disable(LIGHTING)`/`Gu_Enable`+four-light-slot-disable+
`Gu_Ambient` branch structure. Neither name was applied by this pass or is
wrong on its own evidence; this is two independent investigations naming the
same function differently, USA's read as a state setter and EU's as what the
state actually accomplishes. Left unreconciled on purpose - picking a winner
needs a judgement call neither page's own evidence settles by itself, and
recording the disagreement is worth more than guessing at it.

Total documented for `psp-pulse-eu` after this pass: **268** (213 before,
45 functions + 10 data items added here). `psp-pulse-usa/names.tsv`
documents 279 functions plus 25 data items (304 rows total); the remaining
gap is 21 functions (`Options_LoadControlMapping`, `Loading_ThreadMain`,
`Race_ResetCraftBoosts_q`, `Gfx_ViewDepth`, `strtod`,
`Xml_AttributeAsFloatLibc`, `Xml_AttributeAsOwnedString`, and the 13
collision-prone plus 1 rejected from the deep sweep) and 15 data items.

## Loading-screen sweep (2026-08-07, fifth pass)

Carries over the 21 rows
[`psp-pulse-usa/loading-screen.md`](../psp-pulse-usa/loading-screen.md)
recovered. 10 functions and 7 data items landed; 2 functions and 3 data items
did not, and are listed with the reason rather than forced.

### The established method was unavailable, and what replaced it

**`bulk_fuzzy_match` could not be used for this pass.** The program at
`/psp-eu/BOOT.BIN` in the shared Ghidra project is a raw re-import: base
address `0x00000000`, **0 functions, 0 symbols**, 77 memory blocks. A
`bulk_fuzzy_match` from the analysed USA program into it returns an empty
match list because there is nothing on the target side to match against, and
`diff_functions` needs two analysed functions by definition. Both programs
are also literally named `BOOT.BIN`, so the bridge's `program` parameter
cannot disambiguate them by name. This is a tooling state, not a finding: a
future pass with a properly analysed `/psp-pulse-eu/BOOT.BIN` should redo this
with the normal tools and should reproduce the table below.

What replaced it is a **relocation-tolerant byte match run directly on the two
ELFs**, which gives the same guarantee `diff_functions` gives and is checkable
without Ghidra:

1. Take the USA function's exact byte range from the analysed USA program.
2. Canonicalise both binaries' `.text` by zeroing *only* the fields a rebuilt
   binary is allowed to move: the 16-bit immediate of `lui`, `addiu`, `ori`
   and every load/store, and the 26-bit target of `j`/`jal`.
   **Branch displacements are left intact**, so any change in control flow
   breaks the match - that is the same property that makes a `DATA_EXT`-only
   `diff_functions` result trustworthy.
3. Require a **unique, 4-byte-aligned** occurrence of the canonicalised
   function in EU `.text`.

The method self-validates on three addresses derived independently by raw
string search in the EU ELF earlier in the same session: the
`data/defaults/loading/LoadingPulseOverlay.mip` literal (EU `0x08a87878`), the
`"Load screen"` thread-name literal (EU `0x08a878fc`), and the wave envelope
table (EU `0x08a87814`). All three fall out of the aligned `lui`/`addiu` pairs
of the matched functions at exactly the values the string search found.

### Functions

`words` counts whole 32-bit instruction words that are byte-identical between
the two, out of the function's total; the remainder are relocated literal-pool
and data operands.

| USA function (confidence) | EU address | Words identical |
| --- | --- | --- |
| `Loading_Ramp255` (88) | `0x08909c00` | 17/17 |
| `Loading_LerpByte` (85) | `0x08909c44` | 14/14 |
| `Loading_LerpColour` (88) | `0x08909c7c` | 55/59 |
| `Loading_SlewToward` (85) | `0x08909d68` | 25/25 |
| `Loading_DrawText` (85) | `0x08909e6c` | 203/254 |
| `Loading_DrawWave` (88) | `0x0890a264` | 197/225 |
| `Loading_DrawBackdrop` (82) | `0x0890a7d4` | 42/51 |
| `Loading_Show` (90) | `0x0890a8a0` | 402/405 common, see below |
| `Texture_LoadEffectSurfaces` (78) | `0x0890c454` | 126/202 |
| `Texture_BindCausticFrame` (85) | `0x0890d7f8` | 49/55 |

Nine of the ten matched at identical length with identical control flow.

### `Loading_Show`: matched, and it carries a real one-instruction divergence

`Loading_Show` is the one row that did **not** match at identical length: EU is
404 instructions to USA's 405. An alignment-tolerant diff over the
canonicalised words gives 402 common in 3 hunks, and all three are the same
change:

```
USA[370] 3c070001  lui   a3, 0x1        EU[370] 34060010  ori  a2, zero, 0x10
USA[371] 24e77700  addiu a3, a3, 0x7700 EU[371] 34072ee0  ori  a3, zero, 0x2ee0
USA[372] 34060010  ori   a2, zero, 0x10 EU[372] 34084000  ori  t0, zero, 0x4000
USA[373] 34084000  ori   t0, zero, 0x4000
```

`a3` is `sceKernelCreateThread`'s fourth argument, the stack size. **USA asks
for 96,000 bytes (`0x17700`, needing a `lui`/`addiu` pair); EU asks for 12,000
(`0x2ee0`, which fits one `ori`).** That is the missing instruction, and the
third hunk is its only other consequence: `USA[32]`'s `bne` displacement is
`0x15f` against EU's `0x15e`, the same branch retargeted one instruction
earlier. Nothing else in 405 instructions differs structurally, so the name is
carried over and the stack-size difference is recorded as a genuine
region divergence - eight times smaller on the EU disc.

### Data

Recorded **unbased**, per this page's "Method: data" note that EU's data and
BSS are not rebased into the `0x088xxxxx` range the way `.text` is. Each was
read out of the aligned `lui`+load/store pair inside an already-matched
function, at the same instruction index on both sides.

| USA data (confidence) | EU address | Read from |
| --- | --- | --- |
| `g_loading_wave_envelope` (90) | `0x00283814` | `Loading_DrawWave` idx 77/82 |
| `g_loading_thread_id` (90) | `0x002b9078` | `Loading_Show` idx 376/377-379 |
| `g_loading_finished` (80) | `0x002b9084` | `Loading_DrawWave` idx 83/130 |
| `g_loading_screen_type` (88) | `0x002b908c` | `Loading_Show` idx 16/24 |
| `g_loading_start_time` (85) | `0x002b9090` | `Loading_Show` idx 21/23 |
| `g_loading_wave_phase` (85) | `0x002b90a0` | `Loading_DrawWave` idx 28/49 |
| `g_loading_wave_rates` (85) | `0x002b90a4` | `Loading_DrawWave` idx 20/21 |

Six of the seven BSS items are internally consistent at a uniform
`0x23d0` shift from their USA counterparts, and they land in the same
`0x002b9xxx` neighbourhood as `g_force_fixed_30` (`0x002b920c`) and its two
siblings, which the fourth pass resolved independently. `g_loading_thread_id`
is the one derived past `Loading_Show`'s divergence hunk, so its EU index is
shifted by one; three separate instruction pairs in that function all give
`0x002b9078`.

### What did not match

- **`Loading_ThreadMain`** (USA `0x0890b52c`). Independently landed on the same
  EU candidate `0x0890aef0` the fourth pass reached by offset interpolation,
  and independently rejected it: best canonical-word agreement is 105/411 at
  that alignment, and an alignment-tolerant diff over a same-length window
  gives 73.2 % in 24 hunks. That agrees with the fourth pass's
  `body_added=188` finding above. **Two techniques, two rejections** - treat
  this as settled unless a genuinely different candidate address turns up.
- **`Loading_DrawBootLogo`** (USA `0x0890ac68`, 105 instructions). The EU slot
  between the matched `Loading_DrawWave` and `Loading_DrawBackdrop` runs
  `0x0890a5e8`-`0x0890a7d3`, which is **123 instructions - 18 more than USA**,
  with 86 common in 14 hunks (69.9 %). That is a real structural difference,
  not a relocation pattern. Not renamed. It is also unsurprising: this is the
  boot logo and legal screen, exactly where the EU disc is already documented
  to differ (the language picker and `PI000`-vs-`PI012` plugin layout in
  [frontend-boot.md](../../../architecture/frontend-boot.md)).
- **`g_loading_tip`** (USA `0x08af25c0`), **`g_loading_tip_show_help`**
  (`0x08af2699`) and **`g_caustic_frames`** (`0x08b62d90`). None is formed by
  a `lui`+load/store pair that this pass's tracker can follow - most likely the
  base register is renamed through a `move` between the two halves, which the
  tracker does not chase. Left unresolved rather than guessed; a normal
  `decompile_function` on the EU side, once that program is analysed, reads
  these straight out of the pseudo-C the way the fourth pass's data method did.

**These names are not applied to Ghidra.** The EU program in the project has no
functions, so `apply-ghidra-names.py` would `create_function` at each address
in an unanalysed image rather than rename anything. The rows are recorded here
and in `names.tsv`; apply them after the EU binary is analysed properly.

**17 rows added** (10 functions, 7 data); `names.tsv` now carries 289 rows.

Confined finding worth keeping: **everything the wave itself touches is
region-invariant** - the envelope, the rates, the helpers, the draw path - and
both rejections (`Loading_ThreadMain`, `Loading_DrawBootLogo`) plus the one
accepted divergence (the thread stack size) sit in the *boot logo and thread
setup*, not in the effect.
