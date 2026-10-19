# HD's LeachBeam drain is unread, so HD-lineage opponents hold the beam

2026-10-19, lane `campaign-ai-weapons`. Original-driven opponents never
fired anything (measured on all five titles, see
[gameplay/ai.md](../../docs/gameplay/ai.md), "An original opponent never
fired, and now does"). Once they fired, the HD, 2048 and Omega LeachBeam
emptied a full shield in under two seconds and the field wrecked itself.
Maintainer's decision: those titles' opponents hold the beam
(`oag_title::weapons::Weapons::opponents_fire_leach_beam = false`, chosen,
not measured) until the drain is wired from HD's own law. The player's beam
is unchanged.

## Open

- **The per-trip drain.** `LeachBeamManager_Update_q` (`0x0013d7a8`) adds
  stats `damage` (`+0x140`) into `target+0x120` on every update while
  `beam+0xc938 > 0.0`: Pulse's per-call shape. Two things are unread. The
  writer of `beam+0xc938` (only the constructors zero it). And the ship-side
  consumer of `+0x120` with kind `7`. Evidence and addresses:
  [weapons.md](../../docs/ghidra/functions/ps3-hdfury-eu/weapons.md),
  "Where HD does apply it". The trip cadence, 0.3 to 1.0 s on beam length,
  is read at confidence 85 on the same page.
- **The player's beam is as lethal against opponents** on these titles today.
- **The absorb half of `WeaponAi_DecideFireOrAbsorb`** (Pulse, recovered on
  `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`) was ported and reverted
  on 2026-10-19: it moved 11 lone-craft cells of `ai_clean_lap_gate` and the
  de Konstruct gates, through this project's Turbo policy, and its decision
  cadence (every tick or four a second) is unconfirmed. Port it with the
  cadence settled and those gates rebaselined on purpose.

## Next Steps

1. Find the live writer of `beam+0xc938`: a store through a computed base,
   or a `stfs` after `addis r,r,1` with `-0x36c8`. If it is the ball's
   arrival, the drain is per trip.
2. Read the consumer of `target+0x120`, the analogue of Pulse's
   `Ship_ApplyPendingWeaponDamage`, for a `dt` or class scale.
3. Wire the drain for the player and opponents, then set
   `opponents_fire_leach_beam` back to `true` on HD, 2048 and Omega. The
   original-AI walks and the measured readings (`54.51`, `63`, `74`) must stay
   green without loosening anything.
