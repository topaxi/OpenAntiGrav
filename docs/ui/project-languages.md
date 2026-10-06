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
- **Omega already ships Brazilian Portuguese.** A disc language of the same name
  (case-insensitive) suppresses the project one, so Omega's own
  `Languages\portuguesebr` plugin is the base and only our `OAG_` ids are laid
  over it. Its disc table is never replaced by ours.

## Files

| File | Holds | Read by |
| --- | --- | --- |
| `assets/ui/strings/portuguesebr.toml` | this project's own `OAG_` ids | every title |
| `assets/ui/strings/disc/pulse/portuguesebr.toml` | our translation of **Pulse's** disc text, keyed by Pulse's idstrings (1,560 of 1,619) | titles whose `FrontEnd::disc_strings` is `Some("pulse")` |

The disc-keyed file is **per title on purpose**: Pure, HD and 2048 reuse id
spellings, and `overlay` does not know which title it runs on, so one shared
file would turn Pulse's wording on for them. The namespace is `Title` data
(`oag_title::FrontEnd::disc_strings`, ADR-0058), not a `title.name` branch.
Pure, HD, 2048 and Omega are `None` and read the disc's English until their
file is written.

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

## Counts for the next lanes

Disc English entries (`Entry` rows in the English table, measured 2026-10-06):
Pulse 1,619 (done, minus 59 proper names/glyphs/identical), Pure 911, HD about
1,602, 2048 2,965, Omega 2,831 (Omega ships its own pt-BR; no lane needed).
Pure, HD and 2048 are later lanes: one `assets/ui/strings/disc/<namespace>/
portuguesebr.toml` each, plus the title's `disc_strings` value.

## 2048 and Omega check

Checked, differs: Omega ships `portuguesebr` itself, 2048 ships only
`Portuguese` (European; native name `Portugus`, a Latin-1 byte the XML reader
folds). 2048's `Portuguese` is not used as the base for PortugueseBR: it is a
different dialect and the project language stands on 2048's English.
