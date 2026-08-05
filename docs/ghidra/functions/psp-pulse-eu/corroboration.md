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
