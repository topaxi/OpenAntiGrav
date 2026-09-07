# Players should be able to build pilots in game, and a naive editor would eat their comments

## Landed 2026-09-07: rename and delete, and the text entry they needed

The two operations the first cut deferred are in, and so is the thing that
was blocking them - **this project had no way to accept typed text at all**.

- **`crate::prompt`** is that missing piece, and it is deliberately about
  text entry rather than about pilots: `Keyboard` knows nothing of what the
  string it builds is for. A grid of 40 cells stepped with the d-pad, typed
  with Cross, deleted with Square or its own DEL key, accepted with Start or
  its own OK key, cancelled with Circle. `Confirm` rides along because
  delete needs one. Both consume edges through `Input::take`, which is what
  stops the Cross that *opened* a prompt from also accepting it.
- **The character set is exactly what `pilots::check_name` allows**, checked
  in both directions by a test: a grid offering a slash would build a name
  refused only at the end, and a grid missing `_` would show a player a file
  they cannot type back in.
- **A desk keyboard types into it too**, through `main/typing.rs` -
  `rebind.rs`'s sibling, and split out for the same reason. Diverted rather
  than shared, or a letter also bound to a game button would type itself and
  press the grid's selected key at once.
- **`rename_pilot` is a move**, so the file arrives at its new name byte for
  byte - the strongest form of the promise `set_axis` makes more carefully,
  and free. Check-then-move, because `std::fs::rename` silently overwrites
  on Unix. Refused for a pilot with no file, gated on `Entry::from_file`
  rather than on the four built-in *names*: once a player saves an edit to
  `aggressive` a file exists and renaming it is legitimate.
- **Delete asks, and what it asks is the point.** For a file named after a
  built-in the confirm says, by name, that deleting it restores the built-in
  rather than removing a pilot. A `warn_when` on the row could not have said
  it: the loader requires two conditions by design and there is one fact.
- **First caller of `assets/ui/strings/english.toml`'s project-owned ids.**
  Which immediately found a real bug: `menu::definition::resolve` fell back
  to the *id* when a `string_id` missed, so on the EU disc a French player
  read `OAG_PILOT_RENAME` off a row whose label says `RENAME`. It now falls
  back to the row's own `label`.
- **`--menu-prompt`** draws either prompt over `--menu-page`, because that
  path runs no `Menu::update` and a prompt can otherwise never appear
  headlessly. Four layout defects were found by looking at what it produced
  and none by arithmetic: a hint line drawn below the panel, a note drawn
  through the grid, DEL and OK running together, and the confirm's answers
  drawn on top of its own longest message.

## Landed 2026-09-06: list, edit, save, create-from-template

The maintainer's scope call was **list -> edit axis ranges -> save ->
create-from-template**, with delete and rename explicitly out of this cut
(rename needs text entry, which is its own screen and its own thread). All
four landed:

- An **AI PILOTS** page in `assets/ui/menu.toml`, reachable from OPTIONS.
  `PILOT` and `AXIS` choice rows pick what to edit; built-ins are marked
  `(built-in)` in the list, so it does not imply deleting a same-named file
  would remove one - it restores the built-in, per the trap below.
- `LOW`/`HIGH` choice rows are the two-handle control this menu format can
  offer: each is a discretized set of steps across `crate::pilots::limit`'s
  range for the axis, **plus the pilot's own exact value**, so a
  hand-authored number that is not on a round step still seeds correctly
  and SAVE without touching either changes nothing.
- **`toml_edit`, not `toml::to_string_pretty`.** This was the strategy
  decision this thread flagged as blocking, and it is the one taken:
  `crate::pilots::set_axis` reads the document, mutates only the array the
  edited axis names, and writes the rest back byte for byte.
  `pilots.rs`'s module doc now records the choice beside the "nothing is
  rewritten on load" sentence, and
  `editing_an_axis_keeps_the_hand_written_comment_above_it` is the test
  that proves a hand-written comment survives a save - along with a
  separate test that the saved number is `f32::to_string`'s own text
  (`1.05`), never an `f32`-as-`f64` expansion (`1.0499999523162842`).
- **First edit of an untouched built-in starts from `crate::pilots::template`
  of its own resolved numbers**, not an empty document - writing only the
  one edited axis would otherwise leave every other axis silently pulled
  from `balanced` the moment the new file replaced the built-in.
  `a_template_names_every_axis_explicitly` is the test.
- Create-from-template (`NEW FROM PILOT`) copies whichever pilot is
  currently selected, auto-named `pilot-1`, `pilot-2`, ... - no text entry
  needed.
