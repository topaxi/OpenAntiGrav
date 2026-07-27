# Booting the front end

What `oag-game` does between being launched and showing a menu, and how much of
it is the original's behaviour rather than ours.

```sh
just play
```

That reads `data/images/pulse-psp-usa.chd`, plays the intro reel with the
original's frame-counted pauses, and lands on a Language Selection screen built
from the disc's own XML. START, or space, skips the intro.

```sh
# No display needed.
just play --screenshot /tmp/menu.png --until "Language Selection" --hold start
just play --screenshot /tmp/launch.png --until "Launch Game" --press start,cross
```

## The sequence

| State | What happens |
| --- | --- |
| `Intro Screen` | Parent state. Entered at boot. |
| `Intro Screen->IntroMovie1` | The reel plays. Pause at frame 144, pause at 231, finish flag at 260, each held two seconds. START fires the redirect. |
| `DevPubRedirect` | A one-shot redirect state. Its only job is to leave. |
| `Language Selection` | The disc's own picker. Up and down move, cross selects. |
| `Launch Game` | End of this slice. |

Every one of those names is a **string literal from the original**, not one we
invented. `"Intro Screen->IntroMovie1"` and `"DevPubRedirect"` are cached by name
in the intro state's `OnEnter` at `0x088d7d80`; `"Language Selection"` and
`"Launch Game"` are the two states boot chooses between depending on
`0x08ab07e3`. See [main loop](../ghidra/functions/psp-pulse/main-loop.md) and
[frontend video](../ghidra/functions/psp-pulse/frontend-video.md).

### Where we knowingly differ

**The original runs the picker first and the intro second.** The front-end XML's
`Language Selection` screen has a `LanguageAutoRedirect` whose `Default goto` is
`LogoFMV`, and `LogoFMV` is the screen that plays `Data\Movies\Intro.PMF`. So the
disc's order is: pick a language, watch the intro, land on "press START".

This build boots into the intro and ends at the picker, because that was the
order asked for. The divergence is not hidden: `oag-game` prints the disc's own
redirect target at startup, and
[`Frontend::language_auto_redirect`](../../crates/game/src/frontend.rs) exposes
it so [a test](../../crates/game/tests/boot_ground_truth.rs) asserts it is
`LogoFMV`.

`IntroMovie1` plays `Data\Movies\Intro.PMF` here because that is the movie whose
name is resolved. The reel the original's intro state actually plays is one of
the three 260-frame movies, whose names are **not** recovered. `--movie` takes any
entry name, and the frame counters fire at 144, 231 and 260 whichever reel is
loaded, so pointing it at the right one is a one-line change once it is named.

The dev/pub reel that `IntroMovie1` plays is a *different, earlier* movie from
the one `LogoFMV` plays, which reconciles the two orderings: the full boot on
hardware is almost certainly dev/pub logos, then the picker, then the long intro.
That reading is an **inference**, confidence 75; the pieces are all evidenced but
nothing has been run under an emulator.

Selecting a language fires `Launch Game` and logs it. In the original that is the
state boot goes to *instead of* the picker when a language is already saved, so
using it as the picker's exit is our shortcut, not the original's edge.

## Finding the intro movie

The filename is not in the executable. `Movie_ParseAttributes` (`0x088ba284`)
builds it from the widget's `src` attribute plus `.PMF`, or `_US.PMF` when
`localised` is set, so it only exists in the front-end XML.

The chain that resolves it, all against a real disc:

1. `Data.wad` holds 178 name-shortened XML blobs. Expanding them with
   [`fexml`](../formats/fexml.md) and searching for `Movie` finds **exactly
   one** file with a `Movie` element.
2. That file is `Data\Plugins\PI001\GUI\Skin.xml`, confirmed by hashing the name
   and matching `1f38aacf` in the directory. It is the front-end root: 12
   screens, 56 `FEGlobals` variables, 23 `LoadXML` includes.
3. Its three `Movie` widgets give two distinct `src` values,
   `Data\Movies\Intro` and `Data\Movies\Backdrop`.
4. Appending `.PMF` and hashing gives `71d3c1ec` and `18c51e58`, both of which
   are entries in `Data.wad`. `Data\Movies\Intro_US.PMF` hashes to `29533f0c`,
   which is **not** an entry, so the non-localised branch is the right one.

