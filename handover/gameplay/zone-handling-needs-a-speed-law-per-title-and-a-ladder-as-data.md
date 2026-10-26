---
status: ready-to-wire
titles: [pulse, pure, 2048, omega]
updated: 2026-10-12
summary: "Wire Pulse's Zone speed-class ladder as ZoneSpeedClasses data; Pure's and Omega's ladders unread"
---

# Zone handling needs a speed law per title and the ladder as data

2026-10-12, lane `speed-class`. Zone's escalating speed steps are names over one
scale, not handling classes (evidence and confidence:
`docs/gameplay/race-modes.md`, "What Zone's escalating speed steps are").

## Open

- Pulse: the speed law is `34.0 + 4.4 * zone` (`Environment::auto_speed`, wired);
  the HUD name ladder (`FUN_0881ad10`, thresholds 2/3/5/7/12/14/20/22/35, ten names
  Sub-Venom .. Zen) is read and not wired: `oag_title::ZoneSpeedClasses` is `None`
  for Pulse, Pure, 2048 and Omega and only HD carries one.
- Pure, Omega: ladder unread. 2048: seventeen records wired for the grade only.
- Zone craft (`Zone_01`, HD `/data/ships/zone`, 2048 `Zone`/`Zone Battle`/`Detonator`)
  author no `<Class>`; Pulse flies the player's team class, the other titles' Zone
  handling source is unread (the physics question in `zone-races-run-on-all-three-titles-now.md`).

## Next Steps

1. Wire Pulse's ladder as `ZoneSpeedClasses` data (names are on its disc as
   `MSC_*`; Super-Phantom is 22-34 there, 27-34 on HD).
2. Read Pure's and HD's actual Zone speed law and whether either takes handling
   from the class-less block; only then give Zone a handling source.
