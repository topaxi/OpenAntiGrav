# Booting the front end

What `oag-game` does between being launched and showing a menu, and how much of
it is the original's behaviour rather than ours.

```sh
just play
```

That reads `data/images/pulse-psp-usa.chd`, plays `Data\Movies\Intro.PMF` - the
40-second Pulse intro, and the only movie the disc's own boot ever opens - and
lands on a Language Selection screen built from the disc's own XML. START, or
space, skips it. Picking a language goes on to `Show Logo` - the Pulse logo and
PRESS START - and START there fires `Launch Game`, which opens
[the menus](menus.md), from which START on the Race page loads
[a race](../tools/oag-game.md) into the same window.

**The language is remembered.** Once the picker has been through, the choice is
written to `settings.toml` and the next run goes straight past it: the state is
still entered and left the same frame, so the sequence below is unchanged.
`--pick-language` shows it anyway. A remembered language therefore lands on
`Show Logo`, which waits - the run stops at PRESS START rather than at the
picker, which is what the disc does too.

```sh
# No display needed.
just play --screenshot /tmp/menu.png --until "Language Selection" --hold start
just play --screenshot /tmp/logo.png --until "Show Logo" --press start,cross
just play --screenshot /tmp/launch.png --until "Launch Game" --press start,cross --ticks 60
```

## What is on screen before the intro, and why the disc has nothing there

**The window opens in a twentieth of a second and shows the loading screen
until the movies are ready.** The disc does nothing of the kind - its boot goes
straight to the reel - so this is a divergence, and it exists because the wait
is *ours* rather than the original's: the PSP decoded `Intro.PMF` from the UMD
as it played, and this build has to have every frame of it in hand first.

The numbers, measured on `pulse-psp-eu.chd` with every cache already warm and
printed by the boot itself on every run (`the boot's first half took ...`):

| | `--features native-video` (what `just play` uses on Linux) | default build |
| --- | --- | --- |
| Archives, XML, strings, font, sprites | 0.03 s | 0.03 s |
| The intro reel | 2.7 s | 0.03 s |
| The menu backdrop | 2.2 s | ~0 |
| **Total before the window could open** | **4.9 s** | **0.08 s** |

The native path decodes **every frame eagerly into memory** and hands back an
array - see [ADR-0017](adr/0017-gstreamer-native-video.md), which chose that
deliberately and costed it at "the PSP intro's 270 frames ... about 53 MiB".
That figure is the *backdrop*; the intro is 1200 frames, so the two together
are nearer 288 MiB and five seconds. The AV1 cache path (ADR-0008) decodes a
frame at a time on demand and so costs nothing here.

**A cache file that is already there beats the platform decoder, with no flag**,
so a `native-video` build lands in that second column on every run after the one
that built the cache. `open_psmf` asks `movie::cached` before it tries GStreamer;
the question costs a parse of the frame headers, and the pictures are the same
either way because the cache is lossless (ADR-0008, checked byte for byte by
`movie_ground_truth.rs`). Measured on the EU PSP disc, both reels:

| | Media phase |
| --- | --- |
| GStreamer, nothing cached | **4.80 s**, and again on every boot |
| `--prefer-av1-cache`, nothing cached | **36.08 s** once (intro 30.33, backdrop 5.74) |
| A cache file present, no flag | **0.06 s** |

**ADR-0017's default is untouched**, because it is about the case that has a
choice: with nothing cached, the platform decoder still wins. `--prefer-av1-cache`
is the opt-in past that for one run, so the cache exists and every later run
picks it up by itself; `--prefetch` does the same for every movie on the disc.
`--refresh-video` implies it - a re-conversion is what that flag asks for, and
the decoder writes nothing to re-convert.

**`--prefetch` takes it unconditionally, and has to.** The GStreamer path writes
no cache file at all, so a prefetch that took it reported every movie converted,
left the cache empty, and planned the same 22 movies again on the next run.
`prefetch::convert` therefore sets `movie::Decode::prefer_cache` rather than
leaving it to a flag: filling the cache is the worker's entire job.

Either way the load is now in two halves, which is what lets the window come
first:

- `boot::load_shell` - archives, front-end XML, language plugins, the string
  table, the font, the sprite sheet. Hundredths of a second, and enough to
  build both the loading screen and the menus. Runs before the event loop.
- `boot::load_media` - the intro, its ATRAC3+ track, the backdrop, Pure's
  second reel. Runs on a `boot::MediaWorker` thread while the window is already
  up and the wave is drawing.
- `boot::assemble` - joins the two into a `boot::Boot`. It cannot run earlier:
  `Frontend::booting` needs the reels' frame counts.

`boot::load` still does all three back to back, blocking, and that is what
`--dry-run`, every `--screenshot` leg and the ground-truth tests use: none of
them has anywhere to show a wait.

The loading screen therefore now appears on **every** windowed boot rather than
only under `--prefetch`, and **both of its waits are counted**. The media phase
reports its own progress through `boot::MediaWorker::progress`, whose
denominator is `boot::MediaPlan::loads` - two loads per named movie, its picture
and its ATRAC3+ track, plus one for the backdrop, so Pulse's plan is five. The
screen heads that `LOADING MOVIES`; `--prefetch`'s own wait, on the same screen,
is headed `CONVERTING ASSETS`. `loading::Phase` is which of the two is being
counted, and they never overlap: the conversion deliberately does not start
until the boot's own movies are done, because both write `ffmpeg` output into
`data/cache/movies` and two processes writing one file is a corrupt cache.

Only a genuinely empty count draws no bar - a title whose chain names no movie,
on a run with no `--prefetch`. Then the screen is the wave, a tip and the word
`LOADING`, with no `0 / 0` beside a full bar reading `100%`. An earlier build hid
the bar on *every* ordinary boot, because the media phase reported no counts at
all and so looked like that empty case.

### Loading is not transcoding

The three things a movie load can be doing differ by three orders of magnitude,
so the screen names which one it is - `movie::Step`, reported through
`movie::Watch` and carried on `loading::Phase::Media`:

| Step | What it is | How long |
| --- | --- | --- |
| `Cached` | Reading a cache file that already holds the frames | milliseconds |
| `Decoding` | GStreamer decoding H.264 - see [ADR-0017](adr/0017-gstreamer-native-video.md) | seconds |
| `Transcoding` | `ffmpeg` converting to lossless AV1 - see [ADR-0008](adr/0008-av1-movie-cache.md) | 55 s for `INTRO512.PSS`, measured |

