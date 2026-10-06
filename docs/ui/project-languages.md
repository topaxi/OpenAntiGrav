# Project languages and the `human` flag

A **project language** is one this build offers on every title whether or not
that title's disc ships it. There is one today, **PortugueseBR**
("Português (Brasil)"), added 2026-10-06 on the maintainer's decision to support
Brazilian Portuguese across all titles with this project's own translations.
The name is **chosen, not measured**; it is spelled the way Wipeout Omega's own
on-disc plugin names itself (`portuguesebr`), so one saved setting and one file
stem work on every title.

## How it is offered

- `oag_ui::strings::PROJECT_LANGUAGES` lists them. `oag_ui::language::load::
  load_languages` appends each **after the disc's own languages**, so the
  picker and the OPTIONS > LANGUAGE row list them last, and every caller (boot,
  remix, the race HUD, the track panel) sees the same list.
- A project language stands on the disc's **English**: it copies English's
  `plugin`, `entries` and `fonts`, so its table starts as the disc's English
  text and its fonts resolve (Pure keeps English inline in its definition; a
  made-up plugin id would read nothing there). Nothing is ever empty.
- On top of that, in order: the title's disc-keyed overlay, then the project
  `OAG_` file.
- **A release with no plain English stands on American.** 2048's USA
  executable reaches only `american`, `french` and `spanish`
  (`docs/ghidra/functions/vita-2048-eu-v104/language.md`), so the project
  language copies the disc's `American` there, and a boot with nothing saved
  reads `American`.
- **Omega already ships Brazilian Portuguese.** A disc language of the same name
  (case-insensitive) suppresses the project one, so Omega's own
  `Languages\portuguesebr` plugin is the base and only our `OAG_` ids are laid
  over it. Its disc table is never replaced by ours.

## Files

| File | Holds | Read by |
| --- | --- | --- |
| `assets/ui/strings/portuguesebr.toml` | this project's own `OAG_` ids | every title |
| `assets/ui/strings/disc/pulse/portuguesebr.toml` | our translation of **Pulse's** disc text, keyed by Pulse's idstrings (1,560 of 1,619) | titles whose `FrontEnd::disc_strings` is `Some("pulse")` |
| `assets/ui/strings/disc/pure/portuguesebr.toml` | **Pure's** disc text (797 of 888 ids) | `Some("pure")` |
| `assets/ui/strings/disc/hd/portuguesebr.toml` | **HD / Fury's** disc text (1,503 of 1,602 ids) | `Some("hd")` |
| `assets/ui/strings/disc/2048/portuguesebr.toml` | **2048's** disc text (2,704 of 2,965 ids) | `Some("2048")` |

The disc-keyed file is **per title on purpose**: Pure, HD and 2048 reuse id
spellings, and `overlay` does not know which title it runs on, so one shared
file would turn Pulse's wording on for them. The namespace is `Title` data
(`oag_title::FrontEnd::disc_strings`, ADR-0058), not a `title.name` branch.
Omega is `None`: it ships `portuguesebr` itself and reads nothing of ours but
the `OAG_` ids.

### What the three later files leave out (2026-10-06)

Ids left to read as the disc writes them, so they fall back to the disc's own
base text. Counts are of the disc's English ids with `OAG_` ones removed.

| Title | Translated | Left out | Of which |
| --- | ---: | ---: | --- |
| 2048 (American base on the Vita USA release) | 2,704 | 261 | 82 empty on disc; 36 circuit keys (`NN_TRACK`, `_OLD`); 93 team, ship, track, grid and music proper names; 20 unit labels (KMH, MPH, MACH); 10 other-region legal notices; 5 button glyphs; 6 bare numbers or formats; 8 identical words; 1 font test string |
| HD / Fury | 1,503 | 99 | 32 circuit keys; 43 proper names; 9 unit labels; 6 identical words; 5 glyphs; 2 bare numbers; 1 legal notice; 1 font test string |
| Pure | 797 | 91 | 22 bare numbers and percentages; 59 proper names; 4 glyphs; 4 identical words; 2 unit labels |

**The circuit keys stay English on purpose.** HD keys its circuits `NN_TRACK`
and `CircuitNames` resolves them through its own chosen copy of the table, so
nothing there is touched by this overlay (ground truth: `01_TRACK` reads the same
under `PortugueseBR` as under `English`).

