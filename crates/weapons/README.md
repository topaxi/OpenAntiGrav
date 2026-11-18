# oag-weapons

Weapons: the pickup a craft holds, the projectiles it fires, the blasts, beams
and disruptions that land, and the slowdown they credit.

Simulation code. It sits below `oag-gameplay`, whose `World` and `Ship` embed its
state, and reaches a craft only through the generic `Craft` trait. Tracked per
title in [weapon lineage](../../docs/gameplay/weapon-lineage.md).
