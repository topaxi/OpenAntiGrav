# A parser cannot fail on a field it does not know about, so coverage is now measured

2026-08-25, **landed**. It exists because `rcsmodel` lost 40% of HD's geometry to an offset table nobody read, with nothing to report it: every diagnostic here compared what it *drew* against what it *decided to draw*, which cannot surface an absence. [`oag_formats::coverage`](../../crates/formats/src/coverage.rs) claims every range a parser reads and lists the leftover runs **with what sits either side of them**; ratcheted by `coverage_ground_truth.rs`. **Today**: `.rcsmodel` **97.69%** of 719 MB; `.vex` **100.00%** of 742 files. **It found a real bug on its first run, and the shape of it is the lesson**: `rcsmodel::STRIDES` - the widths the stride searches may answer - was `[14, 18, 22]` while the disc's declarations carry **seven**. The three predate `vertex_decl` being decoded and were never revisited, so **a chunk of one of the missing four with no declaration could not be solved by any search: the answer was not on the ballot.** Widening it to the declared set took coverage 96.06% -> 97.69%, undecodable surfaces **371 -> 115**, and disagreements 7 -> 6 while solving 1,387 more chunks. `rcsmodel_stride_ground_truth.rs` pins the invariant that broke: **every declared width must be one the searches may answer**. **A trap it sprang on itself**: the first `.vex` run called 3.0 MB across 666 files unreachable, which is the embedded texture block the header declares at `+0x08` and a *different* entry point reads. **A gap is a lead; check first whether the format's own header accounts for it.** **Still unchased**: 115 surfaces with neither a declared nor a solvable stride, concentrated in the `fe/` track previews. **Where next**: highest value in a format that is a container of tables reached by offsets; lowest in `psarc` and `wad`, which enumerate their directory by construction. It does **not** check whether a *payload* is fully read.

## Open

- 115 surfaces have neither a declared nor a solvable stride, concentrated in the `fe/` track previews.
- `coverage` does not check whether a payload is fully read, only whether the ranges a parser reads are addressed.

## Next Steps

- Apply `coverage` next to formats that are containers of tables reached by offsets (highest value); `psarc` and `wad` are lowest value since they enumerate their directory by construction.
