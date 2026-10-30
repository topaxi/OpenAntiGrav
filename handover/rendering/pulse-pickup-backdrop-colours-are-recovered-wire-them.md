---
status: measure
titles: [pulse, hd, omega, pure, 2048]
updated: 2026-10-30
summary: "Pickup fade wired on Pulse, HD, Omega (2026-10-30); open: 2048 needs a Vita3K frame, HD fade-out and Omega rate unread"
---

# Pickup backdrop fade: wired on Pulse, HD and Omega; 2048 open

Evidence and confidence: [pickup-colour.md](../../docs/ghidra/functions/psp-pulse-usa/pickup-colour.md)
(table 92, fade law 90, live). The colour table itself is wired (`oag_pulse::hud::PICKUP_COLOURS`).

## Open

Nothing is unread on Pulse's backdrop except a natural pad grant and fire (the live run poked
`pickup+0x1bc`; the update reads only that), and the edge case of a grant during the fade-out.

## Next Steps

Wired 2026-10-30 (`pickup-fade-wire`): the fade-in, hold, fade-out, icon gate and (Pulse only) size pulse in
`oag_hud::pickup_fade`; HD and Omega draw the held backdrop from their own table through the same fade,
HD at its read rate `1.5`. Evidence and frames: [pickup-colour.md](../../docs/ghidra/functions/psp-pulse-usa/pickup-colour.md).

1. **2048: the table and update exist, drawing unverified.** The same two tables sit in the ELF
   (`0x8151f104`) and `FUN_81154172` fades them with a `1.5` rate. But the layout authors `PickupBackground`
   with no `<Values>` and the visible arc is `PickupBgFrame` (fixed `0xFFA7A5A7` plus an 8 Hz white blink,
   `Hud_UpdatePickupIcon`): take one Vita3K frame with a weapon held before wiring the table.
   [pickup-backdrop.md](../../docs/ghidra/functions/vita-2048-eu-v104/pickup-backdrop.md).
2. **HD's fade-out and icon gate, and Omega's rate, are inherited, not read**: HD's hold threshold, icon
   gate and the after-use branch of `0x0008df30` (second half of the decompile), whether HD folds the
   backdrop's size, and Omega's `FUN_0125e9b0` rate. A live RPCS3 trace would settle HD; shadPS4 has no
   memory readout.
