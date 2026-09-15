# WipEout: Omega Collection PS4 functions

Functions from `eboot.bin` (WipEout: Omega Collection, PS4, `CUSA05670`, EU),
`x86:LE:64:default`, image base `0x01000000`. Getting the file at all and
importing it with [GhidraOrbis](https://github.com/astrelsky/GhidraOrbis) are
covered in [toolchain.md#ps4](../../../reverse-engineering/toolchain.md#ps4);
where the PKG itself came from and how it decrypts is in
[data/README.md](../../../../data/README.md)'s Omega Collection section and
[source-images.md](../../../reverse-engineering/source-images.md).

**Not a reverse-engineering target in its own right** - Omega Collection is
listed "if feasible" in [the roadmap](../../../overview/roadmap.md), and no
milestone is open on it. What this directory exists for is the same lineage
question [`ps3-hdfury-eu/`](../ps3-hdfury-eu/) and
[`vita-2048-eu-v104/`](../vita-2048-eu-v104/) already answered for each
other: does Omega Collection's PS4 remaster share their `Backend/...` source
tree, or is it a fresh PS4-specific build?

## The lineage question, extended a third time: shared tree, confirmed at the code level

`search_strings("Backend")` against this binary returns 76 `.cpp` debug-tag
paths under `C:\WOPS4\Wipeout\Code\Backend\...`; 74 match a path already
named in `vita-2048-eu-v104/eboot.elf`'s own string table by suffix exactly -
same subsystem, same filename, same subdirectory nesting. That is a string-
table-level check only, the same kind `vita-2048-eu-v104/README.md` used for
its own first pass against `ps3-hdfury-eu`. This directory's first named
function goes one step further: [`MagstripWake_Construct`](ships-effects.md)
is structurally the same constructor on both binaries - matching
tagged-object field offset, matching resource-name lookup, matching flag
bits, matching instance-counter pattern - despite the architecture change
from ARM Thumb-2 to x86-64. See [`ships-effects.md`](ships-effects.md) for
the full comparison and
[`vita-2048-eu-v104/ships-effects.md`](../vita-2048-eu-v104/ships-effects.md)
for that binary's own half of the same finding.

**Consequence for future work on this binary**: when naming an unidentified
`ps4-omega-eu` function, check `vita-2048-eu-v104/names.tsv` (and, through
it, `ps3-hdfury-eu/names.tsv`) for a same-role match first - a shared
`__FILE__`-style tag string is often enough to find the candidate function on
the other side, and a matching field-offset/control-flow skeleton is what
turns that into an actual name rather than a guess. A filename match alone is
not enough on its own - see the confidence rubric - the code has to line up
too, the way it did for `MagstripWake_Construct` and did not (cleanly enough
to name) for the `Collision/SimpleMesh.cpp`-tagged pair checked in the same
session: the PS4 side's candidate decompiles to a ~145 KB function versus
Vita's small, separate `SimpleMesh_Load`/`SimpleMesh_Construct` pair, which
reads as heavier inlining rather than a clean 1:1 match, and so was left
unnamed rather than forced.

## Pages

- [ships-effects.md](ships-effects.md) - `MagstripWake_Construct`, the first
  name recovered here, transferred from and to `vita-2048-eu-v104` in the
  same pass.

Add a row to [`names.tsv`](names.tsv) and the page it cites in the same
change: `scripts/apply-ghidra-names.py` refuses a row whose address and name
do not both still appear on the page named in its last column.
