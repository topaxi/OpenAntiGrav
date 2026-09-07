# The EU name gap: 115 closed by exact hash, 182 functions (plus data) remain

[ADR-0048](../docs/architecture/adr/0048-eu-is-the-psp-pulse-re-target-of-record.md)
makes `psp-pulse-eu` the Ghidra target of record. Measured 2026-09-07 before
any transfer: `psp-pulse-usa/names.tsv` named 581 functions,
`psp-pulse-eu/names.tsv` named 284 of them - a 297-function gap (the file's
own total-row count, 656 against 301, also folds in 25 `data`-kind rows this
thread's method cannot touch at all).

**115 of the 297 are now closed**, via
[`exact-hash-transfer.md`](../docs/ghidra/functions/psp-pulse-eu/exact-hash-transfer.md):
`get_bulk_function_hashes` on both `/psp-pulse-usa/BOOT.BIN` and
`/psp-pulse-eu/BOOT.BIN`, matched only where a USA name's normalized opcode
hash was unique on both sides, then individually confirmed with
`diff_functions` before applying anything - all 115 came back with zero added
and zero removed instructions. Applied live (`just apply-names
docs/ghidra/functions/psp-pulse-eu/names.tsv --program /psp-pulse-eu/BOOT.BIN`,
416 applied/0 skipped/0 failed) and audited clean
(`scripts/audit-ghidra-names.py --binary psp-pulse-eu`: "399 named function(s)
live, 416 row(s) in names.tsv... clean"). **182 remain**, all of them either
sharing a hash with more than one function on the USA side (16, unfixable by
hash matching regardless of technique) or having no exact hash match on the EU
side at all (166). Neither category is data - the 25 `data`-kind rows are a
separate remainder, untouched by this thread's method, same as before.

**The prerequisite this thread originally opened on - the EU program having no
functions analysed at all - is resolved.** All four PSP databases were
reimported with the Allegrex relocation patch on 2026-09-07
(`docs/ghidra/workflow.md`, "The full set is relocated as of 2026-09-07"):
`/psp-pulse-eu/BOOT.BIN` reads 10,672 functions and 108,729 relocations
(zero before), and `get_xrefs_to` now works for code and data alike on it -
confirmed directly, not assumed. If picking this thread up again finds a
program with a near-zero function count, something regressed and needs
re-analysing before any matching technique below is worth attempting.

## What is left, and the method for it

The 182 remaining functions need the techniques
[`corroboration.md`](../docs/ghidra/functions/psp-pulse-eu/corroboration.md)'s
four 2026-08-05 sweeps already proved out for its own 213 rows (before this
pass's 115 brought the total to 399 live-named functions):

1. **`bulk_fuzzy_match` at descending thresholds** - 0.95, then 0.8, then 0.6
   - against whatever remains, cross-referenced only against the *current*
   gap (166 no-hash-match functions; the 16 hash-collision ones need a
   different technique entirely, see below).
2. **Collision filter before anything else, every pass.** For every
   candidate's EU target address, check whether any *other* source function
   (named or not) in the same match set also lands on that target. Drop any
   many-to-one match without individual verification.
3. **`diff_functions` on every surviving candidate, individually, before
   trusting it.** Never rename off a score alone. Confidence tiers already
   established and re-verified across two independent passes in
   `corroboration.md`: diff ratio <=2% or 100%-equal body with only
   callee-name differences, USA confidence minus 8; 2-8%, minus 11; 8-20%,
   minus 14; >20% (still relocation-only), minus 18.
4. **Structural/positional corroboration** for what fuzzy matching still
   misses: address adjacency to an already-matched neighbour; a callee
   surfacing through an accepted pair's own `calls_only_in_a`/
   `calls_only_in_b` diff lines (both preserve calling order, so a 1:1
   unresolved-to-unresolved correspondence at the same call site is a strong
   candidate); a fixed local address offset between two already-matched
   neighbours, applied to a third and checked against `get_function_by_address`
   before diffing. Confidence tiers for this category, steeper than the fuzzy
   ones to reflect the indirect evidence: diff ratio <=2%, USA confidence
   minus 11; 2-8%, minus 14; 8-20%, minus 17; >20%, minus 21.
5. **The 16 hash-collision functions specifically** need one of the
   techniques above rather than a repeat of exact hashing - hash equality
   cannot discriminate between multiple same-body USA functions no matter how
   many times it is tried.

## The failure mode, documented and specific - read this before starting

`scripts/audit-ghidra-names.py`'s own module docstring records a fuzzy
cross-application sweep on this project that **mis-assigned 54 of 64**
`_q`-suffixed names, including `Ship_UpdateCameraRigs` (confidence 82, the
spine of `camera.md`'s field-of-view chain) displaying as
`Ship_UpdateSideshiftInput_q`, and `Gu_Fog` (95) displaying as
`Gu_DepthMask_q`. The GE state-cache wrapper functions - 9 to 22 instructions,
structurally identical apart from which literal-pool base they load - are
exactly the shape a fuzzy matcher permutes. **A wrong name is worse than no
name.** Run `scripts/audit-ghidra-names.py --binary psp-pulse-eu` after
applying anything from a fuzzy or structural pass, the same way this pass's
own 115 rows were checked after applying.

Known **do-not-re-run** exclusions from the 2026-08-05 sweeps, still valid
against the current gap unless the underlying binary changes again:

- **52 USA functions had no fuzzy candidate at any threshold down to 0.6** in
  the deep sweep. Try the structural/positional method first rather than
  re-sweeping fuzzy thresholds that already failed - though note the binaries
  were reimported since that sweep ran, so a function that had no candidate
  against the unrelocated database is worth one retry, not an assumption it
  still fails.
- **13 collision-prone targets** from the deep sweep (`Xml_GetAttributeValue`,
  `Xml_DestroyElement`, `Xml_GetAttributeName`, `Options_ControlSchemeFlag`,
  `Zone_UpdateState`, `atof`, the three-way `Xml_AttributeNameIs`/
  `Xml_AttributeValueIs`/`Xml_ElementNameIs` collision, `Wad_BuildCrcTable`,
  `Body_ClearAccumulators`, `Options_SetControlScheme`, `Pad_Bind`) - dropped
  before individual verification and not worth re-attempting without a
  different technique. Note `Body_ClearAccumulators` and
  `Xml_AttributeValueIs` **were** resolved by this pass's exact-hash method
  instead - a fuzzy-sweep collision does not mean an exact-hash collision,
  since the two techniques fail on different axes.
- **`MoviePlayer_Shutdown`** - rejected twice, independently, by two different
  techniques. Treat as settled unless a genuinely different candidate address
  turns up.
- **`Loading_ThreadMain`** and **`Loading_DrawBootLogo`** - both rejected, real
  structural divergence, not a relocation artefact - the boot logo/legal
  screen is exactly where the EU disc is already documented to differ (the
  language picker, [`frontend-boot.md`](../docs/architecture/frontend-boot.md)).
- **`Options_LoadControlMapping`** - rejected, a genuine extra conditional
  block in USA absent from EU.
- **Three data globals** (`g_loading_tip`, `g_loading_tip_show_help`,
  `g_caustic_frames`) resist the `lui`+load/store pair tracker
  `corroboration.md`'s data method uses. Needs a normal `decompile_function`
  read, not a bigger sweep of the same technique.
- **25 data-kind gap rows total**, untouched by every pass including this
  one's exact-hash method (function-scoped only). `diff_functions`/
  `find_similar_functions_fuzzy` are function-scoped tools; a data address
  needs the xref-through-a-matched-function technique
  `corroboration.md`'s "Method: data" section already used for seven of them.
- **EU's `.data`/`.rodata`/BSS are not rebased into the `0x088xxxxx` range**
  the way `.text` is. Read a data address out of the EU decompiler's own
  `DAT_xxxxxxxx`/`_DAT_xxxxxxxx` symbol, never by applying a `0x088xxxxx`-range
  offset the way a function address would.

## Evidence pages: a transferred row does not inherit the USA page

Per [ADR-0005](../docs/architecture/adr/0005-ghidra-conventions.md), every
name needs a doc page citing its evidence, and `scripts/check-ghidra-names.py`
validates that a row's address and name both still appear on the page its
`names.tsv` row cites. **A transferred EU name cannot cite the USA page
directly** - that page's address is the USA one, and the check would fail the
EU row against it. This pass's own solution, already proven and reusable:
write the EU address and name directly onto an EU-side page (a table row is
enough), citing the USA page as prose context rather than as the row's
`evidence` column. `exact-hash-transfer.md` and `corroboration.md` both do
this today; continue whichever one fits, or open a new EU-side topic page the
way `psp-pulse-eu/camera.md` and `lighting.md` already sit beside them, if a
large addition would push either past the 1,000-line file-size ratchet
(`just check-size`).

## Open

- 182 USA-named functions (16 hash-collision, 166 no-hash-match) have no EU
  counterpart yet
- 25 data-kind gap rows, untouched by every pass so far including this one
- No structural/positional pass has been attempted against the *current*,
  reimported databases - the 2026-08-05 sweeps that proved the technique ran
  against a differently-numbered function set before this project's PSP
  databases were reimported with the relocation patch

## Next Steps

1. Fuzzy sweep at 0.95, then 0.8, then 0.6 against the 166 no-hash-match
   functions, collision-filtering before every individual `diff_functions`
   check
2. Structural/positional pass over the 16 hash-collision functions and
   whatever the fuzzy sweep still misses
3. Write each accepted name onto its own EU-side evidence page (not a
   citation of the USA page), add the row to `psp-pulse-eu/names.tsv`, run
   `just apply-names` against `/psp-pulse-eu/BOOT.BIN`, then
   `scripts/audit-ghidra-names.py --binary psp-pulse-eu` to catch anything a
   fuzzy match mis-assigned
4. The 25 data-kind rows, by the xref-through-a-matched-function technique
