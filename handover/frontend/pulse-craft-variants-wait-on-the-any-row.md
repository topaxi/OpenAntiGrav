# Pulse craft variants: earning is kept, the gate is not read

## Open

- Loyalty is earned (`Race_ComputeLoyaltyAward`) and persisted per team
  (`records::Store::record_loyalty`); the circuit gate landed (`oag_game::unlock`).
- Craft variants (`PI_ModelSkin`/`PI_TeamModel`) carry two `Exclusive` rows,
  `Team=<own>` and `Team="any"`. How `Definition_IsUnlocked` (`0x0888e29c`) combines
  them, and what `any` sums or maxes over, is untraced. Gating on the plain reading
  (own team total >= price) would be a guess; nothing is wired.
- `Team Selection`'s Loyalty bar is undrawn yet: the
  fill law is known (`total * 0.00124` px, `endrace-screens.md`) but the bar's
  own screen read is not wired.
- `+0x16e` (mode-gated per-track byte) and `+0x99` unmodelled.
- The ghidra bridge was down for this lane (no instance); needs a decompile of
  `Definition_IsUnlocked`'s Team handling or a PPSSPP probe setting loyalty.

## Next Steps

1. Decompile `Definition_IsUnlocked` / `Unlock_LoyaltyMet` for the `Team` field.
2. Wire the variant gate and the Loyalty bar through `--unlock-all` too.