So the intro is **`Data\Movies\Intro.PMF`**: 5,736,448 bytes, `PSMF0014`,
480x272, 40.04 s, 1200 frames, H.264 Main profile plus 2-channel ATRAC3+ at
44.1 kHz. Confidence **97**: the name is confirmed by hash and the contents by
[three independent arithmetic checks](../formats/pmf.md#what-pins-the-layout-down).

The same hashing exercise resolved the front-end root's siblings and confirmed
two naming rules from `LoadXML`, both by finding the entry that the rule predicts:
`localised="true"` appends `_US` before the extension
(`Credits_Definition_US.xml`, `0caebe51`), and `PlatformSpecific="true"` appends
`_PSP` (`MemoryStickBootScreens_PSP.xml`, `f523fd2c`).

## Playing it, without shipping a decoder

Per [ADR-0004](adr/0004-asset-pipeline.md), the movie is **converted once and
cached**, not decoded in process.

```text
Data.wad entry  ->  pmf::demux  ->  H.264 elementary stream  ->  ffmpeg  ->  raw yuv420p
                    (ours)                                       (theirs)     (cached)
```

| Step | Where | Why there |
| --- | --- | --- |
| Header parse and demux | [`oag-formats::pmf`](../../crates/formats/src/pmf.rs) | Ours to do, testable without a GPU, no dependencies |
| H.264 decode | `ffmpeg`, out of process | An in-workspace decoder is a large non-Rust dependency or a year of work, to show a logo |
| Colour conversion | [`video.wgsl`](../../crates/game/src/video.wgsl) | Free on the GPU, and keeps the cache at 1.5 bytes per pixel |

The cache lives in `data/cache/movies/`, already gitignored, keyed on the entry's
name hash and size so a different disc image cannot collide:

```text
71d3c1ec-5736448.h264                          the demuxed elementary stream
71d3c1ec-5736448-480x272-yuv420p-261.raw       261 frames, 51 MiB
```

**Only 261 frames are converted**, because the intro state stops at frame 260
whatever the reel's length. Converting all 1200 would be 235 MiB of cache for
eight seconds of screen time. `--full-movie` converts everything.

`yuv420p` was chosen over RGBA for two reasons: it is what the decoder produces
natively, so the transcode does no colour conversion, and it is 1.5 bytes per
pixel rather than 4. Frames are streamed from the file one at a time rather than
held in memory. The shader converts with BT.601 limited range, which is an
assumption; a wrong choice shows up as washed-out colour, not a broken picture.

**Without `ffmpeg` the game still runs.** The sequence plays out over the movie's
real duration against the black backdrop the `LogoFMV` screen puts behind the
movie anyway, and the missing tool is named on stdout. `--no-video` forces that
path, and it reads 2 KiB instead of 5 MiB because only the header is needed. In
that case a frame counter is drawn over the intro, because eight seconds of black
screen is otherwise indistinguishable from a hang; `--overlay` turns it on when
there *is* a picture, which is how the pacing above was checked frame by frame.

Audio is **not** played. ATRAC3+ needs a decoder we do not have, and the demuxer
hands the frames over intact for whenever we do.

## The Language Selection screen

Everything on it comes from the disc.

**The screen** is found two independent ways that agree: the element with
`type="Language Selection"`, and the element with a `DisplayLanguages` child. It
also has a `Menu name="Language" save="true"` and three `Text` widgets whose
`idstring`s are looked up in the string table.

**The languages** are plugins, not a list in code. Each of
`Data\Plugins\PI008..PI012\Definition.xml` carries `<Font Language="French">`
naming itself, `<Entry ID="French" String="Français">` naming itself in itself,
and `<Entry ID="Dynamic Entry File Source" String="Data\Plugins\PI008\entries.xml">`
pointing at its string table. Nothing in the *path* says which language a plugin
is, so this had to be read out rather than assumed:

| Plugin | Language | Shown as | Strings |
| --- | --- | --- | ---: |
| `PI008` | French | Français | 1,600 |
| `PI009` | German | Deutsch | 1,600 |
| `PI010` | Spanish | Español | 1,600 |
| `PI011` | Italian | Italiano | 1,600 |
| `PI012` | English | English | 1,621 |

Confidence **95**: every plugin id, language name and native name is read from
the file and [asserted](../../crates/game/tests/boot_ground_truth.rs).

A PAL disc will have a different set, which is why the list is read rather than
hard-coded. The plugin ids in
[`oag-assets::pulse`](../../crates/assets/src/pulse.rs) are the candidates to
probe, not the answer.

### What the screen does not yet do faithfully

- **The text is not the game's font.** The `.fnt` metrics decode cleanly but the
  atlas's pixel layout does not; see [fnt.md](../formats/fnt.md). The menu uses
  5x7 glyphs written for this project, uppercase only, and characters outside
  that set are **skipped**, so `Français` draws as `FRANAIS`. That is meant to
  look like the approximation it is.
- **The layout is ours.** The `Text` widgets are at their own XML coordinates, in
  their own colours, but the language rows are not: `DisplayLanguages` and `Menu`
  have layout attributes this build does not read yet.
- **Near-black text is lifted.** The picker's title is `0xFF000000`, because the
  real screen sits on the menu's lit background, which nothing draws yet. Black
  on black would look like a bug in our code rather than a missing background.
- `FE_CONFIRM_BUTTON` resolves to a single glyph in the game's own font encoding,
  which our font has no character for, so it draws as nothing.

## The state machine

[`state_machine.rs`](../../crates/game/src/state_machine.rs) is string-keyed and
hierarchical, because the original's is: `StateMachine_TransitionTo`
(`0x0889123c`) takes a name, queries are `strcmp` against the current name, and
children are named `"Parent->Child"`.

Three behaviours were copied deliberately rather than tidied:

1. **Firing queues; applying transitions.** States fire from inside their own
   update, so transitioning immediately would tear down the code that fired.
2. **A transition only unwinds to the common ancestor.** Swapping a child leaves
   its parent entered, and exits are emitted innermost-first.
3. **The intro's redirect target is cleared once fired.** The original zeroes
   `state->devPubRedirect` after firing, so a second START does nothing. Without
   that, the intro's own frame counters would fire the redirect again and bounce
   out of the picker.

Skipping the intro **fires the redirect rather than stopping the player**. The
teardown is a consequence of the transition, not a peer of it, which is the
ordering the original's exit path has.

Input is the [abstract button layer](../ghidra/functions/psp-pulse/input.md)
reproduced as-is: bit indices, not masks, with `activate` mapped to cross and
`cancel` to circle. START is index `0xe`, and reading that as a mask would test
up and down instead, which is why there is
[a test](../../crates/gameplay/src/input.rs) saying so.

That module lives in `oag-gameplay` rather than in `oag-game`, because the
simulation owns the input snapshot type and everything that produces one - the
front end, a keyboard, a replay - depends on it. The keyboard mapping itself is
in `oag-input`. See [workspace layout](workspace-layout.md).

## Timestep

Fixed 60 Hz, per [ADR-0007](adr/0007-fixed-timestep-vs-original.md), driven by
`oag_core::TickClock`. The movie advances at 30000/1001 Hz inside that, so a
2-second hold is 59.94 ticks and the reel's 260th frame arrives on tick 519.

The headless capture uses the same fixed step with no clock at all, so
`--until` and `--ticks` are reproducible.

## Crates

Two crates were added.

**`oag-assets`** is the runtime asset boundary
[the workspace layout reserved](workspace-layout.md) for it. `Archive` opens a
WAD from a path or from `<image>:<path-on-disc>` and reads entries by index, by
name or by name hash. This is the fourth place that code was needed;
`oag-tools`'s `oag-wad`, `oag-view`'s asset loader and its mesh loader still have
their own copies and should migrate.

**`oag-game`** is the composition root, as a thin binary over a library so the
sequence can be tested. Nothing depends on it.

| Module | What it is |
| --- | --- |
| `state_machine` | The string-keyed hierarchical machine |
| `input` | The abstract button layer |
| `screen` | Front-end XML to a screen model |
| `language` | Language plugins and string tables |
| `movie` | The player's pacing, and the transcode and cache |
| `frontend` | The boot sequence. No I/O, no GPU, no clock |
| `boot` | Loading it all out of a disc image |
| `render`, `font`, `keys` | wgpu, glyphs, keyboard |
| `capture` | Headless screenshot, through the same renderer |

`frontend` holding no I/O is what makes the sequencing testable: 21 unit tests
cover the pauses, the holds, the skip, the redirect clearing and the picker.

## Verified, inferred, guessed

**Verified against a real disc**, and asserted by
[boot](../../crates/game/tests/boot_ground_truth.rs) and
[PMF](../../crates/assets/tests/pmf_ground_truth.rs) ground-truth tests:

- the front-end root's archive name, and that it is the only XML with a `Movie`;
- `Data\Movies\Intro.PMF` and `Data\Movies\Backdrop.PMF` as assembled names;
- the intro's dimensions, duration, stream count and 1200-frame length;
- all 17 movies parsing, demuxing with zero strays, and agreeing with their own
  durations;
- three 260-frame reels existing;
- the five languages, their plugin ids, and their names in themselves;
- `LanguageAutoRedirect` going to `LogoFMV`;
- the `_US` and `_PSP` `LoadXML` suffix rules.

**Inferred, plausible, not run:**

- that the 260-frame reels are the dev/pub cards, hence the boot order above
  (75);
- that the three 260-frame reels are regional variants (40; it is a guess);
- that the video is BT.601 limited range (70; it looks right);
- that a two-second hold at frame 260 precedes the redirect, rather than the
  redirect firing at 260 directly. `frontend-video.md` says the finish flag is
  set and the time stamped, and that pauses release after two seconds; treating
  the flag as released by the same timer is a reading (70).

**Not established at all:** the names of eight movies, whether the picker's real
layout matches ours, and anything about audio.