A transcode heads the screen `TRANSCODING MOVIES` rather than `LOADING MOVIES`
and draws a frame counter under the entry name, read out of `ffmpeg -progress
pipe:1` about once a second. **The counter is the point**: eighty seconds of a
number that moves is a wait, and eighty seconds of a still screen is a hang.

**The bar fills continuously rather than stepping once per load.** Each load owns
one slice of the width - a fifth of it, on Pulse's five-load plan - and the
current load fills its own slice from its step (`loading::fraction`):

| Step | Share of its slice |
| --- | --- |
| nothing reported yet | none: the slice has not started |
| `Cached` | all of it - the hit *is* that load's whole work |
| `Decoding` | the first fifth, held: a decode reports no progress and can still fall back to a transcode |
| `Transcoding` | the first fifth, then the frame counter drives the remaining four |
| `Transcoding` with no stated total | the first fifth, held: no denominator, so no fraction to invent |

So the third of five loads spends its minute crossing 40% → 60% instead of
sitting at 40%. The head is a fifth because the two parts are not the same work
and the proportion is what has to be right: opening and demuxing the container is
2.6 s against 55 s of `ffmpeg` on `INTRO512.PSS`. **Monotonic by construction** -
`Cached` takes exactly the whole slice, so the bar is already where `done + 1`
puts it a moment later, and `Decoding` holds at the head `Transcoding` counts up
from. The percentage beside the bar is the same number, so the figure and the
width cannot disagree.

The sound loads hold their slices at the boundary: `at3` has its own cache and
does not report through `movie::Watch`. They are normally the fast ones, and on a
cold audio cache they are two slices where the bar stands still.
Every parse failure on that pipe is deliberately silent - a `ffmpeg` build
spelling its keys differently stops the counter and does not fail the
conversion - which is why the reporting has a ground-truth test of its own
(`a_transcode_reports_its_frames_as_it_encodes_them`, in
`crates/game/tests/ps2_source_ground_truth.rs`, against the PS2's loose `.PSS`
because a `.PMF` on a `native-video` build never transcodes at all).

```sh
# Both media states, without a display or a cold cache.
just play ps2 --screenshot /tmp/transcoding.png \
  --loading-screen 2/5 --loading-step transcoding:340/1200 --ticks 1
just play ps2 --screenshot /tmp/cached.png \
  --loading-screen 4/5 --loading-step cached --ticks 1
# The real thing, which transcodes for about a minute on a cold cache:
just play ps2 --refresh-video
```

```sh
# Both states, without a display.
just play --screenshot /tmp/loading.png --loading-screen 0/0 --ticks 1
just play --screenshot /tmp/converting.png --loading-screen 37/115 --ticks 1
```

## The sequence

| State | What happens |
| --- | --- |
| `LogoFMV` | Entered at boot. `Data\Movies\Intro.PMF` plays straight through, 1200 frames of it. START or cross skips it. |
| `LogoFMVRedirectScreen` | Only on the skip. A one-shot redirect state, entered and left the same frame; the movie *ending* does not pass through it. |
| `Language Selection` | The disc's own picker. Up and down move, cross selects. |
| `Show Logo` | The Pulse logo and PRESS START. **START, and only START.** No timeout. |
| `Launch Game` | End of the front end. The composition root opens [the menus](menus.md), which are **ours** - see that page for why they are not the disc's `MainMenu_Definition.xml`. |

Every one of those names is a **string literal from the original**, not one we
invented. `"LogoFMV"` is a screen in the disc's front-end XML, and so is its
`Movie` widget's `src`; `"Language Selection"` and `"Launch Game"` are the two
states boot chooses between depending on `0x08ab07e3`. See
[main loop](../ghidra/functions/psp-pulse-usa/main-loop.md) and
[frontend video](../ghidra/functions/psp-pulse-usa/frontend-video.md).

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

### The PS2 places its widgets in a different grid

**`Skin.xml` on the PS2 disc authors in 640x448, not the PSP's 480x272, and that
grid is shown at the PSP's own 480/272 rather than as itself. Confidence 95 on
the grid, 95 on the display aspect - see
[aspect-ratio](../ps2/aspect-ratio.md), which is where the second half was
measured and where the 4:3 this page used to claim was retired.**

Most of the PS2 layout is the PSP's scaled by the resolution ratio, and the
coordinates that do scale agree on this grid and no other - which is what makes
it an arithmetic identification rather than a guess:

| Widget | PSP | PS2 | ratio | expected |
| --- | --- | --- | --- | --- |
| `BOOT_PRESS_START` `x` | 460 | 613 | 1.3326 | 640/480 = 1.3333 |
| `BOOT_PRESS_START` `y` | 220 | 362 | 1.6455 | 448/272 = 1.6471 |
| `Show Logo` logo `y` | 72 | 119 | 1.6528 | 1.6471 |
| extreme `x` over the file | 460 | 613 | 1.3326 | 1.3333 |
| extreme `y` over the file | 252 | 415 | 1.6468 | 1.6471 |

Every one lands within a tenth of a percent, and 640x448 is PAL's own frame.
Pinned by `the_grid_ratio_is_the_one_the_scaling_coordinates_agree_on`.

**The `y=220` in that table is the USA PSP disc's, not the EU one's.** The EU PSP
release moved `BOOT_PRESS_START` down to `y=230` on its own (see below); the PS2
EU disc did not follow it, and `362/230` misses the ratio by 4% while `362/220`
hits it. So the PS2 layout was scaled from the USA-era artwork. A small thing,
but it is why an EU-to-EU comparison of that one widget looks wrong.

#### "Most", and why the word matters

**This page said "scaled by exactly the resolution ratio" until 2026-08-10, and
that was a reading the table above cannot support.** A table of extremes and
hand-picked samples can show which ratio the scaling coordinates follow; it
cannot show that every coordinate scales, because an exception in the middle is
invisible to it. Measured one coordinate at a time - USA PSP against the PS2
disc, in `crates/game/tests/frontend_grid_ground_truth.rs` - **30 of the 43
coordinates the two files share land within a pixel of the ratio and 13 do
not**:

