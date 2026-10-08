# Pulse craft variants are gated on loyalty; one byte and the live check are not

## Open

- 2026-10-08 `pulse-unlocks` census: circuit gate and variant gate both apply on PS2 Pulse through the shared code (fresh Track Select 3 circuits on PSP and PS2); Pure's 17 `<Unlock>` rows are another shape (`MedalCount`, `Track`+`medal`, `Tournament`), unread and not wired. A fresh-profile loyalty earn-and-unlock walk through the real race loop is still unplayed (ground truths cover the law).

- Landed 2026-10-02: `Definition_IsUnlocked`'s row combine and `Unlock_LoyaltyMet`'s `any` are traced (`race-box-screens.md`, "Traced 2026-10-02"); Team Selection's livery row and the RACE page's VARIANT row gate on them (`oag_game::unlock::loyalty_unlocked`, `--unlock-all` lifts it, ours); the Loyalty block is drawn.
- RACE REMIX's craft-side VARIANT row (`resupply_remix_variant`) stays ungated **by decision** (maintainer, 2026-10-02): RACE REMIX is this project's own mode, not the original's, so no disc unlock applies to it.
- The original's own list builder also checks a class virtual (`vtable+0xd4`) on each skin; unread.
- Context-filter attributes on an `<Unlock>` row (`Class`, `Team`, `Track`, `Tournament`, `Mode`, `Grid`, `FUN_0888e6e8`) are not modelled; no shipped variant row authors one.
- `+0x16e` is `availableInZone` (closed 2026-10-02, `oag_game::unlock`'s module doc); `+0x99` is unmodelled.
- Not run live on PPSSPP: the combine is a static read at confidence 88.

## Next Steps

1. Watch `Definition_IsUnlocked` in PPSSPP with loyalty written to confirm the `any` branch live.
