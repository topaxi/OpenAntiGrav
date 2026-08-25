# The barrel roll is identified and unimplemented

A headline Pulse mechanic we do not have. The tap-history chain is read link by link at confidence 80 in [`input-bindings.md`](../docs/ghidra/functions/psp-pulse-usa/input-bindings.md#the-tap-history-path-is-the-barrel-roll). A follow-up needs two unread things: `FUN_08840770`'s energy cost, and what consumes `craft+0x1c0 & 0x400` - presumably the visual roll, which would decide whether the phase is an angle. `DAT_08b36bf0`, the ramp rate, is `.bss` and needs a live read.

## Open

- The barrel roll is a headline Pulse mechanic this project does not have implemented
- `FUN_08840770`'s energy cost is unread
- What consumes `craft+0x1c0 & 0x400` is unread - presumably the visual roll, which would decide whether the phase is an angle
- `DAT_08b36bf0`, the ramp rate, is `.bss` and has not been read live

## Next Steps

- Read `FUN_08840770`'s energy cost
- Determine what consumes `craft+0x1c0 & 0x400`
- Take a live read of `DAT_08b36bf0` (the ramp rate)
