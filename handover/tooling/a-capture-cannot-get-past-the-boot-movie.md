---
categories: [tooling, frontend]
---

# A `--screenshot --until` capture cannot get past the boot movie

2026-09-10. Reported by the member working the alpha-test cutout thread, which
needed a front-end frame to judge a rendering change as a player would and could
not get one. **Not independently reproduced yet** - this file records the report,
the constant behind it, and the arithmetic that says the constant's own stated
premise does not hold.

## What was reported

`just play --screenshot out.png --until "TIME"` never reaches its target state.
The run stops at `LogoFMV`, the boot movie leg, having spent its whole tick
ceiling there.

This is not the black-screenshot problem (`no-gui-windows-on-desktop`): the
headless capture path draws offscreen and does not need a window. The frame that
comes back is a real picture of the wrong thing.

## The constant, and why its premise is suspect

`crates/game/src/capture.rs:138`:

```rust
/// How many ticks the runner will take before giving up on `until`.
///
/// The boot movie is forty seconds, and the `--reel` leg is eight plus three
/// two-second holds, so a minute of simulated time covers either and is still
/// bounded.
const MAX_TICKS: u32 = 60 * 60;
```

The doc comment's own reasoning should hold: forty seconds at the fixed 60 Hz
timestep is 2,400 ticks, comfortably inside 3,600. So if a capture really does
exhaust the ceiling inside `LogoFMV`, **one of the premises is wrong**, and which
one is the whole question:

1. The boot movie is longer than forty seconds on the disc actually being read
   (the recipe defaults to `pulse-psp-eu.chd`; the forty-second figure may have
   been measured on another pressing, or on the `--reel` leg rather than the
   boot).
2. A movie tick does not advance the front-end state machine one-for-one, so
   3,600 ticks buy fewer than 3,600 ticks *of the movie* - decode cost, a
   worker handoff, or a frame-pacing mismatch would all produce this.
3. `until`'s state name never matches, and `LogoFMV` is simply where the run
   happens to be when the ceiling expires - in which case the ceiling is a
   symptom and the string matching is the bug.

**Do not "fix" this by raising `MAX_TICKS`.** The ceiling exists to keep a
capture bounded, and a raise that papers over cause 2 or 3 would make every
future capture slower without making any of them correct.

## Why it matters beyond one thread

Every rendering pass on this project is asked to judge its change "as a player
would, at the size a player sees, over more than one frame". `--screenshot
--until` is the only route to a front-end frame that does not need a working
display, and this machine has none (see `HANDOVER.md`'s note on GUI presentation).
So while this is open, a rendering member's visual evidence is limited to
offscreen probes of geometry it can reach directly, and front-end work cannot be
checked on screen at all.

## Open

- Which of the three causes above is the real one - unmeasured.
- Whether `--until`'s state-name matching works at all on the front end, or has
  only ever been exercised on states reachable before the movie.
- Whether `just play-screenshots`' own `--until "Language Selection"` and
  `--until "Launch Game"` legs still work, or are broken the same way and nobody
  has run them lately. That recipe is in `justfile` and is the cheapest existing
  probe of the same code path.

## Next Steps

1. Reproduce, and print the state name each tick rather than guessing:
   `just play --screenshot /tmp/oag-until.png --until "TIME" --log-every 60`
   against `data/images/pulse-psp-eu.chd`. If the log shows the state advancing
   and simply not reaching `TIME` in time, it is cause 1 or 2; if it shows the
   state machine parked, it is cause 3.
2. Time the boot movie directly off the disc rather than trusting the forty
   seconds - the `.PMF` carries its own duration, and `oag-video` reads it.
3. Only once the cause is named, decide whether the fix is a correct ceiling, a
   movie-skip flag for captures, or a fix to the state matching. A `--skip-movie`
   capture flag is the obvious candidate for cause 1, and would make every future
   front-end capture cheaper as well as possible.
