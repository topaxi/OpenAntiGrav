# `/psp-pure-eu/BOOT.BIN` - corroborated names

Same status as `psp-pure-usa`: not a reverse-engineering target, here for
opportunistic format corroboration only, see
[source-images.md](../../../reverse-engineering/source-images.md#pure-psp-euchd---wipeout-pure-psp-eu).
See [psp-pure-usa/corroboration.md](../psp-pure-usa/corroboration.md) for why
this binary mostly does not fuzzy-match against `psp-pulse-usa` the way the
EU Pulse binary does - the short version is that Pure and Pulse are different
executables from different SDK eras, so "shared engine lineage" mostly did
not hold as a match hypothesis here.

## `strcasecmp` -> `0x088dfe9c`

Confirmed two independent ways:

- Directly against `psp-pulse-usa`'s `strcasecmp` at `0x089732d8`
  (confidence 92, see [xml-reader.md](../psp-pulse-usa/xml-reader.md)):
  fuzzy match score 0.8664, 40/46 instructions equal.
- Against `psp-pure-usa`'s own `strcasecmp` at `0x088e05e8` (this page's
  sibling, [corroboration.md](../psp-pure-usa/corroboration.md)): fuzzy match
  score 0.9093, the expected same-title/cross-region shape seen for Pulse.

Both diffs show only literal-pool address differences, not logic changes.
Standard PSP SDK libc, not game code.

## Full sweep (2026-08-05)

Ran `bulk_fuzzy_match` across every one of `psp-pulse-usa`'s 10,703 functions
at threshold 0.8 (11 paginated calls, `offset` 0 through 10000 - lower than
the 0.95 used for the [`psp-pulse-eu` full sweep](../psp-pulse-eu/corroboration.md#full-sweep-2026-08-05)
because the one confirmed Pure match above only scores 0.8664; 0.95 would
have missed it), then cross-referenced the results against every row
`psp-pulse-usa/names.tsv` documents. Of 10,703 source functions only 7 named
ones matched anything at all - a small fraction of the pulse-eu sweep's 44,
which is the expected shape given [`psp-pure-usa/corroboration.md`](../psp-pure-usa/corroboration.md)'s
finding that most Pure/Pulse correspondences don't hold.

**Two excluded as collision-prone**, the same signal that ruled out four
names in the pulse-eu sweep: `Wad_BuildCrcTable` (`0x08940cb0`) matched
`0x0889408c`, a target that an unrelated USA function
(`FUN_089458dc`, and `FUN_08970e00`, both at exactly the same 0.8743 score)
also matched; `Xml_CloseDocument` (`0x08953fc0`) matched `0x0897779c`, which
`FUN_088c29e4` also matched. Neither was applied.

**Five survived**, verified individually with `diff_functions`:

| USA function (confidence) | EU address | Fuzzy score | Body equal |
| --- | --- | --- | --- |
| `Options_LoadDefaultControlMapping` (88) | `0x0880b928` | 0.8657 | 40/43 |
| `Body_Integrate` (88) | `0x08933f84` | 0.8013 | 115/150 |
| `Ship_UpdateEngine` (84, `_q`) | `0x0892eb8c` | 0.8118 | 105/279 |
| `Ship_ApplyQuadraticDrag` (80, `_q`) | `0x0892c53c` | 0.8076 | 17/35 |
| `Ship_ApplyAngularDamping` (80, `_q`) | `0x0892c5c8` | 0.8318 | 15/26 |

Only `Options_LoadDefaultControlMapping` clears the "literal-pool address
only" bar the pulse-eu sweep used: 40 of 43 instructions equal, the other
three lines are two `lui`/`addiu` pairs building a relocated data pointer and
one register substitution for an equivalent store.

The other four are noisier than that bar - fuzzy scores in the 0.80-0.83
range instead of 0.95+, and raw body-equal ratios as low as 42% for
`Ship_UpdateEngine`. Read literally that looks like real logic drift, but
walking the actual diffs shows most of it is not: the Pure build's compiler
picked a different scratch register for the same computation throughout
(`lui a1,IMM:48998` / `ori a1,...` in the USA function versus `lui a0,...` /
`ori a0,...` in Pure, identical constants, just `a0` instead of `a1`, which a
line-based diff can only see as a remove+add pair even though it is the same
instruction), used `bc1fl`/`bnel` branch-likely forms with a different
delay-slot fill where the USA build used a plain branch, and in a few spots
loads a genuinely different immediate - expected, since those immediates are
game-tuning float constants (e.g. a drag coefficient) that Pure and Pulse
have no reason to share.

The signal that survives that noise, and the reason these four were applied
rather than left alone despite falling short of the clean bar: **all three
`Ship_*` functions show the exact same `-0x48` field-offset shift** between
the USA source and the Pure target (`0x2ec` -> `0x2a4`, `0x2a4` -> `0x25c`,
`0x2cc` -> `0x284`, `0x290` -> `0x248`, `0x2d4` -> `0x28c`, `0x2e0` -> `0x298`,
all consistently 0x48 apart), and `Body_Integrate` independently shows a
consistent `+0x40` shift across nine of its own field accesses. Three
unrelated functions agreeing on one constant delta each is not something a
spurious fuzzy match produces by chance - it reads as the Ship and
rigid-body structs genuinely having a different layout in this two-years-older
build, which is exactly what carrying the same subsystem across a compiler/SDK
generation gap would look like. `calls_only_in_a`/`calls_only_in_b` are empty
for all four diffs (no divergent call targets), consistent with the same
function rather than a different one.

Confidence for the four noisier ones is capped low and `_q`-suffixed to
reflect that this is weaker evidence than the clean pulse-eu matches, not
because the correspondence itself is in doubt.