| Group | Count | What they are |
| --- | --- | --- |
| `<Animation><Key>` `y` | 3 | A *travel*, not a position - identical on both discs |
| `Language Selection` confirm prompt | 4 | Re-placed on both axes, outward on `x` |
| `NavigationController` prompts | 4 | `x` only; their `y` scales and their internal gap scales |
| `TextInfo`, `USLegalText` `y` | 2 | 5 and 12 pixels short of the prediction |

The first group is the one with consequences beyond this page: **`x` and `y` do
not mean the same thing on every element**, so anything sweeping a layout for
out-of-screen widgets has to know which kind of coordinate it is holding.
`Arcade_HUD.xml`'s `TimeDiffIcon` is the same shape in the HUD - see
`oag_hud::inside_screen`. No mechanism is claimed for the ten hand
re-placements; a PAL title-safe inset is the obvious guess and does not fit,
since one group moves inward and the other outward.

The identical over-reading was made about `Arcade_HUD.xml` and corrected in
`3266e50`. Twice is a pattern, and the pattern is *deriving a universal from
extremes*.

**The grid and the display aspect are two numbers, and conflating them is a
silent 24% error.** 640/448 is 1.429; the frame is shown at the PSP's own
480/272 = 1.765, because the port stretched a 480x272 layout into it on each
axis separately and its pixels are that far from square. Three places need one
or the other, and on the PSP they are the same number so a wrong choice shows
nothing:

- the renderer's `screen` uniform, which maps a draw rect onto the viewport,
  wants the **grid**;
- `render::letterbox_in`, which fits that grid into a window, wants the
  **display aspect** - feeding it 640/448 leaves the front end in a column a
  fifth narrower than the window it should fill exactly;
- `frontend::pillarbox_in` wants **both**: it returns a rect in grid
  coordinates, but whether a picture is wider or narrower than the screen is a
  question about display aspects.

**That display aspect was 4:3 here until 2026-08-23**, taken from the PAL
television rather than measured, and the same sources that place the widgets
also *size* them: the PS2 draws the PSP's own textures at `640/480` across and
`448/272` down, and the executable applies those two factors itself in
`FUN_001e9370`. [aspect-ratio](../ps2/aspect-ratio.md) is the measurement, what
the disc's own 4:3/16:9 menu option really does (it moves the camera and nothing
else), and why `INTRO512.PSS` fills this frame rather than being boxed inside
it.

`frontend::Space` carries the pair, `boot::load` sets it from the archives' own
platform, and `SCREEN` stays the PSP constant it always was - our own layouts
(the loading screen, the menus), the HUD's bounds checks and `oag_race`'s
authored aspect are all written against it, and a source that authors elsewhere
opts in rather than redefining it.

**What this fixed.** Before it, every PS2 widget landed off the bottom-right of a
screen a third too small and the PS2's `Show Logo` drew nothing whatever. It now
draws the Pulse logo, `START-Taste drücken` where the XML puts it, and the
`.ipf` backdrop filling the frame.

The logo needed a second, independent thing that landed the same day: the sheet
could not decode a PS2 texture at all, so `pulse_logo.mip` was reported missing
when it was merely unreadable - `sprite.rs` now falls back to `ps2_texture::parse`
and it decodes as 512x128, 8bpp. Worth knowing which half was which, because the
older note that it "is not found in its WADs" reads like an archive problem and
is not one: `read_front_end_first` searches the companion archive *and* the bulk
one, and reports only the bulk one's complaint because it is the last tried.
`gameshare_backdrop.mip` genuinely is absent from that disc, and is on no screen
this build draws.

### Where we knowingly differ

**The original runs the picker first and `LogoFMV` second.** The front-end XML's
`Language Selection` screen has a `LanguageAutoRedirect` whose `Default goto` is
`LogoFMV`. So the disc's order is: pick a language, watch the intro, land on
"press START".

This build boots into `LogoFMV` and ends at the picker, because that was the
order asked for. The divergence is not hidden: `oag-game` prints the disc's own
redirect target at startup, and
[`Frontend::language_auto_redirect`](../../crates/ui/src/frontend.rs) exposes
it so [a test](../../crates/game/tests/boot_ground_truth.rs) asserts it is
`LogoFMV`.

**`LogoFMV`'s own exits differ too, though less than they did.** On the disc the
movie finishing fires `AutoRedirect` to `Show Logo` - the Pulse logo and PRESS
START - and any of start, circle, square, triangle or cross fires
`LogoFMVRedirectScreen`, which goes to the same place. Both hops are modelled
now, and both still land on the picker rather than on `Show Logo`, because the
picker is what this build has next; only start and cross are read, because those
are the buttons the abstract layer carries.

**`Show Logo` is not a child of `LogoFMV`.** An earlier version of this page
wrote its path as `LogoFMV->Show Logo`, and extracting `Skin.xml` and reading the
nesting settles that it is wrong: `LogoFMV` only names it as a `goto` target, and
the screen itself lives at `Top FE Screen->FE Screen->Show Logo`. Confidence
**98**, read off the file. That matters for more than pedantry, because
**`FE Screen` is what owns `Data\Movies\Backdrop`** - so on hardware the logo and
PRESS START sit on the moving menu backdrop, and the runtime capture below
showing `Backdrop.PMF` opening during boot is explained by it rather than being a
loose end. **This build draws it that way**; the `--until "Show Logo"` capture in
the commands above lands on the backdrop under the logo, not on black.

That was a documented gap for one revision and is not one now. Three things
agree: the nesting in `Skin.xml`, the boot capture opening `Backdrop.PMF` ~15 s
in while the state is still `LogoFMV` - i.e. **before** `Show Logo`, so the loop
is already running rather than starting when the screen appears - and a player
who has run the original confirming the picture. The playhead therefore runs from
the start of the sequence rather than from the moment `Show Logo` is entered.

Three implementation notes worth keeping, because all three are traps:

- **One feed, not two.** `Session` already builds the backdrop's decode thread
  when the window opens, well before the menus ask for it, so the front end
  borrows the same [`movie::Feed`](../../crates/game/src/movie.rs) the menus will
  and no second decoder is spawned. It was already outliving the menu stage on
  purpose; it now outlives the front end's own stage backwards as well.