**Pure keys most strings by their English text** (535 of its 888 ids read
`key == value`, e.g. `Overwrite Current State Save?`, and most of the rest are
English phrases with spaces). Spelling those ids in a committed file would commit
disc text, which legal.md forbids even for a key, so the maintainer ruled
(2026-10-06): **the file stores them as `h_xxxxxxxx`**, `h_` and the 32-bit
FNV-1a of the exact id string in eight hex digits (`oag_ui::strings::hashed_key`;
exact bytes, not the folding CRC of `oag_formats::wad::hash_name`, because Pure has
ids that differ only by case). `overlay_disc` hashes every id of the base table at
load and lays each `h_` entry over its match (`resolve_hashed_keys`), for every
title and language, so it is one mechanism: a plain id still matches by name, and
an `h_` key that matches no id is dropped. 719 of Pure's 797 entries are hashed;
the 78 code-like names (`HUD_Lap`, `Text->Options->Option->Music`) and single
words stay keyed by name. Ground truth: every `h_` key of the file matches a base
id, and none survives as an id (`the_pure_disc_text_...`).

**Ids shared with 2048.** HD shares 1,439 ids with 2048 whose English text is
identical, and those carry the same Portuguese in both files (the HD file was
prefilled from the 2048 one, then the 129 HD-only texts were written). Where an
id also exists in Omega's own `portuguesebr`, the terms follow Omega's and the
sentence is ours: no text of seven words or more is byte-identical to
Omega's (checked over every id the two share, with no length cap; shorter
formulaic labels such as `SEM ARMAS` converge and are left).

## Entry shape and the `human` flag

Every entry in every `assets/ui/strings/**/*.toml` is
`ID = { text = "...", human = false }`. `human` is **required** (no serde
default, `deny_unknown_fields`): a missing flag fails the parse instead of
reading as a silent `false`. It is `true` only when a human wrote or approved the
text. A model-written or machine-translated entry is `false`.

Why an inline table and not a parallel `[human]` table: the flag stays on the
line it describes, a diff of one string is one line, and nothing has to be kept
in step. `just check-strings` rejects any other shape and **reports** the
`human = false` count per file; it never fails on it.

The existing English and French entries were marked from git history: an entry
whose introducing commit carries a Claude `Co-Authored-By` trailer is `false`.
All 25 commits that introduced a string in those two files carry one, so all 128
English and 86 French entries are `human = false`.

## Terminology and the legal line

Words are matched to Omega's own `portuguesebr` table (NAVE, EQUIPE, VOLTA,
ZONA, ELIMINATÓRIA, TORNEIO, ARMAS, FOGUETES, MÍSSEIS, MINAS, ESCUDO, TREMOR,
PILOTO AUTOMÁTICO, COLETÁVEL, FREIO AÉREO, CONTROLES, OPÇÕES, VOLTAR, skin).
The sentences are this project's own. Our own translations of UI text, keyed by
disc string ids, are allowed; verbatim copies of disc text are not
([legal.md](../overview/legal.md)). The English originals are never committed
beside a translation.

Chosen, not measured: "Preta"/"Branca" for a track's black and white runs
(Omega has no such table), "Outdoor" for a billboard, and every word Omega's
table does not carry.

## German (disc language, fills only)

German is a disc language on every title, so it is not a project language: the
disc's own table is the base and **only gaps are filled**.

- `assets/ui/strings/german.toml`: all 128 `OAG_` ids (Race Remix, controls,
  graphics and display settings, loading wording, pilot editor). Matched to the
  discs' own German words (RENNKAMPAGNE, RENNBOX, REKORDE, TEMPOKLASSE, WAFFEN,
  STRECKE, SCHIFF, OBJEKT, LUFTBREMSE, SCHUB, EMPFINDLICHKEIT, SEITLICHE BEWEGUNG
  for a sideshift, KI). `human = false` throughout.
- `assets/ui/strings/disc/<namespace>/german.toml`: disc ids the title's own German
  table leaves empty or lacks, read through `StringTable::fill` (never replaces a
  non-empty disc entry, so Pulse PS2's own ghost words stand while PSP EU, which
  shares the `pulse` namespace, is filled). Disc languages are stamped with the
  title's `disc_strings` in `load_languages`; before 2026-10-06 only a project
  language was, so a disc-language file would never have loaded.