- Session wiring lives in `crates/game/src/main/session/pilot_editor.rs`
  (new file, kept out of `session/menus.rs` and `session/apply.rs`, both
  close to the 1,000-line ceiling) plus one line in each of those two and
  in `session.rs`/`app.rs` for the roster field. `PILOT`/`AXIS`/`LOW`/`HIGH`
  are read live off the menu's own rows (`Entry::chosen`) at save time -
  nothing is cached as "the pending edit".
- Pilot spans still only reach a race through `race::start`'s own
  `pilots::load()`, called fresh at race start and never cached on
  `Session` - the editor's own `Session::pilot_roster` is a separate copy,
  purely for the page to list and edit, so a save mid-race cannot change
  the race in progress.


**Requested by the maintainer 2026-09-06**: an in-game menu offering classic
CRUD over the pilot `.toml` files, so a player can build and experiment with
their own AI definitions without leaving the game or opening an editor.

Nothing here is reverse engineering. The original has no such feature and never
could - pilots are this project's own invention. This is a PC-game feature on
project-owned data, and it is squarely in scope; see below.

## It fits the existing menu tree, and that is already settled

`assets/ui/menu.toml`'s own header answers the "should an invented menu exist"
question before it is asked:

> This file is OURS. It is not a transcription of the disc's
> `MainMenu_Definition.xml` and is not meant to become one: this project needs
> the menus a PC game has (input bindings, graphics settings) and the original
> had no reason to carry those.

