# The Omega Collection's PS4 patch PKG adds four archives, not a merge of the base's five

2026-09-15. Split out of
[omega-collections-ps4-build-imports-and-decrypts.md](omega-collections-ps4-build-imports-and-decrypts.md),
which had left "whether the current extraction folded in the patch, and
whether that merge was clean" as an open, unverified lead for the block-data-
location mystery. It's now closed, and the answer turned up new content
rather than resolving the mystery: full evidence and the exact commands are
[source-images.md](../../docs/reverse-engineering/source-images.md#omega-ps4-eupkg--omega-ps4-eu-patchpkg---wipeout-omega-collection-ps4)
and [psarc.md](../../docs/formats/psarc.md#block-data-location---open-and-a-realzero-split-rather-than-uniformly-broken).

`omega-ps4-eu-patch.pkg` extracts, via the same `PkgTool.Core pkg_extract`
verb the base `.pkg` uses (confirmed from `PkgTool/Program.cs` source: it
takes one `.pkg` in, one output directory out, no patch-chain logic at all),
into `data/extracted/ps4/omega-eu-patch/uroot/`: a newer `eboot.bin`,
`sce_discmap.plt`/`sce_discmap_patch.plt`, `sce_module/*.prx`, and four
`dataNN.psarc` archives - `data05` (654 MiB), `data07` (20 MiB), `data08`
(5.3 GiB), `data09` (6.9 MiB). **No `data06`, and no `data00`-`data04`.** The
base `.pkg`'s five archives are untouched by the patch; the title's real
asset namespace is at least nine archives across the two packages.

The already-open real/zero split (some fraction of a real, named entry's
declared bytes reads as its actual content, the rest as zero, no predictor
found) is present in these four at a similar magnitude to the base archives,
measured with `crates/assets/examples/psarc_sweep` (committed this session -
see its own doc comment for what it checks and the trap in getting that
check right):

| Archive | Real entries | Non-zero | |
| --- | ---: | ---: | ---: |
| `data05.psarc` | 1,207 | 415 | 34% |
| `data07.psarc` | 7 | 6 | 86% |
| `data08.psarc` | 1,789 | 796 | 44% |
| `data09.psarc` | 121 | 84 | 69% |

## Open

- **What `data08` (5.3 GiB, by far the largest of the four) actually holds**
  is not surveyed at all - no `psarc_list` census run against it yet. The
  original thread speculated `data03.psarc` (the base `.pkg`'s smallest
  archive) might be DLC-shaped; that was never chased to a conclusion there,
  and `data08` is a better candidate to check first now - large, patch-only,
  and not present in the base install at all.
- **Why `data06` is skipped** - reserved, unchanged-and-therefore-omitted-
  from-the-patch, or an artifact of some numbering this project doesn't have
  context for yet. Not investigated.
- `sce_discmap_patch.plt` (present in the patch's `uroot/`, absent from the
  base's) is unexamined - PS4 patch metadata, format not looked at.

## Next Steps

- Run `psarc_list` (`crates/assets/examples/`) against all four patch
  archives for a full per-archive file/extension census, the same way the
  original thread's own Next Steps proposed for `data03.psarc` and never
  got to. Decide from the census whether `data08` reads as DLC/bonus content
  (a distinct `Data/...` subtree, ship or track names not in the base
  install) or as more of the same namespace the base `.pkg` already covers.
- Not a reverse-engineering target in its own right yet, same standing as
  the parent thread - Omega Collection stays "if feasible" in the roadmap.
