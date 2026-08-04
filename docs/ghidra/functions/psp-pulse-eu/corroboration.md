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
