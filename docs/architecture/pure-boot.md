# Wipeout Pure's boot sequence

What runs between power-on and `Title Screen` on Pure's PSP UMD, and where this
build departs from it. Pulse's own sequence is
[`frontend-boot.md`](frontend-boot.md); the two titles differ enough that they
get separate pages, and the differences are the reason
[ADR-0023](adr/0023-boot-sequence-as-title-data.md) exists.

Every row below was observed under PPSSPP from a **true cold boot** on
2026-08-10, and the method matters more than usual here - see
[The stale-savedata trap](#the-stale-savedata-trap), which is why an earlier
reading of this same sequence was wrong.

## The sequence

| # | State | What it does | Leaves on | Confidence |
| --- | --- | --- | --- | --- |
| 1 | `Language Selection` | The picker, on `Intro Screen`'s white. **No movie plays before it.** | cross confirms | 90 |
| 2 | `Developer Publisher Screen` | Plays the dev/pub reel off the parent's `IntroMovie1` widget, holding at frames 144 and 231 | the reel running out, ~12.7 s | 95 |
| 3 | `MemoryStickWarning` | Storage-access disclaimer, six text widgets from the XML | cross ("PRESS X TO CONTINUE") | 90 |
| 4 | `FMV Intro` | Plays `Data\Movies\WoFMVNew_US.PMF`, 2847 frames, ~95 s | plays out, or a skip | 90 |
| 5 | `Title Screen` | "wipEout pure" and PRESS START BUTTON | **not established** | 90 |

Observed on **both** pressings, `pure-psp-usa.chd` and `pure-psp-eu.chd`, with
the same shape and the same timings. The only difference found between them is
the picker's contents: USA offers three languages (English, Español, Français),
EU five (those plus Deutsch, Italiano). Screenshots for each step are under
`data/shots/pure-cold-boot-2026-08-10/`.

The chain is also exactly what Pure's own `Skin.xml` declares, redirect for
redirect: `Language Selection`'s `LanguageAutoRedirect` goes to
`Developer Publisher Screen`, whose `DevPubRedirect` goes to
`MemoryStickWarning`, whose `MemoryStickRedirect` (`StartEnabled="true"`) goes
to `FMV Intro`, whose `FMVRedirect` goes to `Title Screen`. Runtime and data
agree, which is what puts these rows at 90 rather than lower.

**What advances past `Title Screen` is not established.** Its `TitleRedirect`
carries no `forward` attribute and its `Default goto` names
`Profile Manager`, a screen this build does not have. Do not guess; see
`oag_pure::frontend::states::TITLE_SCREEN`'s own doc comment.

## No movie before the picker - and two after it

Nothing plays before `Language Selection`; the first frame after power-on is the
picker itself. Confidence **90**, cold-boot observed on both pressings. That is
the whole of the narrow claim, and it is the only part of the original reading
that survived.

**`Data\Movies\IntroMovieP1_US.PMF` is very much on the boot path**: it is the
dev/pub reel, and it plays on `Developer Publisher Screen`, the step straight
after the picker. So Pure's boot is two movies, not none - the reel, then
`WoFMVNew` on `FMV Intro`.

### How this was got wrong, twice

Worth recording, because both errors have the same shape.

First, a stale PPSSPP save profile made the boot look like it opened on a movie,
and this build played `IntroMovieP1` *before* the picker on that basis.

Then, correcting that, this page asserted the opposite over-strongly: that the
reel was "not on the boot path" at all and that its widget belonged to a screen
state the runtime never enters. The reasoning was that
`Developer Publisher Screen` declares no widgets, so its two cards had to be
drawn by engine code - a **false dichotomy**, because a Pure child screen
inherits its parent's widgets. That is exactly how the picker gets its white
background, three sections down this same page. `FMV Intro` declares no widgets
either, and this build already had it playing the parent's movie.

The decisive check was one nobody had run: decode the reel and compare it with
the captures. Frame 144 is the "SONY COMPUTER ENTERTAINMENT ... PRESENTS" card
and frame 231 is "A STUDIO LIVERPOOL GAME", each matching the captured screen to
within resampling noise. Everything read as engine-drawn - the Studio Liverpool
logo, the frame graphics, the barcode, the small arrow - is video.

**The lesson is cheap to state and was expensive to learn: a screenshot shows
what a screen looks like, never what drew it.** Both errors came from reasoning
about appearance instead of checking the asset.

### The stale-savedata trap

An earlier pass concluded Pure booted movie-then-picker, mirroring Pulse, and
that conclusion was implemented before it was checked. It was wrong, and the
cause was a PPSSPP save profile predating this project
(`~/.config/ppsspp/PSP/SAVEDATA/UCUS98612P0000`, dated 2024-09-19) that was
silently skipping an unknown amount of the real boot. Moved aside, the very
first frame is the picker.

**A "cold boot" is not cold until the savedata directory is checked.** This is
recorded permanently in
[`ppsspp-debugger.md`](../reverse-engineering/ppsspp-debugger.md) because it cost
this project a wrong implementation, not merely a wrong note.

## `Developer Publisher Screen`: the reel, with frame holds

Pure's `Skin.xml` gives this screen no content of its own - a single
`<Item></Item>` (commented "Requires a default item to keep screen alive") and
`DevPubRedirect`. It plays the `IntroMovie1` widget declared on its **parent**,
`Intro Screen`, which names `Data\Movies\IntroMovieP1` - the dev/pub reel.
Confidence **95**: captured frames match the decoded video to within resampling
noise.

Measured at 0.5 s granularity from the language confirm, `pure-psp-usa`:

| t | What is on screen |
| --- | --- |
| 0.0-1.5 s | the picker clears |
| 2.0-4.0 s | the reel runs up to its first card, "SONY COMPUTER ENTERTAINMENT AMERICA PRESENTS" |
| 4.5-5.0 s | held - frame 144 |
| 5.5-6.0 s | running again |
| 6.5-9.0 s | up to the second card, "A STUDIO LIVERPOOL GAME", with the Studio Liverpool logo, frame graphics, "A-G RACING", "//2197", a barcode and an SCE address line |
| 9.5-10.0 s | held - frame 231 |
| 10.5-12.5 s | running out |
| 13.0 s | `MemoryStickWarning` |

**The frame holds are Pure's own, not borrowed from Pulse.** `144`, `231` and
`260` appear as three `li` immediates in Pure's `BOOT.BIN` - the only such site
in 3.6 MB - each beginning an identical pause-and-reload block, the last also
setting a done flag. So this title implements the pause logic itself; the
identical constants in Pulse's executable describe the same reel.

The arithmetic agrees: 260 frames at 29.97 fps is 8.68 s, plus two 2-second
holds is **12.68 s**, against a measured 11-12.5 s. Plain playback without holds
would be 8.68 s, which the measurement rules out.

What is **not** established: the hold *duration*. Pure loads it from a global
rather than an immediate, so `HOLD_SECONDS = 2.0` remains Pulse's measurement,
imported. A 0.1 s re-film of the first plateau would settle it - about 22
consecutive identical samples for a 2 s hold, at most 2 for none.

The four regional cuts are named in `oag_pure::names::INTRO_MOVIE_CUTS`; see
[the localisation note](#the-regional-cuts-and-a-real-bug) below.

## Where this build knowingly differs

- **`MemoryStickWarning`'s text is deliberately modernised.** The disc's six
  `MSInfo`/`MSWarning` strings are about a Memory Stick Duo being physically
  removed. This is a reimplementation on hardware where storage is assumed
  present, so the screen keeps its structure, colours, rules and cross gate but
  takes its own wording, phrased around autosave rather than a removable card. A
  deliberate product decision, not a faithfulness gap - recorded here so it is
  not "fixed" later by someone reading the disc's strings. Everything geometric
  *is* the disc's: rules at y=10 and y=240, body at x=15 from y=30, prompt at
  y=245, and the screen's own `MSWarningColour1`/`MSWarningColour2`/
  `MSWarningScale` globals.
- **`Movie::entry_name` picks `_US` for every source.** See below; it is the one
  known-wrong thing left on this path.

## The regional cuts, and a real bug

`IntroMovieP1` and `WoFMVNew` are both `localised="true"`, and each ships **four**
cuts. All four names were recovered by hashing the `_<REGION>` suffix pattern and
confirming the entry resolves:

| Region | Reel | Size | FMV |
| --- | --- | --- | --- |
| EU | `IntroMovieP1_EU.PMF` | 266,240 | `WoFMVNew_EU.PMF` |
| US | `IntroMovieP1_US.PMF` | 268,288 | `WoFMVNew_US.PMF` |
| JAP | `IntroMovieP1_JAP.PMF` | 264,192 | `WoFMVNew_JAP.PMF` |
| KO | `IntroMovieP1_KO.PMF` | 266,240 | `WoFMVNew_KO.PMF` |

Both Pure pressings carry the first three; the Korean cut is on the EU disc only.
Frame 144 of the EU cut reads "EUROPE" where the US cut reads "AMERICA", so the
earlier "the EU wording is not established" note is resolved - and the three cuts
Pulse ships are these same files, which is why
`oag_pulse::names::DEVPUB_REEL`'s bare `hash:b1ba72c3` is simply
`IntroMovieP1_EU.PMF` under another name.

**The bug**: `oag_game::screen::Movie::entry_name` appends `_US.PMF` to every
localised widget on every source, so a European pressing is currently shown the
American card. That hardcode was defensible while `_US` was the only known name;
with all four recovered it wants to resolve against the chosen region. What the
original selects on has **not** been read out of any binary - that a pressing
carries a cut for its own region is the evidence that it selects at all.

### The order was wrong once, for a structural reason worth remembering

Between first implementing Pure's chain and this page's current form, the build
walked `Language Selection` straight to `FMV Intro`, skipping both screens above.
Two causes, and the second is the instructive one:

1. Neither screen had any behaviour, so both were stepped over.
2. **The mechanism could only resolve one step after the picker.** It carried a
   `start` and an `after_language`, which is enough for Pulse's three screens and
   cannot express five however much is implemented - so the order would have
   stayed wrong even after both screens were drawn.

The fix was to make the resolved sequence a *chain* the front end walks, with one
`advance()` that every screen leaves through, rather than a `fire(...)` spelled
out per screen. Each of those spellings was individually defensible and together
they described Pulse's sequence on Pure's disc.

### The order was wrong once, for a structural reason worth remembering

Between first implementing Pure's chain and this page's current form, the build
walked `Language Selection` straight to `FMV Intro`, skipping both screens above.
Two causes, and the second is the instructive one:

1. Neither screen had any behaviour, so both were stepped over.
2. **The mechanism could only resolve one step after the picker.** It carried a
   `start` and an `after_language`, which is enough for Pulse's three screens and
   cannot express five however much is implemented - so the order would have
   stayed wrong even after both screens were drawn.

The fix was to make the resolved sequence a *chain* the front end walks, with one
`advance()` that every screen leaves through, rather than a `fire(...)` spelled
out per screen. Each of those spellings was individually defensible and together
they described Pulse's sequence on Pure's disc.

## Pulse boots differently, and that is measured too

Worth stating on this page because the obvious inference from Pure is wrong.
Pure's `Skin.xml` runs the picker first and Pure's runtime agrees. **Pulse's
`Skin.xml` also declares the picker first** (`Language Selection`'s
`LanguageAutoRedirect` goes to `LogoFMV`), but **Pulse's runtime does not**: a
cold boot of `pulse-psp-eu.chd` opens straight into `LogoFMV` playing
`Data\Movies\Intro.PMF`, whose own first frames are an SCEE presents card, and
runs on to `Show Logo` with no picker in between. Confidence **90**,
cold-boot observed; frames under `data/shots/pulse-cold-boot-2026-08-10/`.

Two consequences:

1. **A front-end XML's declared entry point is not the runtime's.** It holds on
   Pure and fails on Pulse, so neither title's order can be taken from its XML
   alone - which is the strongest argument for the boot sequence being a
   measured table per title rather than anything derived.
2. This build's Pulse order - `LogoFMV`, then the picker, then `Show Logo` - is
   **correct about the movie coming first** and was previously recorded as
   merely "the order asked for". Where the picker belongs on Pulse is a separate
   open question: the disc did not show one between `LogoFMV` and `Show Logo` on
   a cold boot. See [`frontend-boot.md`](frontend-boot.md).

## Verified, inferred, not established

**Verified** (cold-boot observed on both pressings, reproduced, and cross-checked
against the assets): the five-state chain and its order; no movie before the
picker; `Developer Publisher Screen` playing the dev/pub reel, its two cards
matched frame-for-frame against the decoded video; `MemoryStickWarning` gating on
cross; `FMV Intro` playing `WoFMVNew` and reaching `Title Screen` by itself; both
pressings carrying every regional cut of both movies at the sizes tabled above;
both movies being 480x272 with identical PSMF stream descriptors; the frame-hold
constants 144/231/260 existing in Pure's own `BOOT.BIN`.

**Inferred**: that the hold *duration* is Pulse's measured 2.0 s (Pure loads it
from a global, so the value has not been read out of this title); that the
pause site found in Pure's binary is the one this screen reaches, it being the
only such site rather than a traced call path.

**Not established**: what advances past `Title Screen`; what the original selects
a regional cut on; where Pulse's language picker belongs; why Pulse carries the
reel and its pause code but never enters that state at boot.

**Falsified, and left here on purpose**: that the cards were engine-drawn
teletyped text; that the reel was off Pure's boot path; that the pink arrow was
`Intro Screen`'s `ArrowSelect`; that the EU wording was unknowable. Each of those
was recorded on this page with a confidence score, and each came from reading a
screenshot rather than the asset behind it.
