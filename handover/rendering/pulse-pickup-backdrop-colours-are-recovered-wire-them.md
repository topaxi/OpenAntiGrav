---
status: ready-to-wire
titles: [pulse, hd, omega, pure, 2048]
updated: 2026-10-30
summary: "Pulse pickup fade law measured (90): wire fade-in, hold, fade-out; HD, Omega and 2048 share the table; Pure has no backdrop"
---

# Pulse's pickup backdrop colours and fade are recovered: wire them

Evidence and confidence: [pickup-colour.md](../../docs/ghidra/functions/psp-pulse-usa/pickup-colour.md)
(table 92, fade law 90, live). The colour table itself is wired (`oag_pulse::hud::PICKUP_COLOURS`).

## Open

Nothing is unread on Pulse's backdrop except a natural pad grant and fire (the live run poked
`pickup+0x1bc`; the update reads only that), and the edge case of a grant during the fade-out.

## Next Steps

1. **Pulse (PSP, PS2 inherits): wire the fade.** Per update `T` is `dt * 1.2`, capped at `0.5` while
   held; a grant starts at `T = 0`. Backdrop word = row word with all four channels (alpha too)
   scaled by `E(3T, 1)`, channels truncated and clamped to 255, where `E(x, k) = ease(x) + k *
   bump((x - 0.5) * 2)`, `ease(x) = (1 - cos(pi x)) / 2` clamped to `[0, 1]` and
   `bump(u) = exp(-25 (u - 0.5)^2)` for `u` in `[0, 1]` else `0` (it overshoots to about 1.8x at
   `T = 0.25`). Size pulse on the same widget: width `w / f`, height `h * f` with
   `f = ease(2T * 0.99 + 0.01)` (the layout's size at `f = 1`). Icon: white scaled by `E(3 * timer, 1)`
   with its own timer growing `dt * 1.2` from `0` once `T >= 0.4`, cap `0.5`. After use: the last
   weapon's word at full strength while that row's timer (counting down from `0.5` at `dt * 1.2`) is
   above `0.1`, then `E((2 - 2T) * 1.5, 1)` with `T` running on from `0.5` to `1.0`, then nothing;
   the icon fades on `E(3 * timer, 1)` as its timer falls. Tests that fail if the curve is dropped:
   the 29 live words of the page's Missile table.
2. **HD and Omega: the same table is authored** (`0x00920598`, `0x0193f750`, words identical to
   Pulse's). Wire the held hexagon colour from it on both; the fade constants on HD (`Hud_UpdatePickupBar`'s
   tail, helpers `0x00676fe8`, `0x00677528`) and Omega's HUD update (`FUN_0125e9b0`) are unread: read the
   rate and the bump width before inheriting Pulse's curve, or inherit it labelled as such.
   [hd-pickup-table.md](../../docs/ghidra/functions/ps3-hdfury-eu/hd-pickup-table.md),
   [omega-pickup-table.md](../../docs/ghidra/functions/ps4-omega-eu/omega-pickup-table.md).
3. **2048: the table and update exist, drawing unverified.** The same two tables sit in the ELF
   (`0x8151f104`) and `FUN_81154172` fades them with a `1.5` rate. But the layout authors `PickupBackground`
   with no `<Values>` and the visible arc is `PickupBgFrame` (fixed `0xFFA7A5A7` plus an 8 Hz white blink,
   `Hud_UpdatePickupIcon`): take one Vita3K frame with a weapon held before wiring the table.
   [pickup-backdrop.md](../../docs/ghidra/functions/vita-2048-eu-v104/pickup-backdrop.md).
4. **Pure: nothing to wire.** No backdrop widget exists; the ten icon models draw in their authored
   colours and are only shown and hidden.
   [pickup-icons.md](../../docs/ghidra/functions/psp-pure-usa/pickup-icons.md).
