# Pure's dev/pub hold duration, and how the original picks a regional cut

The two things still unread after `Developer Publisher Screen` was traced to the dev/pub reel (see [pure-boot.md](../docs/architecture/pure-boot.md)). **(1) The hold duration.** `144`, `231` and `260` are three `li` immediates in Pure's own `BOOT.BIN` - the only such site in 3.6 MB, at VA `0894b70c` under PPSSPP's `0x08804000` base - each starting an identical pause-and-reload block. The duration itself is *not* an immediate: the block loads a float from `*(base+0x53d68) + 0x44`, so `oag_pulse::frontend::HOLD_SECONDS = 2.0` remains Pulse's measurement, imported. Two ways to settle it - read that global, or re-film the screen at **0.1 s** sampling and count consecutive identical frames on the first plateau (about 22 for a 2 s hold, at most 2 for none). Also unconfirmed: that `0894b70c` is reached *from* the `Developer Publisher` screen-type handler - it is the only candidate, which is strong, but it is inference rather than a traced call path. **(2) Region selection.** All four cuts are now named; what the original selects between them on has not been read out of any binary.

## Open

- The hold duration is a float loaded at runtime, not an immediate - `oag_pulse::frontend::HOLD_SECONDS = 2.0` is Pulse's measurement, imported rather than confirmed for Pure
- That `0894b70c` is reached from the `Developer Publisher` screen-type handler is inference, not a traced call path
- What the original selects between the four regional cuts on has not been read out of any binary

## Next Steps

- Read the global float at `*(base+0x53d68)+0x44`, or re-film the screen at 0.1 s sampling and count consecutive identical frames on the first plateau (about 22 for a 2 s hold, at most 2 for none)