- **The draw names its movie.** `Draw::Video` carries a `source` of `Intro` or
  `Backdrop`, because the sequence now plays two movies and reading a frame out
  of the wrong one produces *a picture* rather than an error - the same trap
  [`capture.rs`](../../crates/game/src/capture.rs) already warns about for
  `--menu-page`. Nothing infers the movie from the current state.
- **One playhead, and it is asked for by position rather than by frame.** A
  `movie::Player` reports both: `frame` wraps at the movie's length, `position`
  counts every loop. A `movie::Feed` decodes forward forever, so on the second
  time round the 270-frame backdrop it holds positions 270..274 while `frame` has
  gone back to 0 - asking it with `frame` takes nothing from that point on and
  the picture freezes. `Draw::Video` therefore carries both, and the feed only
  ever sees `position`. See [menus.md](menus.md#one-playback-not-one-per-screen)
  for the other half of this, which is that the playhead is *handed to* the menus
  rather than replaced by one of theirs.

**There is a plane-geometry guard, and both discs pass it.** The front end is
drawn by one renderer with one set of I420 planes, sized once from the intro, and
`upload_frame` slices a frame by *those* dimensions rather than by the frame's
own - so a differently-shaped picture is garbled output, not an error. `boot::load`
therefore declines to set the backdrop at all unless the two movies agree, and
says so in its report.

Whether that branch ever fires was checked rather than assumed, and **it does
not**: the PSP's `Intro.PMF` and `Backdrop.PMF` are both 480x272 (asserted on the
real disc by `the_backdrop_and_the_intro_share_one_plane_geometry`), and the PS2's
`INTRO512.PSS` and `BG512.IPF` are both 512x512. So **the PS2 gets the backdrop
under `Show Logo` too**, pillarboxed - what differs there is the *display aspect*,
not the plane size, and that is handled per-movie by the rect rather than by this
guard. An earlier draft of this page claimed the PS2 kept a black background;
running it disproved that. The guard stays as a guard - for a future source, or a
`--movie` override pointing the intro somewhere else.

**`Show Logo` sits one state later here than on the disc.** The disc runs picker,
`LogoFMV`, `Show Logo`; this build runs `LogoFMV`, picker, `Show Logo`, which is
the same swap already described above and no additional one - `Show Logo` still
comes after both of them and still immediately precedes the menus. What the disc
has in between is `RemoveMemoryStickWarning`, `NameSetup2FromBoot`,
`TagSetup2FromBoot` and `CreateFromBoot`, four screens serving Memory Stick
mechanics this build deliberately does not have. They are skipped, not
unimplemented; see `HANDOVER.md`.

**Its advance condition is read, not guessed.** `Show Logo` carries exactly two
`Redirect` widgets, one `forward="start"` and one with no `forward` attribute at
all, both `goto="RemoveMemoryStickWarning"`. There is no frame counter, no
`delay` on either redirect and no timer anywhere on the screen, so this build
waits for START and nothing else - a **narrower** set than `LogoFMV`'s five,
which is what a screen reading "Press START button" should have. Confidence
**95**, off the USA disc's `Skin.xml`, and the EU disc's copy has the same two
redirects. The unnamed redirect is the open question: no button and no name is
the shape `Language Selection` gives `LanguageAutoRedirect`, a redirect the code
fires rather than the pad, so something can advance this screen without a press
and **what that something is has not been found**. It is left unfired here rather
than modelled as a timeout, because a duration would be invented.

**`BOOT_PRESS_START` now pulses.** The widget carries `pulse="true"` and
`delay="1"`, so on hardware it fades in a second after the screen and throbs.
Both attributes are decoded and drawn: `Text::pulse`/`Text::delay`
(`crates/game/src/screen.rs`) carry the XML through, and
`frontend::draw::pulse_alpha` computes the alpha multiplier `draw_screen_at`
applies - invisible until `delay` elapses, then a sine throb between the
measured floor and the widget's own authored alpha (`0x7FFFFFFF`, at
`x="460"`, `y="220"`, `align="right"`), ramped in over its first cycle rather
than popping straight to the floor. `draw_screen`, the public entry point
`--screen` and every other test use, still freezes any pulsing widget at its
authored colour - only the live boot order (`draw_list`) animates it, so
nothing that inspects a screen's static appearance changed.

