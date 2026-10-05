# ADR-0018: Own the mixer, and treat cues as a per-tick output

## Status
Accepted

## Context

M5 opens audio, and until now the workspace had none: no `oag-audio` crate, no
sample played anywhere, and `crates/game/src/movie.rs` demuxing the ATRAC3+
track out of every movie and discarding it. The data side was already well
ahead - PS-ADPCM decode, the PS2 PCM archives and the PMF audio demuxer are all
implemented and ground-truthed.

Three things about the original constrain the shape of a runtime.

**The PSP does not software-mix.** 26 `sceSasCore` functions are imported,
including `__sceSasSetADSR`, `__sceSasSetADSRmode`, `__sceSasRevType` and
`__sceSasCoreWithMix` (`docs/ghidra/functions/psp-pulse-usa/imports.md`). The
synth does envelopes, mixing and reverb in hardware, so an accurate path
eventually means modelling those semantics rather than playing samples through
whatever mixing model a library happens to have. The hardware has exactly 32
voices.

**There are two volumes, not one.** The options menu strings are `"Music
Volume"` (`0x08a78658`) and `"SFX Volume"` (`0x08a78668`), and nothing else.

**Sounds are triggered by cue name from all over the code.** `FUN_089392b0` has
37 callers; it allocates a request node and pushes it onto a per-owner list, and
a leading `~` on the cue name means "hand me back a handle to a looping voice"
(`"~ENGINE"`). Cue emission is therefore something that happens *at an edge*
during a tick, and the per-tick outputs the simulation already produces -
`oag_race::Outcome`, `oag_physics::Evaluated` - are exactly the shape that
suits.

Against that: `docs/architecture/determinism.md` requires the simulation to
produce bit-identical state across platforms, CI asserts committed hashes in
`oag_core::hash`, and CLAUDE.md forbids editing those constants to make the test
pass. Meanwhile CI has no sound card, and this project is run headless with
`--screenshot` rather than in a window - so an audio path that only works with a
device attached cannot be checked at all.

## Decision

**A new `oag-audio` crate, on the non-simulation side of the dependency
boundary**, alongside `oag-render` and `oag-input`.
`scripts/check-dependency-rules.py` already listed `oag-audio` in the set
forbidden to gameplay crates before the crate existed, so the rule bound it from
its first commit.

**`cpal` for the device, and our own mixer above it.** `cpal` is a thin
cross-platform device layer; voices, mixing, resampling, pitch and gain are this
crate's code.

**Split state from device**, the way `oag_fx::sparks::Sparks` splits from
`sparks::Pipeline`:

- `Mixer` is plain data with no device handle: a 32-voice pool, two buses, and
  the sample loop.
- `Output` owns the `cpal` stream, or none at all.

**A null backend is a first-class mode, not a test fixture.** `Output::null`
still mixes and still renders samples; only the handoff to hardware is missing.
This mirrors `oag_input::Controls::without_pad`, which exists for the same
reason.

**The mixer never reads a clock.** Elapsed time is whatever the caller renders.
`render_tick` derives its frame count from the tick rate alone.

**Cues are a per-tick output of the simulation, never `World` state**, and the
control half is serviced inside the fixed-step loop while the device half is
serviced once per frame outside it. This follows the existing rule at
`crates/game/src/race.rs:2046-2050`, where exhaust and camera advance inside
`tick` precisely so a headless capture and a window agree at the same tick
count.

**Pitch and gain now; ADSR and reverb deferred.** The pitch and volume law is
actually recovered - `Exhaust_UpdateEngineSound` at confidence 80 gives the
engine's pitch target, its first-order lag, and its per-craft random note
spread. The envelope and reverb semantics are not.

## Alternatives considered

**`rodio`.** A sink/source API over `cpal`; faster to first sound. Rejected
because its mixing model is fixed, and per-voice pitch and eventual ADSR control
would be fighting it rather than using it. The thing we most need to control is
the thing it most abstracts.

**`kira`.** Game-audio crate with mixing, tweens and effects built in. Rejected
as the largest dependency of the three, and because it brings its own scheduling
model, which sits awkwardly beside a fixed 60 Hz `TickClock` that must stay the
only clock the simulation sees.

**Cue emission as `World` state.** Rejected outright: it would move the
committed determinism hashes, and those are never edited to make a test pass.
Modelling cues as an output costs nothing, because `Outcome` and `Evaluated`
already establish the pattern - and `Outcome`'s own doc comment was written
anticipating this, naming "a sound" as the thing it exists for.

**Voice stealing when the pool is full.** Rejected for now: stealing needs a
priority, the original keeps its priorities in the `.bnk` cue table, and that
table's per-sound layout is undecoded. A stealing rule invented here would be a
guess dressed as behaviour. The mixer refuses instead, and counts the refusals.

## Consequences

**A refused voice is a missing sound.** With 32 slots and no stealing, a busy
race can drop a cue that the original would have played by evicting something
less important. The starvation counter makes it visible rather than silent, but
it is a real fidelity gap until the cue table is decoded.

**The audio callback can emit a frame of silence.** It takes the mixer lock with
`try_lock` and fills silence on contention, because blocking the audio thread on
a tick that is mid-update is a dropout for every voice instead of a late update
for one. Under heavy lock contention this is audible.

**We now own resampling quality.** Linear interpolation between source frames is
cheap and correct enough for playback, but it is not what the PSP's hardware
does, and any future comparison against a hardware capture will show it.

**We have taken on `sceSasCore`.** Choosing to own the mixer means the ADSR and
reverb work is ours to do rather than a library's, and until it is done the
audio will be recognisably flatter than the original. This is the cost that buys
the ability to do it at all.

**Nothing here is covered by the determinism contract.**
`docs/architecture/determinism.md` puts audio outside the simulation
explicitly, so the mixer may allocate, lock and thread freely. The guarantee it
does make - same control sequence, same samples - is weaker, and is asserted by
test rather than by the cross-platform hash job.
