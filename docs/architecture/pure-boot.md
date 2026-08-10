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
| 2 | `Developer Publisher Screen` | Two teletyped phases; see below. Declares no widgets of its own. | a timer, ~11 s | 85 chain / 60 duration |
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

## No movie before the picker

`Data\Movies\IntroMovieP1_US.PMF` is real, present on both pressings, and
decodes - and it is **not on the boot path**. Nothing plays before
`Language Selection`; the first frame after power-on is the picker itself.
Confidence **90**, cold-boot observed on both pressings.

That file is byte-for-byte Pulse's American dev/pub reel cut, and Pure's own
`Intro Screen` declares it as an `IntroMovie1` widget - a widget whose screen
state the runtime never enters, which is precisely the shape Pulse's own
`Intro Screen->IntroMovie1` reel has on Pulse. **Both titles ship an off-path
reel state**, and in Pure's case the widget exists while the state does not.
Confidence **85** on the shared shape: two independent observations of the same
pattern, no explanation for why either is unreachable.

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

## `Developer Publisher Screen`, and why it is expensive

Pure's `Skin.xml` gives this screen **no content at all** - a single
`<Item></Item>` (commented "Requires a default item to keep screen alive") and
`DevPubRedirect`. Everything it draws is therefore engine code, not data.

Measured at 0.5 s granularity from the language confirm, `pure-psp-usa`:

| t | What is on screen |
| --- | --- |
| 0.0-1.5 s | the picker clears |
| 2.0-4.0 s | phase 1 teletypes "SONY COMPUTER ENTERTAINMENT AMERICA PRESENTS", wrapped over three lines |
| 4.5-5.0 s | held complete |
| 5.5-6.0 s | cleared |
| 6.5-9.0 s | phase 2 teletypes "A STUDIO LIVERPOOL GAME", with the Studio Liverpool logo, frame graphics, "A-G RACING", "//2197", a barcode and an SCE address line |
| 9.5-10.0 s | held complete |
| 10.5-12.5 s | cleared |
| 13.0 s | `MemoryStickWarning` |

Confidence **85** on the phase structure and ordering (directly observed,
reproduced), and **60** on every individual duration: 0.5 s screenshot sampling
bounds each transition to within half a second and no better, and the teletype
rate was not measured at all. The phase-1 string is region-specific
("AMERICA"), so the EU pressing's own wording is **not established** - it was
not captured.

Both phases render in the front-end bitmap font at the same left margin as the
picker's rows, in the picker's own `TextColor`, with `Intro Screen`'s pink
`ArrowSelect` glyph at the left. That the arrow is `ArrowSelect` rather than a
separate glyph is **inferred from position and colour**, not confirmed -
confidence **50**.

**Not yet reimplemented.** Faithful reproduction needs the string source (string
table or binary literal), the real timings, the teletype rate and the logo and
frame assets, all of which are engine-side and none of which a screenshot can
supply. See `HANDOVER.md` for the Ghidra work this needs.

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
- **`Developer Publisher Screen` keeps its place and its timing, not its
  content.** It is walked in order and holds for
  `DEVELOPER_PUBLISHER_SECONDS` (11 s, confidence 60), drawing the parent's
  white and nothing else. The two teletyped phases and the Studio Liverpool logo
  are engine-drawn and need the Ghidra work above, so they are absent rather than
  approximated. An honest gap inside a correct sequence.

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

**Verified** (cold-boot observed, both pressings, reproduced): the five-state
chain and its order; no movie before the picker; `MemoryStickWarning` gating on
cross; `FMV Intro` playing `WoFMVNew_US.PMF` and reaching `Title Screen` by
itself; both pressings carrying both movies at identical sizes, with
`WoFMVNew_US.PMF` byte-identical across regions; both movies being 480x272 with
identical PSMF stream descriptors.

**Inferred**: that `Developer Publisher Screen`'s pink arrow is `Intro Screen`'s
`ArrowSelect` (50); that the two titles' off-path reel states are the same
phenomenon (85).

**Not established**: what advances past `Title Screen`; every duration on
`Developer Publisher Screen` beyond ±0.5 s, and its teletype rate; the EU
pressing's phase-1 wording; why either title ships an unreachable reel state;
where Pulse's language picker belongs.
