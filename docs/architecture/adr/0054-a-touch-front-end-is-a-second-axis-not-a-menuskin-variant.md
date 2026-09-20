# ADR-0054: A touch front end is a second axis on `FrontEnd`, not a `MenuSkin` variant

## Status

Accepted, 2026-09-20. Extends [ADR-0022](0022-title-packages.md) (the title
axis `FrontEnd` sits on) and narrows one part of
[ADR-0025](0025-a-boot-chain-carries-its-provenance.md)'s own reasoning about
`FrontEnd`'s shape - see "Consequences" below for exactly which part.

## Context

`docs/formats/2048-frontend.md` read Wipeout 2048's front end in full: its
boot chain (five screens, corroborated on one Vita3K cold boot), its
seventeen language plugins, its colour globals, and its menu presentation.
The last of these does not fit anywhere. `oag_title::FrontEnd::menu` was
`&'static MenuSkin`, mandatory, built for the vocabulary Pulse, Pure and
Wipeout HD all share and disagree on the numbers of: a `FEGlobals` block
driving a scrolling `<Menu>` list or a `<HorizMenu>` strip. 2048 authors none
of it. Grepped across all 25 `NEWGUI` documents plus the v1.04 patch's 32:
zero hits for `FEGlobals`, `MenuXOffset`, `MenuScale`, `TitleXOffset`,
`TitleColor`, `<Menu>` or `<HorizMenu>`. What it authors instead is two
touch-icon grids (`GameModeChoice`'s four buttons, `Home`'s five) over a
persistent `<FE3DCanvas>` scene, and a 3D ship model inline on its team
screen - a Vita-native touch idiom, not a ported PSP/PS3 menu.

Because `menu` was mandatory, `FrontEnd` as a whole had exactly one way to
express "this front end's menu vocabulary is unlike anything `MenuSkin`
holds": leave `oag_title::Title::front_end` itself `None`. That is what
`docs/formats/2048-frontend.md`'s "Why `front_end` stays `None`" section
did, and it cost more than a menu. `oag_title::FrontEnd::language_plugins`
is real and read - seventeen plugins, each naming a `2048HUD` font role this
title's in-race HUD needs - but nothing could reach it while the field
holding it stayed unreachable behind `front_end: None`. `crates/game/src/
race/hud.rs` and `race/load.rs` fall back to an empty slice whenever
`front_end` is `None`, which is not a bug in either file: it is the correct
reading of a type that had no way to say "the boot chain is real, only the
menu is not."

This is the same trap [ADR-0025](0025-a-boot-chain-carries-its-provenance.md)
solved for Wipeout HD, one field over. HD's layout was fully recovered and
its *order* was not, and `FrontEnd` held both as one inseparable unit, so HD
was briefly forced into `None` too, refusing a title whose menus were ready
to draw. ADR-0025's fix was `BootProfile::provenance`: a field that lets
`chain` be real while saying how much its *order* is worth, rather than
gating the whole struct on the weaker of two measurements. 2048's version of
the same shape is a menu vocabulary that plain does not exist on this title,
not a chain whose order is merely uncertain - so the fix is not a third
`Provenance` value, it is making `menu` answer for itself rather than for
`FrontEnd` as a whole.

Filling `MenuSkin`'s mandatory fields with placeholder numbers to get 2048's
`front_end` to `Some` was considered and rejected before this pass started:
`docs/formats/2048-frontend.md`'s own "Open" section calls it out as exactly
the "plausible-looking stand-in" `CLAUDE.md`'s "never invent what the assets
already author" section forbids - numbers attributed to a disc that states
none of them, for a menu shape (a scrolling list, a horizontal strip) 2048
never draws.

## Decision

**`oag_title::FrontEnd::menu` becomes `Option<&'static MenuSkin>`.** `None`
is a measurement about *this front end's presentation idiom* - "no
`FEGlobals`/`<Menu>`/`<HorizMenu>` vocabulary exists to read" - not a
placeholder for "unread." Pulse, Pure and Wipeout HD all become `Some` of
their existing, unchanged `MenuSkin` constants; nothing about their own
layout numbers moves.

**`FrontEnd` gains a second, independent axis for the vocabulary 2048 does
draw**: `touch: Option<&'static oag_title::TouchFrontEnd>`, a new type in a
new `oag_title::touch` module. It holds exactly what
`docs/formats/2048-frontend.md` measured and nothing it did not:

