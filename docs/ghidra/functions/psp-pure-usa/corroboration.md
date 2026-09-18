# `/pure/BOOT-psp-pure-usa.BIN` - corroborated names

Not a reverse-engineering target - Pure is here as the format ancestor per
[source-images.md](../../../reverse-engineering/source-images.md#pure-psp-usachd---wipeout-pure-psp).
No simulation work on Pure is planned; ADR-0009 defers that. This page exists
only because a small, honest sweep for cross-binary matches is cheap and
worth recording when it lands.

## Method and what it actually found

Tried cross-binary fuzzy matching (`find_similar_functions_fuzzy`) from the
already-documented `psp-pulse-usa` target against this binary, across a
sample of ~20 of `psp-pulse-usa`'s 304 documented functions - not an
exhaustive sweep. Pure predates Pulse by about two years and is a different
executable built against a different compiler/SDK snapshot, so the
"shared engine lineage" that makes this work well for Pulse USA-vs-EU (see
[psp-pulse-eu/corroboration.md](../psp-pulse-eu/corroboration.md)) does not
carry over here - it is a different hypothesis and it mostly did not hold.

Most attempts (`Wad_HashName`, `atoi`, `atof`, `strtol`, `strtod`, `Gu_Fog`,
`Gu_DrawArray`, `Gu_CallList`, `powf`, `Math_TransformVec4`) returned zero
matches at threshold 0.75. A handful of GU wrapper attempts (`Gu_DepthFunc`,
`Gu_StencilFunc`, `Gu_StencilOp`, `Gu_TexWrap`) returned weak, multi-candidate
matches (0.75-0.77 with 2-12 near-tied candidates) - the shape of a false
positive on a tiny, generic function, so none of those were pursued or
trusted. `strrchr` returned one candidate at 0.7614 that `diff_functions`
showed was a real function with only 3 of 17 instructions equal - a false
positive caught by verification, not applied.

## The one function that held up

### `strcasecmp` -> `0x088e05e8`

Source: `strcasecmp` at `0x089732d8` in `psp-pulse-usa`, confidence 92, see
[xml-reader.md](../psp-pulse-usa/xml-reader.md). Fuzzy match score 0.8664,
40/46 instructions equal; the differences are the address of the ctype-style
lookup table the function indexes into, consistent with a rebuilt binary's
literal pool moving, not a logic change. This is standard PSP SDK libc, not
game code, so the confidence here reflects "this is the same library
function," not evidence about anything Pure-specific.

## Full sweep (2026-08-05)

The sample sweep above was ~20 of `psp-pulse-usa`'s 304 documented functions.
Went back and ran `bulk_fuzzy_match` across all 10,703 `psp-pulse-usa`
functions at threshold 0.8 (11 paginated calls, `offset` 0 through 10000),
then cross-referenced every row `psp-pulse-usa/names.tsv` documents against
the results - the same full-coverage method as the
[`psp-pulse-eu` sweep](../psp-pulse-eu/corroboration.md#full-sweep-2026-08-05),
at a lower threshold since this binary's one confirmed match (`strcasecmp`,
above) only scores 0.8664. Only 7 named functions matched anything at all
across the entire binary - confirming the "mostly did not hold" finding
above at full coverage rather than a ~20-function sample.

Two were excluded as collision-prone (an unrelated USA function matched the
same target, the same signal used to exclude four names in the pulse-eu
sweep): `Wad_BuildCrcTable` (`0x08940cb0` -> `0x0889483c`, also matched by
`FUN_089458dc` and `FUN_08970e00` at the identical 0.8743 score) and
`Xml_CloseDocument` (`0x08953fc0` -> `0x08977e4c`, also matched by
`FUN_088c29e4`).

The other five - `Options_LoadDefaultControlMapping`, `Body_Integrate`,
`Ship_UpdateEngine`, `Ship_ApplyQuadraticDrag`, `Ship_ApplyAngularDamping` -
landed on the *same* USA source functions and at the *same* fuzzy scores as
this binary's `psp-pure-eu` sibling (expected: USA and EU Pure are the same
build, region-shifted), so the full method, the per-function diff read, and
the reasoning for why the four noisier ones were still applied despite
falling short of the "literal-pool only" bar are written up once in
[`psp-pure-eu/corroboration.md`](../psp-pure-eu/corroboration.md#full-sweep-2026-08-05)
rather than duplicated here.

| USA function (confidence) | USA address | Pure USA address | Fuzzy score | Body equal |
| --- | --- | --- | --- | --- |
| `Options_LoadDefaultControlMapping` (88) | `0x0883672c` | `0x0880bae8` | 0.8657 | 40/43 |
| `Body_Integrate` (88) | `0x0884e230` | `0x089345e8` | 0.8013 | 115/150 |
| `Ship_UpdateEngine` (84, `_q`) | `0x0884c5c8` | `0x0892f1f0` | 0.8118 | 105/279 |
| `Ship_ApplyQuadraticDrag` (80, `_q`) | `0x08848e28` | `0x0892cba0` | 0.8076 | 17/35 |
| `Ship_ApplyAngularDamping` (80, `_q`) | `0x08848ed0` | `0x0892cc2c` | 0.8318 | 15/26 |
