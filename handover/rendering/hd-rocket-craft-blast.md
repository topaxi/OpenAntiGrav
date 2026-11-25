---
status: ready-to-wire
titles: [hd, omega, 2048]
updated: 2026-11-26
summary: "HD Rocket craft blast: play its sprites (ready); it travels with the struck craft (read the attach first); full-screen quads unidentified"
size: M (45)
difficulty: medium (50)
---

# HD Rocket craft blast: why its picture does not match

Measured 2026-11-26 on three hits:
`docs/rendering/particle-fill.md`, "Against the original at a near blast". Films:
`data/reference/hd-capture/rocket-craft-hit/hit1`, `hit2`, `hit3`
(`scripts/rpcs3-hd-rocket-film.py`).

## Open

| # | Gap | Next step | State |
| --- | --- | --- | --- |
| 1 | `WO_ROCKET_EXPLO` draws the procedural profile; the disc's sprites decode and give a fireball like the original's | add it to HD's `Effects::sprites`; find why `FIREBALL_TUMULT`'s sheet does not fit; re-judge against the films | ready to wire |
| 2 | The original's blast travels with the struck craft (cloud 6 to 19 units behind it while it moves 24 to 55); ours stays at the hit point | read what HD's `Psys_Spawn_q` call at `0x0014a7d8` attaches to (emitters riding, or inherited velocity), then wire; check Pulse's own | re-first |
| 3 | Full-screen single-quad additive draws (64x64 texture, half height 2 x depth, yellow to white) from the hit to 0.23 s; ours washes nothing on HD | name the node that draws them (break on the particle flush, or the texture's owner) | re-first |
| 4 | HD's `.pob` streaks: where they are drawn, and whether their near rule applies to ours (25 of 1,055 of our non-flat quads nearer than 1 unit) | read the streak path's draw | re-first |

- Omega and 2048 author their own `WO_ROCKET_EXPLO`: item 1's list is HD's alone, since both
  already play their sprites. Items 2 and 3 are not checked on either.
- The pixel comparison of the near law cannot be re-judged until items 1 to 3 land: by pixels
  ours is further from the original with the law than without, because ours is much darker.
  The law's cull and cap are read on the original's blast quads and are not in question.

## Next Steps

1. Wire item 1 and shoot ours against `hit2` at 0.23 s.
2. Ghidra read for item 2.
