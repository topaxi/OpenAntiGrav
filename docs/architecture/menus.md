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

So the tree is ours and it is written down in one file that says so. What the
disc's own menu XML contains is a separate, still-unread question, and it belongs
on a page under [`docs/formats/`](../formats/README.md) when someone reads it.

## What still comes off the disc

Not the *structure*, but the *contents*, wherever the contents are a property of
the release rather than of this project:

| Row | Where its options come from |
| --- | --- |
| TRACK | `Data\Plugins\PI001\Definition.xml`, and the label from the language's string table |
| LANGUAGE | the language plugins `PI008`-`PI012` |
| TEAM | `oag_formats::handling::TEAMS`, pinned to the eight shipped `handlingstats.xml` files by a test |
| SPEED CLASS | `oag_physics::SpeedClass::ALL` |
| MONITOR | winit's own monitor list, read every time the menus open |
| WINDOW MODE / SIZE / ASPECT / RENDER SCALE / BRIGHTNESS / GAMMA / FIELD OF VIEW | `oag_game::display`, pinned to its own `ALL`/`OFFERED` lists by a test |
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
| A page nothing links to | A menu somebody wrote and forgot to hang off anything. Parses, resolves, and ships silently without this |

Three more tests pin the asset against the code's own lists, which is the class
of mistake the format check cannot see:

- The TEAM row must *be* `handling::TEAMS`, and every anisotropy value must
  parse as `Anisotropy`. Both would otherwise fail deep inside an archive lookup
  at race load, with a message about a missing WAD entry rather than about a
  menu.
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

The menus never read or write settings, and never touch a disc. Three narrow
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
- `MenuEvent::Changed { setting, value }` - what the player moved. The
  composition root applies and persists it.

A `choice` option carries a stored `value` and a displayed `label`, and they are
different strings exactly when the list came off a disc: `16_Track` against
"Talon's Junction White". That distinction is what lets the menus name a circuit
without this repository containing its name.

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
- **Anything a HUD needs.** When `oag-ui` exists (M5, see
  [workspace layout](workspace-layout.md)), this module moves into it.

## What escape does

**Back one level, everywhere**, which is one rule and three places it lands:

| Where | What happens |
| --- | --- |
| The menus | Pops a page, exactly as circle does. On the root page there is nothing behind the menus, so it quits |
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

**Leaving a race discards it.** There is no pause and no resume: the `World` is
dropped and re-entering loads a fresh race. A player who backs out expects to
lose the race; one who backs out and finds a *stale* one would not, so a
half-built pause would be worse than none.

## The background the menus sit on

The rows are drawn over **the disc's own looping menu backdrop**, and that is a
recovery rather than decoration this project invented. `FE Screen` - the
original's main menu - carries a `Movie` widget naming
`Data\Movies\Backdrop.PMF`, and a cold boot under PPSSPP with `MoviePlayer_Open`
armed from reset opens exactly two movies in ten minutes: the intro, and this
one. See [frontend boot](frontend-boot.md) and
[`frontend-video.md`](../ghidra/functions/psp-pulse/frontend-video.md). The menu
*tree* is ours; what it sits on is the disc's, and citing which widget names it
is what keeps the two apart.

On PSP it is a 480x272 `.PMF`, 270 frames, 9.01 seconds. On PS2 the same name
resolves through `boot::LOOSE_MOVIES` to `BG512.IPF` / `BG640.IPF` - IPU video,
512x512 or 640x448 with a declared 4:3 display aspect, so it is **pillarboxed
rather than stretched**, exactly as the `.PSS` intro cuts already are.

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

## DISPLAY against GRAPHICS

Two pages under OPTIONS rather than one, and the line between them is **whether
the renderer would notice**:

| | Rows | What they have in common |
| --- | --- | --- |
| DISPLAY | MONITOR, WINDOW MODE, WINDOW SIZE, ASPECT RATIO, VSYNC, FRAME LIMIT, BRIGHTNESS, GAMMA | The picture's container, and how a finished frame reaches a screen |
| GRAPHICS | RENDER SCALE, ANISOTROPIC FILTERING, FIELD OF VIEW, PERFORMANCE OVERLAY | How the picture is drawn |

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
