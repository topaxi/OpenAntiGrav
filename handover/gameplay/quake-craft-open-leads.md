# What a Quake does to the craft it reaches: open leads

**Status: wired 2026-10-26 (`quake-wire`), leads left.** Evidence:
[cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md),
section "2026-10-26: what a Quake does to the craft it reaches".

Landed: each craft carries `Ship::quake_lift` (`h += (raw - h) * 8 * dt`, every tick, kept
decaying after the wave expires); the hit lands on the rising edge of `h > 0.1` in place of the
flat `radius` test; a laid mine trips on the raw height `> 0.1`; the cockpit eye rises by `h`
along the ship's up (`oag_render::camera::internal::eye`). Per title as
`oag_title::weapons::Weapons::quake_hit` (`QuakeHit::WaveHeight`, Pulse measured, the rest
inherited, see the "Checked axes" row in `docs/gameplay/weapon-lineage.md`). No kick and no
body ride: the original does neither. **Pure holds the eye level** (2026-10-26,
`pure-no-quake-lift`; `QuakeHit::WaveHeightEyeLevel`, the maintainer's rule "in Pure this does
not exist", chosen) over the same inherited gate.

## Open

1. **No rear-view camera exists in ours** (`<BackwardCamera>`), so its lift is not applicable
   until one does. When it lands it adds `h` to the eye's world `y` and builds the aim from the
   lifted eye (`Ship_UpdateBackwardCamera`, `0x0883f7b8`).
2. **`CockpitHitFx_Arm_q`** (`0x088eeaf8`, a cockpit-only overlay on any weapon hit to the
   human) is not in ours. Its picture is unread (shield.md, confidence 60). Not Quake-specific.
3. **One-frame lag.** The original's rigs read last frame's `h`; ours reads the value after the
   tick, one tick later. Chosen, not measured, and invisible at 60 Hz; reproduce it only if a
   matched capture shows a difference.
4. **HD's per-craft camera path** was not traced; HD's film shows the camera rising with the
   road, and HD ships `Quake_Fired/Impact/Incoming_Rumble.xml`. Whether HD's lift is the same
   smoothed flat amplitude is unread. Recipe: in the EU `EBOOT.elf`, find the `QUAKEHIT` cue's
   caller (string at `0x00782678`) and look beside it for a per-craft `+= (x - y) * 8.0 * dt`;
   then find that field's camera readers. Until then HD inherits Pulse's law.
5. ~~**Pure** camera lift~~ - closed 2026-10-26: held level, **chosen**. Pure's own camera path is
   still unread; if it is ever read and rises, flip `crates/pure`'s `quake_hit` back to
   `WaveHeight` with the measured origin.
6. **`QuakeStats::radius`** is copied by the original into the firing record's `+0x190` at
   fire time and that copy's reader is unidentified (recipe on the evidence page). Nothing in
   ours reads it now.

## Next Steps

1. Item 4 is the one with a recipe and a player-visible payoff (HD's cockpit view).
2. Item 1 waits on a rear-view camera existing at all.