Gaps measured 2026-10-06 (the disc's English id set against its German table):

| Title | Gap ids filled | Left, and why |
| --- | ---: | --- |
| Pulse PSP EU | 2 (`M_SAVE_GHOST`, `HM_SAVE_GHOST`) | `MSC_QUICKBROWNFOX` (font pangram) |
| Pulse PS2 EU | 0 | - |
| Pure PSP EU | 0 | 32 ids: track/music/ship proper names, two glyph ids, and key-spelling siblings (`Awarded`/`awarded`, `Staus`, a doubled `Option->`) the front end never asks for |
| HD | 0 | `MISSING_JAPANESE_CHARS` (font test string) |
| 2048 | 4 (`3D_STRENGTH`, `FE_CAMPSEL_MODES`, `FE_PERCENT_COMPLETE`, `STATS_ONLINE_RACES_LOST`) | other-region legal notices, `fe_hd_livery_normal`, `test_test` |
| Omega | 0 | `MT_01` (a music title, proper name) |

2048 and Omega check: checked, differs in outcome. Omega's German carries the four
2048 ids, so Omega needs no namespace; 2048's wording matches Omega's.
Ground truth: `crates/game/tests/german_ground_truth.rs`.

## Glyph coverage (measured 2026-10-06)

Every face of every title's English plugin was read and checked for
`ãõçáéíóúâêôàÃÕÇÁÉÍÓÚÂÊÔÀ`
(`crates/game/tests/portuguese_br_ground_truth.rs`):

| Title | Missing |
| --- | --- |
| Pulse PSP EU, Pulse PS2 EU | `ç` `Ç` in `PulseHud.fnt` and `small.fnt` (the `HUD`/`HUDSmall` roles); none in `pulse_text`, `Pulse_14`, `Pulse_20` |
| Pure PSP EU | `ç` `Ç` in `HUDFont.fnt` (`HUD`); none elsewhere |
| HD, Omega, 2048 | none (the button-glyph face `PS_BUTTONS.fnt` carries no letters on any title) |

No glyph is invented. `oag_ui::font::Atlas::cell` already falls back to the
**base letter** when a face has no accented glyph, so a missing `ç` draws `c`.
That fallback is **chosen, not measured**, the pre-existing rule for every
language. Pulse's only HUD word with `ç` is `Posição`.

**A new gap, found when the Pure file landed (2026-10-06): the ordinal
indicators.** The disc files write `1º` for the English `1st`. Pure's
`small.fnt` carries no `º`, and its `FX300ANG.fnt` carries no `ª`; before this
change a missing `º` drew as nothing, so `1º` read `1`. `base_letter` now maps
`º` to `o` and `ª` to `a` (chosen, not measured, the same rule as `ç` to `c`),
the lookup also tries the upper-case letter for an all-caps face, and the Pure
file avoids `ª` altogether (`Nova pontuação de zona em 2º lugar`). The ground
truth for Pulse, Pure, HD and 2048 asserts every Portuguese letter of its file draws on every
face of the base language, as itself or as its base letter.

## Counts

Disc English entries (`Entry` rows in the English table, measured 2026-10-06):
Pulse 1,619, Pure 888 (911 with our `OAG_` ids), HD 1,602, 2048 2,965, Omega
2,831 (Omega ships its own pt-BR; no file needed). **All four non-Omega titles
now have a disc-keyed file**; no id is waiting to be written. What is open is a
native-speaker pass: every entry is `human = false`.

## 2048 and Omega check

Checked, differs: Omega ships `portuguesebr` itself, 2048 ships only
`Portuguese` (European; native name `Portugus`, a Latin-1 byte the XML reader
folds). 2048's `Portuguese` is not used as the base for PortugueseBR: it is a
different dialect and the project language stands on 2048's English.

Checked for the disc files (2026-10-06): **ported**. 2048 and Omega share 2,642 ids
(2,559 with non-empty text) and HD shares 1,354 with Omega. The `2048` file
reads its terms from Omega's table and the `hd` file from the `2048` one; a long
text is never byte-identical to Omega's. The reverse direction is not needed:
Omega already ships its own Brazilian Portuguese. The 2048 USA release stands on
`American` (lang-manifests) and the file reads over it id for id; the
EU release (`data/extracted/vita/PCSF00007`) stands on `English` and reads the same
file id for id. Ground truth: `the_2048_disc_text_is_translated_...` (USA) and
`the_2048_eu_release_reads_the_same_file_...` (EU) in
`crates/game/tests/portuguese_br_ground_truth.rs`.

**The `º`/`ª` fallback is shared code** (`oag_ui::font::base_letter`), so it
reaches every language on every title, not only Portuguese: any text with an
ordinal indicator (Spanish, Italian and Portuguese disc tables, Pulse's own
`IG_HUD_1ST` = `1º`) on a face without the glyph now draws `o`/`a` where it drew
nothing.
