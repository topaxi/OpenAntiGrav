# Zone hull is shared on 2048 and Omega; native-craft livery and `ship_zone.vex` are open

2026-10-06, `zone-craft` then `omega-2048-craft`. `ZoneCraft::OwnShipAt` loads
`hdships\Zone\Ship.vex` for every craft on 2048 v1.04 and Omega, and now swaps
the team texture by the definition's own `PI_TeamModel name="zone"` livery.
Omega's 2048-era craft start races (`GuestRoster.handling_dir`).

## Open

- 2048-era and 2048-native craft author no zone livery; the original's fallback
  (`Zoneship_Faisar` literal, entry `+0x1ac`/`+0x218`) was not read, so they keep
  the default skin. Whether the original paints them Feisar is unread.
- On Vita the Zone hull renders bloomed white, so livery paint is not visible
  there (Omega shows it). Not a livery bug.
- The 18 `ship_zone.vex` files: three generic `%s...vex` templates on Vita
  (`0x814e3388`, `0x81464134`, `0x814ca6a0`) not decompiled.
- Zone sibling files (flare, shield, wreck) still compose under the player's team.
- The base (v1.00) 2048 executable's loader was not read.
- Omega Race Remix: CRAFT TITLE "Wipeout 2048" is now offered with no 2048 disc,
  and Omega's own list drops the 2048-era teams to it; confirm that is wanted.

## Next Steps

1. Decompile the three templates' callers to close the `ship_zone` question.
2. Read the entry's livery-name field for a 2048-era craft to settle its Zone skin.
