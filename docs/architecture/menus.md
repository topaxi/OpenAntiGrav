# The menus

**Ours, not the disc's.** Every other document in this tree describes something
recovered from a Wipeout Pulse executable or a Wipeout Pulse asset. This one
describes a thing this project invented, and says why inventing it was the right
call rather than a shortcut.

Implemented in [`crates/game/src/menu.rs`](../../crates/ui/src/menu.rs), with
the tree itself in [`assets/ui/menu.toml`](../../assets/ui/menu.toml) and the
disc-side lists in [`crates/raceplay/src/catalogue.rs`](../../crates/raceplay/src/catalogue.rs).

## Why not the front-end XML

The disc carries `Data\Plugins\PI001\GUI\MainMenu_Definition.xml`, and
[`screen.rs`](../../crates/ui/src/screen.rs) already parses that dialect - the
language picker is drawn from it. Reproducing the main menu from it is
technically the shorter path, and it is still the wrong one.

The original's menus are the menus *Pulse* has: on a PSP, for a player with a
Memory Stick, with no mouse, no window, no monitor to pick a refresh rate for
and no keyboard to rebind. This project needs the menus a PC game has. Extending
a recovered tree with branches the original never had produces a structure that
is neither faithful nor ours, and - worse for a project whose main deliverable is
its documentation - **every entry we add starts to look like a recovered one.**
A reader six months from now cannot tell which rows came off the disc.

So the tree is ours and it is written down in one file that says so.

**The disc's own menu XML has since been read**, and it went where this page
said it would: [front-end menu definitions](../formats/fe-menu-definitions.md).
That changes nothing above. The argument here was always about *structure* -
which rows exist, and what a reader six months on can tell apart - and reading
the file does not make the disc's row list any more the right one for a game
with a monitor and a keyboard.