**The throb's period and depth are now measured, off a real capture.**
`pulse-psp-eu.chd` under PPSSPPSDL (Xvfb, no window manager, no compositor -
see [`ppsspp-debugger.md`](../reverse-engineering/ppsspp-debugger.md#running-without-a-real-display-xvfb-works-no-compositor-needed)),
left sitting on `Show Logo` untouched for 20 seconds, screenshotted through
`import -window root` cropped to the `PRESS START BUTTON` glyphs at roughly
11-18 Hz over two independent runs (`gpu.buffer.screenshot` still does not
work over the websocket debugger, so this is a compositor grab, not a live
memory read). The crop's mean luminance oscillates cleanly: autocorrelation on
both runs peaks at lag 1.10 s and its harmonics out to eight cycles (heights
0.93 down to 0.55 at 8.84 s in the denser run), an FFT of that run puts its
dominant frequency at 0.90 Hz - a 1.11 s period - at ten times the magnitude of
its nearest rival, and raw peak-to-peak/trough-to-trough medians land at
1.10-1.11 s in both runs. Three independent methods agree to within 0.01 s.
**Period: ≈1.10 s.**

The waveform is not a clean sine - a least-squares sine fit only reaches
R²=0.53 - because it holds rather than oscillates smoothly: mean crop
luminance sits flat at ≈55/255 for a beat, eases up, sits flat at ≈129/255 for
a beat, eases down, repeatable to within a couple of luminance units across 18
independent trough reads and a comparable number of peak reads in the second
run. **It never reads as fully faded**: the dim phase is ≈42% of the bright
phase's luminance (55/129), not zero, so the throb toggles between a dim and a
bright state rather than fading to black each cycle.

Confidence **75**: two independent 20 s captures converging on the same figure
three different ways, but this is a screen-pixel luminance proxy, not a direct
read of the widget's own alpha out of PSP memory - so the luminance ratio
measures the modulation's period and *shape* confidently, but is a lower bound
on depth rather than the exact 0-255/0.0-1.0 alpha pair an implementation
wants. The captured frames and analysis scripts are session-local, not
committed - a screen capture off the disc is game content like any other, per
[`legal.md`](../overview/legal.md).

**Turning the ratio into the implemented multiplier, and the fade-in's shape,
were judgement calls, not further measurement**, and are recorded as such
rather than presented as decoded: `frontend::draw::PULSE_FLOOR` (0.42) is the
measured luminance ratio taken directly as the alpha-fraction floor, on the
reading that the widget's own already-authored alpha (`0x7F` of `0xFF`, and
close to the capture's own ceiling luminance) already **is** the throb's
ceiling - so only one new number, not two, needed inventing. The fade-in's
own duration and shape were never captured at all (the 20 s captures started
well after boot), so it reuses the one measured timescale there is - one
throb period - as a ramp, and a plain sine stands in for the throb's own
shape, which the capture shows is not quite sinusoidal (it holds near each
extreme rather than smoothly reversing, a detail not modelled). See
`frontend::draw`'s `PULSE_PERIOD`/`PULSE_FLOOR`/`pulse_alpha` doc comments
(`crates/game/src/frontend/draw.rs`) for the exact reasoning.

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
[frontend video](../ghidra/functions/psp-pulse-usa/frontend-video.md); it is now
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
`oag_pulse::hashes::DEVPUB_REEL_SCEE`. Confidence **75**. The disc this
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
([`sprite.rs`](../../crates/hud/src/sprite.rs)) and drawn through the same quad
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
sequence, which is how this was checked while the boot order did not reach it.
`Show Logo` is now in the boot order and draws through the same code, so the two
views agree by construction; the flag stays useful for the screens the sequence
still does not enter.

### Image layout

**An `Image` with no `x` is left-pinned, not centred**, confidence **90**,
settled against a live capture rather than the texture alone. `Show Logo`'s
image gives a `y` and no `x`, and its texture is 512 wide on a 480-wide
screen, so centring and left-pinning differ by 16 pixels. The texture's own
transparency (out to column 23 on the left, from column 488 on the right)
only says the art is authored centred *inside its own 512* - it says nothing
about where that 512 lands on the 480-wide screen, which is the actual
question an untouched texture can't answer. That took a real frame: a live
`PPSSPPSDL` capture of `Show Logo` (2026-09-02, `pulse-psp-usa.chd`, driven
through the websocket debugger - see
[`ppsspp-debugger.md`](../reverse-engineering/ppsspp-debugger.md)), cropped to
the SDL window's own `960x544` geometry (a clean 2x of the PSP's native
`480x272`, so no scaling ambiguity) and read back at native resolution.
Column-by-column, the frame is exactly `0` (pure black) through screen
column 22 and the first non-zero pixel is column 23 - the same column the
texture's own transparency edge sits at, with **no** 16px shift. Centred
would have put that edge at screen column 7 instead, which the capture does
not show. The right edge is not independently useful for this: `Show Logo`'s
backdrop (`gameshare_backdrop.mip`, a separate, stacked `Image`) carries its
own diagonal gradient into that area, so pixel brightness there never drops
to a clean background baseline before the window edge either way - the left
edge is the one place the background is genuinely black on both sides of the
question, which is why it is what settles this.
`crates/game/src/frontend/draw.rs` drops the `x == 0.0` centring special case
accordingly and draws every `Image` at its authored `x` directly.

**Colour space is not a detail here.** A `.mip` palette holds sRGB bytes, so
declaring the sheet's texture `Rgba8UnormSrgb` makes sampling return linear -
right for a window surface that itself encoded on write (true when this was
measured), and wrong for the headless capture, which targets `Rgba8Unorm`
deliberately so the PNG is not double-encoded. Caught by measurement: the
logo's dominant teal is `(36, 147, 153)` in the texture and came out
`(5, 74, 81)` in a capture, which is that colour linearised exactly once. Text
and solid fills were reasoned to be unaffected either way, because they write
their colour straight through with no sRGB source, rather than sampling a
texture that could be declared either format.

**[ADR-0020](adr/0020-gamma-authoritative-colour-space.md) later removed the
window's encode entirely** - `config.format.remove_srgb_suffix()` - so the
sheet's format fork (`format.is_srgb()`) now always takes the raw side on
every path, and window and capture run the identical pipeline. That closes the
question of whether the capture paths need encode-on-write by construction,
but the text/fill reasoning above was never independently measured the way
the logo's teal was - until now. `Language Selection`'s menu
text is authored `0x33A6B9` (`(51, 166, 185)`); a headless `--screenshot` of
that screen reads the glyphs' solid-fill pixels back as `(51, 165, 184)`, a
1-level difference consistent with glyph-edge rounding, not the ~30-level
shift a single gamma encode/decode of a colour this saturated would produce
(compare the teal figure above, or ADR-0020's `73/255` plume table). Text and
fills reach the screen as authored, measured, not just reasoned.

**`Viewport` recursion and `BOOT_LEGAL` wrapping are both implemented now
(2026-08-25).** They used to be one open gap; they turned out to be two, found
in the wrong order. [`screen.rs`](../../crates/ui/src/screen.rs) already
recurses `collect_widgets` straight through a `Viewport` rather than stopping
at it (`a_text_inside_a_viewport_is_collected_alongside_its_siblings`), so
`BOOT_LEGAL` was never actually missing from the parsed `Screen` - checked
directly against `pulse-psp-usa.chd` before touching anything, since the
premise here had gone stale once already. What *was* still true: `draw_screen`
pushed the collected `Draw::Text` unwrapped, at `font="small" scale="0.7"`,
118 UTF-8 bytes / **114 characters** (`©` is one codepoint, two bytes) wide -
comfortably over both the 400-pixel `Viewport` and the 480-pixel screen. So the
line drew, just wrong: one row running off both edges, which is worse than an
absence and harder to notice as a bug.

```xml
<Viewport><Values x="40" y="0" height="480" width="400"></Values>
  <Text name="USLegalText" delay="0" transition="0">
    <Values align="left" idstring="BOOT_LEGAL" font="small" scale="0.7"
            x="40" y="250" widthlimited="true" color="0x7FFFFFFF"></Values>
</Viewport>
```

- **The clip itself does nothing here.** The rect is 400 wide and *480* tall on
  a screen that is 480x272, so vertically it does not clip at all, and the one
  widget inside it starts at the rect's own left edge. A scissor rect for this
  screen would be dead code.
- **`widthlimited="true"` means wrap, and the width is the enclosing
  `Viewport`'s own `width`.** `Text::wrap_width` carries it down through
  `collect_widgets`, `Draw::Text` gained the same field, and `crate::render`'s
  `push_wrapped_text` breaks the string at word boundaries with
  [`wrap`](../../crates/game/src/loading.rs) - the same function `loading.rs`
  already used for a tip of the day, reused rather than reimplemented - and
  stacks the lines downward by the atlas's own line height at `scale`.
- **Measured against the real `Small` face** (`Data\FE\Fonts\Pulse_14.fnt`,
  line height 17): the string wraps to exactly two lines, both under 400px, the
  second starting at `y=261.9`. By the nominal line-height box that is 1.8px
  past the 272-tall screen; by the string's own tallest glyph ink (14px cell,
  9.8 scaled) it clears the bottom edge by 0.3px. So top-anchored, growing
  downward from the widget's own `y`, is right for this string on this screen
  with no bottom-anchor question to resolve - asserted directly against the
  disc in `crates/game/tests/boot_legal_wrap_ground_truth.rs`.

**This affects the USA disc only**: the EU disc drops the `Viewport` and the
line with it, and moves `BOOT_PRESS_START` from `y="220"` to `y="230"` to fill
the gap. The other two `Viewport`s in the file hold `Animation` and `TextInfo`
widgets this build does not model at all, so nothing else was waiting on this.

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

**`Backdrop.PMF` is now drawn**, behind [our own menus](menus.md#the-background-the-menus-sit-on)
rather than behind a reconstruction of `FE Screen`. `repeat="true"` on that
widget is what the player here is built with, and `sound="false"` is why no audio
path was needed for it. It is loaded during boot alongside the intro, because the
menus can be opened by a keypress out of a race and a first-run transcode is
about thirteen seconds.

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
[`frontend.rs`](../../crates/ui/src/frontend.rs) fires the transition the
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
| Header parse and demux | [`oag-video::pmf`](../../crates/video/src/pmf.rs) | Ours to do, testable without a GPU, no dependencies |
| H.264 decode | `ffmpeg`, out of process | An in-workspace decoder is a large non-Rust dependency or a year of work, to show a logo |
| AV1 encode | `ffmpeg`'s `libaom-av1`, lossless | Once per movie. Lossless, so the picture is bit-for-bit what the H.264 decoder produced |
| Container | [`oag-video::ivf`](../../crates/video/src/ivf.rs) | 32-byte header, 12 bytes per frame. Cheaper to parse by hand than to pull in an MP4 demuxer |
| AV1 decode | [`oag-video::av1`](../../crates/video/src/av1.rs), in process | `re_rav1d` is pure Rust, so no C toolchain enters the build |
| Colour conversion | [`video.wgsl`](../../crates/game/shaders/video.wesl) | Free on the GPU, and keeps the decoded frame at 1.5 bytes per pixel |

The cache lives in `data/cache/movies/`, already gitignored, keyed on the entry's
name hash and size so a different disc image cannot collide:

```text
71d3c1ec-5736448.h264                        the demuxed elementary stream
71d3c1ec-5736448-480x272-av1ll-1200.ivf      1200 frames, 33 MiB
```

`av1ll` in the name is the encoding, and it is in the **filename** so that
changing the encoder invalidates by name rather than silently reusing an older
file. That matters because of the encoder traps below.

**Three tool traps in this chain, each of which fails silently.** Moved here
from `HANDOVER.md` on 2026-08-09.

- **`ffmpeg -lossless 1` on its own is silently a lossy encode.** The quantiser
  must be pinned and rate control disabled too: `-b:v 0 -crf 0 -qmin 0 -qmax 0
  -aom-params lossless=1`. Even then `-cpu-used` below 6 is reproducibly not
  bit-exact; 6 and 8 are exact, and faster.
- **`ffprobe -show_entries` does not print fields in the order requested** - it
  uses its own. Parse `key=value` pairs by name (`-of
  default=noprint_wrappers=1`, keys kept); positional parsing reads the wrong
  field and says nothing.
- **A GStreamer `appsrc` set to `AppStreamType::Seekable` performs an initial
  seek on start, and that seek can fail with no exception at the call site that
  set the property.** `pipeline.set_state(Playing)` just returns `Err` with an
  empty bus; `GST_DEBUG=3` is what shows `gst_app_src_do_seek: seek failed`
  underneath it. `crates/game/src/movie/gst.rs`
  ([ADR-0017](adr/0017-gstreamer-native-video.md)) pushes one whole buffer and
  calls `end_of_stream()`, which is exactly what `AppStreamType::Stream`
  describes; `Seekable` is for sources that support real seeking.

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
[`oag-pulse`](../../crates/pulse/src/lib.rs) are the candidates to
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
  real (`oag_pulse::names::fonts`), but nothing here confirms a `Menu`
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

## What the original's handoff looks like, and what ours still does not

A user report that both handoffs - the intro into `Show Logo`, and START into
what follows - are "not as smooth as the original" turned out to be three
separate black-frame mechanisms in our code plus one unimplemented widget
attribute. The three are fixed and written up in
[menus.md](menus.md#a-continuous-playhead-is-not-a-continuous-picture); this is
what the disc itself says about the transition, and what we still do not
reproduce.

**The picture is one uninterrupted playback, and every screen in the sequence
sits on it. Confidence 85.** `Show Logo` is a *child* of the `FE Screen` that
owns the `Movie` widget naming `Data\Movies\Backdrop`, and that widget is
`autostart` and `repeat="true"`. So on hardware there is no handoff to smooth:
pressing START changes which widgets are drawn over a loop nobody stopped. The
evidence is the XML's own nesting and widget attributes, plus a player who has
run the original confirming the picture does not jump; that is a strong reading
of a structural fact rather than a measurement of the running game, which is
why it is 85 and not higher. The consequence for us is that **any screen between
the intro and the menus that draws no backdrop is wrong**, which is what
`Launch Game` was.

**`PRESS START` fades in a second late and throbs, and now ours does too.**
`BOOT_PRESS_START` carries `pulse="true"` and `delay="1"`, confidence 90 that
this is what those attributes mean (reading `Movie_ParseAttributes`'s own
boolean convention against how every other authored fade/redirect attribute
here has been read); this build's `frontend::draw::pulse_alpha` reads both and
animates the widget's alpha accordingly - see the `BOOT_PRESS_START now
pulses` section above for the period, floor and ramp it uses, each at
confidence 75. This closes **the named residual on the intro-to-`Show Logo`
transition**: closing the flash no longer leaves a static text pop as a
second, smaller discontinuity where the disc has a fade.

**What is deliberately not claimed.** Nobody has captured the original's boot
frame by frame, so there is no evidence here about whether the intro's last frame
holds, crossfades, or cuts to `Show Logo`; the "no jump" claim above is about the
*backdrop's phase*, not about a transition effect. A capture was attempted and
abandoned rather than faked, and the reason is worth recording because it will
bite the next attempt too: **on this machine's DRI-less `Xvfb` an accelerated
window's contents never reach the root framebuffer**, so `ffmpeg -f x11grab` of
our own window returned 2,400 uniformly black frames while the game was
demonstrably drawing - which is what forced the in-process framebuffer readback
that produced our own evidence. The PPSSPP leg is weaker and is *not* offered as
proof of the same: `import -window root` also came back black there, and
`[Recording] DumpFrames` produced no file in one attempt, but that run shared a
display with another session's emulator and so is confounded. Treat the emulator
capture as **untried**, not as failed. A maintainer with a real display can
settle it directly:

```sh
just launch-pulse-psp data/images/pulse-psp-eu.chd
# then capture the boot with any screen recorder and step the frames around
# the intro's end and the START press.
```

Until that is run, the three claims above rest on the front-end XML, which is the
disc's own authored data and is the stronger source for *structure*; only the
question of a transition *effect* needs pixels.

### The EU disc (`pulse-psp-eu.chd`)

Compared byte-for-byte against `pulse-psp-usa.chd`'s `Data.wad`, per
[source-images.md](../reverse-engineering/source-images.md):

- **`Skin.xml` (hash `1f38aacf`) is identical for the Language Selection
  screen.** The only diff anywhere in the file is on `Show Logo`: the EU disc
  drops the US-only `BOOT_LEGAL` legal-text `Viewport` and shifts
  `BOOT_PRESS_START`'s `y` from 220 to 230 to fill the gap. Every `Menu`,
  `Text` and `DisplayLanguages` element on `Language Selection` itself -
  position, scale, colour, font - is byte-identical. Confidence **97**: a
  direct diff of the extracted file, not inferred.
- **The EU disc has the same five languages, differently numbered.** French
  (`PI008`), German (`PI009`), Spanish (`PI010`) and Italian (`PI011`) sit at
  the same plugin ids as the USA disc and their `entries.xml` string tables
  are the same translations (a sorted diff of French's 1,669-vs-1,671-entry
  table shows only the same `BOOT_LEGAL` drop and two minor wording tweaks).
  **English moves from `PI012` (USA) to `PI000` (EU)** - confirmed by hash
  (`Data\Plugins\PI000\Definition.xml` = `e21d955a`,
  `Data\Plugins\PI000\entries.xml` = `4438799d`, both present only on the EU
  disc at exactly the sizes the two USA-absent EU-only WAD hashes turned out
  to be) and by content (`Font Language="English"` inside). **The EU disc
  drops two USA-only plugins**: `PI003` (a button-glyph substitution table,
  still `Language="English"` internally) and `PI005` (a hidden Japanese
  language plugin, present on the USA disc but never offered as a picker
  choice there either). Confidence **90**.
- **The boot-time plugin manifest confirms the same set from the executable
  side.** `Plugin_LoadManifest` (USA `0x0888b980`) walks an 11-pointer table at
  `0x08ab110c` loading `PI012, PI010, PI008, PI009, PI011, PI001, PI004, grids,
  music, loading, news` in that order (first entry = English, loaded first).
  Its EU counterpart - found by positional correspondence through the
  already-matched `Game_MainLoop`/`FUN_0888adcc`↔`FUN_0888ac28` chain
  (`diff_functions`: 258/277 and 65/70 instructions equal, every difference a
  relocated immediate) - is the same `Plugin_LoadManifest` at EU `0x0888b7dc`,
  whose own base-pointer symbol and `strings_only_in_a/b` diff read
  `Data\Plugins\PI012` (USA) vs `Data\Plugins\PI000` (EU): the executable's own
  manifest, not just the disc's file layout, confirms English is `PI000` on EU
  and loads first. Reading EU's manifest table directly (`08ab088c`, 11
  pointers, `strings` search) gives `PI000, PI008, PI009, PI010, PI011, PI001,
  PI004, grids, music, loading, news` - same 11 entries, same first slot,
  French/German/Spanish/Italian in a different sub-order than USA's.
  Confidence **80** on both addresses (`_q`-worthy evidence tier per the
  deep-sweep formula, applied without the suffix since 80 clears the
  threshold). Renamed 2026-09-02 and recorded in `psp-pulse-usa/names.tsv` and
  `psp-pulse-eu/names.tsv` against this page. **A second, independent
  confirmation of the walked table's address**: the function's own loop
  starts at `piVar5 = &DAT_002ad10c` and walks it as a null-terminated pointer
  list (`piVar5 = piVar5 + 1` each iteration until `*piVar5 == 0`) - resolving
  the pseudo-address the same way this project's own `psp-pulse-usa` warts do
  (`real = pseudo + 0x08804000`, [`workflow.md`](../ghidra/workflow.md#reading-an-unrelocated-database-until-it-is-reimported))
  gives `0x002ad10c + 0x08804000 = 0x08ab110c` - the exact 11-pointer table
  address this bullet already names from the string-search side, now also
  read off the function's own instructions. No direct caller was found for
  either renamed address (`get_function_callers`/`get_xrefs_to` both report
  none on the USA side); the likely cause is the documented `jal`-target wart
  on this binary (`workflow.md`'s "a narrower, related wart survives on `jal`
  call targets" - `R_MIPS_26` relocations left unapplied), which drops the
  call edge from Ghidra's own xref database entirely rather than implying an
  indirect or table-driven call. `FUN_0888adcc`/`FUN_0888ac28`, the landmark
  chain used to find the EU address, remain unrenamed - their own role beyond
  "matched instruction-for-instruction" is unread.
- **Live capture (PPSSPP, headless, cold boot, no input) shows the EU disc's
  `Language Selection` state is entered and then unconditionally exits to
  `LogoFMV` in under one frame** (516,292 of 222,000,000 PSP cycles/sec, ≈0.14
  frames, measured via `cpu.status.ticks` at the two consecutive
  `StateMachine_TransitionTo` hits) - and **holding `down` through the
  transition did not change this**. This rules out "player input during the
  capture window happened to redirect it" as an explanation. It does **not**
  by itself explain why real EU hardware is documented (external knowledge,
  not this project's own capture) to show a picker on first boot: the
  likeliest reconciliation, not yet tested here, is that the redirect is
  gated on PPSSPP's own configured PSP system language (`en_US` in this
  environment's `ppsspp.ini`) matching an available plugin, auto-selecting
  and skipping the picker the same way a real console with its system
  language already set would - rather than on the number of on-disc
  languages, which is 5 for both regions, not 1. **The once-repeated claim
  that USA "never shows the picker... because the build is English-only" is
  not supported by this session's static findings** (the USA manifest also
  lists 5 languages) and is corrected below. No screenshot
  of a rendered picker was captured this session - the state never stayed up
  long enough to reach one.

### The offered languages are each executable's plugin manifest

A disc carries a superset of what its release offers, so the language list is
not "whichever plugins resolve" and not one constant per title. Each PSP/PS2
executable holds a null-terminated pointer table to `Data\Plugins\PI0NN`
strings that the boot walks in order; the language entries of it are the list.
Read 2026-10-06 from the ELF program headers (table address is the pseudo-address
used elsewhere on this page; the strings sit 20 bytes apart):

| Release (serial) | Table | Languages, in manifest order | Not offered although on the disc |
| --- | --- | --- | --- |
| Pulse PSP EU (`UCES-00465`) | `0x2ac88c` | `PI000` English, `PI008` French, `PI009` German, `PI010` Spanish, `PI011` Italian | none |
| Pulse PSP USA (`UCUS-98712`) | `0x2ad10c` | `PI012` English, `PI010`, `PI008`, `PI009`, `PI011` | `PI003`, `PI005` |
| Pulse PS2 EU (`SCES-54748`) | string run at file offset `0x1ab4b0` | as USA | not probed |
| Pure PSP USA (`UCUS-98612`) | `0x2abd18` | `PI000` English, `PI010` Spanish, `PI008` French | `PI009`, `PI011` (German, Italian), `PI003`, `PI005` |
| Pure PSP EU (`UCES-00001`) | `0x2a54d8` | `PI000`, `PI010`, `PI008`, `PI009`, `PI011` | `PI003`, `PI005`, `PI012` |

Pure USA's manifest also loads `PI012`, a US-spelling overlay on `PI000` with no
`<Font>`: not a language, so it is not in the offered list (applying its
spellings is open fidelity work). The three offered languages and their order
match the USA picker observed in `pure-boot.md`, and the EU five match too, which
is what makes manifest order the picker order on Pure (confidence 85). Pulse's
picker exits in under a frame and was never captured, so its order is Pure's rule
carried over (confidence 70); Pulse's old constant (French first, English last)
had no source. Neither Pure manifest names `PI003` or `PI005`: the Japanese
plugin is never loaded, which closes the question `pure-status.md` left open.

**Why it mattered:** the Pulse PSP EU disc has no `PI012`, so a list ending in
`PI012` loaded no English on this project's default image. Now
`oag_title::FrontEnd::offered_languages(serial)` returns the release's own list
(`oag_pulse::LANGUAGE_MANIFESTS`, `oag_pure::LANGUAGE_MANIFESTS`), and the title's
`language_plugins` stays the superset default for a caller with no serial.
Pinned per image by `language_offered_ground_truth`.

HD, 2048 and Omega: every plugin on the disc still loads (16, 17 and 23). Whether
their executables narrow it the way Pure's does is **not read**; Omega's store
listing offers 12 screen languages of its 23 plugins, which suggests it does.
Open, named in the language thread.

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

[`state_machine.rs`](../../crates/ui/src/state_machine.rs) is string-keyed and
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

Input is the [abstract button layer](../ghidra/functions/psp-pulse-usa/input.md)
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

The headless capture uses the same fixed step, and is reproducible **so long
as the movie itself is tick-clocked rather than audio-clocked.** Per [ADR-0019](adr/0019-atrac3plus-out-of-process.md), a machine with a real
audio device paces a playing movie against real time
instead, and a headless run finishes in far less real time than a movie takes
to play - so an `--until` capture that has to wait one out, with nothing
holding or pressing a skip button, may never reach its target inside
`MAX_TICKS` (`crates/game/src/capture.rs`). `--no-audio` (or no device at
all, which is what CI has) forces the tick-clocked path and makes `--until`
and `--ticks` reproducible again.

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

### Wipeout 2048: a chain driven off its own redirects, then a touch front end

2048's five boot screens (`frontend/wipeout2048.rs`) are not each a handler
of their own the way every screen above is. Each is driven off its **own**
`<Redirect>` widgets, which the reader keeps whole for the first time:
`Redirect::delay` (the Studio Liverpool card's authored `4.0`), and the
button on each redirect matched against the pad. A target that is the next
chain step goes through `advance`, so the movie step still gets its player;
`TitleScreen`'s `GameModeChoice` is fired by name, and that is where the
chain hands over - not to `Launch Game` and this build's menus, but to the
disc's own touch grids (`frontend/touch.rs`) and campaign map
(`frontend/campaign_map.rs`), and finally to the disc's own `Launch 2048`,
which carries an `oag_ui::frontend::Launch` out for the composition root:
a campaign event for `race::load_event`, or one of this build's two own
pages. Two screens leave the tick they are entered - the network check
and the save check, both conditions this build has nothing to check. Read
the whole account, with what is authored, measured and chosen on each
screen, in [2048-frontend.md](../formats/2048-frontend.md#wired-the-boot-walks-and-the-grids-draw-2026-09-21).

The other three titles' screens are unchanged by this; what they gained is
mechanism they do not use yet: `Image::centred`, `Text::middle`, a font
role scaled by its own `.fnt` line height (`Frontend::set_face_scales`,
set only on a title with a touch front end), and `boot::includes` following
a root's `<LoadXML>` list, which only a title whose root declares no screen
needs.

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
