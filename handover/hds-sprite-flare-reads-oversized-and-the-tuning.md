# HD's sprite flare reads oversized against the original, and the tuning file's other 19 rows are read

2026-09-01. Started from a user report from play: the exhaust flare "too
big/too far out", with the trail also suspected. Confirmed the first half
with a matched-ish screenshot pair rather than guesswork -
`data/reference/hd-capture/flare-size/` holds `original-rpcs3.png` (a driven
Talon's Junction lap via `scripts/rpcs3-drive.py race --drive 15 --shots`)
against `ours.png` (`just play hd --race --press cross --ticks 300`), same
track and a comparable chase framing. The original's core is a compact glow
at each nozzle, narrower than the hull; this engine's is one white disc
spanning past both nozzles, roughly hull-width. Full writeup, including what
was ruled out (a second locator, the constants themselves) and the six
tuning-file rows read for the first time (`Flare Highlight Power/Boost`,
`Flare Size Clamp`, `Engine Flare Particles Min Alpha`/`Enable`, on top of
confirming `Spikes Thrust/Boost Max Scale` and `Shockwave Cycle Speed` stay
unread) is on
[engine-trail.md](../docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md),
"All 41 rows, read 2026-09-01".

**Why this isn't a code change yet.** `Flare Fadeout Dist`/`Range` (15.0
world units each) is the leading candidate - a near/far fade landing "small
at ordinary chase distance" matches both screenshots - but it is a reading
of two numbers, not a traced consumer, and `hd.rs`'s existing doc already
flagged the same shader pair (`engineflare_vp`/`fp`) as unresolved through
the registry script two sessions ago. Writing a fade curve to fit two
screenshots is exactly the invented-stand-in shape CLAUDE.md rules out, so
`hd::Sprite`'s radius law is untouched. Two static leads were run down and
closed rather than followed further: `EngineFlare_Init`'s `+0xe4`/`+0x144`
writes are never read inside `EngineFlare_PlaceShapes` or
`EngineFlare_Update` (grepped both functions' full decompile), and the
function sitting between `EngineFlare_Init` and `EngineFlare_PlaceShapes` in
address order (`FUN_002a17e8`) turned out to be the engine-sound cue state
machine, not a draw call. The function that actually turns `Flare Radius`
into a world-space quad is still unlocated.

## Open

- The consumer of `Flare Radius`/`Flare Radius Min`/`Max Radius Jitter` -
  the function that builds the sprite's world-space quad - is unlocated.
  Two candidates ruled out this session: `EngineFlare_PlaceShapes` and
  `EngineFlare_Update` (neither reads `+0xe4`), and `FUN_002a17e8` (the
  engine-sound state machine, not a draw call).
- `Flare Fadeout Dist`/`Range` (15.0/15.0): whether a near/far fade is what
  is missing, and if so its curve - linear, smoothstep, or something else -
  is unread. The leading candidate for the oversized-flare finding, but
  unverified.
- `Flare Size Clamp` (50.0): newly named, unread past the tuning file. The
  order-of-magnitude gap against `Flare Radius` (3.0) suggests it clamps
  something other than a raw world half-size, but that is a hypothesis from
  the numbers alone.
- `Engine Flare Particles` (`Enable`, `Min Alpha` 0.25): a third flare
  component distinct from the always-on flame model and the sprite, neither
  of which this project draws. The small bright dashes trailing under the
  nozzle in `original-rpcs3.png` are the leading candidate for what this
  is. No `.pob` or emitter table has been matched to it.
- `Flare Highlight Power`/`Boost` (32.0/1.0), `Spikes Thrust/Boost Max
  Scale` (1.5/2.0), `Shockwave Cycle Speed` (0.01): named, unread.
- The trail itself is also suspected inaccurate per the user's report.
  **Checked and refuted**: a separate HD-vs-Fury trail asset (no - one
  asset, disc-wide, colour-mixed by the already-implemented `engineTrail`
  parameter) and a Zone-specific trail material (no - neither the trail's
  nor the flame's material declares any of the 16 known zone-parameter
  hashes). See "Checked: no separate trail for HD vs. Fury" on
  engine-trail.md. If the trail still reads wrong in Zone specifically, the
  live candidate is HD's still-unlocated Zone environment recolour
  ([zone-effectsettings-loader.md](../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md))
  reaching ships too, not a second trail asset - unconfirmed either way.

## Next Steps

- Find the sprite's actual draw call, most likely by extending
  `scripts/rpcs3-trail-dump.py`-style live dumping to the RSX vertex data
  the flare quad submits (its four corners, world-space, independent of the
  still-unsolved camera problem the capture harness carries) rather than
  more static disassembly - three static leads dead-ended this session.
- Once the consumer is read, either confirm and implement the
  `Flare Fadeout Dist/Range` law or find what actually scales the sprite.
- Separately assess the trail's own accuracy against the original, per the
  user's report - not investigated this session.
