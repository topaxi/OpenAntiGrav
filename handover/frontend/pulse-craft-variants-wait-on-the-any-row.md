# Pulse craft variants are gated on loyalty; RACE REMIX and two bytes are not

## Open

- Landed 2026-10-02: `Definition_IsUnlocked`'s row combine and `Unlock_LoyaltyMet`'s `any` are traced (`race-box-screens.md`, "Traced 2026-10-02"); Team Selection's livery row and the RACE page's VARIANT row gate on them (`oag_game::unlock::loyalty_unlocked`, `--unlock-all` lifts it, ours); the Loyalty block is drawn.
- RACE REMIX's craft-side VARIANT row (`resupply_remix_variant`) is still ungated: it is scoped to `remix.craft_title`, whose team catalogue is not the booted shell's.
- The original's own list builder also checks a class virtual (`vtable+0xd4`) on each skin; unread.
- Context-filter attributes on an `<Unlock>` row (`Class`, `Team`, `Track`, `Tournament`, `Mode`, `Grid`, `FUN_0888e6e8`) are not modelled; no shipped variant row authors one.
- `+0x16e` (mode-gated per-track byte) and `+0x99` unmodelled.
- Not run live on PPSSPP: the combine is a static read at confidence 88.

## Next Steps

1. Gate RACE REMIX's variant row off its own craft catalogue.
2. Watch `Definition_IsUnlocked` in PPSSPP with loyalty written to confirm the `any` branch live.