What it did change is everything below the structure. See
[the original's menus](../ui/menus-original.md) for where the rows sit, what
colour they are, and how a page change moves; the numbers now come from the
title's own table rather than from round numbers picked to be legible. The
boundary is worth stating in one line, because it is the thing a reader will
want:

> **The tree is ours. The presentation is the disc's.**

**And "presentation" turned out to include the axis.** Wipeout HD's main menu is
a `<HorizMenu>` - horizontal - where both PSP titles' are a column, so a build
that drew every title's root page as a stack of rows was drawing one of them a
way its own disc authors nowhere. `oag_title::MenuStrip` is that axis and
`crates/game/src/menu/strip.rs` draws it; the tree it arranges is still the one
below. See [hd-frontend](../formats/hd-frontend.md#the-main-menu-is-horizontal-and-it-is-drawn-that-way-now)
for the census that licensed the field, for which pages this build applies it to
and why that rule is ours, and for the two figures inside a strip that are ours
because the widget states neither.

**And the page is drawn inside the disc's own frame.** What a menu is cleared
to, the rules above and below it and the mark in the corner are all authored on
one screen - HD's `FE Screen` - so the title package names *the screen*
(`oag_title::FrontEnd::menu_frame`) and `crates/game/src/menu/frame.rs` reads the
widgets off it at boot. Nothing here holds a coordinate or a colour, which is
what keeps it right when the disc says something unexpected: HD's `HD_*` palette
turned out to be its **FE style**, black-and-red in one archive and
white-and-teal in another, and a frame read at runtime resolves to whichever
archive the boot served rather than to whichever one a person happened to open.
**Pulse's frame is read too**, since `oag_pulse::FRONT_END` started naming
`FE Screen`: a top bar at `y=0`, two footer strips at `y=236`/`y=249`, and -
first in document order - a full-screen `gameshare_backdrop.mip`. That last one
is a *background*, not chrome, which is what
[the ordering rule below](#a-full-screen-mark-is-a-backdrop-not-chrome) exists
to say. Wipeout Pure names the same screen; Wipeout HD's is the `FE Screen`
described above.

## What still comes off the disc

**Two whole screens do, since 2026-09-09: the race box's Track Select and
Ship Select.** They are not pages of the tree below - `menu.toml` does not
name them - but the disc's own `Track Creation` and `Team Selection`, read
off `Selection_Definition.xml` widget by widget and opened over the RACE
page by its START row, each writing the same `race.*` setting its RACE-page
row edits. The tree is still ours: which rows exist and where START leads is
this file's call, and a title whose front end authors no such screen
launches from START directly. See
[selection-screens.md](../ui/selection-screens.md) for what they draw and
[`oag_ui_screens::picker`](../../crates/ui-screens/src/picker.rs) for how.

Not the *structure*, but the *contents*, wherever the contents are a property of
the release rather than of this project:

| Row | Where its options come from |
| --- | --- |
| TRACK | `Data\Plugins\PI001\Definition.xml`, and the label from the language's string table |
| LANGUAGE | the language plugins `PI008`-`PI012` |
| TEAM | `values_from = "teams"` - the roster [`boot::load_teams`](../../crates/game/src/boot/roster.rs) read off the open source's own declared definition plus any mounted pack, not a repository list |
| SPEED CLASS | `values_from = "speed_classes"` - the booted title's own per-team `handlingstats.xml` ladder, off `oag_title::SpeedClasses`, **narrowed to `is_offered_outside_remix`**: four rungs, always, even on a Wipeout Pure boot whose own ladder authors five. On the RACE REMIX page it is `values_from = "remix_speed_classes"` instead: the **union** across every title this machine can open a source for, filtered only by `is_selectable`. Four rungs on Pulse and HD, **five on Wipeout Pure**, whose `VECTOR` sits below `VENOM` - so the remix row is five exactly when a Pure source is mounted, and no flag implements that. See [below](#speed-class-vector-is-confined-to-race-remix) for why the two pages disagree |
| MONITOR | winit's own monitor list, read every time the menus open |
| WINDOW MODE / SIZE / ASPECT / RENDER SCALE / RECONSTRUCTION / MSAA / UPSCALER SHARPNESS / BRIGHTNESS / GAMMA / FIELD OF VIEW | `oag_display::display`, pinned to its own `ALL`/`OFFERED` lists by a test |
| PERFORMANCE OVERLAY / FRAME LIMIT / VSYNC | `oag_present::perf`, pinned the same way |

MONITOR is the one supplied row that is a property of **the desk** rather than
of the disc, and it is supplied for the same reason the others are: a definition
file cannot know it, and a row offering a screen that is not plugged in would be
offering something that cannot be selected. It stores a **name**, never an
index - a compositor is free to enumerate screens in whatever order it woke up
in, and an index would silently move the game somewhere else. A name that is no
longer there falls back to the default with a note; `default` is always first on
the list, because it is the way back from a screen that has since been unplugged.

That split is what keeps [ADR-0006](adr/0006-no-copyrighted-content.md)
satisfied while the Race page still says "Talon's Junction White": the words are
read out of the player's own disc at runtime and this repository contains a
plugin id at most.

**A circuit is an entry, not a directory**, and getting that wrong is the trap
this arrangement exists to avoid. `16_Track` and `32_Track` are two rows on the
menu and one folder on the disc - the second is the first driven the other way,
distinguished by `Reversed="True"` on its `Values` node. Listing
`Data\Environments\*` would therefore show twelve circuits where the game offers
twenty-four, and no amount of path arithmetic recovers the difference. The
settings file stores the plugin id and only the source can say which `.vex` that
id loads.

## Unlocks: Pulse's circuits and craft variants gate

**Circuits (Pulse): gated, since 2026-10-02.** A profile now exists
(`records.toml` keeps campaign medals and per-team loyalty), so Pulse's Track
Select offers a circuit only when its `<Unlock Grid="...">` is met - 3 on a
fresh profile, as the original's, up to all 24. A locked circuit is **absent**
from the list rather than greyed (`TrackSelection_PopulateList` filters). The
mechanism is `oag_game::unlock::Gate`; `--unlock-all` lifts it and is ours, a
developer and capture escape. The original's mode-gated per-track byte
(`+0x16e`) is not modelled and stays open. Every other title's race box still
offers everything: Pure authors no `<Unlock>`, HD and Omega are not wired.

**Craft variants (Pulse): gated, since 2026-10-02.** Each `PI_ModelSkin` and
`PI_TeamModel` carries an own-team and a `Team="any"` `Exclusive` loyalty row;
`Definition_IsUnlocked` ORs them, and `any` is met by the best single team's
total, not the sum (`race-box-screens.md`, "Traced 2026-10-02"). Team
Selection's livery row therefore offers `Classic` plus only the skins the
profile's loyalty has earned, a locked one **absent** as in the original
(`oag_game::unlock::loyalty_unlocked`), and the RACE page's VARIANT row drops
`Concept` the same way (`gated_variant_choices`). `--unlock-all` lifts both.
RACE REMIX's craft-side variant row is ungated by decision: RACE REMIX is
this project's own mode, so no disc unlock applies to it (maintainer,
2026-10-02).

### What the originals actually do, because the contrast is the point

Two gates, on two different axes, and a single "unlocked" flag models neither.

- **Circuits gate on a named campaign grid.** In Pulse's
  `Data\Plugins\PI001\Definition.xml`, a `<PI_Track>` either carries an
  `<Unlock Grid="...">` child or it does not, and the `Grid` attribute holds a
  **name** (`Grid0`, `Grid1`, ...) rather than an integer. **Exactly three
  `PI_Track` entries carry no `<Unlock>` at all** - `16_Track`, `03_Track` and
  `18_Track` - so Pulse's Custom Race track select opens as a wrapping list of
  three entries, against the twenty-four the same file declares. Confidence 95.
- **Craft variants gate on per-team loyalty, which is not campaign progress at
  all.** Every `PI_TeamModel` / `PI_ModelSkin` leaf carries *two* `<Unlock>`
  rows, both `Exclusive="true"`: a cheap own-team price and an expensive
  `Team="any"` one (`Alternative` 4,000/60,000 up to `Concept`
  25,000/100,000). That second axis is what `Team Selection`'s `Loyalty` bar
  displays. Confidence 95.

So the shipped game's answer to "what may I race" is two independent
progression systems, not one flag - which is precisely why standing in for them
with a flag would be worse than not gating at all.

**Wipeout Pure already agrees with us**, and that is worth recording because it
shows the ungated shape is not unheard of in the lineage: Pure authors **zero**
`<Unlock>`, zero `Grid=` and zero `GSDisableEntriesBitField` across all eleven
of its front-end definition files, and its `Show Unlocks` subtree is a
post-race reward reveal rather than a gate. Confidence 94.

## SPEED CLASS: `VECTOR` is confined to RACE REMIX

**This is also a divergence from what the disc does, and it is a decision
rather than a gap.** Wipeout Pure's own front end authors **five**
`<Menu name="Class">` entries - `VECTOR`, `VENOM`, `FLASH`, `RAPIER`,
`PHANTOM` - confidence 94, and this build's engine can name and race every one
of them: see
[handling-stats.md](../formats/handling-stats.md#pures-fifth-rung-is-raceable-and-speedclass-still-has-four-variants)
for how `Stats::class_named`, `Global::class_named` and
`oag_weapons::pickup::table_for` resolve a rung by the name the disc spells,
with no enum widened to fit it. Despite that, **the ordinary per-title RACE
page offers four**, even when the booted title is Wipeout Pure - `VECTOR` is
offered on the RACE REMIX page only.

The mechanism is `oag_title::SpeedClasses::is_offered_outside_remix`, layered
on top of `is_selectable` rather than replacing it: `is_selectable` still says
`true` for `VECTOR`, which is what keeps RACE REMIX's union, `oag-trace`'s
`--class` spell-check and every resolution path exactly as they were. Only the
one menu row - the ordinary RACE page's `speed_classes` supply - asks the
narrower question and drops the one name that fails it.

**Why offer four when Pure authors five:** asked whether a Pure-only boot
offering five classes on the ordinary RACE page (a side effect of making
`VECTOR` selectable at all, wider than what had been asked for) was wanted,
the maintainer's answer was **"Confined to remix."** That is the whole of the
reasoning - the same kind of call as the unlocks section above, made by the
person who gets to make it, and it is recorded here for the same reason: so
the four-rung RACE page reads as this decision to the next contributor and
not as an unimplemented feature or a bug to "fix" back to five.

**The row shrinking is not enough by itself, because `race.class` is one
setting shared with RACE REMIX.** `Session::launch_remix` reads the exact same
`race.class` string RACE's own `LaunchRace` handler does, and RACE REMIX's own
row still offers `VECTOR` when Pure is mounted - so settling it to `"vector"`
there and then opening the ordinary RACE page leaves a stored value the RACE
page's own row no longer lists. `Menu::supply` only ever moves the widget's
*display* index, never the stored setting, so without a further check the row
would show `"venom"` while the race launched on `"vector"` underneath it - a
menu saying one thing and a race doing another, which is exactly the silent
mismatch this project's menus forbid. `resolve_race_page_class`
(`crates/game/src/main/session/menus.rs`) closes that: the ordinary RACE
page's launch clamps a stored class the row does not offer to the row's own
first entry, the same fallback CIRCUIT and TEAM already use for a stored value
their own source does not have.

**The clamp deliberately never writes back to `settings.race.class`.** It
changes only the `race::Options` this one launch builds, so a `"vector"`
settled on RACE REMIX survives in storage and a visit to the ordinary RACE
page warns and clamps again on every launch from that state rather than
silently adopting `"venom"` as the player's new saved choice. Writing back
would clobber the remix pick the moment the player so much as glanced at the
ordinary page - the clamp is a per-launch correction, not a migration.

## The format

A **flat list of pages keyed by id**, not a nested tree. Three consequences, all
of them the reason:

- TOML past two levels of nesting is unpleasant to hand-edit, and this file is
  meant to be hand-edited.
- The back stack becomes a `Vec<usize>` of page indices, which cannot hold a
  dangling page.
- "Every target resolves, every action is known, every page is reachable" is one
  pass over a flat map.

```toml
version = 1
root = "main"

[[page]]
id = "main"
title = "OPENANTIGRAV"

[[page.entry]]
kind = "submenu"
label = "OPTIONS"
target = "options"
```

Six entry kinds: `submenu`, `action`, `choice`, `toggle`, `binding`, `back`.

### One row disabling another

A `choice` or a `toggle` may carry `disabled_by`, naming **another row's setting
and the value that makes this one inert**:

```toml
[[page.entry]]
kind = "choice"
label = "FRAME LIMIT"
setting = "display.frame_limit"
disabled_by = { setting = "display.vsync", value = "on" }
```

**A value and not just a setting**, and the one place it is used is why. VSYNC
has three modes and only the middle one makes a frame limit meaningless - see
below - so a condition that could only say "while that toggle is on" would have
been wrong here.

This is the **only** cross-row logic in the menus, and it stays inside the rule
that this module knows nothing about what a setting means: the answer is read
off the other *row*, not out of the settings file. `menu.rs` has not heard of
vsync.

**The front end runs under a cap derived from FRAME LIMIT** (maintainer's rule,
2026-10-07; **chosen, not measured**), `oag_present::perf::FrameLimit::front_end`:
unlimited or 120 and above gives 120, 60 up to 119 gives 60, below 60 keeps the
limit. Exactly 120 counts as "120 and above". The launcher, boot reel, loading
wave, menus and the in-race pause menu use it (the pause menu is a `Stage::Menu`
over a held picture with the race parked, so no race scene is drawn behind it);
every `Stage::Race`, including a race still building, keeps the limit as
configured. Under VSYNC on nothing is consulted, so the cap is inert there. The
session logs `frame limit in force: N (configured M)` on every switch.

A disabled row is **greyed, not hidden**. Hiding it changes the row count under
the cursor, and a player looking for a setting that has silently vanished has no
way to find out what took it away. It still takes the cursor, still shows its
value, and does not move; `adjust` returns no event, so a keypress on it cannot
persist a change nobody made.

Three things the loader refuses, each with a test: a `disabled_by` on a kind
that cannot be adjusted (it would parse and do nothing), one naming a setting no
row edits, and - the one that catches a typo - **one naming a value the deciding
row cannot hold**. `value = "onn"` is a startup error rather than a row that
quietly never greys out. A row whose list comes off a disc has no values yet at
load, and there the check stops at existence rather than guessing.

#### What `disabled_by` deliberately cannot express

It names a **setting**, and `Definition::check_condition` refuses one that no
row edits. So a row cannot be greyed by anything that is not itself a row - and
the case that makes the boundary worth writing down is a **runtime hardware
capability**.

TARGET FPS is the example. On an adapter with no `TIMESTAMP_QUERY`
there is no cost signal, so the controller never moves the render extent
whatever the row says. That is exactly the shape `disabled_by` is drawn for and
exactly the thing it cannot say, because "this adapter has a GPU timer" is not
a setting and no row edits it. Three answers were available: a runtime
`values_from` list holding `off` alone, which is the RENDERER precedent and
would make the row unmovable; a `restart_required`-style note, whose meaning -
*stored, and nothing on this machine will act on it* - is already exactly true
here; or nothing in the menu at all, with the controller returning the ceiling.
The last two were taken.

An earlier version of `docs/rendering/dynamic-resolution.md` claimed
`disabled_by` covered this. It was written before anyone read
`check_condition`, and this paragraph is the correction of record.

**`action` and `values_from` are closed sets**, checked at load against enums in
`menu.rs`. A typo in the asset is a startup error naming what it did know, not a
row that does nothing when pressed. Adding an action means adding it to the enum
*and* handling it in the composition root, which is the point: "defined but not
wired up" cannot ship.

### What the loader refuses

Each has a test, and the last is the only one that cannot be caught a row at a
time:

| Refused | Why it matters |
| --- | --- |
| An unknown `version` | A definition written against a different entry vocabulary would otherwise load with rows silently missing |
| A duplicate page id | A `target` would be ambiguous |
| A dangling `target` | |
| An unknown `action` | Reported with the set it does know |
| An unknown `button` | |
| `values` **and** `values_from` | A list has to come from exactly one place, or it is undecidable which wins when the source turns out to be empty |
| Neither `values` nor `values_from` | A choice with nothing to choose |
| A `restart_required` with an empty message, or on a kind that cannot be adjusted | A marker with nothing under it says something is wrong and not what to do about it; on a `back` row it would parse and do nothing |
| A page nothing links to | A menu somebody wrote and forgot to hang off anything. Parses, resolves, and ships silently without this |

Three more tests pin the asset against the code's own lists, which is the class
of mistake the format check cannot see:

- The TEAM row must carry no spelled-out `values` at all - the roster is what
  the open source offers plus what a mounted pack adds, and no list in this
  repository can know either, so `values_from` is the only legal source for
  it - and every CLASS value must parse as `oag_physics::SpeedClass`. A
  spelled team would fail deep inside an archive lookup at race load, with a
  message about a missing WAD entry rather than about a menu; see
  `the_race_page_offers_only_teams_and_classes_the_game_accepts` in
  [`crates/game/src/menu/tests/definition.rs`](../../crates/game/tests/menu_definition_settings.rs).
- Every value on DISPLAY and GRAPHICS must parse, and each list must *be* its
  type's own `ALL`/`OFFERED`. A value that does not parse is ignored at runtime
  with a message on stderr, which a player meets as "this row does nothing".
- **Every row on those two pages must be one `settings::menu_seeds` fills in**,
  which is the one that catches a setting renamed on one side of the split and
  not the other. A row the seeds do not know opens on its list's first option
  instead of the player's own value - and since the seeds and the composition
  root's `apply_setting` are written against the same key strings, it catches a
  row nothing applies too.

**All of this runs in CI**, unlike most of this repository's interesting tests,
because the thing under test is ours and needs no disc image.

## The seams

The menus never read or write settings, and never touch a disc. Four narrow
calls carry everything:

- `Menu::supply(source, options)` - what a row *may* be set to, for lists that
  come off a disc. **A row keeps the value it was already on** if the new list
  still has it. Resetting to the first option instead is the bug this rule
  exists to stop: a row seeded to the player's language and then handed its list
  would draw the wrong one, and the first nudge of it would persist that as a
  deliberate choice.
- `Menu::seed(setting, value)` - what a row *is* set to. A value the row does not
  offer is ignored rather than added, so a stale config file cannot smuggle an
  unreachable option onto the list.
- `Menu::in_effect(setting, values)` - what the game is *doing*, for a
  `restart_required` row. The one thing `seed` cannot stand in for: it reads the
  settings file, and this is the setting where the file and the running build
  are allowed to disagree. Returns whether any row took it, so a key nothing
  defers is a line on stderr rather than a note that never appears.
- `MenuEvent::Changed { setting, value }` - what the player moved. The
  composition root applies and persists it.

A `choice` option carries a stored `value` and a displayed `label`, and they are
different strings exactly when the list came off a disc: `16_Track` against
"Talon's Junction White". That distinction is what lets the menus name a circuit
without this repository containing its name.

## The one screen that is not a menu

The [disc chooser](../tools/oag-game.md#finding-the-disc-image) has rows, a
cursor and the same abstract buttons, and it is deliberately **not** a
`menu::Menu`. It runs before any archive is open, so it has no title - and
therefore no `oag_title::MenuSkin` to lay it out with and no disc font to draw
it in. Building it as a menu would mean choosing a skin in order to choose
which disc the skin should come from. It is its own small module,
`oag_game::launcher`: one list, one cursor, the engine's own 5x7 glyphs, and a
`Vec<Draw>` like everything else here.

Nothing else should follow it out of `menu.rs`. The test is whether a screen
can have a title behind it, and every screen but this one can.

## Rebinding

`oag_input::keys::map_key` used to be the hardcoded `match` a `binding` row
could only display, never change. `oag_input::bindings::Bindings` is the table
now: a permutation over `keys::candidates`'s eighteen fixed keys, resolved by
`Keyboard`/`Controls` in place of the free `map_key` function, and persisted
complete in `[controls] bindings` - see `settings::Controls::bindings`'s own
doc comment for the format and its fallback rules.

Confirming a `binding` row (Cross or Start) does not go through
`Menu::activate`, which still treats it as a no-op: a rebind needs the literal
key just pressed, and that key is gone by the time an event reaches this
crate's abstract buttons, having already been resolved - or, for a key nothing
is bound to yet, dropped - by `Controls::set_key`. `Session::maybe_begin_binding`
catches the confirm press upstream of `Menu::update`, off the same shared
`Input` the front end's own skip-intro press reads directly; the next raw
keyboard event then goes to `crate::rebind::decide` instead of
`Controls::set_key`, which either binds the key, cancels the capture (Escape,
not the menus' own back key) or leaves the capture open for anything else,
including a key off the closed candidate set.

**Confirming a row now says so.** While `Session::awaiting_binding` is `Some`,
`Session::draw` calls `crate::rebind::prompt` - `OAG_BINDING_CAPTURE_PROMPT`
resolved against the button's own `Display` name, upper-cased (`CROSS`,
`CIRCLE`, and so on - `rebind::prompt` does `button.to_string()`, the
button's own name, not the row's `label`) - and hands the result to
`MenuStage::render` as one more
parameter, the same way
`bound_keys` and `frozen_race` already cross that seam: this stage holds no
session state of its own, so anything it draws that depends on one has to
arrive already resolved. `MenuStage::render` draws it last, over everything
else - even the pause overlay and the pilot-editor prompt, neither of which
can actually be `Some` at the same time as a capture the way this build
reaches the menus today, but there is no invariant enforcing that - with
`oag_game::prompt::message_draw`, a small third shape `crate::prompt` now
draws alongside `Keyboard` and `Confirm`: a status line with no input model
of its own, since the raw key that resolves a capture is decided upstream of
the whole crate, in `crate::rebind::decide`.
It reuses `crate::prompt`'s own `SCRIM` tint rather than a colour of its own -
**chosen, not authored, the same footing `PAUSE_OVERLAY` and `SCRIM` are
already on**: neither PSP title's front-end XML defines a rebind prompt to
read one off, and there is nothing else on the disc to check either - a PSP
has no keyboard, so a raw-key capture is not a feature the original has a
version of at all, on-screen or otherwise (see "What this does not touch"
below for the one rebinding the original *does* have, which is a different
axis). See `OAG_BINDING_CAPTURE_PROMPT`'s own doc comment in
`assets/ui/strings/english.toml`.

This landed as a `main/`-side change rather than the `menu.rs` one this
section used to say was blocked, exactly per the option this paragraph named
before it landed: `MenuStage::render` (in `crates/game/src/main/menu_stage.rs`,
not baselined and nowhere near its own 1,000-line ceiling) gained the
parameter, and `crate::prompt::message_draw` (in `crates/game/src/prompt.rs`,
also not baselined) is the shared drawing code - `crates/game/src/menu.rs`,
which really does sit at its own `BASELINE` ceiling with no headroom, was
never touched.

`crate::rebind::prompt` sits in `crates/game/src/main/rebind.rs`, next to
`decide`, and for the same reason `decide` is a free function at all rather
than inline in `app.rs`'s winit handler: `Session::draw` needs a live `Gpu` to
reach, so the one piece of the lookup that is pure - a button and an
`Option<&StringTable>` in, a line of text out - lives here under its own unit
tests rather than only inside the closure nothing can drive headlessly.

Both the button-name text and the layout it is drawn with are reachable
headlessly, but through two different mechanisms, worth keeping straight: the
*text* is `crate::rebind::prompt` under its own unit tests (a table entry
overriding English and still substituting; no table falling back to English
and still naming the button). The *drawing* - the scrim, the centring, the
wrap - is `oag_game::prompt::message_draw`, and `--menu-page controls
--menu-prompt binding` reaches that same function through
`crate::capture::menu_page::prompt_draws`'s own new `"binding"` arm, off
whichever row's `button` the CONTROLS page opens on first (not off
`crate::rebind::prompt`, since a still capture opens no row and confirms no
key) - so a live capture and this flag cannot draw two different *layouts*
for the same text, the same guarantee `rename`/`rename-note`/`delete`/
`delete-built-in` already gave the pilot editor's prompts, even though the two
paths derive the text itself independently.

Three choices worth stating outright, since none of them are the only
defensible one:

- **The table lives in `oag-input`, not in `oag-gameplay`.** `oag-input` is
  what already owns the abstract button layer, per the crate table in
  `CLAUDE.md`, and rule 1 of `docs/architecture/workspace-layout.md` forbids
  the simulation depending on it at all - a rebindable table is exactly the
  kind of input-system state that rule exists to keep out of `InputSnapshot`'s
  producer rather than its consumer.
- **A conflict is a steal, not a refusal.** `Bindings::resolve` returns one
  `Option<Button>`, so a key cannot mean two things; refusing a rebind that
  would collide would dead-end a player against eighteen keys and twelve
  buttons with no way to free one up except finding the other row first.
  `Bindings::rebind` also **replaces** the target button's whole key list
  rather than adding to it - there is no menu affordance to remove one key at
  a time, so appending would only ever grow a row's display. A button a rebind
  leaves with nothing is a real state and not a failure: `keys::bound_keys`'s
  own doc comment already said so before rebinding existed, empty being what a
  fresh WASD layout draws for `Button::Any`.
- **An entry `Bindings::from_pairs` cannot parse - an unknown key name, or a
  value that is neither a button name nor the `"none"` sentinel - falls back
  to that key's default and is reported, never a load failure.** The same
  tolerance `main::args`'s `resolve_scheme`/`resolve_triggers` already give
  every other control token, for the same reason: a settings file travels
  between builds, and a row a newer build added should not stop an older one
  from booting.

**What this does not touch**: the original's own `Control_Type = "custom"`
path (`ControlMapping2`, see
`docs/ghidra/functions/psp-pulse-usa/input-bindings.md`) rebinds *which
abstract button each of its eight actions fires* - a PSP gamepad's buttons
onto the eight actions, not a keyboard's keys onto twelve abstract buttons.
That is a different axis with no menu row here, and building one is not
implied by anything above.

### The row set names what a button actually does

Every `binding` row's label was checked against
`oag_gameplay::controls::ship_controls`, `Race::spend_pickup`
(`crates/raceplay/src/weapons.rs`, the two weapon buttons) and
`oag_gameplay::input` (the button/`InputSnapshot` wiring) rather than
trusted by eye, after `circle`'s row was found reading "BRAKE" when circle
does not brake - there is no brake action anywhere in `ship_controls`,
braking being the airbrakes (`l`/`r`) already on their own rows. Circle
absorbs a pickup and `square`, which had no row at all, fires one; both now
read "ABSORB PICKUP" and "FIRE PICKUP", matching the verbs
`OAG_HINTS_RACE_KEYS` already used for the same two buttons. `cross`
(thrust), `left`/`right` (steer), `up`/`down` (pitch) and `l`/`r` (airbrakes)
were all confirmed correct against the same functions.

The check also went the other way - not just "does every row say the truth"
but "does every recovered action have a row". `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`
(confidence 90) names eight original actions; **seven have a row here and
one does not.** Action 3, `OPT_CTRL_LBACK` ("look back"), is bound to
`TRIANGLE`, and `Button::Triangle` genuinely reaches nothing beyond the
button-name tables, the pad mapping and its default keyboard key (`V`) -
grepped directly, no consumer anywhere in `oag_gameplay`, `oag_physics` or
`oag_ai`. So there is no row for it because there is nothing yet for a row
to name; a LOOK BACK row bound to a button nothing reads would be exactly
the "plausible-looking stand-in" this project's own root doc warns against,
not a fix. `steer`/`pitch` are not among the eight actions at all (they are
the original's analog axes, not action-table entries), which is why this
page carries ten `binding` rows against eight recovered actions rather than
a one-to-one count.

Two rows this page deliberately does not add:

- **No `start` (pause) row.** Unlike the camera cycle - added above for
  being "the one control a player has no other way to discover" - pause is a
  near-universal convention a player finds without a hint, it is a
  system-level action rather than a driving or weapon control, and it is
  already named in the general key hints (`OAG_HINTS_RACE_KEYS`'s "space or
  start pauses"). Adding a row would not fix a discoverability gap the way
  FIRE PICKUP/ABSORB PICKUP did.
- **No separate sideshift-button row.** The SIDESHIFT row already on this
  page picks the *gesture* (`controls.scheme`, veteran double-tap vs. novice
  hold-and-flick, see `oag_gameplay::controls::ControlScheme`'s own doc
  comment) - it is not a key binding and needs no key of its own. The
  physical button the gesture uses either way is `l` or `r`, which already
  has its own AIRBRAKE LEFT/AIRBRAKE RIGHT row; a third row naming the same
  key for the same gesture would repeat information rather than add it.

## AI PILOTS: the axis preview

The AI PILOTS page's own `AXIS`/`LOW`/`HIGH` rows show a raw axis name and two
numbers - `commitment`, `0.93`, `1.05` - and nothing on that page says what
`commitment` *is* or why `1.05` is close to a wall. `pilots::axis_preview_for`
(`crates/raceplay/src/pilots.rs`) turns whichever axis `AXIS` is currently on into
one plain-language line, read live off the row the same way `session::
pilot_editor::held_text` reads `pilot.axis` for saving - so it tracks the
player without a second copy of "which axis is this."

**Chosen, not measured, and carries no confidence score.** The original has
no pilots and no such preview; every word is this project's own gloss on
numbers [`Pilot`](../../crates/ai/src/pilot.rs)'s own doc comments and
`pilots::limit` already state - the mechanism, the unit, and the allowed
range - never an invented scale or a claim about what a number "feels like"
in play. Where the source says only "how readily" with no stated direction
(`trigger`), the line says the same, in fewer words, rather than guessing a
sign. `pilots::axis_gloss` is the per-axis table; `axis_names_cover_every_draw`
already checks `AXES` against `Pilot::spans`, and `every_axis_has_a_preview`
is its sibling for this table specifically, so a landed axis with no line here
fails a test instead of just having a gap. `lean` is deliberately absent: it
is a `Lean`, not a `Span`, `AXIS` can never select it, and a line about a row
that cannot be reached would be a preview of nothing on screen.

**One function reached from two crates, because the two draw paths are in
different ones.** `axis_preview_for` lives in the library crate
(`oag_raceplay::pilots`) rather than beside `session::pilot_editor`'s other pure
functions, which are the binary's: `MenuStage::render` (binary,
`main/menu_stage.rs`) and `capture::menu_page` (library, for
`--menu-page pilots`) both need the same line, and the library cannot depend
on the binary. `prompt::axis_preview_draw` (`crates/game/src/prompt.rs`) is
the matching draw-side split - not modal, drawn straight over the page, in the
established amber `NOTE` colour a live remark already uses for
`Keyboard`'s own note.

**Its position is `menu::rows::draw`'s own reserved note slot, and that slot
now knows where the chrome actually starts.** A first capture of this page
put the preview line squarely on top of Pulse's own bottom-of-screen mark
(`frame.marks`, drawn on every page regardless of content) - legible, amber
against both the mark's dark and bright halves, but visibly cluttered. Found
by capturing the real page and looking, the way this project's own rule says
to, not by the arithmetic passing. The first fix was a tuned scale and an
upward nudge in `axis_preview_draw`, scoped to this one page - the slot
itself, `menu::visible_rows`'s "clear the 272-pixel screen" budget, had never
accounted for `frame.marks`, so any future warning or restart note on a page
that scrolled to exactly its own `visible_rows` would have found the same
mark waiting in the same place.

**That general fix has landed.** [`Frame::content_bottom`](../../crates/ui/src/menu/frame.rs)
reads the lowest mark in the screen's lower half - Pulse's footer sits at
`y=236` of 272 - and `menu::visible_rows` clears *that*, not the screen's own
edge. A page whose rows can carry a note reserves one line's worth of room
under them by calling `visible_rows` with `reserve_note: true` -
`pilots::page_reserves_axis_preview` is what decides that for AI PILOTS,
off the page's own `pilot.axis` row rather than the row's live value, so the
window does not resize on the keystroke that gives `AXIS` a value. Pulse's
own skin still fits seven rows with nothing reserved - `236 - 32 = 204`,
`204 / 28` floors to seven, matching every page that has no note to show -
and six with the reservation on, which is what leaves AI PILOTS' own preview
line room to sit above the footer instead of on it.
`prompt::axis_preview_draw` no longer carries a tuned scale or offset of its
own: it draws in the same slot, at the same scale, `menu::rows::draw`'s own
warning/restart message uses, off `Menu::visible_rows` (kept fresh every
frame by `MenuStage::render`, since a player can navigate onto AI PILOTS
without anything else in the frame loop recomputing it). See
`crates/game/src/menu/skin.rs`'s own doc on `visible_rows` for the
arithmetic and `crates/game/src/menu/frame.rs`'s on `content_bottom`.

`--menu-page pilots --screenshot <path>` is how to look at it without a
window.

## RECORDS

A records browser: MODE and TRACK rows, then a live table of what
`oag_game::records::Store` holds for whichever track/mode is currently
picked - one line per speed class, the standing best time. Closes the
"no records browser" gap [`docs/architecture/persistence.md`](persistence.md)'s
own "what is not built yet" section named.

**The disc authors this screen, and it was checked before anything here was
designed.** `Data\Plugins\PI001\GUI\RecordGrid_Definition.xml` is a real
"Record Grid" screen - see
[fe-menu-definitions.md](../formats/fe-menu-definitions.md)'s own new
section for the read, confidence 92. Its `Speed Lap Records` sub-screen is a
track picker over a table with one row per speed class and a time column -
**that shape is this page's own**, reused because it is the one the disc's
four sub-screens that this project's own persisted schema can actually
back: `oag_game::records::Record` is one row per `(track, mode, class)`,
with exactly one best lap and one best total kept per row, never a ranked
list of attempts and never a pilot's tag or team. The disc's own Time
Trial/Race sub-screens assume the richer shape a ranked multi-attempt board
would need, and Zone's own assumes a per-run zone/score breakdown this
project does not persist either - so RECORDS draws Speed Lap's shape for
every mode rather than picking a different, invented shape per mode, and
leaves the TAG/TEAM/boost/perfect-lap columns off entirely rather than
filling them with something this project cannot back. That omission is the
same "draw nothing and say so" rule `CLAUDE.md`'s own root doc states for a
`.pob` that will not parse - a card this project has no answer for.

**MODE collapsing the disc's four sub-screens into one row is ours - the
tree is ours, the same line every other section on this page draws.** The
per-class rows and the time column underneath are the disc's.

**Shares `race.mode`/`race.track` with the RACE page - the same reuse RACE
REMIX's own MODE and AI DIFFICULTY rows already model**, and for the same
reason: browsing here moves what a launched race would use next, which
this project treats as a feature (the row you were just looking at is the
row RACE opens on) rather than a leak to guard against. No new setting was
added for "which track/mode RECORDS is showing" on purpose - a second,
shadow copy of `race.track`/`race.mode` would need its own
`Menu::supply`/`resupply` wiring kept in step with the real one for no
reader-visible benefit.

**Which of `best_lap_ticks`/`best_total_ticks` fills the time column depends
on the mode**, off `scoreboard::show_total`: Speed Lap and Zone never
finish a race at all - `oag_game::records`'s own module doc names both by
name for why `Record::best_total_ticks` is permanently `None` there - so
their column reads the best single lap instead. Every other mode reads the
best total.

**Drawn past the page's last real row, not through the AI PILOTS note
slot above.** Three real rows (MODE, TRACK, BACK) plus four class rows is
seven - exactly the row budget Pulse's own skin already fits with nothing
reserved (see the AI PILOTS section above), so nothing here needed a new
reservation, and `oag_ui_screens::prompt::record_row_draw` continues the ordinary
row pitch and colour rather than the smaller, amber note text
`axis_preview_draw` draws in. Widening the note slot to fit several lines
was the other option and was not taken: `menu::visible_rows`'s
`reserve_note` argument is a `bool` read from three call sites, one of
which (`crates/game/src/capture/menu_page.rs`) is outside what this page's
own member could touch, and widening it there and not here would let the
live session and a headless `--menu-page` capture disagree about how much
room a page reserves - the exact AI-PILOTS-preview-on-the-footer mismatch
this page's own "The general fix has landed" paragraph above already
records paying for once.

The pure resolution behind the table lives in `oag_game::scoreboard`, not in
the `main` binary: `scoreboard::records_table` is the page-id guard, row
resolution and per-class lookup together, and `scoreboard::class_table`/
`scoreboard::show_total`/`scoreboard::speed_class_choices` are its own
pieces, each unit-tested with a hand-built class list, `Menu` fixture and
`Store` - no `Shell`, no `'static oag_title::Title` boot and no window to
fabricate. `crates/game/src/main/records_page.rs` is now a thin wrapper: its
`table_for` supplies `records_table` with the one thing that differs for the
live session, a track lookup drawn from `Shell::tracks_for(mode)` (which
carries a distinct Zone list). `oag_ui_screens::prompt::record_row_draw` is the
drawing half, and `crate::menu_stage::MenuStage::render` wires it in next to
`axis_preview`'s own chain.

**`--menu-page records --screenshot` now shows the table too, not only
MODE/TRACK/BACK.** `crates/game/src/capture/menu_page.rs`'s own
`records_draws(model, skin, title, tracks, records)` calls the same
`scoreboard::records_table`, supplying a track lookup over its own
boot-survey `tracks` list in place of `Shell::tracks_for`, and draws the
result through `oag_ui_screens::prompt::record_row_draw` the same way
`MenuStage::render` does - the one
difference is that a capture's list carries no distinct Zone tracks, so a
capture with `race.mode` seeded to `zone` resolves no track and draws no
table rather than one built against the wrong list (see that function's own
doc). The store itself is read the same "read-only, off whatever `<config
dir>/oag/records.toml` already holds" way `race::CaptureOptions::previous_best`
already reads it for a race capture, loaded once in `capture.rs` right
before the call rather than inside `menu_page` itself, so the function stays
testable against a hand-built `Store`.

Captured and read back off `pulse-psp-eu.chd` with `--menu-page records
--screenshot --no-audio`: title "RACE RECORDS", MODE/CIRCUIT/BACK (this
disc's own French label set was current: "RETOUR") all correct, and four
class rows (VENOM/FLASH/RAPIER/PHANTOM) drawn below them, nothing
overlapping the footer bar. The machine's own real `records.toml` and
`settings.toml` were used unmodified - not seeded - so every class row read
`-`: the settings file's own `race.track` is `01_Track` (Basilico Black),
whose `Data\Plugins\PI001\Definition.xml` entry carries no `Reversed`
attribute - confirmed by extracting the entry with `oag-wad cat --expand`
rather than assumed from the id - so `Track::entry_name` resolves it to
`Data\Environments\01_Track\track.vex`, and no `[[records]]` row on this
machine names that path at all (`grep -in 01_track records.toml` matches
nothing). That is the honest result for this machine's own data under its
current `race.mode` (`single_race`), per `CLAUDE.md`'s do-not-invent rule,
and the same machine's `records.toml` does carry real times for other
track/mode pairs (`03_Track`/`single_race`, for one) - not captured here to
avoid editing the real `settings.toml` just to pick a different row for a
screenshot.

## A per-row subtitle

**Built 2026-09-15.** A row's second line, drawn only under the row currently
selected - `menu.toml`'s own `subtitle`/`subtitle_string_id`, resolved the
same way `label`/`string_id` already are, and stored on
[`Page::subtitles`](../../crates/ui/src/menu.rs), parallel to `Page::entries`
rather than a seventh field on every `Entry` variant, so adding it touched
no construction site that builds an `Entry` today.

**The wording is ours; the geometry is the disc's.** The original's own
`Main Menu` authors exactly this widget - seven `helptext` elements, one 18
pixels below its own row, only the selected one ever drawn - and its wording
was read in full alongside the row labels themselves, see
[fe-menu-definitions.md](../formats/fe-menu-definitions.md#the-seven-rows-and-their-help-text-read-in-full).
That table is not what draws: `menu.toml`'s own header already commits this
file to no shipped disc content, the same rule that keeps a circuit's real
name out of a `values_from` row, and a line like "The definitive WipEout®
single player experience" is marketing copy off the disc rather than a
number or a label word - closer to the kind of text that rule was written
for than to `RACE CAMPAIGN`/`RACEBOX` themselves, which this project already
ships as literal English for the rows they name. So every row's `subtitle`
in `assets/ui/menu.toml` is this project's own sentence about what the row
actually does, and the disc's own wording stays in the docs page above as
the reverse-engineering record it is.

**The layout numbers came from a capture and are Pulse's alone.**
`oag_title::MenuSkin::help_text` (a `HelpText { offset_y, scale, color }`,
the same shape `MenuList`/`MenuBlocks` already use for a title's own
optional geometry) is `Some` only for Pulse - `18.0` below the selected row,
white, a face two-thirds the row's own size - and `None` for Pure and HD,
which is what leaves them drawing no subtitle at all rather than borrowing
Pulse's numbers, the same discipline `MenuSkin::selected_pulse_period_secs`
already states for its own axis. `crate::menu::skin::Skin::help_text`
resolves that into the grid being drawn in, and
`crate::menu::rows::draw_text_rows` is the one place that reads it - the HD
list idiom (`draw_list_rows`) draws no subtitle, since nothing has measured
where one would sit on a title whose rows are boxes rather than bare text.

**`RACE CAMPAIGN` and `RACEBOX` were also reordered and reworded in the same
change**, to match the disc's own first two `Main Menu` rows
(`FE_RACE_CAM`/`FE_RACEBOX`) rather than this project's earlier `RACE`/`RACE
CAMPAIGN` naming and order - see the same section of
[fe-menu-definitions.md](../formats/fe-menu-definitions.md#the-seven-rows-and-their-help-text-read-in-full)
for the read this settled it against. The row still opens the same `race`
page; only its label and its place in the list moved. This is presentation
catching up with the disc, not a change to the tree itself - the disc's
other five rows (`MULTIPLAYER & SHARING`, `WIPEOUT-GAME.COM`, `PROFILE`,
`OPTIONS`, `EXTRAS`) have no single row here with the same destination:
`RECORDS` and `OPTIONS` cover pieces of what `PROFILE`/`OPTIONS` describe but
are not a row-for-row match, and multiplayer, the web link and extras have
nothing at all - so none of the five was added as a row that would go
nowhere or only partway there.

## What is not built

- **Localised labels.** Row labels are literal text, and `string_id` **is**
  now read: `menu::definition::resolve` looks it up in whatever `StringTable`
  its caller passes and shows the answer in place of `label`. What is missing
  is content, not mechanism - `assets/ui/menu.toml`'s own rows below still
  don't name a `string_id`, since nothing has translated them yet, so
  `assets/ui/strings/english.toml` stays empty. The table a caller can pass
  differs by call site: `prepare::definition` (the `--menu`/built-in path)
  passes a project-only one from `oag_game::strings::project_table`, because
  it runs before any disc is open; `capture::menu_page` passes the real,
  disc-merged one, because it draws a page of an already-open title.
- **Switching renderer without relaunching.** The RENDERER row writes the
  setting; the adapter it names is chosen once, at boot. Applying it live would
  mean destroying the device and with it the surface, the upscaler's
  framebuffer, every pipeline and every uploaded mesh, then rebuilding whatever
  stage is on screen - a `Race` holding all of that. That is a substantially
  larger change than the row, and deferring it costs a relaunch. The row *says*
  so now, from the moment it is moved off the adapter the game is drawing with -
  see `restart_required` above - and the adapter that made the device is printed
  at startup, so "which one am I on" is answerable from a log as well as from
  the menu. What is *not*
  deferred is recovery: `Gpu::bring_up` retries on `default` when the named
  adapter enumerates but will not produce a device or configure the surface,
  because a setting reachable from inside the game must not be able to lock a
  player out of it.
- **Anything a HUD needs.** When `oag-ui` exists (M5, see
  [workspace layout](workspace-layout.md)), this module moves into it.

## What escape does

**Back one level, everywhere**, which is one rule and four places it lands:

| Where | What happens |
| --- | --- |
| The menus, race parked underneath | Pops a page, exactly as circle does. On the root page it resumes the parked race instead of quitting - see below |
| The menus, nothing parked | Pops a page. On the root page there is nothing behind the menus, so it quits |
| A race | Hands the window back to the menus |
| The boot sequence, or a `--race` run | Nothing is behind it, so it quits |

It used to call `event_loop.exit()` on the first keypress from anywhere, which
is the behaviour a player is least likely to want and the one they cannot undo.

Two things this cost that are worth knowing before touching it again:

- **Escape is not a game button and must not become one.** Mapping it onto
  `circle` would give the menus their back key for free and give a race a
  brake. So the window layer calls `Menu::back` directly, out of band with the
  tick loop - safe only because the menus hold no input state of their own.
- **`repeat` matters now.** winit resends `Pressed` while a key is held, and
  "back one level" thirty times a second walks out of the menus and quits. It
  did not matter while the first press exited.

**Leaving a race parks it, unless the race is over.** `Session::open_menus`
keeps the outgoing `Stage::Race` in `Session::suspended_race` instead of
letting it drop, and backing all the way out of the menus reaches
`Session::resume_race` in place of quitting - the fourth row in the table
above. A finished race is the one exception: there is a results table behind
it rather than a track, nothing to resume into, so that one is still
discarded exactly as every race used to be. See
`crates/game/src/main/session/menus.rs`.

**And backing into the menus over a parked race now draws it, dimmed,
rather than the ordinary menu screen.** `Session::draw` (the draw half of
`Session::frame`, split into `session/draw.rs` once this pushed the whole
function past `scripts/check-file-size.py`'s ceiling) renders the parked
`RaceStage`'s scene into the scene target whenever `Stage::Menu` has one
waiting, exactly the call `Stage::Race` itself makes, and `resolve_scene`
carries it into the presentation target the same way a live race's picture
gets there. `MenuStage::render` then draws over that picture with
`LoadOp::Load` instead of its usual black clear, leaves out the disc's own
looping backdrop movie (the race's picture takes its place), and draws a
translucent `Draw::Fill` first so the rows read over it. The overlay's own
tint is **chosen, not authored**: neither PSP title's front-end XML defines
a pause screen to read one off - `Skin.xml`'s `LoadXML` list is exhaustively
22 files, per [fe-menu-definitions.md](../formats/fe-menu-definitions.md),
and none of them is a `Pause` definition - and the one HUD string that
names pausing, `IG_PAUSE_QUIT`, is drawn by nothing (see
[hud.md](../ui/hud.md)'s "There is no pause"). See `PAUSE_OVERLAY` in
`crates/game/src/main/menu_stage.rs`.

**The race's own music already worked this way**, and is the reason the
World's turn was safe to build the same way. Its playlist - through the
sixteen soundtrack tracks, distinct from the menu's loop - already paused and
resumed, its position kept in `Audio` for the process's lifetime rather than
reset on every race, well before the World did. That was audio state outside
the simulation, the same way `docs/architecture/determinism.md` already puts
every other sound outside it, not race state being kept alive - but it meant
`Session::resume_race` had `Audio::start_race_music` to call rather than
anything to invent. See `crates/sound/src/lib.rs`'s `start_race_music` and
`pause_race_music`.

## Wipeout 2048 reaches these menus from two tiles of its own

2048's front end is not a `MenuSkin` and is not this menu tree: it is the
disc's own touch grids and campaign map, walked by `oag_ui::frontend`
([2048-frontend.md](../formats/2048-frontend.md#wired-the-boot-walks-and-the-grids-draw-2026-09-21)),
and its `SINGLE PLAYER CAMPAIGN` starts a race without ever opening a page
here. What it has no counterpart for is the race box and RACE REMIX - the
disc ships neither, and every other title reaches both from this tree's
root. So, at the user's request, two tiles that **are this build's own and
not on the disc** sit on 2048's `GameModeChoice` after the four the file
authors: `RACEBOX` and `REMIX`, labelled with the same `OAG_MENU_RACEBOX`/
`OAG_MENU_REMIX` ids the root page's rows are, drawn in the 122x96 text-box
shape the disc's own game-list screen authors, at positions **chosen, not
measured** (`oag_ui::frontend::touch::EXTRA_TILES`). A tap opens these
menus and walks straight into the `race` or `remix` page
(`oag_ui::menu::Menu::push`, which keeps the stack so BACK lands on the
root), where everything on this page applies as it does on any title - the
race box offers 2048's ten circuits and five teams at their default craft
slot, and REMIX degrades to an ordinary race when 2048 is the only title on
the search path, exactly as the REMIX write-up above says it does.

## A mouse and a finger

**Ours, on every title, and nothing on any disc is being reproduced.** The
line at the top of this page - a PSP, no mouse - is still true of every
original in the lineage, so every rule below is this build's own, marked so
in the code, and evidence for none of it is claimed. The vocabulary is
`oag_ui::pointer::Pointer`: one tick's worth of *where the pointer is, in the
screen's own grid*, and whether it moved, clicked, pressed its secondary
button or turned its wheel. A default `Pointer` is one that is not there,
so a headless capture that never builds one draws exactly what it always
did.

**Why the pointer is not a button.** `InputSnapshot` feeds the `World` and
the committed state hash, and a pointer has a position - a pair of
window-relative floats. Putting one in the snapshot would be a
[determinism](determinism.md) failure waiting for a second monitor. So the
pointer stops at the front end: the models in `oag-ui` consume it directly,
beside the button edges they already consume, and it never reaches a
simulation crate. The one place the two vocabularies meet is
`Controls::tap` - the composition root pressing start and cross for a click
on a screen that has nothing to point at (the boot movies, `PRESS START`,
Pure's storage warning, the results table), onto the keyboard's own latch,
so the screen reads it exactly as it reads a key. 2048's touch grids and
campaign map take the pointer themselves, the way the language picker
does: hover selects, a click taps, and a click on a marker already
selected launches its event - `Frontend::pointer` says which screens do. Never in a running race:
a click is not thrust, and a race is not offered the pointer until it is
paused or over.

**The window's cursor is hidden and the game draws its own**, in the
title's own palette - `assets/cursors/*.svg`, one per title plus this
build's for the chooser, named by `oag_title::Title::cursor` and painted
last in `Session::draw` by `oag_game::cursor`. A compositor's cursor over a
borderless game is whatever that compositor feels like showing, which on a
handheld is nothing, and a player with no visible pointer has no way to
find out the menus answer one. Drawing it ourselves is also what lets it
vanish exactly when it means nothing: during a race, and whenever the
mouse was not the last device to speak. A finger draws no arrow - it would
sit under the fingertip, covering the row it is on - and neither does a
pad or a keyboard, where an arrow parked over a row the player is not
using would say that row is pointed at when nothing is. `Window::other_device`
(off a raw key event in `app.rs`, off `Controls::pad_spoke` in the tick
loop) hides it, and the next mouse move, click or wheel turn brings it
back, the way every desktop game does. 2048's `data/FE/Images/cursor.gxt` was checked before its cursor
was invented - it is a 32x16 mark, the same shape as HD's strip-underline
`cursor.gtf`, not a pointer; whether PS TV mode draws one is unread.

**Grid space, not window pixels.** `oag_game::render::to_grid` runs the
renderer's own fitting backwards - `display::viewport`, `letterbox_in`,
`ui.wgsl`'s `to_clip` - so a model hit-tests against the rects it just
emitted and never learns what a window is. The letterbox is undone as well
as the viewport, for the reason `MenuStage`'s `overlay_rect` already exists:
sizing to `space.size` is only right when the aspects happen to agree, and a
click in the bars maps to a coordinate off the grid, where nothing is hit.
`crates/game/src/render/tests.rs` round-trips it against an independent
reimplementation of the shader's mapping on both spaces and four viewport
shapes.

**Hit regions come from the drawing's own arithmetic.** Four layout paths
draw a page - a PSP column of text rows, HD's list rows with their step
arrows, HD's strip with block art, and the strip's measured fallback - and
`menu::pointer::regions` asks the same two modules that draw them
(`rows.rs`, `strip.rs`) for the rects, computed from the same numbers.
A second copy of the layout kept in step by hand is the drift `rows.rs`
already refuses between `Menu::scroll` and the skin. Each path's test
draws the page and asserts every row's label pen lies inside that row's
region, and that HD's arrow sprites *are* the step regions, rect for rect.
The prompts, the selection screens, the language picker, the disc chooser
and the Race Campaign's `Grid Selection`/`Cell Selection` each factor their
geometry the same way (`prompt::Grid`, `picker::pointer::targets`,
`frontend::rows::language_rows`, `launcher::row_at`,
`campaign::pointer::{grid_targets, cell_targets}`) with the same drift
guard. The campaign screens' own hex targets go one step further: a hex's
hit region is tested as the hexagon `hex_filled.mip`/`hex_outline.mip`
actually draw (`pointer::hex_contains`), not its bounding box, because the
grid is staggered and neighbouring hexes' boxes overlap at their corners -
see [campaign-screens.md](../ui/campaign-screens.md).

**A row of plain text gets its band from the face's ink, not the pen.** A
disc font's glyph box is the whole atlas row, and a capital sits at the
bottom of it under room kept for accents: Pure's `Default` face keeps
seven of its sixteen rows empty above the cap line, so at the language
picker's scale of 1.15 the capitals of a row whose pen is at `y` are drawn
from `y + 8` to `y + 18` grid units while the rows step by 15. A band laid
from the pen covered the gap above one row and the ink of the next, and
pointing at DEUTSCH lit ESPAÑOL - reproduced on 2026-09-14 with `xdotool`
at window pixel (200, 290) of a 1600x900 window, the drawn cursor's tip
inside DEUTSCH. `oag_ui::pointer::RowInk::measure` reads the first and
last inked row of `A`..=`Z` off the atlas the renderer actually draws
`Draw::Text` with, and the language picker (`Frontend::set_row_ink`, from
`Stage::frontend`) and the selection screens (`menu::Skin::set_row_ink`, from
`Session::open_menus`) centre each row's band on that. The menu pages do
not need it: their rows carry highlight fills laid out by the same skin
that places the text, and `menu::pointer::regions` takes those rects.

**The rules, and why each is what it is:**

| Gesture | Menus | Elsewhere |
| --- | --- | --- |
| Hover | selects the row under it - only on a tick the pointer *moved*, so a mouse resting on a row does not fight the keyboard for the cursor. On HD the block grows through `focus.rs`'s easing, which is what makes a mouse menu feel right | the same on the language picker, Pure's listed selection rows, the keyboard grid, a confirm's answers and the chooser |
| Click | activates the row under it, whatever the cursor was on - one gesture, "that one". On a choice or toggle it steps forward as cross does; on one of HD's step arrows it steps the way the arrow points; on a binding row it opens the key capture | **two taps on the language picker and the selection screens**: a click selects, a click on the row already selected confirms. The choice is persisted the moment it confirms and the screen never comes back on its own, so a finger one row off must not race the whole game in the wrong language. Pulse's one-at-a-time selection screen confirms from its info panel or preview instead, having no rows; its `up arrow`/`down arrow` images step, as do the livery arrows |
| Wheel | on a page longer than the screen, scrolls the view one row per detent; on a page that fits, walks the cursor one row per detent. Either **stops at the ends**: a list that jumps from last to first under a wheel has lost the player's place | steps the entry on a selection screen, the language on the picker, the row on the chooser |
| Secondary button | back, which is what circle is on the same page; closes from the root, as circle does | back on a selection screen, cancel on a prompt, "back to the menus" on a paused race; nothing on the boot sequence, which has no back |
| A disabled row | selectable and inert, exactly as for a pad: `Menu::adjust` refuses it and a click goes through the same gate | a chooser row that will not open is not a target at all, the rule `Launcher::step` already applies |

**Touch is a pointer that is only there while it is down, and a tap is
decided on lift.** A finger has no hover, so a tap has to select and
activate together, which is what the models do with `moved` and `clicked` in
the same tick and why the two-tap screens above compare against the row
selected *before* the gesture. Because a finger can also drag, `Started`
only remembers where it landed; `Ended` reports the position, a move and a
click in one tick if the finger stayed within `DRAG_THRESHOLD` (12 physical
pixels, **chosen, not measured**) of it, and then takes the position away so
nothing stays highlighted under a finger nobody is holding there. Past the
threshold the finger is a drag for good, even if it comes back: its travel is
`Pointer::drag` (grid units, the content follows the finger) and it reports
no position, no hover and no click, so a scroll never selects. One finger is
followed; a second is ignored until the first lifts. No fling: the list
stops with the finger (chosen).

**What scrolls, and how** (the whole set; checked across Pulse, Pure, HD,
Fury, 2048 and Omega, which share these models):

| Surface | Wheel (one detent; trackpad pixels accumulate at 40 px, chosen) | Finger drag |
| --- | --- | --- |
| A `Menu` page longer than the screen (GRAPHICS, CONTROLS and the like) | scrolls the **view** a row per detent, clamped at both ends, the cursor pushed inside the window as a drag pushes it (until 2026-10-07 it walked the cursor, which the maintainer found wrong: the list should move, not the selection) | scrolls the **view** a row per row-pitch of travel (pitch read off the page's own regions); the cursor is pushed only as far as keeps it in the new window, so selection and view stay coherent |
| 2048's campaign map | pans one marker row (`PITCH.1`) down the canvas, chosen; selection untouched | pans the canvas with the finger, clamped to it; selection untouched |
| Language picker, selection screens, the race box chooser, campaign grids | steps the entry as before; none of them has more rows than the screen holds | nothing to scroll |

No scroll indicator or scroll arrow is drawn by any of the front-end models
(2048's `<TouchScroll>` scrollbar thumb is only measured in the frame
captures, not drawn), so there is nothing to make tappable; none was invented.

**The order is the pad's order.** `session::frame` routes a click through
the same precedence it routes a button: a selection screen takes the tick
whole, then a binding capture freezes the page, then a modal prompt eats the
input, and only then do the rows see it. A click reaching the rows behind a
scrim is the bug that ordering already prevents for cross - with one
difference a `Pointer` forces: a button edge is *consumed* by whatever
takes it (`Input::take`), so a prompt may close on the very tick it reads
cross and the rows below see nothing, but a `Pointer` is a value nothing
consumes, so the rows are gated on whether a prompt was up *before* the
tick rather than after. Otherwise the click that answered KEEP would also
land on the row under it. The latch is the
keyboard's too: `crate::pointer::Window` accumulates winit's events between
ticks and hands out one `Pointer` per tick, so a click between two ticks is
delivered on the next rather than lost - finding U5's argument, applied to a
button with no repeat rate. Focus loss drops the pending presses beside
`Controls::release_all`.

### The `Confirm`/`Back` footer, and the `NavigationButtons` census that gates it

**2026-09-25.** Both PSP titles' shared front-end root (`Skin.xml`) and
Wipeout HD/Fury's and Omega's own carry an identical shape: one
`<NavigationController>`, off `Top FE Screen -> FE Screen`, authoring
`Confirm`/`Back` (plus, on HD/Omega, an online `Invite` pair,
`StartEnabled="false"` and irrelevant here) - the literal wrapper name on
HD/Omega's own disc is `BodgeScreenContainingNavigationController`, not this
project's own naming. `oag_ui_screens::campaign::footer::NavigationLegend` reads it
directly off the raw parsed tree (`oag_tables::fexml::parse`, not
`Screens::from_xml`, which never keeps the tree this needs), and is now read
once at boot (`boot::screens::read_nav_legend`) and carried on `Shell`/
`Boot` for both the live session and `--menu-page`, in addition to the
per-open read `crate::campaign::load` already made for `Cell Selection`.
`MenuStage::render` draws it on every ordinary menu page.

**Which prompt shows where is read off the XML on Wipeout HD/Fury, and
chosen - corroborated, not guessed - for this build's own tree.** HD/Fury's
own front-end XML authors a `NavigationButtons="N"` attribute on most of its
navigable `<Screen>`s (Pulse's own `Skin.xml`/`CellMode_Definition.xml`
author no such attribute anywhere - confirmed by direct read of both, so
"unconditional, chosen" is what stands for Pulse, unchanged). A census of
every copy of every `*_Definition.xml` this project reads (`DATA00`-`06`)
finds four values, and their pattern reads as a two-bit mask - bit 0
(`1`) enables `Confirm`, bit 1 (`2`) enables `Back`:

| Value | Meaning (inferred) | Screens carrying it |
| --- | --- | --- |
| `0` | neither - the screen draws its own prompts | Confirmation dialogs with their own Yes/No buttons (`GenericInviteConfirm`, `LeaveConfirm`, `Play Now`) |
| `1` | `Confirm` only | `Main Menu` (the tree's own root), every `EndRace_Definition.xml` screen (`Results`/`Rewards`/`Menu`, which also author their own *local* controller with no `Back` `Text` at all - consistent), `PurchaseGame` |
| `2` | `Back` only | `Manual`/`Manual Part 1`-`5` (page-turning, nothing to confirm), the `Stats`/`OnlineStats` sub-pages, `Race Records` |
| `3` | both | `InGame Settings`/`InGame Settings P2`/`InGame Settings MP`/`InGame 3D Settings`/`InGame Audio Settings` (a save-or-cancel pause page) |

Confidence 65 on the bitmask reading itself (an inference from the pattern,
not a traced consumer - `NavigationButtons` is read nowhere in this
project's own Ghidra names yet). Confidence 80 on the boundary this project
actually uses (`Confirm` always, `Back` only past the root), because it is
independently **measured**, not only inferred: two real RPCS3 captures
already in this repository (`data/reference/hd-main-menu-screenshot/00.png`,
`data/reference/hd-settings-screenshot/00.png`) show `Main Menu`'s own
footer as `(X) CONFIRM` alone with no `Back` glyph anywhere, and `OPTIONS`
(a page reached *from* Main Menu) as `NAVIGATION (X) CONFIRM (O) BACK` -
exactly the `1` vs `3` split the census above reads off the XML, on the
real title rather than this build's own tree.

**This build's own tree has no disc screen to read a literal
`NavigationButtons` value off, since the tree is ours** (see this page's own
opening section) - so `MenuStage::render` gates on `Menu::depth() > 1`
instead: `Confirm` always, `Back` only past the root. That boundary is
**chosen** for exactly the same reason the tree itself is, but it is the
boundary the two captures above corroborate rather than one picked for
symmetry. `Cell Selection` (both Pulse's own screen and, since 2026-09-25,
Wipeout HD/Fury's and Omega's) draws both unconditionally regardless - the
CellMode dialect never authors `NavigationButtons` on that screen on any
title, so there is nothing narrower to read, and Pulse's own capture already
measured both prompts showing there
(`docs/ui/campaign-screens.md`'s 2026-09-21 section).

**Wipeout HD/Fury's own icon glyph draws nothing, on every screen this
reads one off.** `ControlTextConfirmButton`/`BackButton` author
`font="buttons"` (`ps_buttons.fnt`) rather than Pulse's `font="small"`, and
this build loads no atlas for that face at all -
`boot::fonts::load_font`/`load_menu_font` only ever resolve the language
plugin's own `<Font>` role slots, never a literal `font=` attribute. The
resolved idstring is the identical `"ε"`/`"γ"` codepoint Pulse's own icon
half uses, which happens to render correctly through Pulse's own loaded
`menu`-role atlas (confirmed live, `docs/ui/campaign-screens.md`'s
2026-09-21 pass) - but that is Pulse's own font actually carrying the
remapped glyph art at that codepoint, not a fact about the codepoint
itself, and nothing confirms Wipeout HD/Fury's own loaded body face carries
the same remapping rather than the plain Greek letter. Per `CLAUDE.md`'s
"never invent what the assets already author", a wrong glyph is worse than
none, so the icon half is left out and the resolved word
(`ControlTextConfirm`/`Back`, `font="default"`, a face this build does load)
draws alone.

## The background the menus sit on

The rows are drawn over **the disc's own looping menu backdrop**, and that is a
recovery rather than decoration this project invented. `FE Screen` - the
original's main menu - carries a `Movie` widget naming
`Data\Movies\Backdrop.PMF`, and a cold boot under PPSSPP with `MoviePlayer_Open`
armed from reset opens exactly two movies in ten minutes: the intro, and this
one. See [frontend boot](frontend-boot.md) and
[`frontend-video.md`](../ghidra/functions/psp-pulse-usa/frontend-video.md). The menu
*tree* is ours; what it sits on is the disc's, and citing which widget names it
is what keeps the two apart.

On PSP it is a 480x272 `.PMF`, 270 frames, 9.01 seconds. On PS2 the same name
resolves through `boot::LOOSE_MOVIES` to `BG512.IPF` / `BG640.IPF` - IPU video,
512x512 or 640x448, drawn at the frame it was cut to fill exactly as the `.PSS`
intro cuts are. Neither is boxed: the 4:3 those cuts declare is a tag their own
picture does not honour, measured in [aspect-ratio](../ps2/aspect-ratio.md).

Five decisions worth having written down:

- **It loads at boot, not when the menus first open.** The 270 frames cost about
  thirteen seconds to transcode once and nothing on every run after; boot already
  pays that for the intro's 1200. The menus, by contrast, open on a keypress out
  of a race, where a thirteen-second freeze would read as a hang.
- **Always the whole movie**, whatever `--movie-frames` says. That flag exists so
  a first run need not transcode 1200 intro frames. Capping a *loop* at four
  frames is not a shorter version of the same thing; it is a stutter.
- **Absent is an ordinary outcome.** A source that does not carry it,
  `--no-video`, or a missing `ffmpeg` all end with no backdrop, and the menus
  draw on black exactly as they did before this existed. The renderer is built
  **without** the video pipeline there, because building it and never filling it
  draws a green rectangle rather than nothing.
- **The frames are decoded on a worker thread, not on the thread that draws.**
  Decoding one costs up to 30 ms and a frame at 240 Hz has 4.17 ms, so the two
  cannot be the same thread: `movie::Feed` runs a decoder four frames ahead and
  the menus upload whichever frame is ready. This was measured *because* of the
  backdrop and fixed for both movies -
  [ADR-0010](adr/0010-movie-decode-thread.md) has the numbers, and the reason it
  does not touch the [determinism](determinism.md) rules.
- **`--menu-page` shows it too**, at frame zero. That flag exists to look at a
  layout without walking to it, and a flag that quietly stops showing what a
  player sees would defeat the purpose. It needs care in the capture path: a menu
  page's `Draw::Video` is the *backdrop*, while the sequence's is the *intro*, so
  the movie the frame is read from and the plane geometry the pipeline is built
  for both have to be swapped together. Getting that wrong is neither a compile
  error nor a crash. **And it is not a check on the live menus**: the capture path
  holds its own `movie::Movie` with its frames intact, where the window's have
  moved onto a decode thread, so a `--menu-page` screenshot can look perfectly
  right while a real window draws the rows on black. Only a screenshot of an
  actual window covers that.

`menu.rs` is handed **a frame index and a rectangle**, not a movie. Which frame
is showing is timing and where it goes is the source's display aspect; neither is
a menu's business, the same way what a setting *means* is not. `main.rs` owns the
player, and it wraps rather than finishing - which is the whole difference
between this movie and the intro.

The index it is handed is **the frame in the renderer's planes**, not the one the
playhead names, and the two can differ by a frame while the decode thread catches
up. `None` rather than a frame means no picture has arrived yet, and then the
video draw is left out altogether - the rows sit on black for a frame or two after
the menus open, because zeroed planes would be green.

**The language picker deliberately does not get one.** The comment in
`frontend.rs` guessing that its black-on-black title colour is black *because*
the real screen sits on a lit background is a hypothesis, and the evidence is
against acting on it: only three screens in that XML carry a `Movie` widget and
`Language Selection` is not one of them, and the picker runs before `LogoFMV`, so
the backdrop being loaded by then is not established either. Settling it means
finding what the picker's parent draws. Until then the colour stays lifted and
the picker stays black.

**The XML's own nesting now argues the same way, and it is stronger than the
count.** Extracting `Skin.xml` and reading the tree shows the backdrop `Movie`
belongs to `Top FE Screen->FE Screen`, and that `Language Selection` is a
top-level screen *outside* that subtree - a sibling of `LogoFMV`, not a
descendant of the screen that owns the backdrop. So the picker cannot inherit it
by nesting, which is the mechanism that would have made "it sits on a lit
background" true for free. Confidence **90**, read off the file. What the same
reading *does* settle is the neighbouring case: `Show Logo` **is** a child of
`FE Screen`, so on hardware the Pulse logo and PRESS START sit on the moving
backdrop - confirmed since by a player who has run the original - and this build
draws it that way. [The boot page](frontend-boot.md) has the detail.

**So the backdrop's feed is now borrowed by two stages, not one.** It was already
built when the window opens rather than when the menus open, and already outlived
the menu stage so a menu -> race -> menu round trip did not pay for a second
decoder; the front end's `Show Logo` borrows that same feed on the way *in*.
Nothing about the menus' own use of it changed, and the page's `Draw::Video` now
carries `source: Backdrop` to say which of the front end's two movies it means.

### A full-screen mark is a backdrop, not chrome

`FE Screen` authors both the movie and four `<Image>` widgets, and the first of
those images covers the whole grid: `Data\FE\Images\gameshare_backdrop.mip`, a
still of the same ship reel the movie plays. Drawn with the rest of the marks -
which is what `Frame::backdrops` did when Pulse's frame was first wired - it
painted over the movie on every frame, and because the still *is* a picture of
the reel, the menu background read as the loop having stopped rather than as
something covering it.

That is the failure worth recording, not the fix: every measurement pointed at
the player. The playhead advanced, the feed's ring stayed full, the frame index
uploaded to the planes tracked the playhead, and `--menu-page` captures looked
correct - a still capture cannot tell a moving picture from a covered one. What
settled it was capturing the same page under `--no-video`: visually identical.

So `Frame::backdrops` layers by rect - the clear, `MenuSkin::background` and any
mark covering the whole screen first, then the movie, then everything else - and
a race behind the menus drops that whole first group rather than filtering it
twice. **Ordered by geometry because the disc's own order is not kept**:
`crate::screen::Screen` holds `images` and `movies` in two vectors, so whether
the `<Image>` or the `<movie>` is authored first is lost by the time the frame is
read. What settles it is not the XML anyway - a player who has run the original
reports the reel playing behind the menus, which a full-screen still over it
makes impossible. Recovering the sibling order is the change to make if a title
ever authors a full-screen mark it means to draw *over* its movie.

One consequence, and it is the right one: with no movie at all (`--no-video`, no
`ffmpeg`, a source that carries none) the still now shows instead of black,
which is exactly the case it exists for - a game-share client has no UMD to play
a reel off.

### One playback, not one per screen

**`Show Logo` and the menus sit in front of the same, never-interrupted playback
of `Backdrop.PMF`.** That follows from the nesting above - `Show Logo` is a
*child* of the `FE Screen` that owns the movie widget, so pressing START changes
which widgets are drawn over a loop that is already running and does not stop -
and a player who has run the original confirms it: the picture does not jump when
the boot sequence ends.

This build got it wrong for one revision, in two separate ways, and both are
worth recording because each hid the other:

1. **Two playheads.** `Frontend` owned one `movie::Player` and `open_menus` built
   a second one at frame zero, rewinding the shared `movie::Feed` to match. The
   comment justifying the rewind was right about what it was solving - a second
   menu open must not compare a fresh player's positions against a worker
   hundreds of frames ahead - but the fix was aimed at the wrong end. There is
   only one movie, so there should only ever be one playhead: the front end now
   *hands its own over* (`Frontend::take_backdrop`, `menu_playhead` in
   `main.rs`), and nothing is rewound on that path.
2. **The feed was asked with a wrapped frame index.** `Draw::Video` carried only
   `frame`, which wraps at 270, while a feed counts positions that never wrap. So
   nine seconds into the boot sequence the front end started asking for positions
   the feed was already past, took nothing ever again, and froze on its last
   picture - which then made the menus' restart look like the only thing wrong.
   The draw carries `position` as well now, and the feed only ever sees that.

Two consequences that are easy to undo by accident:

- **The front end pumps the backdrop's feed on every frame, whether or not
  anything draws it.** The playhead runs from the start of the sequence but only
  `Show Logo` puts the picture on screen, and a feed nobody takes from parks four
  frames in. Without the pump, `Show Logo` would appear and then rush through
  nine seconds of movie at decode speed catching up. `FrontendStage::sync_video`
  therefore takes the frame at the playhead always and *uploads* it only when the
  backdrop is the movie being drawn.
- **Leaving a race does start the loop over**, and that is a judgement call
  rather than a finding. Nothing advances the playhead during a race and the
  stage that owned it is gone, so `escape` gets a fresh player and the feed is
  restarted to match - which is at least consistent with `FE Screen` being torn
  down for a race and rebuilt after it, with its `autostart` movie starting
  again. Nobody has measured the original's loop phase across a race. If it turns
  out to continue there too, the change is small: stash the playhead on `Session`
  when a race starts and hand it back to `menu_playhead`, with no feed restart -
  the feed parks at most four frames past where the menus stopped taking from it.

### A continuous playhead is not a continuous picture

Fixing the two playheads above made the *loop* continuous and left the *picture*
flashing black, so the user's report came back: the intro-to-`Show Logo` handoff
was still not as smooth as the original, and the START press into the menus still
flickered. Neither was a playhead bug. Three mechanisms, all measured on a real
windowed run of the EU PSP disc under Xvfb, with a temporary probe printing the
draw list's named movie, the position it asked for, what the feed handed back and
whether the video draw survived:

1. **`Feed::take_upto` pops, and the pump threw the frame away.** The pump above
   is required, but it took the frame and dropped it on every frame that was not
   drawing the backdrop. A ring has no memory of what it handed over, so the
   first frame that actually wanted to *draw* the backdrop asked for a position
   the ring was already past and got `None`. Caught in the act: at window frame
   480 the front end was in `Language Selection`, drew no video, and took
   position 119; at frame 482 `Show Logo` asked for position 119 and got
   nothing, so its video draw was trimmed and the logo drew on black for two
   frames. Timing-dependent - it reproduced on roughly one boot in three -
   which is exactly the shape of an intermittent "sometimes it is not smooth".
2. **The menus get a fresh `Renderer`, and a fresh renderer is empty planes.**
   `open_menus` builds one so the menus can also be opened from somewhere that is
   not the front end. Its video planes start zeroed, and zeroed I420 is green
   rather than black, so `MenuStage` refuses to draw the quad until a frame has
   arrived. With the pump having just discarded the frame at the playhead, that
   took **three window frames every single boot** - deterministic, unlike (1).
   Frame 1803 was `Launch Game`, drew no video and took position 450; frames
   1804-1806 were the menus asking for position 450 and getting nothing, drawing
   their rows on black.
3. **Two screens on the boot path emitted no video draw at all**, so no amount of
   feed bookkeeping could have covered them. `Launch Game` is the one that
   matters: `Session::frame` draws it **once and then stalls** building the
   menus, so it is on screen for the whole of that load - a few hundred
   milliseconds of black with `LAUNCH GAME` written across it, which is most of
   what the START press looked like.

The fixes mirror the causes. `FrontendStage` keeps the newest backdrop picture it
took (`held_backdrop`) instead of dropping it, uploads it on the first frame the
backdrop becomes the drawn movie, and tracks whether the planes still hold it
(`backdrop_in_planes`) because one set of planes serves both movies and an intro
upload displaces it. `open_menus` clones that picture into the new renderer's
planes before the first menu frame and seeds `Backdrop::shown` to match. And
every screen between the intro and the menus now inserts the backdrop draw, not
only `Show Logo` - `Frontend::insert_backdrop`.

**Do not "fix" the pop.** Popping is what frees a ring slot for the worker to
decode into; stop *discarding*, not popping. `movie.rs`'s
`a_frame_taken_is_a_frame_gone_even_at_the_same_position` pins the invariant, and
`frontend.rs`'s `the_screens_after_the_intro_all_sit_on_the_backdrop` pins the
draws.

After the change, a full boot reports **zero** frames anywhere between the intro
and the menus that draw without the backdrop, across four runs including the
timings that used to reproduce (1).

**`escape` -> menus was a fourth instance of the same thing, closed 2026-08-19.**
The seeding above read the front-end stage only, so leaving a race opened the
menus with no picture and showed black until the restarted feed produced frame
0 - one to three frames, by mechanism (2). That the loop *restarts* there is
the deliberate judgement call already argued above and is unchanged; that it
showed black first was not a deliberate call, and the original would show
frame 0 whichever way the loop-phase question goes.

The fix is the boot-path mechanism one stage later, not a new one:
`MenuStage`'s own `Backdrop` gained a `held` field, filled from `take_upto`
the same way `FrontendStage::held_backdrop` is, and `Session::launch_race`
moves it out into `Session::held_menu_backdrop` before the outgoing
`MenuStage` is dropped - the picture, not the playhead, which still restarts.
`open_menus`'s seeding logic reads whichever of the two candidates the
outgoing stage supplies, through a small pure function
(`session::menus::backdrop_seed`) kept separate from the `Renderer`-touching
code around it precisely so the seeding *decision* is unit-testable without a
GPU - see `crates/game/src/main/tests.rs`.

**Independently verified on a live capture, 2026-08-19.** The earlier black
screenshots and unresponsive keys both traced to one cause, not two: this
sandbox has a live Wayland session (`WAYLAND_DISPLAY` set) alongside Xvfb, and
winit prefers Wayland when both are set - so the app was rendering into that
invisible session while every capture and key-injection tool pointed at Xvfb.
Launching with `WAYLAND_DISPLAY` unset routes the window through X11 as
intended, and screenshots stopped being black immediately. Real `xdotool`
keypresses still needed one more thing: with no window manager running on
Xvfb, X input focus never lands on the app's window on its own, so an
explicit `XSetInputFocus` (raw Xlib, no WM required) was needed before
`xdotool key` had anywhere to deliver to. With both in place: navigated the
real menus into a race exactly as a player would (`RACE` -> `START`), let it
run, pressed a real `Escape`, and captured six frames in rapid succession
immediately after - none black, the backdrop's animation visibly continuing
frame to frame. Held `Escape` down for 1.2s from one menu level deep landed
exactly one level back and left the process running, confirming the
`!event.repeat` guard holds against real OS-level key repeat, not just the
synthetic events the unit tests drive. The window title updated to
`OpenAntiGrav - menu` with no stale `- race` suffix. See HANDOVER's former
ESC-rewiring row (deleted 2026-08-19 once this closed it) for the fuller
account of the two-symptom-one-cause diagnosis.

## DISPLAY against GRAPHICS

Two pages under OPTIONS rather than one, and the line between them is **whether
the renderer would notice**:

| | Rows | What they have in common |
| --- | --- | --- |
| DISPLAY | MONITOR, WINDOW MODE, WINDOW SIZE, ASPECT RATIO, VSYNC, FRAME LIMIT, BRIGHTNESS, GAMMA | The picture's container, and how a finished frame reaches a screen |
| GRAPHICS | RENDERER, RENDER SCALE, RECONSTRUCTION, UPSCALER SHARPNESS, MSAA, ANISOTROPIC FILTERING, FIELD OF VIEW, PERFORMANCE OVERLAY | How the picture is drawn |

RENDERER is the most literal case of that criterion there is - it is *what*
draws - which is why it sits on GRAPHICS despite resembling MONITOR in every
mechanical respect: both are hardware the definition file cannot enumerate, both
come through `values_from`, both fall back rather than fail on a name the machine
no longer has. See [`crate::adapter`](../../crates/game/src/adapter.rs) for what
it can offer, and in particular for why "render on the CPU" is a row that appears
only on a machine with a software driver installed rather than a switch this
project could provide.

A row can also be **warned** rather than greyed, which is a different thing and
the RECONSTRUCTION row is why it exists. A greyed row cannot be changed *now* because
another row rules it out. A warned row can be changed, is stored, and simply
does not have the effect its label promises while some other row holds a
particular value - FSR 1 above a render scale of 100 % being the case in hand.
Greying that would say "you cannot change this", which is false; the setting is
live and the *combination* is pointless. It draws as an amber `!` in the margin
and one line of amber text under the rows, on its own colour channel because the
three the rows already use mean selected, normal and inert.

A warning takes **several** conditions, all of which must hold, because a
conflict is between settings plural: one names this row's own offending value,
one names the other row's. A single condition would describe a row that is
always pointless under some other value, and that is a row that should not exist
rather than one needing a warning - the loader refuses fewer than two for
exactly that reason. It is not a hypothetical: the first version of the upscaler
warning named only the render scale and so fired at every scale of 100 % and
above whether or not the upscaler was even selected.

A row can be **deferred** as well, which is the third and last thing a row can
say about itself and the only one that is not about another row. `disabled_by` is
"not now, because of that row"; a warning is "stored, and pointless next to that
row"; `restart_required = "..."` is "stored, and nothing on this machine will act
on it before the next launch". RENDERER is the only row that is like that, and
until it said so the deferral was documented here and invisible in the game.

It is drawn exactly like a warning - amber `!`, one line under the rows, the
first noted row in page order taking the shared message slot - because to a
player the two are the same sentence: this row is not doing what it says.

What it is measured against is the difference. A warning reads another *row*, and
so needs nothing from outside the menus. "Has this been changed since boot?"
cannot be answered from inside them at all, and answering it from the settings
file would be wrong in a way that only shows on the second visit: by then the
file holds the value the player chose, and the note would clear itself while the
old adapter was still drawing. So the composition root says what is running
through a third narrow call, `Menu::in_effect(setting, values)`, and the note
fires when the row holds none of them.

**Values, plural, and that is the case it exists for**: a game that let wgpu pick
is on `default` *and* on whatever wgpu picked, so a player naming that adapter
explicitly has changed their settings file and nothing about the picture. One
value would have told them to restart for a frame already on screen.
`adapter::choose` returns both spellings for that reason, and after a fallback -
a named adapter that enumerates but will not make a device - it returns the one
that actually worked, so the note is about the picture rather than about the
file.

The conditions themselves take the same shape as `disabled_by`, and both now
accept `values = [...]` as well as `value = ...`: "the upscaler does nothing" is true at
four of the six scales RENDER SCALE offers, and a one-valued condition could not
say so. The loader rejects a condition naming a value its row cannot hold, and a
test pins the warning's list to `upscale::magnifies` from the other side, so the
guard that declines to run the upscaler and the message that says it did not
cannot drift apart.

TARGET FPS and MINIMUM RESOLUTION sit under RENDER SCALE for the
same reason, and the floor's warning is the second place a `warn_when` list is
pinned to something outside the asset. `Condition` compares values and has no
ordering - deliberately, because an ordering is the first thing that would make
`menu.rs` know what a setting *means* - so "a floor at or above the ceiling" is
enumerated as one warning per offered floor, thirty-six comparisons in all.
`the_floor_warns_at_exactly_the_scales_it_cannot_fall_below` generates the same
set from `Scale::OFFERED`, so a value added to either row cannot quietly leave
the list behind. The other half of the pairing is not in the menus at all:
`drs::Limits::new` brings the floor under the ceiling where the two first meet,
because a warning tells a player rather than stopping them.

**And a ceiling is not a drawn size.** Three warnings on this page were written
when `render_scale` *was* the size the frame is drawn at, and a controller that
goes below it makes each of them wrong - the `fsr1` one most visibly, since at
a 100 % ceiling with a 50 % minimum it reported a magnifying FSR 1 as having no
effect. What each wants is the lowest size the rows permit, which is an OR
across two rows, and `Condition.all` is an AND: so each is two entries sharing
one message, one requiring the controller off and one requiring its floor high
enough. That is what `Entry::warnings` being a list buys - and since
[ADR-0041](adr/0041-one-row-for-what-resolves-the-frame.md) both surviving
warnings sit on the *same* row, RECONSTRUCTION, so the list is now also what
lets one row say two unrelated things at two ends of the render-scale range.

TARGET FPS also warns when it is above DISPLAY's FRAME LIMIT - and is
**clamped** to it, which is the same statement made twice on purpose: the row
says what was asked for and `drs::Target::at_most` holds the controller to what
the loop will produce. Neither half is honest alone. Both stop under `vsync =
on`, where the display is the bound and nothing here knows its refresh; the
warning says so by naming the two vsync modes it applies to, which is the same
value-listing `disabled_by` does on FRAME LIMIT itself.

RENDER SCALE itself carries **no** warning about any of this, which is a
decision and not an oversight. Since ADR-0038 the row changes nothing on the
screen a player moves it from, and *"this row's effect is not visible here"* is
a different kind of statement from *"this row is redundant given that one"*.
Overloading one amber line with both was judged worse than no warning at all.

RECONSTRUCTION sits directly under RENDER SCALE because the pairing is what
makes several of its values mean anything: at 100 % there is nothing to
upscale, and the choice is between a blit, a spatial anti-aliaser and a
temporal reconstruction that has work to do at any scale. **One row where there
were two** - see [ADR-0041](adr/0041-one-row-for-what-resolves-the-frame.md),
which folded an ANTI-ALIASING row and an UPSCALER row onto one axis and deleted
five of this page's seven warnings by making their combinations
unrepresentable. MSAA came off that axis in the same change and is greyed under
`fsr3`, which anti-aliases the same frame temporally. UPSCALER SHARPNESS is
greyed out on the three values that run no sharpen at all, which leaves nothing
to adjust - greyed rather than hidden, for the reason FRAME LIMIT is:
a row that vanishes gives a player no way to find out what took it away. Its
unit is **stops**, upstream FidelityFX's own, where zero is maximum and each
whole step halves it; that runs the opposite way to what a reader expects and is
kept anyway, because a number a player reads about elsewhere should mean the
same thing here.

Turning vsync off changes nothing about the frame that is drawn, only about when
it is shown; halving the render scale changes the frame itself. That test is
what settles the two rows a reader would otherwise argue about:

**One caveat a player will notice before a reader does.** Since
[ADR-0038](adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md)
RENDER SCALE applies to a race and to nothing else - a stage with no 3D scene
draws at presentation resolution regardless, because scaling a menu saves no GPU
and only softens it. So moving that row *from a menu* changes nothing visible
until a race starts, where it used to resample the menu under the player's hand.
Whether the row should say so is open: the existing `warn_when` vocabulary says
"this row does nothing given another row", and this is "this row does nothing on
this screen", which is a different sentence to say to somebody.

- **Brightness and gamma are DISPLAY**, even though they are a fragment shader.
  They are a calibration of somebody's monitor, applied to a frame the game has
  already finished drawing - see below.
- **The performance overlay is GRAPHICS**, even though it is a diagnostic and
  not decoration, and even though it is now drawn *after* the brightness and
  gamma the row above it calls DISPLAY - see below. What it reports is what the
  graphics settings cost, which is the question the page it sits on is about.

`settings.toml` is split the same way, into `[display]` and `[graphics]`, so
there is one vocabulary rather than two. Everything was in `[graphics]` before
the split, and a file that still is gets its moved keys carried across on load
rather than silently dropped - serde ignores a table field it does not know, so
without that a player who had set borderless at 120 Hz would have opened the
game windowed at 240 with no message and nothing wrong in the file.

The value types all still live in one module,
[`crates/display/src/display.rs`](../../crates/display/src/display.rs). That module is
the vocabulary, not the page list: `Scale` sits next to `Aspect` there and they
are on different pages, and that is fine. What each value may be and how it is
spelled is one question; which page a player finds it on is another.

## Brightness and gamma

Both are applied in the **blit that puts the finished frame on the surface**,
which is the one pass every frame goes through. The front end, the menus and a
race are three renderers that share nothing else; grading in each would be three
places to get it wrong and three places to forget when a fourth stage lands.
Doing it once also means a player calibrating the picture watches the menu they
are standing on change as they do it, which is the only way to calibrate
anything.

Both are **percentages, and 100 leaves the frame alone** - offered on both lists,
so there is always a way back. Brightness multiplies; gamma bends the midtones
without moving black or white, which is the one that makes a dark corner of a
track visible without washing out the rest. Brightness is a multiply and not a
lift because adding a constant raises black off black, and a race at night then
looks like a race in fog.

Two honest limitations, both recorded rather than papered over:

- The blit samples values that are **linear** when the surface format is an sRGB
  one, so this is not the display-gamma knob a CRT-era game shipped and is not
  claimed to be. What it does is the thing that knob was used for.
- **A screenshot is not graded.** `--screenshot` and the race capture write the
  offscreen frame straight out without going through this pass. That is the
  wanted answer: these are settings about somebody's monitor, and baking them
  into a PNG that goes into a bug report would make every capture disagree with
  every other one.

## Field of view

`GRAPHICS -> FIELD OF VIEW` is a **percentage of the field the disc's own data
authored**, not an angle, and that is a deliberate refusal rather than a
simplification. `<ExternalCameraFar fov>` comes off the disc and **its unit is
unrecovered** - `Race::projection` reads it as degrees and says so in the load
report. A row that let a player type `90` would be quietly asserting a unit this
project has not established; a multiplier says what it does - wider or narrower
than the game frames it - without claiming to know what the number underneath
means.

It scales the **tangent** of the half-angle rather than the angle, because the
tangent is what the projection matrix is built from: 150 % then means half again
as much across the screen at every starting angle, where scaling the angle would
mean less and less the wider the field already was.

The setting is applied *before*
[`fit_vertical_fov`](../../crates/render/src/camera/mod.rs), so the two compose
in the order a reader expects: the player widens the field the disc asked for,
and the result is then fitted to whatever shape the window is. At 100 % - the
default - nothing is applied at all and the value comes back bit-identical, so
every capture under `data/traces/` is still taken against the same projection it
always was. A test asserts exactly that.

## The performance overlay

`GRAPHICS -> PERFORMANCE OVERLAY` is `off`, `fps` or `pacing`, and it is the one
row here that draws over every stage rather than configuring one.
[`crates/present/src/perf.rs`](../../crates/present/src/perf.rs) has the reasoning; the
part that belongs on this page is why it is **three values and not a toggle**:
`fps` answers "is there headroom", `pacing` answers "are the frames evenly
spaced", and the second question is the one an average frame rate is incapable
of answering. A game can hold a perfect 60 and stutter visibly, and only the
graph shows it.

It is drawn **onto the surface, after the frame has been resolved onto it** -
so it is rasterised at presentation resolution whatever the render scale is,
per [ADR-0036](adr/0036-ui-composites-at-presentation-resolution.md). It used
to go into the offscreen target at the render scale, on the argument that it
should cost what the game costs; that holds for a scale a player sets once and
not for one that moves, and this is the row somebody reads to judge what a
resolution controller is doing. Two visible consequences: it is **not graded**,
brightness and gamma riding in the resolve it now comes after, which is
deliberate for an instrument and is ADR-0036's one standing exception; and it
is no longer edge-detected by FXAA or SMAA, nor sharpened by FSR 1. It uses
this project's own 5x7 glyphs rather than the disc's font, because `--race`
never loads a font and an overlay that vanishes on the route where it is most
wanted is not an overlay.

It is **window-only**. `--screenshot` runs the sequence as fast as it can with
nothing presenting, so a frame time from it would be a real measurement of
something nobody is asking about.

### Vsync, and where triple buffering went

VSYNC has three values, because on a modern API "double or triple buffering" is
not a setting - the **present mode** is, and it decides both whether the picture
can tear and whether the loop may run ahead of the display:

| Row value | Vulkan | What a player gets | Measured, 60 Hz panel |
| --- | --- | --- | --- |
| `off` | `Immediate` | Tears. The lowest latency there is, and the default: this is a racing game, and vsync's cost is a frame of latency on the one thing a player feels directly. | 1356 fps unlimited |
| `on` | `Fifo` | Never tears, and has the half-rate cliff - miss a refresh by a microsecond and you wait a whole one, so 144 becomes 72. | 59 fps, 16.8 ms mean, **34.1 ms max** |
| `smooth` | `Mailbox` | Triple buffering done properly: never tears *and* never halves, at the price of the frames it discards. | 240 fps unlimited; 120 under a 120 limit |

Two things in that last column are worth reading rather than skimming. The
34.1 ms maximum under `on` **is the cliff, caught in the act** - one frame in
the window missed its refresh and waited for the next, and that single doubled
frame is the whole reason `smooth` is a row. And `smooth` does not pin to the
refresh: it ran at four times it, because acquiring an image blocks only once
they are all in flight. Four times the refresh is not a rate anybody asked for,
which is the practical argument for keeping the limit live there.

Spelled `smooth` and not `triple` because the buffer count is not what a player
is choosing. What they are choosing is "never tears, never halves"; how many
images the swapchain holds to deliver it is the driver's business.

**Only `on` greys the limiter out**, which is the whole reason `disabled_by`
names a value. `off` and `smooth` both leave the loop free to run ahead, and
under `smooth` the limit is the only thing stopping the GPU rendering frames
that are then thrown away. The setting ships at **240** - the top of the tier
displays are actually built at (144, 165, 240), so it caps nothing common while
still stopping a menu page running the GPU at four figures.

Only `Fifo` is guaranteed to exist; Vulkan makes the other two optional and
configuring a surface with a mode it does not have is a *panic*, not an error.
So each row value is a **chain** - `off` falls back to `Mailbox` before `Fifo`,
because what it asked for is an unblocked loop and mailbox keeps that - walked
against what the surface reported, and a fallback is printed rather than left
looking like a setting that did nothing.

The limiter that the overlay measures lives in `about_to_wait` and not in the
frame: producing fewer frames means *asking* for fewer, not drawing one and then
sleeping on a submitted command buffer. `ControlFlow::WaitUntil` hands the
waiting to the platform's own timer.

**The deadline is anchored to the previous deadline, not to when the loop woke**,
and that is the difference between a 240 limit delivering 240 and delivering 220.
A timer fires at or after its target, never before, and the platform's
granularity is around a millisecond - a quarter of a 4.17 ms period. Measuring
the next deadline from the wake-up banks that overshoot into every frame;
measuring it from the previous deadline puts the frames on a fixed grid, so a
late wake is followed by an early-relative one and the average is the rate that
was asked for. Measured: 220 before, a flat 240 after.

None of this touches the simulation. The timestep is fixed at 60 Hz per
[ADR-0007](adr/0007-fixed-timestep-vs-original.md) whatever the frame rate is,
so a 30-limited run takes two ticks a frame rather than running in slow motion.

## What deliberately bypasses the menus

`--race`, the `--screenshot` capture path and `oag-trace` all go straight to a
track. A physics test must not depend on menu navigation, and a capture harness
is not a shell. `--menu-page <id>` draws one page with `--screenshot` for
looking at a layout without walking to it.

## Persistence

`settings.toml` holds `[display]`, `[graphics]`, `[race]` (class, team, circuit
id), `[source]` and `language`. A setting is written on the keypress that
changes it, not on the way out, because there is no way out that is guaranteed
to run - a player quits with the window button as often as with the menu, and
the file is a few hundred bytes.

The file is also **rewritten complete on every load**, with any key it was
missing filled in at its default, so what is on disk is always a full reference
for what can go in it. That rewrite is what finishes the `[graphics]` to
`[display]` migration described above: it runs once per install in practice, and
`[display]` wins if a hand edit ever puts an old key back.

`language` is what skips the picker on the second run. The picker state is still
entered and left the same frame rather than being skipped in the state machine,
so the transition sequence `boot_ground_truth.rs` asserts on is unchanged. A
language this source does not carry falls back to asking, with a note.

## Switching language without relaunching

**Fixed 2026-09-27.** The LANGUAGE row used to only write `settings.language`;
the string table, the tables built from it and the menu's own faces stayed
whatever they were at boot, so the switch a player just made showed nowhere
until the next launch - reported by a maintainer playing Pulse on the PSP,
alongside a second bug this shares its root with (see below).
`Session::resupply_language` (`main/session/menus.rs`) is the reload, and it
runs the LANGUAGE row's own `apply_setting` arm:

- Re-opens the source through `boot::load_shell` - the same "cheap half of the
  boot" a fresh launch calls, measured at 0.05 s on the EU disc - with
  `boot::Options::language` set to the new pick, and folds the result through
  `Shell::from_boot`, the one function both a boot and this reload build a
  `main::session::Shell` from.
- Re-`supply`s [`menu::ValueSource::Teams`]/`Tracks`/`RaceModes`/`Languages`/
  `FrontEndStyles` onto the **already open** `Menu`, rather than building a
  new one - a title's row structure does not depend on which language draws
  its labels, so the player's cursor, page and scroll position are untouched.
- Copies the new `nav_legend`, `ticker`, `frame` and fonts onto the live
  `MenuStage`, and pushes the new faces into the GPU-side atlases through
  `Renderer::set_face_atlas`/`set_buttons_atlas` - the same setters a fresh
  menu stage build already uses, so a language whose plugin names a different
  face (`boot::fonts::role_font`, below) redraws in it immediately.
- Calls `Session::reload_loading_assets`, so the loading screen's own tip text
  follows too.

**What this does not reach.** `self.shell.definition` - the row *tree itself*,
including every literal row title `assets/ui/menu.toml` authors - is left
alone: those titles resolve through `oag_ui::strings::project_table`, a
project-owned translation layer parsed once at process start and entirely
separate from the disc's own table (see "Localised labels" above), and this
build ships one today for English and French only. Picking German changes
nothing there at a fresh boot either, so there is nothing a live reload could
show that a restart would not also fail to. A race already parked behind
`Escape` (`Session::suspended_race`) keeps the `hud::Assets` it loaded when it
started; only a fresh `LAUNCH RACE` picks up the new language, through
`race::Options::language` below.

**Verification gap, stated rather than papered over.** The player-visible
before/after of the HUD case (below) is a real `--race --screenshot` run
against the EU disc; the OPTIONS page's own live switch is not, because
nothing in this build's headless input model can reach it. `--press`/`--hold`
apply one fixed button set on alternating ticks for the whole run, and
reaching LANGUAGE means a scripted sequence - several `down`s, a `cross`, more
`down`s - that model cannot express; `--menu-page` draws a named page but
"takes no input and runs no state machine" (its own doc). `--input-script`
does express a sequence, but is `requires = "race"` and a `--race` run never
opens these menus at all. What is verified instead is
`language_reload_ground_truth.rs`
(`crates/game/tests/language_reload_ground_truth.rs`): three ground-truth
tests proving `boot::load_shell` itself - the call `resupply_language` repeats
- resolves to the requested language's own plugin and produces row labels
that move between German and Italian, which is the exact, shared, GPU-free
mechanism `Shell::from_boot` builds `Teams`/`RaceModes` from on both a fresh
boot and a live switch. The GPU-touching half - the re-`supply` onto an
already-open `Menu` and the atlas swap - is `main`-binary-only code with no
integration-test surface, and is verified by reading `resupply_language`
against `Session::open_menus`'s own construction line for line rather than by
a captured frame. A future session with a way to script multi-step headless
input could close this the rest of the way.

## The HUD following the front end's language

The second half of the same report: Pulse's in-race HUD (`race/hud.rs`) reads
its own language plugins and string table independently of the front end's,
because a `--race` run has no menus to read a saved choice from at all. Its one
call site (`race/load.rs`) used to hand it `None` unconditionally - "the
chain's own default" - rather than the player's saved `settings.language`,
which is invisible on whichever language a title's plugin order happens to
put first (French, on the PSP EU pressing) and wrong on every other pick.
`race::Options::language` is the fix: every launch site sets it from the live
`settings::Settings::language` - `main::prepare::Pending::race_options` for
`--race`, refreshed again in `main::session::Session::launch_race` for every
menu-driven route, because `Session::race_options` can otherwise sit stale
from whenever it was last built while the LANGUAGE row keeps moving
`self.settings.language` underneath it.

`boot::fonts::role_font` had the same bug one level down, shared by
`race::hud::hud_font`: both scanned every language plugin in source order for
the `.fnt` a role names, with no preference for the language actually chosen,
so a role whose file genuinely differs by plugin would draw whichever plugin
happened to load first. Both now ask the chosen language's own slot before
falling back to the scan.

## PAUSE ON FOCUS LOSS

`display.pause_on_focus_loss` (DISPLAY page, `on`/`off`, default `on`, chosen, not measured):
whether a race pauses into its pause menu when the window loses focus. Minimising and Android
`Suspended` pause whatever it says. See `docs/tools/android.md`, "Pausing when the window goes
away".
