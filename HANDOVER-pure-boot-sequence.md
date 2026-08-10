# Plan: per-title boot/menu profiles, and Pure's faithful boot sequence

## Context

This continues the work in `HANDOVER.md`'s Pure-related rows (search for
"Pure" - several entries dated 2026-08-10 cover the session this plan comes
out of: Ghidra imports, the boot-movie/FMV-Intro/Title-Screen work, the
dangling-`FEGlobals` fixes). Read those first; this file is the next step,
not a replacement.

**Why this plan exists rather than more direct edits**: the session that
found the facts below was mid-edit on a *third* ad hoc `if screens.by_name(X)`
branch (this one deciding Pure's *starting* state) when it became clear the
pattern itself was the problem, not any one instance of it. `boot.rs` and
`frontend.rs` now have Pulse-vs-Pure branches in `default_boot_movie`,
`language_confirm_target`, `load_fmv_intro`, and were about to get a fourth
in `Frontend::booting`'s initial `transition_to`. Two titles is already
uncomfortable; the roadmap's M8 names WipEout 2048 and HD/Fury as later
titles on the same engine, and a fifth branch per title per decision point
does not scale. Untangling this now, before Pure's own sequence is even
finished, is cheaper than untangling it after a third title copies the
pattern.

## Part 1: the architectural split

**Goal**: each title crate (`oag-pulse`, `oag-pure`, and whatever comes
later) owns its own boot/menu *sequence knowledge* - which screen it starts
on, what movies play in what order, what to do when a widget's colour or
font is unrecoverable - and `oag-game` holds one generic mechanism that
consumes whichever title's profile matches the opened source. Today it is
the reverse: `oag-game` hardcodes "if Pulse do X, if Pure do Y" at every
decision point, and the title crates hold only leaf data (name constants,
state-name strings, a flat fallback-globals table).

This mirrors the ADR-0022 split the project already committed to for
*format* data (`oag_pulse::names`, `oag_pure::names`) - the ask here is to
extend the same split to *behavioural* data (what the boot sequence actually
does), which today lives entirely in `oag-game`.

### Suggested shape (not gospel - the implementing agent should feel free to
adjust once real code is in front of them)

A trait in `oag-game`, implemented by each title crate:

```rust
// crates/game/src/title_profile.rs (new)
pub trait TitleProfile {
    /// Does `screens` belong to this title at all? The generic dispatcher
    /// tries each registered profile in turn and uses the first match -
    /// see `Frontend::booting`'s existing `screens.by_name(pure_states::TITLE_SCREEN)`
    /// checks for what this replaces.
    fn recognises(&self, screens: &Screens) -> bool;

    /// The screen the boot sequence starts on, before any movie or picker.
    /// Pulse's is `states::LOGO_FMV` (a screen that does not literally exist
    /// in the disc's own tree either, by design - see `Leg`'s own docs).
    /// Pure's is `states::LANGUAGE_SELECTION` directly - see Part 2.
    fn initial_state(&self, screens: &Screens) -> &'static str;

    /// The default boot movie for `initial_state`, if any. `None` means the
    /// sequence starts with no movie at all - Pure's own case, now that
    /// Part 2 below is understood.
    fn default_boot_movie(&self) -> Option<&'static str>;

    /// Where confirming a language goes next. Pulse: `states::SHOW_LOGO`.
    /// Pure: `pure_states::FMV_INTRO`.
    fn post_language_target(&self, screens: &Screens) -> Option<&'static str>;

    /// `FEGlobals` this title's own `Skin.xml` is known to leave undeclared,
    /// evidenced by measurement (see `oag_pure::frontend::FALLBACK_GLOBALS`
    /// for the pattern and its own confidence notes). Empty for Pulse today.
    fn fallback_globals(&self) -> &'static [(&'static str, &'static str)];
}
```

`oag-game` holds a small fixed list (`const PROFILES: &[&dyn TitleProfile] = &[&oag_pulse::PROFILE, &oag_pure::PROFILE];`, or similar - a `Vec` built once if `dyn` statics turn out awkward) and replaces every
`if screens.by_name(pure_states::X).is_some() { .. } else { pulse_default }`
with "find the first profile that recognises `screens`, ask it."