- `game_mode_choice`/`home`: the nine `<TouchButton>` widgets across both
  grids, each an `id` (the widget's own `idstring`, present on all nine
  where `name` is not - two of `Home`'s five omit it), `x`, `y`, `width`,
  `height` - read directly off `NEWGUI/Definition.xml`, confidence 92.
- `team_screen`/`team_model_origin`: the `team` screen's own `<Model
  name="ShipModel">` origin, `(1400.0, 264.0)` - real evidence 2048 previews
  a mesh here, on the same footing `FrontEnd::preview_meshes` exists to
  carry for Pulse and Pure, but routed through this axis instead because
  `preview_meshes` is defined against `race_box`'s picker, which 2048 does
  not fill.
- `canvas_screen`/`canvas_texture`: **names, not coordinates.** `<FE3DCanvas>`
  carries dozens of `<CanvasLabel>` hotspots, and `2048-frontend.md` measured
  that all 69 in the base file cluster in one small corner of the canvas -
  but that bounding box is a tool computing over the label set
  (`campaign_map_preview.rs`), not a number the disc states anywhere.
  Carrying it in this type would be the hand-transcribed-table failure
  `CLAUDE.md` names, and it would stop being true the moment a label moved.
  This field names the screen and the atlas instead, the same choice
  `FrontEnd::menu_frame`'s own doc comment already makes for the same reason,
  and lets a caller walk the real label list.

`oag-game`'s `boot::load_shell` gains a second refusal beside its existing
"no front end at all" one: a title whose `front_end` is `Some` but whose
`menu` is `None` refuses the *menu-driven* boot by name, pointing at
`--race` instead, rather than either crashing on an `Option` it did not
expect or silently drawing an empty layout. `oag_game::main::session::
placeholder` - the module that already stood in this build's own menu for a
title with no front end at all - now triggers on the same, wider condition
("no `MenuSkin`-shaped menu", not "no front end"), so 2048 keeps working the
same way it did before this ADR from a player's point of view: its own
placeholder menu, not a crash and not a wrong picture.

## Alternatives considered

**A `MenuSkin` variant or enum wrapping either vocabulary.** Rejected. The
two idioms share almost nothing - a touch grid has no `menu_x`/`title_scale`/
`selected_pulse_period_secs` to speak of - so a shared enum would mostly be
one empty arm, and every one of `MenuSkin`'s ~90 call sites across
`crates/ui`/`crates/game` would need to match on it instead of reading a
field. `Option` on two independent fields costs those call sites nothing
extra for the titles that only ever fill one.

**Fold the touch layout into `oag_title::MenuSkin` as more optional fields.**
Rejected on [ADR-0022]'s own bar: `MenuSkin` is licensed because two
disagreeing corpora measured the *same* vocabulary. A touch-icon grid is not
a `MenuSkin` reading with different numbers, it is a different vocabulary
with no numbers in common, and folding it in would make every consumer of
`MenuSkin` (which assumes a `menu_x`/row-pitch shape) responsible for
knowing which sub-vocabulary a given instance actually carries.

**Leave `front_end: None` and accept the `language_plugins` loss.** The
status quo before this ADR, and the reason it stopped being acceptable: a
real, read, seventeen-plugin language table sat unreachable behind a type
that could only say "front end: yes or no," and 2048's in-race HUD font
(`2048HUD`) has been drawing its 5x7 fallback since `2048-hud.md` first
looked, for a reason that had nothing to do with the font being unlocated.

**A confidence score on `menu` instead of `Option`.** Rejected on the same
grounds [ADR-0025](0025-a-boot-chain-carries-its-provenance.md) rejected it
for `Provenance`: the question a consumer asks is binary - is there a
`MenuSkin` to draw, or not - and a number invites an arithmetic nobody
defined. The rubric score belongs on `2048-frontend.md`, where it is.

## Consequences

- **`oag_title::FrontEnd::menu` is `Option` for the first time**, which
  narrows the "why the two are one field" reasoning in `FrontEnd`'s own doc
  comment: a chain with a skin that cannot be honestly filled (2048) is now
  a real, constructible state, where "a skin with no chain" stays impossible
  because `FrontEnd::boot` is still mandatory. `oag-game` needs exactly two
  refusals now instead of one - "no front end at all" and "no `MenuSkin`-
  shaped menu" - both by name, neither a crash.
- **2048's HUD font bug closes as a side effect.** `race/hud.rs`/`race/
  load.rs` read `front_end.language_plugins` through the same `Option`
  handling they always used; once `front_end` is `Some`, the seventeen
  plugins reach the loader for the first time, and `2048HUD`'s `.fnt`/`.gxt`
  pair - located since `2048-hud.md` but never wired to anything - is what a
  race actually draws with.
- **`oag_title::touch` is a single-corpus type, deliberately.** Wipeout 2048
  is the only title in this lineage with a touch-icon front end, so
  `TouchFrontEnd` does not clear ADR-0022's own two-corpora bar the way
  `MenuSkin` did. It exists anyway because `FrontEnd` already has to answer
  *some* way for every title it is `Some` on, the same argument that put
  `BootProfile` on `Title` rather than selected apart from it - and because
  `Option` costs the other three titles nothing. A second touch-driven title
  would be the test of whether this shape generalises; none exists yet.
- **`~90` call sites across `crates/ui` and `crates/game` were audited for
  `.menu`.** The great majority read a *different* `.menu` entirely -
  `Screen::menu` (a parsed `<Menu>` widget) and `Menu`-stage state
  (`stage.menu`) share the field name by coincidence, not by type. Five
  production sites in `crates/game/src/boot.rs` and roughly two dozen test
  sites that read `oag_pulse::FRONT_END.menu`/`oag_pure::FRONT_END.menu`/
  `oag_hd::frontend::FRONT_END.menu` directly needed a change - an
  `.expect`/`.unwrap()` or a new local binding - because all three titles do
  still author a `MenuSkin`, so every one of those reads is exactly as valid
  as it always was, just through one more `Option`.
- **`docs/formats/2048-frontend.md`'s "Why `front_end` stays `None`" section
  is now describing history**, not the current state - the page gets a dated
  addendum rather than an edit, per this project's rule that a page records
  what was found and when, not a single always-current answer.

[ADR-0022]: 0022-title-packages.md
