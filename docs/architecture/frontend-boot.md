# Booting the front end

What `oag-game` does between being launched and showing a menu, and how much of
it is the original's behaviour rather than ours.

```sh
just play
```

That reads `data/images/pulse-psp-usa.chd`, plays `Data\Movies\Intro.PMF` - the
40-second Pulse intro, and the only movie the disc's own boot ever opens - and
lands on a Language Selection screen built from the disc's own XML. START, or
space, skips it. Picking a language fires `Launch Game`, and `Launch Game`
starts [a race](../tools/oag-game.md) in the same window.

```sh
# No display needed.
just play --screenshot /tmp/menu.png --until "Language Selection" --hold start
just play --screenshot /tmp/launch.png --until "Launch Game" --press start,cross --ticks 60
```

## The sequence

| State | What happens |
| --- | --- |
| `LogoFMV` | Entered at boot. `Data\Movies\Intro.PMF` plays straight through, 1200 frames of it. START or cross skips it. |
| `Language Selection` | The disc's own picker. Up and down move, cross selects. |
| `Launch Game` | End of the front end. The composition root loads a track and a ship and hands the window to a race. |

Every one of those names is a **string literal from the original**, not one we
invented. `"LogoFMV"` is a screen in the disc's front-end XML, and so is its
`Movie` widget's `src`; `"Language Selection"` and `"Launch Game"` are the two
states boot chooses between depending on `0x08ab07e3`. See
[main loop](../ghidra/functions/psp-pulse/main-loop.md) and
[frontend video](../ghidra/functions/psp-pulse/frontend-video.md).

`--reel` boots a second, **off-path** leg instead:

| State | What happens |
| --- | --- |
| `Intro Screen` | Parent state. |
| `Intro Screen->IntroMovie1` | The 260-frame dev/pub reel plays. Pause at frame 144, pause at 231, finish flag at 260, each held two seconds. START fires the redirect. |
| `DevPubRedirect` | A one-shot redirect state. Its only job is to leave. |