**Where the existing per-title data already lives, and should stay**:
`oag_pulse::frontend::states::*` / `oag_pure::frontend::states::*` (screen
name constants), `oag_pulse::names::*` / `oag_pure::names::*` (archive entry
names), `oag_pure::frontend::FALLBACK_GLOBALS`. The profile trait's
implementations reference these, not duplicate them.

**Scope check before landing this**: run `python3 scripts/check-dependency-rules.py`
after - a `TitleProfile` trait in `oag-game` that `oag-pulse`/`oag-pure`
implement is fine under the existing two rules (neither rule restricts a
title crate depending on `oag-game`... check this is actually true; if
`oag-pulse`/`oag-pure` currently have no dependency on `oag-game` at all,
introducing one is a new, real dependency edge and worth a moment's thought
about whether the trait should instead live somewhere both `oag-game` and
the title crates can see without `oag-game` depending "downward" oddly -
`oag-title` is the existing crate that predates any title's own data and
might be the more correct home for the trait itself, with `oag-pulse`/`oag-pure`
implementing it and `oag-game` only consuming it).

## Part 2: Pure's real boot sequence, evidenced

**Found this session, cold-boot-verified, not carried over from the
already-contaminated PPSSPP session `HANDOVER.md`'s earlier Pure rows cite**:
the PPSSPP profile used earlier in the day
(`~/.config/ppsspp/PSP/SAVEDATA/UCUS98612P0000`) predates this project
entirely (dated September 2024) and was skipping an unknown amount of the
real boot silently. Moved aside
(`mv ~/.config/ppsspp/PSP/SAVEDATA/UCUS98612P0000{,.bak}`), PPSSPP relaunched
fresh under Xvfb, and the very first frame captured
(screenshot evidence was at `/tmp/cold-boot-0.png` in that session's
scratchpad - regenerate rather than expect the file to still exist) is:

**`Language Selection`, directly. No movie plays before it at all.**

