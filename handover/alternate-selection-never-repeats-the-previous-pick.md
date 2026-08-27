# Alternate selection never repeats the previous pick, and `Banks::pick` does not know that

2026-08-27. [`docs/ghidra/functions/ps3-hdfury-eu/sound.md`](../docs/ghidra/functions/ps3-hdfury-eu/sound.md#0x19---alternate-selection-decoded)
decodes opcode `0x19`, the grain that chooses which of a cue's alternate
waveforms plays: pick uniformly among the `count` key-ons that follow, **but
never repeat the immediately previous pick** - re-roll once if the random draw
matches what was cached from last time, wrapping around. The pick is cached
per cue (a byte in the cue's own command data, mutated at runtime) and reused
as the "previous" value on the next play.

`crates/game/src/audio/sfx.rs`'s `Banks::pick` is a uniform `rng.below(len)`
with no memory of the last pick - it can play the same alternate twice in a
row, which the original never does. This is the concrete gap the RE finding
opens up; it was not touched this session because reading the opcode and
wiring the generator are different kinds of work (see the RE thread this
grew out of, `handover/a-cue-can-play-other-cues-and-that.md`, and its own
`## Next Steps`).

Two things worth reading before wiring this:

- `Banks::pick` is called per-event (see call sites in `sfx.rs`, e.g. the
  shield pickup and blowup cues), not per-voice, so "previous pick" state
  needs to live somewhere that survives across calls for the *same cue* -
  probably a small `HashMap<Cue, usize>`-shaped field on `Banks` itself (or
  the ground-truth-safe equivalent - this project bans `HashMap` iteration
  feeding simulation state, but a lookup with no iteration is fine per
  `docs/architecture/determinism.md`).
- The re-roll-once-on-repeat shape is specific: it is not "reject and re-roll
  until different" (which would bias small counts), it is "roll once, and if
  it matches, advance by exactly one and wrap" - reproducing the original's
  actual bias (or lack of it) means matching that shape, not a naive retry
  loop.

## Open

- Whether `Banks::pick`'s callers care about the difference (most call sites
  here are one-shot events - pickup, blowup - where "previous pick" may not
  even apply across plays the way it does for a `.COLLISIONS`-style repeated
  cue). Worth checking whether this matters for any currently-wired cue before
  spending effort on it.
- Whether the per-cue cache should be seeded (first play - no previous pick to
  avoid) the same way the original's cached operand byte starts, or if that
  detail does not matter (the original's initial byte value was not read this
  session - see `sound.md`'s own `Not determined`).

## Next Steps

- Read `crates/formats/src/sblk/child.rs` and `sfx.rs`'s `Banks` struct to see
  where a per-cue "last pick" would fit without breaking determinism rules,
  then add it and match `0x19`'s re-roll-once-on-repeat shape.
- Add a test that a cue with two or more alternates never repeats consecutive
  picks across repeated `pick` calls for the same `Cue`, the way
  `Scream_DoGrainAlternate` does not.
