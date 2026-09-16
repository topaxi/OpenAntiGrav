# The Omega Collection's PS4 patch PKG adds four archives, not a merge of the base's five

2026-09-15. Split out of
[omega-collections-ps4-build-imports-and-decrypts.md](omega-collections-ps4-build-imports-and-decrypts.md),
which had left "whether the current extraction folded in the patch, and
whether that merge was clean" as an open, unverified lead for the block-data-
location mystery. It's now closed, and the answer turned up new content
rather than resolving the mystery: full evidence and the exact commands are
[source-images.md](../../docs/reverse-engineering/source-images.md#omega-ps4-eupkg--omega-ps4-eu-patchpkg---wipeout-omega-collection-ps4)
and [psarc.md](../../docs/formats/psarc.md#block-data-location---the-first-byte-oracle-was-wrong-and-the-corrected-picture-is-three-way-not-binary).

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

## Census, all four archives (`psarc_list`, 2026-09-15)

**None of the four read as DLC or bonus content - all four patch the existing
roster, not extend it.** `data08` names every HD circuit already known to
this project by directory (`01_Vineta_K`, `02_Metropia`, `03_Moa_Therma`,
`04_Chenghou_Project`, `05_Ubermall`, `10_Sebenco_Climb`, `12_Sol_2`,
`15_Anulpha_Pass`, `amphiseum`, `modesto_heights`, `talons_junction`,
`tech_de_ra`, `zone_1`-`zone_4`) plus **every 2048 zone-mode environment by
name** (`altima`, `arena`, `bridge`, `cathedral`, `mall`, `park`, `shared`,
`sol`, `square`, `subway`, `tower`, under `data/environments2048/`) - no name
outside either already-documented roster. `data05` is the same shape at
smaller scale (`data/environments`, `data/environments2048`,
`data/Weapons2048`, `data/particles2048` all present). This is the first
time this project has found **2048 content bundled inside another title's
own package** rather than inferred from shared code/debug-tag paths -
corroboration, at the asset level, of the same lineage the parent thread's
["codebase-lineage question"](omega-collections-ps4-build-imports-and-decrypts.md#the-codebase-lineage-question-extended-a-third-time)
established by function and source-tree matching alone.

| Archive | Entries | Dominant extensions | Shape |
| --- | ---: | --- | --- |
| `data05.psarc` | 1,205 | 1,155 `.rcsmaterial`, 34 `.pob`, 11 `.xml` | A material-table patch across HD + 2048 environments/weapons/particles - no textures, no models, no audio |
| `data07.psarc` | 6 | 3 `.gnf`/`.fnt` (Korean fonts), 1 `.xml`, 1 `.rcsmodel` | Korean localization plus one `Zone` ship model fix |
| `data08.psarc` | 1,786 | 1,265 `.rcsmaterial`, 312 `.gnf`, 113 `.wem`, 35 `.xml`, 17 `.vex`, 15 `.rcsmodel`, 8 `.EnvSettings`, 7 `.bnk` | A comprehensive texture/model/audio/lighting patch across the full HD + 2048 roster |
| `data09.psarc` | 120 | 120 `.xml` | Frontend/language/grid definitions (already sampled directly, see the parent thread) |

**A new format surfaces in `data08`: Wwise, not the PSP-era in-house sound
bank this project already parses.** `data/audio/sound/*.wem` (113 of them,
many under `English(US)/` - localized voice lines) plus `speech.bnk`/
`speech_zbattle.bnk` - `.wem` is Audiokinetic Wwise's own "Wwise Encoded
Media" extension, unrelated to the PSP `SBlk` `.bnk` bank format
[`psp-audio.md`](../../docs/formats/psp-audio.md) already documents despite
sharing the `.bnk` extension by coincidence (both HD's own `speech_zone.bnk`
and this one use the name; the PS4 `.bnk` is very likely a Wwise "bank" file,
a different container entirely). No reader for either half exists in this
project.

## Open

- **The Wwise `.wem`/`.bnk` pair in `data08`** - format, not touched. Whether
  this specific `.bnk` is Wwise's own `SoundBank` container or something else
  wearing the same three-letter extension is not checked; `.wem` almost
  certainly is Wwise (that extension has no other common meaning), which
  would make this the first Wwise-era asset format this project has needed a
  reader for anywhere in the lineage.
- **Why `data06` is skipped** - reserved, unchanged-and-therefore-omitted-
  from-the-patch, or an artifact of some numbering this project doesn't have
  context for yet. Not investigated.
- `sce_discmap_patch.plt` (present in the patch's `uroot/`, absent from the
  base's) is unexamined - PS4 patch metadata, format not looked at.
- The real/zero split (see the table above) still has no predictor on any of
  these four archives either, same as the base `.pkg`'s five - this thread
  only ruled out *why the split exists* being an extraction-provenance
  question, not the split itself.

## Next Steps

- **The executable (not these asset archives specifically) became an RE
  target 2026-09-15** - see the parent thread and
  [`docs/ghidra/functions/ps4-omega-eu/README.md`](../../docs/ghidra/functions/ps4-omega-eu/README.md).
  These four `.psarc` archives themselves are still asset-format groundwork,
  not gameplay/simulation work. `.wem`/Wwise is the next format gap to scope
  (the way `.gnf` already is for the base `.pkg`'s textures) rather than
  `data08`'s file listing, which this census now closes.
