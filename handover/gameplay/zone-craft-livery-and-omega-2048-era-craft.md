# Zone hull is shared on 2048 and Omega; the livery swap and Omega's 2048-era craft are open

2026-10-06, `zone-craft`. `ZoneCraft::OwnShipAt` loads `hdships\Zone\Ship.vex` for
every craft on 2048 v1.04 and Omega (evidence:
`docs/ghidra/functions/vita-2048-eu-v104/zone-craft.md`).

## Open

- The player's pick selects a livery: loader substitutes material `zoneship_zone`
  (Omega: `zoneship_team`) with the craft's livery name over
  `hdships/Zone/Zoneship_<Team>/`. Not wired; the hull draws in its default skin.
  Which livery name a native 2048 craft carries (`+0x1ac`) is unread.
- Omega 2048-era craft (`feisar2048\3`) fail every race: handling looked up under
  `hdships\`. Not Zone-specific.
- The 18 `ship_zone.vex` files: three generic `%s...vex` templates on Vita
  (`0x814e3388`, `0x81464134`, `0x814ca6a0`) not decompiled.
- Zone sibling files (flare, shield, wreck) still compose under the player's team.
- The base (v1.00) 2048 executable's loader was not read.

## Next Steps

1. Find how HD/Pulse substitute a livery by material name and reuse it for the Zone key.
2. Decompile the three templates' callers to close the `ship_zone` question.