Those names are literals too - `"Intro Screen->IntroMovie1"` and
`"DevPubRedirect"` are cached by name in that state's `OnEnter` at `0x088d7d80` -
and the code is real and evidenced. What is **not** established is what triggers
it: the disc's own boot never enters it, so it is behind a flag rather than in
the boot order. See [what the disc actually does](#what-the-disc-actually-does-at-boot).

### Where we knowingly differ

**The original runs the picker first and `LogoFMV` second.** The front-end XML's
`Language Selection` screen has a `LanguageAutoRedirect` whose `Default goto` is
`LogoFMV`. So the disc's order is: pick a language, watch the intro, land on
"press START".

This build boots into `LogoFMV` and ends at the picker, because that was the
order asked for. The divergence is not hidden: `oag-game` prints the disc's own
redirect target at startup, and
[`Frontend::language_auto_redirect`](../../crates/game/src/frontend.rs) exposes
it so [a test](../../crates/game/tests/boot_ground_truth.rs) asserts it is
`LogoFMV`.

**`LogoFMV`'s own exits differ too.** On the disc the movie finishing fires
`AutoRedirect` to `LogoFMV->Show Logo` - the Pulse logo and PRESS START - and any
of start, circle, square, triangle or cross fires `LogoFMVRedirectScreen`, which
goes to the same place. Here both go to the picker, because the picker is what
this build has next; and only start and cross are read, because those are the
buttons the abstract layer carries.

**This build booted the dev/pub reel for a while, and that was wrong.** The reel
is the one whose contents fit `IntroMovie1`'s frame counters, and pointing the
boot at it was reasoned from those constants rather than from what the disc does.
The consequence was visible: the three reels are Pure-era shared assets, so
launching Wipeout Pulse showed eight seconds of white-and-cyan Sony/Studio
Liverpool cards with no Pulse branding anywhere, and the machine's own intro
never played. `--reel` is where that leg lives now.

An earlier version of this page also guessed that the full boot on hardware is
dev/pub logos, then the picker, then the long intro, at confidence 75. **That is
wrong, and running it settled it** - see [what the disc actually
does](#what-the-disc-actually-does-at-boot).

## The dev/pub reel

`Data.wad` holds exactly three 260-frame movies, at entry indices 440, 441 and
442, immediately after `Data\Movies\Intro.PMF` at 439. All three are 480x272,
8.68 s, `PSMF0012`, with 2-channel ATRAC3+ audio, and none of their names is
recovered.

Decoding them settles what the intro state's frame counters mean. Measured as the
mean absolute luma difference between consecutive frames, all three are **exactly
static** across frames 144 and 231 - the two the state pauses at - and moving on
either side of them. Rendering those frames shows why:

| Frame | What is on screen |
| ---: | --- |
| 144 | `SONY COMPUTER ENTERTAINMENT <region> PRESENTS` |
| 231 | `A STUDIO LIVERPOOL GAME`, with the Studio Liverpool mark and an "A-G RACING //2197" plate |
| 260 | The reel ends |

So the two-second holds are the publisher card and the developer card, held in
turn. That reading was an inference at confidence **82** in
[frontend video](../ghidra/functions/psp-pulse/frontend-video.md); it is now
**95**, from the picture itself. `Data\Movies\Intro.PMF` is the negative control
and it fails cleanly: it is moving at all three frames, with a mean interframe
luma delta of 5.57 against the reels' 0.15.

The three differ only in the publisher line, which makes them regional cuts:

| Name hash | Frame 144 reads |
| --- | --- |
| `b1ba72c3` | Sony Computer Entertainment **Europe** presents |
| `41fbd22f` | Sony Computer Entertainment **Inc.** presents |
| `3d2c85f8` | Sony Computer Entertainment **America** presents |

The three are otherwise indistinguishable: their per-frame luma deltas are
identical to two decimals, so nothing but that one line of text separates them.

**The set is region-invariant, so the disc's region does not choose the cut.**
*Wipeout Pure*'s USA disc carries these same three hashes at the same three sizes
(`b1ba72c3` 266,240, `3d2c85f8` 268,288, `41fbd22f` 264,192) in its own
`Data.wad`, at entry indices 243, 244 and 245. Two different games, one shipped
set. Whatever picks between them does so at runtime, and that mechanism has not
been read out of the binary.

**That shared-asset fact is also why these are not boot movies.** Nothing in the
three reels is Pulse: the frames are the Pure-era white background with thin cyan
wireframe, a publisher line, a developer line, and no game title at any point.
Booting into one made a launch of Wipeout Pulse look like a launch of Wipeout
Pure, which is exactly how it was reported. `Data\Movies\Intro.PMF` and
`Data\Movies\Backdrop.PMF`, by contrast, are Pulse's own and are on neither
Pure disc: `71d3c1ec` and `18c51e58` are absent from Pure's `Data.wad`.

`b1ba72c3`, the European cut, is what `--reel` defaults to, via
`oag_assets::pulse::hashes::DEVPUB_REEL_SCEE`. Confidence **75**. The disc this
project reads has a `UCUS-98712` serial in its `PARAM.SFO` and that is the *only*
American thing about it:

| Signal | Says |
| --- | --- |
| `PARAM.SFO` serial | `UCUS-98712` - USA |
| `BOOT.BIN` strings | 18 `UCES00465`, **zero** `UCUS` |
| Save-data id it writes | `UCES00465P0000` |
| ISO volume id, publisher | `SCEE`, `SCEE` |
| Directory on the disc | `PSP_GAME/USRDIR/UCES00465/`, with its own `SYSDIR/BOOT.BIN` |

The executable is the EU build. `--movie hash:3d2c85f8` and
`--movie hash:41fbd22f` select the other two.

## The front end's own images

An `Image` widget with a `src` names a `.mip` entry, and those now draw. The
textures are decoded once at load into a single RGBA sheet
([`sprite.rs`](../../crates/game/src/sprite.rs)) and drawn through the same quad
pipeline as text and solid fills, chosen per quad by a mode flag rather than by a
second pipeline - so the draw list's own back-to-front order is kept without
splitting the pass the way the movie has to.

Two things this settled that were worth finding out:

- **The images are not all in one archive.** `pulse_logo.mip` is in `FE.wad`
  *and* `Data.wad` at the same size, which makes `FE.wad` look sufficient;
  `gameshare_backdrop.mip` is in `Data.wad` only. Both are searched.
- **A sheet needs a gutter.** Stacked without one, linear filtering at an
  image's top edge samples the last row of the image above it, which drew a
  faint line across the Pulse logo from the backdrop stacked over it.

`--screen "Show Logo"` renders a named screen out of the XML without running the
sequence, which is how this is checked while the boot order is unchanged.

### Image layout

**An `Image` with no `x` is centred**, confidence **85**. `Show Logo`'s image
gives a `y` and no `x`, and its texture is 512 wide on a 480-wide screen, so
centring and left-pinning differ by 16 pixels. The texture settles it: its art is
transparent out to column 23 on the left and from column 488 on the right, so it
is authored centred inside its own 512. Centred on screen the art lands at 7..471
with even margins; pinned at `x = 0` it would run to 487 and lose its right edge.
Measured from the one image there is, not read out of the widget's layout code.

**Colour space is not a detail here.** A `.mip` palette holds sRGB bytes, so
declaring the sheet's texture `Rgba8UnormSrgb` makes sampling return linear -
right for the window, whose surface encodes on write, and wrong for the headless
capture, which targets `Rgba8Unorm` deliberately so the PNG is not double-encoded.
The sheet's format therefore follows the target's. Caught by measurement: the
logo's dominant teal is `(36, 147, 153)` in the texture and came out
`(5, 74, 81)` in a capture, which is that colour linearised exactly once. Text and
solid fills never showed it, because they write their colour straight through with
no sRGB source.

**`Viewport` is not implemented.** `Show Logo`'s `BOOT_LEGAL` text sits inside
one, and [`screen.rs`](../../crates/game/src/screen.rs) only reads a screen's
direct children, so that line is absent from the render. A `Viewport` is a
clipping rectangle; nothing about it is decoded yet.

### What the disc actually does at boot

Run under PPSSPP from a cold boot, with breakpoints on `MoviePlayer_Open`
(`0x089138bc`), the intro state's `OnEnter` (`0x088d7d80`) and the profile-movie
state (`0x088e3938`), armed from reset:

```text
     1s MoviePlayer_Open 'Data\Movies\Intro.PMF'      state='Language Selection'
    15s MoviePlayer_Open 'Data\Movies\Backdrop.PMF'   state='LogoFMV'
no further hit after 615s
```

Two things follow, and both matter more than they look:

1. **The disc's boot never enters `Intro Screen->IntroMovie1`.** Its `OnEnter`
   did not fire once in ten minutes. The order is the picker, then `LogoFMV`
   playing the long intro - which is what the XML said, and there is no dev/pub
   leg in front of it.
2. **The three 260-frame reels are never opened at boot at all.** Only `Intro.PMF`
   and `Backdrop.PMF` are. Where they *are* played is not established; they are
   on the disc, in three regional cuts, and something reaches them.

So the reel is paired with the state by its **contents matching that state's
constants**, not by having been watched playing. That pairing is strong - 260
frames, static at exactly 144 and 231, publisher card then developer card, and a
redirect literally named `DevPubRedirect` - but it is inference, and the state it
belongs to is not on this disc's boot path.

**The state named in each line is the state at the moment `MoviePlayer_Open` was
called, which is one ahead of the screen that shows the movie.** The XML settles
which is which and there is no ambiguity in it: `Language Selection` has no
`Movie` widget at all, so it cannot be playing anything; `LogoFMV`'s widget is
`src="Data\Movies\Intro"`; and the only widget naming `Data\Movies\Backdrop`
is on `FE Screen`, with `repeat="true"` and `sound="false"`, which is the looping
backdrop behind the menus. So `Intro.PMF` is armed while the picker is still up
and shown by `LogoFMV`, and `Backdrop.PMF` is armed while `LogoFMV` is still
current and shown by `FE Screen` after it. Reading the labels as "the state that
plays it" would put the silent looping backdrop on the logo screen, which is both
XML-contradicted and audibly wrong.

`LogoFMV` is therefore what this build boots into, playing `Data\Movies\Intro.PMF`
whole and with no frame holds - the widget has no frame counters. The reel leg is
kept, because it is real code with real constants, and is reached with `--reel`;
[a ground-truth test](../../crates/game/tests/boot_ground_truth.rs) still drives
its holds at 144, 231 and 260 so the path cannot rot. What is *not* claimed is
that a real PSP shows the reel at power-on. It does not.

### Why a hash and not a name

The filename is not recovered and the search for it has been run to the end:

- No `Data\Movies\` string in `BOOT.BIN` names anything but the four
  `Data\FE\Profile\*_movie.pmf` save-data icons.
- Expanding all 178 `Data.wad` XML blobs, plus every blob in `FE.wad`,
  `FEData.wad` and `BEData.wad`, finds `Movie` widgets in **one** file, with
  `src` values `Data\Movies\Intro` and `Data\Movies\Backdrop` and nothing else.
  `Intro Screen->IntroMovie1` appears in no XML at all, so that state is code-side
  and its reel is not XML-named.
- 27,328 assembled candidates (`Data\Movies\` and three sibling directories,
  crossed with logo/dev/pub/legal/region stems and three separators) hash to none
  of the three.
- A live capture under PPSSPP, breaking on `Wad_HashName` (`0x08940d0c`) and
  reading its argument, recovers real entry names - 32 of them in 25 minutes -
  but breaking on every hash call runs the emulator far below real time, and it
  reached no movie load. It would not have helped anyway: the reels are not
  opened at boot at all (above), so the name has to be caught wherever they *are*
  played, and that place is unknown.

Per the naming rules, a name nobody has evidence for does not get invented, so
the reel is addressed by the hash the WAD directory actually stores. `--movie`
takes `hash:XXXXXXXX` for exactly this.

Selecting a language fires `Launch Game` and logs it. In the original that is the
state boot goes to *instead of* the picker when a language is already saved, so
using it as the picker's exit is our shortcut, not the original's edge.

**And `Launch Game` starts a race directly.** The original has its menus in
between: the root XML's `LoadXML` list pulls in `MainMenu_Definition.xml`, and
none of it is built, so there is nothing to put there. What happens instead is
that the composition root loads the track and the ship named on the command line
and hands the window over - one window, one GPU device, the state machine's own
transition as the trigger. The front-end model does not know about it:
[`frontend.rs`](../../crates/game/src/frontend.rs) fires the transition the
original's own string literal names and stops, and `oag-game`'s main loop decides
what that means. See [`oag-game`](../tools/oag-game.md#how-the-front-end-hands-over)
for the mechanics and
[`race_ground_truth.rs`](../../crates/game/tests/race_ground_truth.rs) for the test
that boots to `Launch Game` off a real disc and flies what it hands off to.

## Finding the long intro movie

This is the `LogoFMV` reel - the 40-second one that plays *after* the picker -
not the dev/pub reel above.

Its filename is not in the executable either. `Movie_ParseAttributes`
(`0x088ba284`) builds it from the widget's `src` attribute plus `.PMF`, or
`_US.PMF` when `localised` is set, so it only exists in the front-end XML. Unlike
the dev/pub reel, this one *is* XML-driven, which is why it resolved.

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
cached**, not decoded in process. Per
[ADR-0008](adr/0008-av1-movie-cache.md), what the cache holds is **lossless AV1**,
which is decoded in process on playback.

```text
Data.wad entry -> pmf::demux -> H.264 elementary stream -> ffmpeg -> lossless AV1 -> rav1d
                  (ours)                                   (theirs)   (cached)       (ours)
```

| Step | Where | Why there |
| --- | --- | --- |
| Header parse and demux | [`oag-formats::pmf`](../../crates/formats/src/pmf.rs) | Ours to do, testable without a GPU, no dependencies |
| H.264 decode | `ffmpeg`, out of process | An in-workspace decoder is a large non-Rust dependency or a year of work, to show a logo |
| AV1 encode | `ffmpeg`'s `libaom-av1`, lossless | Once per movie. Lossless, so the picture is bit-for-bit what the H.264 decoder produced |
| Container | [`oag-formats::ivf`](../../crates/formats/src/ivf.rs) | 32-byte header, 12 bytes per frame. Cheaper to parse by hand than to pull in an MP4 demuxer |
| AV1 decode | [`oag-formats::av1`](../../crates/formats/src/av1.rs), in process | `re_rav1d` is pure Rust, so no C toolchain enters the build |
| Colour conversion | [`video.wgsl`](../../crates/game/src/video.wgsl) | Free on the GPU, and keeps the decoded frame at 1.5 bytes per pixel |

The cache lives in `data/cache/movies/`, already gitignored, keyed on the entry's
name hash and size so a different disc image cannot collide:

```text
71d3c1ec-5736448.h264                        the demuxed elementary stream
71d3c1ec-5736448-480x272-av1ll-1200.ivf      1200 frames, 33 MiB
```

`av1ll` in the name is the encoding, and it is in the **filename** so that
changing the encoder invalidates by name rather than silently reusing an older
file. That matters because `ffmpeg`'s `-lossless 1` on its own is ignored and
produces a lossy encode - see the trap in [`HANDOVER.md`](../../HANDOVER.md).

**Every frame is converted**, because `LogoFMV` plays the movie to its end and
its last frame is the Wipeout Pulse logo - a cap would stop the sequence before
the thing it exists to show. That is 33 MiB and about 80 seconds of `ffmpeg`,
once, and a line on stderr says so while it happens rather than after.
`--movie-frames <n>` caps it by hand; the `--reel` leg needs only 261, because
`IntroMovie1` stops at frame 260 whatever the reel's length.

`yuv420p` was chosen over RGBA for two reasons: it is what both decoders produce
natively, so nothing in the chain converts colour, and it is 1.5 bytes per pixel
rather than 4. Frames are decoded one at a time rather than held in memory, in
stream order; seeking backwards - which a looping movie does at every wrap -
flushes the decoder and replays from the start. The shader converts with BT.601
limited range, which is an assumption; a wrong choice shows up as washed-out
colour, not a broken picture.

**Without `ffmpeg` the game still runs.** The sequence plays out over the movie's
real duration against the black backdrop the `LogoFMV` screen puts behind the
movie anyway, and the missing tool is named on stdout. `--no-video` forces that
path, and it reads 2 KiB instead of 5 MiB because only the header is needed. In
that case a frame counter is drawn over the movie, because forty seconds of black
screen is otherwise indistinguishable from a hang; `--overlay` turns it on when
there *is* a picture, which is how the pacing above was checked frame by frame.

Audio is **not** played. ATRAC3+ needs a decoder we do not have, and the demuxer
hands the frames over intact for whenever we do.

## The Language Selection screen

Almost everything on it comes from the disc; the one thing that does not says
so below.

**The screen** is found two independent ways that agree: the element with
`type="Language Selection"`, and the element with a `DisplayLanguages` child. It
also has a `Menu name="Language" save="true"` and three `Text` widgets whose
`idstring`s are looked up in the string table.

**The `Menu`'s own layout** - `x`, `y`, `scale`, `color`, `align`, `font` - is
read the same way `Text` and `Image` widgets already are, `FEGlobals->`
indirections included: `x="FEGlobals->MenuXOffset"` (50), `y="46"` (a literal,
not an indirection), `scale="FEGlobals->MenuScale"` (1.0), `color="FEGlobals->
TextColor"` (`0xFF33A6B9`), `align="left"`. Confidence **95**: every value is
read from the disc's own file and
[asserted](../../crates/game/tests/boot_ground_truth.rs) against it directly,
not inferred.

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

- **The `Menu`'s row spacing is inferred, not measured.** `DisplayLanguages`
  populates the picker's `Menu` with one row per language, but nothing in the
  XML states a row height - only the widget's own `x`, `y`, `scale`, `color`,
  `align` and `font` are real, and every row shares them. Row spacing is taken
  from the `Menu`'s own font's documented line height (13px for `Default`,
  scaled by `MenuScale`), which is the same quantity a line of that font steps
  by anywhere else it is used, but it is an inference rather than a value read
  off the disc. Confidence **60**: the font-name-to-line-height mapping is
  real (`oag_assets::pulse::names::fonts`), but nothing here confirms a `Menu`
  list actually paces its rows by its font's line height rather than some other
  constant - no runtime capture of the picker exists to check against (see
  below).
- **Near-black text is lifted.** The picker's title is `0xFF000000`, because the
  real screen sits on the menu's lit background, which nothing draws yet. Black
  on black would look like a bug in our code rather than a missing background.
- **The selection highlight has no XML backing.** The filled bar behind the
  current row and its white text are this viewer's own affordance for showing
  what is selected, not something the disc's own `Menu` widget draws.

The font itself is no longer on this list: the `.fnt` atlas decodes for real
now (see [fnt.md](../formats/fnt.md)), so `Français` draws with its own ç and
`FE_CONFIRM_BUTTON` draws the game's own cross-button glyph. The 5x7 built-in
font survives only as a fallback for a font that fails to decode.

**This screen was very likely carried over from Wipeout Pure with only its
`FEGlobals` retuned.** Pure's own `Skin.xml` (`data/images/pure-psp-usa.chd`)
has the same `Language Selection` screen: the same widget names
(`LanguageText`, `ControlTextConfirmButton`, `ControlTextConfirm`), the same
`Menu name="Language" save="true"`, and the same literal `y="46"` on the `Menu`
widget - the one number on it that is not behind a `FEGlobals->` indirection.
Pure's `MenuXOffset` (21) and `MenuScale` (1.15) differ from Pulse's (50, 1.0),
so those get retuned per game, but the screen template - down to that one
pixel offset - reads like it was never touched.

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
3. **The reel state's redirect target is cleared once fired.** The original
   zeroes `state->devPubRedirect` after firing, so a second START does nothing.
   Without that, `IntroMovie1`'s own frame counters would fire the redirect again
   and bounce out of the picker.

Skipping on the reel leg **fires the redirect rather than stopping the player**.
The teardown is a consequence of the transition, not a peer of it, which is the
ordering the original's exit path has. `LogoFMV` has no such indirection: the
disc gives it five plain `Redirect` widgets, one per button.

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

`frontend` holding no I/O is what makes the sequencing testable: its unit tests
cover both movie legs - `LogoFMV` playing through and being skipped, and the
reel's pauses, holds and redirect clearing - as well as the picker.

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

- that the 260-frame reels are the dev/pub cards (95, from the picture) but
  **not** that anything at boot plays them: the state whose counters they fit is
  never entered at boot, so what triggers them is unestablished;
- that the three 260-frame reels are regional variants (40; it is a guess);
- that the video is BT.601 limited range (70; it looks right);
- that a two-second hold at frame 260 precedes the redirect, rather than the
  redirect firing at 260 directly. `frontend-video.md` says the finish flag is
  set and the time stamped, and that pauses release after two seconds; treating
  the flag as released by the same timer is a reading (70).

**Not established at all:** the names of eight movies, **what plays the three
dev/pub reels**, whether the picker's real layout matches ours, and anything
about audio.
