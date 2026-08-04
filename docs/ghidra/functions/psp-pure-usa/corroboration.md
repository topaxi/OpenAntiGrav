# `/psp-pure-usa/BOOT.BIN` - corroborated names

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
