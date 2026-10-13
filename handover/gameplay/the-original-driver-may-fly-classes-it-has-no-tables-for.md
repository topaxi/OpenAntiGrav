# The original driver may fly classes it has no tables for

2026-10-13, queued by the maintainer. Starts after the lane that makes the
recovered driver the default in every mode (`original-ai-default`) lands.
No work started.

Today a class gets opponents only when something defines how to fly it
(`docs/gameplay/ai.md`, "Which classes a controller covers"): this project's
driver on VENOM, FLASH, RAPIER, PHANTOM and Pure's VECTOR, the original's on
the classes its title's AI tables author. Any other class, such as RACE REMIX
on Super Phantom or a modded class, runs with no opponents, greyed out on the
setup screen.

That restriction was written for this project's driver, which needs a tuned
speed plan. The original driver may not need it: `Ai_SnapToLane` moves the
body toward its lane every tick (up to 70 % of the lateral distance and 17.5 %
of the sideways velocity, above 50 u/s, near the surface, touching nothing),
the one place it bypasses physics (`docs/gameplay/ai.md`, the original driver
section). The maintainer's observation from play is that it flies every track
at every speed.

## Open

- **What a class without tables borrows.** The per-class parts are the thrust
  and balancing tables (`AIRaceStats_<class>.xml`, Pure's per-class block) and
  the steering controller (`AIControlStats.xml`'s `<Controller>`; cross-track
  gain falls and damping rises as the class gets faster). Candidate: the
  nearest authored class at or below the race's speed, or 2048's SUPERPHANTOM
  for Super Phantom. Chosen, not measured; named in the loader report.
- **Whether it actually flies.** Not tested. The rail is a reason to expect it,
  not evidence.
- **Mods.** A modded class with no AI tables would fly the same borrowed rule
  unless the mod ships its own tables.

## Next Steps

1. A sweep with the recovered driver: every circuit, every class with no
   tables, a lone opponent and a full field; count clean laps, wall contact
   ticks, respawns and lap times against the nearest authored class. Half a
   day.
2. If it holds, add a borrowed-tables rule to the original driver's coverage
   (data on the controller, not a title branch), un-grey those classes in race
   box and remix, and update `docs/gameplay/ai.md` and `docs/architecture/menus.md`.
   Half a day.