A pilot editor is the same category as input bindings and graphics settings. It
belongs in that tree, not in anything read off the disc, and it introduces no
tension with [the menus that do draw the disc's layout](the-menus-draw-the-discs-layout-the-chrome.md).

No shipped content is involved either: every number in a pilot file is the
project's own, per
[ADR-0006](../docs/architecture/adr/0006-no-copyrighted-content.md).

## The trap this feature walks straight into

`crates/game/src/pilots.rs` states, as a deliberate difference from
`settings.rs`:

> **Nothing is rewritten on load.** `settings.rs` rewrites its file every run so
> every key is discoverable; these are hand-authored files with the author's own
> comments in them, and clobbering those would be hostile.

`assets/ui/example-pilot.toml` is itself mostly comments - it is written to be
read and edited by a person, and it invites exactly that ("small enough to paste
into a forum post").

**A round-trip editor built the obvious way destroys all of it.** `settings.rs`
saves with `toml::to_string_pretty` (`settings.rs:701`, `:882`), which
serialises a struct and emits canonical TOML: every comment, every blank line,
every hand-chosen key order, gone. A player who writes a pilot by hand, tweaks
one axis in the menu and saves has silently lost their own annotations.

Three ways out, in the order they should be considered:

1. **Format-preserving edit** (`toml_edit` rather than `toml::to_string_pretty`).
   Reads the document, mutates only the values touched, writes the rest back
   byte-for-byte. This is the right answer if the dependency is acceptable -
   check `just check-deps` and the workspace's existing dependency posture
   before assuming it is.
2. **Editor-owned files only.** The menu may only modify files it created, and
   refuses to save over one it did not - hand-authored pilots stay read-only
   from in game. Honest and cheap, but it makes the feature half a feature.
3. **Write a fresh file and keep the original.** Save-as rather than save.
   Sidesteps the problem without solving it, and multiplies files.

Whichever is chosen, **the choice must be recorded in `pilots.rs`'s own module
doc**, beside the sentence quoted above, because that sentence will otherwise
read as still-true and it will not be.

## The blocking gap: nothing in this project can accept typed text

Checked 2026-09-06: there is **no text entry of any kind** - no text field, no
on-screen keyboard, nothing in `crates/game/src/` or `crates/input/src/` that
turns keystrokes into a string. The whole menu layer is rows and abstract
buttons.

Creating or renaming a pilot needs a name. So **text entry is a prerequisite,
not a detail of this feature**, and it is the single largest piece of work here.
It also has to work on a pad, not only a keyboard, since the input layer is
built around abstract buttons - which usually means an on-screen keyboard, and
that is its own screen with its own layout.

A create-only-from-template flow that auto-names (`pilot-1`, `pilot-2`) would
dodge it for a first cut. Whether that is worth shipping without renaming is a
maintainer call, not one to make here.

## Details that will each cost time if they are discovered late

- **Deleting a built-in-named file does not delete a pilot.** A file called
  `aggressive.toml` *replaces* the built-in of that name rather than adding one
  (`assets/ai/example-pilot.toml`, and `crates/ai/src/pilot.rs:281` for the four:
  `balanced`, `aggressive`, `passive`, `shy`). So deleting it **restores the
  built-in**. The menu must say that, or a player will delete `aggressive` and
  be baffled that it is still there.
- **Every axis is a range `[low, high]`, not a value.** Each craft flying the
  pilot draws its own value inside it, which is the whole point - four craft on
  one pilot are four drivers. The editor needs a two-handle control, not a
  slider, and `[0.4, 0.4]` must stay expressible for a player who genuinely
  wants them identical.
- **Unknown keys are an error, by design.** A menu cannot produce one, but it
  can produce an out-of-range value: `commitment` is capped at 1.05 whatever the
  file says, and other axes have their own limits. The editor should refuse or
  clamp at the point of editing rather than writing a file the loader will
  reject on next launch.
- **`pilots::directory()` returns `None`** on a platform `dirs` cannot place a
  config directory on (`pilots.rs:47`). Loading already handles that; a *writing*
  feature has a second failure mode - the directory exists but is not writable -
  and needs an error path that reaches the player rather than a panic.
- **Pilot spans feed the world hash.** `pilots.rs:145` hashes `span.low` and
  `span.high`. Editing a pilot must not be able to change a race already in
  progress; resolve pilots at race start and leave the running race alone, the
  way the control scheme already resolves once at startup
  ([sideshift-has-no-runtime-leg.md](sideshift-has-no-runtime-leg.md)).
- **A new axis appends to the end of the draw order, for ever.** If the editor
  ships alongside the AI barrel-roll axes
  ([ai.md](../docs/gameplay/ai.md), which absorbed that thread when the work landed),
  the ordering rule in `docs/gameplay/ai.md` binds both, and an editor that
  writes keys in its own order must not imply the *draw* order changed.
- **New menu actions are validated at startup.** `menu/definition.rs:415` parses
  every `action` against `Action::all()` and fails naming the unknown one, so the
  CRUD actions must be added there - a page referencing an action that does not
  exist is a startup error, not a dead row. `--menu <path>` runs an alternate
  tree without a rebuild, which is the way to iterate on the layout.
- **Labels have no translations.** `assets/ui/menu.toml` supports `string_id`
  against `assets/ui/strings/<language>.toml`, but this build's own labels do not
  use it yet. A new screen full of invented English is the subject of
  [invented-ui-text-has-no-translation-and-the.md](invented-ui-text-has-no-translation-and-the.md);
  at minimum, do not make that thread's problem bigger without noting it.

## Open

- **The keyboard's layout is ours: chosen, not measured.** Pulse authors no
  keyboard and does not call `sceUtilityOsk` - it has a `<TagInput>` cell
  scroller instead, whose 70-character alphabet cannot spell a filename.
  That is a recorded result rather than an assumption; see
  [pulse-authors-its-own-text-entry-and-it-is-not-a-keyboard.md](pulse-authors-its-own-text-entry-and-it-is-not-a-keyboard.md),
  which is the thread for building the disc's own widget for the screens it
  really is the idiom for.
- **The keyboard offers no uppercase and no space**, because a pilot name
  becomes a filename. The moment a second caller wants a *display* name
  rather than a filename, `KEYS` has to become a parameter rather than a
  `const` - it is one field, and guessing at it now would be worse.
- **`Confirm` cannot measure its own wrapped message.** The answers are
  anchored to the panel's bottom so nothing overlaps whatever the message
  turns out to be, which is correct rather than merely safe - but a
  translation long enough to fill the whole panel would still run into
  them. `menu::draw_list` already threads a `measure` closure through for
  the strip; the same could be done here if it ever matters.
- Whether a player can see what a pilot actually does without racing it. A
  preview - "this pilot brakes late and defends hard" derived from its axes -
  would make experimentation much cheaper, and is entirely invented UI.
  Untouched by this cut.
- **Landed 2026-09-07**: every row on the AI PILOTS page now names a
  `string_id`, the page itself names a `title_string_id` (new mechanism,
  resolved the same way a row's `label` is), and `pilot_choice`'s
  "(built-in)" suffix is looked up too. `just check-strings`
  (`scripts/check-strings.py`) now gates this: a new `menu.toml` row/page
  with no id fails the build. Every other page in the file still carries
  none - see
  [invented-ui-text-has-no-translation-and-the.md](invented-ui-text-has-no-translation-and-the.md)
  for the ratchet that bounds it.

## Next Steps

1. The axis preview above - it needs no new mechanism, only reading
   `Pilot`'s own numbers into a sentence, and it is what makes
   experimenting cheap rather than blind.
2. Give the rest of `assets/ui/menu.toml`'s rows a `string_id` - the AI
   PILOTS page's own are done now, and the mechanism (including the
   `title_string_id` a page carries too) is proven end to end; the
   remaining eight pages are bookkeeping rather than design, one
   `BASELINE_LABELS`/`BASELINE_TITLES` row in `check-strings.py` deleted per
   row converted.
3. Reuse `crate::prompt::Keyboard` for the next thing that needs a name -
   a profile, a replay, a saved setup. It was built to be reused and
   nothing about it knows what a pilot is; the only thing to decide is
   whether that caller wants a wider character set.
