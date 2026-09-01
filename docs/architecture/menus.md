# The menus

**Ours, not the disc's.** Every other document in this tree describes something
recovered from a Wipeout Pulse executable or a Wipeout Pulse asset. This one
describes a thing this project invented, and says why inventing it was the right
call rather than a shortcut.

Implemented in [`crates/game/src/menu.rs`](../../crates/game/src/menu.rs), with
the tree itself in [`assets/ui/menu.toml`](../../assets/ui/menu.toml) and the
disc-side lists in [`crates/game/src/catalogue.rs`](../../crates/game/src/catalogue.rs).

## Why not the front-end XML

The disc carries `Data\Plugins\PI001\GUI\MainMenu_Definition.xml`, and
[`screen.rs`](../../crates/game/src/screen.rs) already parses that dialect - the
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
Both PSP titles' frames are unread, so their menus draw exactly as they did.

## What still comes off the disc

Not the *structure*, but the *contents*, wherever the contents are a property of
the release rather than of this project:

| Row | Where its options come from |
| --- | --- |
| TRACK | `Data\Plugins\PI001\Definition.xml`, and the label from the language's string table |
| LANGUAGE | the language plugins `PI008`-`PI012` |
| TEAM | `values_from = "teams"` - the roster [`boot::load_teams`](../../crates/game/src/boot/roster.rs) read off the open source's own declared definition plus any mounted pack, not a repository list |
| SPEED CLASS | `oag_physics::SpeedClass::ALL` |
| MONITOR | winit's own monitor list, read every time the menus open |
| WINDOW MODE / SIZE / ASPECT / RENDER SCALE / UPSCALER / UPSCALER SHARPNESS / BRIGHTNESS / GAMMA / FIELD OF VIEW | `oag_game::display`, pinned to its own `ALL`/`OFFERED` lists by a test |
| PERFORMANCE OVERLAY / FRAME LIMIT / VSYNC | `oag_game::perf`, pinned the same way |

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
  [`crates/game/src/menu/tests/definition.rs`](../../crates/game/src/menu/tests/definition.rs).
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

## What is not built

- **Rebinding.** `oag_input::keys::map_key` is a hardcoded `match`; a `binding`
  row displays what it returns and cannot change it. Rebinding needs a table,
  persistence and conflict handling of its own. The row exists so the format is
  settled before that lands and so the gap is visible in the game rather than
  only here. `keys::bound_keys` asks `map_key` itself about a candidate set
  rather than keeping a second table, so what the page shows cannot drift from
  what the game does.
- **Localised labels.** Row labels are literal text. `string_id` is accepted and
  not read, so adding localisation later is not a format change.
- **Applying a language without relaunching.** The LANGUAGE row writes the
  setting, and the string table it selects is loaded once at boot; the change
  therefore lands on the next launch. Anisotropic filtering is the same, and is
  less noticeable because a race is built after the menus anyway.
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
discarded exactly as every race used to be. There is deliberately no pause
*overlay* yet - backing into the menus over a parked race draws the ordinary
menu screen, not a translucent layer over the frozen picture - only the
World's own lifetime changed. See `crates/game/src/main/session/menus.rs`.

**The race's own music already worked this way**, and is the reason the
World's turn was safe to build the same way. Its playlist - through the
sixteen soundtrack tracks, distinct from the menu's loop - already paused and
resumed, its position kept in `Audio` for the process's lifetime rather than
reset on every race, well before the World did. That was audio state outside
the simulation, the same way `docs/architecture/determinism.md` already puts
every other sound outside it, not race state being kept alive - but it meant
`Session::resume_race` had `Audio::start_race_music` to call rather than
anything to invent. See `crates/game/src/audio.rs`'s `start_race_music` and
`pause_race_music`.

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
| GRAPHICS | RENDERER, RENDER SCALE, UPSCALER, UPSCALER SHARPNESS, ANISOTROPIC FILTERING, FIELD OF VIEW, PERFORMANCE OVERLAY | How the picture is drawn |

RENDERER is the most literal case of that criterion there is - it is *what*
draws - which is why it sits on GRAPHICS despite resembling MONITOR in every
mechanical respect: both are hardware the definition file cannot enumerate, both
come through `values_from`, both fall back rather than fail on a name the machine
no longer has. See [`crate::adapter`](../../crates/game/src/adapter.rs) for what
it can offer, and in particular for why "render on the CPU" is a row that appears
only on a machine with a software driver installed rather than a switch this
project could provide.

A row can also be **warned** rather than greyed, which is a different thing and
the UPSCALER row is why it exists. A greyed row cannot be changed *now* because
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

UPSCALER sits directly under RENDER SCALE because only the pairing means
anything: at 100 % there is nothing to upscale and the choice is between a blit
and a sharpen. UPSCALER SHARPNESS is greyed out when the upscaler is off, which
leaves no sharpen to adjust - greyed rather than hidden, for the reason FRAME LIMIT is:
a row that vanishes gives a player no way to find out what took it away. Its
unit is **stops**, upstream FidelityFX's own, where zero is maximum and each
whole step halves it; that runs the opposite way to what a reader expects and is
kept anyway, because a number a player reads about elsewhere should mean the
same thing here.

Turning vsync off changes nothing about the frame that is drawn, only about when
it is shown; halving the render scale changes the frame itself. That test is
what settles the two rows a reader would otherwise argue about:

- **Brightness and gamma are DISPLAY**, even though they are a fragment shader.
  They are a calibration of somebody's monitor, applied to a frame the game has
  already finished drawing - see below.
- **The performance overlay is GRAPHICS**, even though it is a diagnostic and
  not decoration. It is drawn *into* the offscreen target, at the render scale,
  over whatever stage is running; measuring a frame nobody is presenting is
  exactly the way to get it wrong.

`settings.toml` is split the same way, into `[display]` and `[graphics]`, so
there is one vocabulary rather than two. Everything was in `[graphics]` before
the split, and a file that still is gets its moved keys carried across on load
rather than silently dropped - serde ignores a table field it does not know, so
without that a player who had set borderless at 120 Hz would have opened the
game windowed at 240 with no message and nothing wrong in the file.

The value types all still live in one module,
[`crates/game/src/display.rs`](../../crates/game/src/display.rs). That module is
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
[`crates/game/src/perf.rs`](../../crates/game/src/perf.rs) has the reasoning; the
part that belongs on this page is why it is **three values and not a toggle**:
`fps` answers "is there headroom", `pacing` answers "are the frames evenly
spaced", and the second question is the one an average frame rate is incapable
of answering. A game can hold a perfect 60 and stutter visibly, and only the
graph shows it.

It is drawn **into the offscreen target**, so at a render scale of 50 % the
overlay is drawn at 50 % too - it costs what the game costs, or it is measuring a
frame that does not exist. It uses this project's own 5x7 glyphs rather than the
disc's font, because `--race` never loads a font and an overlay that vanishes on
the route where it is most wanted is not an overlay.

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