This changes what was implemented earlier the same session under a wrong
assumption: `boot.rs`'s `default_boot_movie` currently resolves
`Data\Movies\IntroMovieP1_US.PMF` (`oag_pure::names::INTRO_MOVIE`) and plays
it before Language Selection, on the belief that Pure's boot mirrors
Pulse's own movie-then-picker order. **That belief is now falsified by a
true cold boot.** `IntroMovieP1` is real (verified present, verified
decodable, verified to actually be Pulse's own `DEVPUB_REEL` content byte-
for-byte - see `HANDOVER.md`'s "Pure's real boot movie is the same file as
Pulse's American dev/pub-reel cut" entry, or whatever it ends up titled) -
it is simply **not on Pure's normal boot path**, the same way Pulse's own
`Intro Screen->IntroMovie1` reel is confirmed not on Pulse's normal boot
path either (`frontend.rs`'s own module docs: "the disc's own boot never
enters that state"). The two titles apparently share more than the file.

**Confirmed sequence, cold-boot to menu, Pure PSP (both regions expected -
only USA was actually driven cold this session; EU should be spot-checked
too before trusting it, same procedure):**

```
Language Selection (no movie played first)
  -> confirm a language ->
FMV Intro (second boot movie, Data\Movies\WoFMVNew_US.PMF)
  -> START/CROSS skips it, or it plays out ->
Title Screen ("WipEout pure" / PRESS START BUTTON)
  -> advance mechanism NOT evidenced, screen is genuinely inert today
```

### What needs to change in code

1. **`default_boot_movie`** (`boot.rs`) should return `None` for Pure's
   default leg - no movie, not `IntroMovieP1`. Once Part 1's profile split
   exists, this becomes `PureProfile::default_boot_movie() -> None`; done
   standalone (without Part 1), it is: stop calling `load_movie` for the
   primary intro at all when the source is Pure and the leg is the default,
   which also saves the one-time ~95s `ffmpeg` transcode of a movie nothing
   ever shows (measured this session: `Data\Movies\WoFMVNew_US.PMF` alone -
   the *second* movie, which does play - is already 2848 frames/95s to
   transcode once; doing that unnecessarily for a movie that never draws
   is real wasted time on every fresh checkout).
2. **`Frontend::booting`**'s initial `machine.transition_to(leg.state())`
   needs to start at `states::LANGUAGE_SELECTION` instead of
   `states::LOGO_FMV` for Pure's default leg. Concretely (pre-Part-1): add
   the same `screens.by_name(pure_states::TITLE_SCREEN).is_some()`-shaped
   check used everywhere else in this file, and transition there instead.
   Post-Part-1: `initial_state()` on the matched profile.
3. **Re-verify `FMV Intro` and `Title Screen` still transition correctly**
   from this new starting point - they should, since nothing about
   `confirm_language`/`update_fmv_intro` assumed a particular *prior* state,
   only the *target* states, but confirm rather than assume.
4. **The `--reel` flag** (`Leg::DevPubReel`) is Pulse-specific machinery
   (`states::INTRO_MOVIE`, `PAUSE_FRAMES`, `FINISH_FRAME`, all evidenced off
   Pulse's own binary) - leave it alone for Pure; it is an explicit opt-in
   flag, not part of any title's default sequence, and nothing found this
   session suggests Pure has an equivalent worth modelling yet.

### Verifying against PPSSPP - do this for real, don't trust a stale profile

```sh
# Move aside any existing save before trusting a "cold boot" - check first:
ls ~/.config/ppsspp/PSP/SAVEDATA/ | grep -i 98612   # USA
ls ~/.config/ppsspp/PSP/SAVEDATA/ | grep -i 00001    # EU, if it has its own id
mv ~/.config/ppsspp/PSP/SAVEDATA/<id> ~/.config/ppsspp/PSP/SAVEDATA/<id>.bak

Xvfb :98 -screen 0 1280x720x24 &
printf '[General]\nRemoteDebuggerOnStartup = True\nRemoteDebuggerLocal = True\nRemoteISOPort = 47823\n' > /tmp/debugger.ini
DISPLAY=:98 SDL_VIDEODRIVER=x11 setsid PPSSPPSDL --appendconfig=/tmp/debugger.ini \
    --windowed data/cache/pure-psp-usa.iso < /dev/null > /tmp/ppsspp.log 2>&1 &
disown
sleep 10   # first attempt commonly needs a Vulkan->OpenGL fallback retry - check the log
DISPLAY=:98 import -window root /tmp/cold-boot-check.png

# Restore the save afterward:
mv ~/.config/ppsspp/PSP/SAVEDATA/<id>.bak ~/.config/ppsspp/PSP/SAVEDATA/<id>
```

See `docs/reverse-engineering/ppsspp-debugger.md` for the full trap list
(Vulkan-first-attempt-fails-retry-OpenGL, `import -window root` needing the
window repositioned if it lands off-canvas, etc.) - this session hit the
Vulkan one again and it is exactly as documented.

## Part 3: two confirmed bugs from an independent review, not yet fixed

An independent agent reviewed the `FMV Intro` video-wiring diff this session
(prompted by a user report that turned out to be something else entirely -
see below) and found two real, separate issues worth fixing alongside this
work:

1. **`Frontend::is_playing_movie()`** (`frontend.rs`) omits
   `pure_states::FMV_INTRO`:
   ```rust
   pub fn is_playing_movie(&self) -> bool {
       self.machine.is(states::LOGO_FMV) || self.machine.is(states::INTRO_MOVIE)
   }
   ```
   `main.rs` calls `self.audio.stop_movie()` whenever this is `false`, so
   entering `FMV Intro` immediately tears down movie audio, and
   `movie_playhead()` returns `None` for the whole leg (falls back to
   dt-pacing instead of audio-pacing). Separately, nothing anywhere starts
   `FMV Intro`'s *own* audio track (`Data\Movies\WoFMVNew_US.PMF` declares
   `sound="true"` in the XML, so it should have one) - there is no
   `audio.start_movie(...)`-shaped call near the `stage.feed =
   stage.fmv_intro_feed.take()` swap in `main.rs`'s tick loop. Fix: add
   `|| self.machine.is(pure_states::FMV_INTRO)` to `is_playing_movie`, and
   decode+wire the second movie's own PCM the way `Boot.movie_sound` already
   does for the first (see `load_movie_sound` in `boot.rs` for the pattern -
   it needs the same treatment for `fmv_intro`, including a `Boot.fmv_intro_sound`
   field and the equivalent of `audio.start_boot_movie` called at the right
   moment in `main.rs`).

2. **`FrontendStage::sync_video`** (`main.rs`) assumes a draw list has at
   most one `Draw::Video`:
   ```rust
   let drawn = list.iter().find_map(|draw| match draw {
       Draw::Video { position, source, .. } => Some((*position, *source)),
       _ => None,
   });
   ```
   `Frontend::draw_list`'s `FMV_INTRO` branch is the first place in the
   codebase that can push both a `Video::Backdrop` (via `insert_backdrop`)
   *and* a `Video::Intro` into the same list. `find_map` silently picks
   whichever comes first. **Not exploitable on Pure's current PSP discs**
   (neither carries `Data\Movies\Backdrop.PMF`, so `Frontend::backdrop` is
   always `None` there and `insert_backdrop` no-ops) - but latent, and live
   the moment any title ships both a menu backdrop and a state like `FMV
   Intro`. Fix: either make `sync_video` handle a list with multiple
   `Draw::Video`s explicitly (upload each to its own target), or make
   `insert_backdrop` a no-op when the list already contains a `Draw::Video`
   (the two are conceptually alternatives - foreground movie vs. background
   loop - never simultaneous on any screen this build knows about).

## Part 4: what the "menu is unusable" report actually was (not a bug)

Recorded so the next agent does not re-investigate this: a user report of
"the language menu flickers, then the video plays on top of it, the menu is
essentially unusable" was **not** a rendering overlap bug (independently
verified - state machine, draw dispatch, and render clear-per-frame were all
checked and are correct). The actual cause: this development machine's own
`~/.config/oag/settings.toml` already has `language = "German"` saved from
earlier testing, so `Frontend::update_language_selection`'s existing,
title-agnostic auto-confirm-from-settings path fires in ~1 tick - the picker
was never meant to be watched on a repeat run, and this has always been true
for Pulse too, just invisible there because Pulse's post-picker screen
(`Show Logo`) is static rather than a movie. Confirmed with
`just play pure-usa --pick-language`, which forces the real picker and
behaves normally. **Nothing to fix here** - it is worth deciding, as a
product question rather than a bug, whether skipping straight into a
*movie* on a repeat boot feels different enough from skipping onto a static
screen that it deserves its own UX treatment, but that is a choice, not a
defect.

## Suggested order of work for the implementing agent

1. Do Part 2 standalone first (it is a correctness fix independent of the
   refactor, and small: two call sites) - re-verify the whole cold-boot
   chain (Language Selection -> FMV Intro -> Title Screen) still works and
   screenshot it, the same way `HANDOVER.md`'s existing Pure rows do.
2. Fix Part 3's two bugs (small, contained, already fully diagnosed above -
   no further investigation needed, just implementation).
3. Only then attempt Part 1's refactor, informed by a codebase where Pure's
   sequence is actually correct - refactoring around a still-wrong sequence
   risks baking the wrong shape into the new abstraction.
4. Once Part 1 lands for Pulse+Pure, do the same faithfulness pass Part 2
   did for Pure against **Pulse's own** boot sequence - `frontend.rs`'s
   module docs already flag known, deliberate reorderings from the disc's
   real order (`LogoFMV`-before-picker vs. the XML's own picker-first
   order), which were accepted as "the order asked for" rather than
   evidenced as correct. Worth the same le grade of cold-boot scrutiny Pure
   just got, now that the tooling and the pattern both exist.
5. Full gate after each step: `just` (fmt, clippy, test, check-docs,
   check-deps) plus a real `--screenshot` walk of both titles' boot
   sequences, not just unit tests - this whole investigation started because
   unit tests all passed while the real behaviour was wrong.
