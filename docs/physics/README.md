# Physics

> **Not yet started.** Milestone M4. This directory will hold the ship dynamics
> model: the single most important system to get right.

## Scope

- The air cushion: how the craft is held above the track
- Thrust, drag and top speed
- Steering, and how it varies with speed
- Airbrakes, the series' signature control
- Pitch, and its effect over jumps and on landing
- Collision: track surface, walls, ship-to-ship
- Magstrips and inverted sections
- Speed pads and boosts
- Barrel rolls

## Open questions

| Question | Why it matters |
| --- | --- |
| Fixed-point or float? | Decides whether bit-exactness is available for free. See [ADR-0002](../architecture/adr/0002-determinism-model.md). |
| What is the simulation tick rate? | Every constant here is scaled by it. Blocks all of M4. |
| What are the coordinate conventions? | Handedness, units, angle representation. Get this wrong and nothing else parses. |

## Approach

Physics is where "it feels right" is most tempting and least trustworthy. Every
part of this system gets verified against a trace from the original before it is
considered done. See the
[verification protocol](../reverse-engineering/verification-protocol.md), whose
early scenarios exist specifically to isolate the systems on this page.
