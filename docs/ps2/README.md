# PS2

Findings specific to the PlayStation 2 release. Pulse on PS2 was Europe-only.

| Document | Covers |
| --- | --- |
| [Pulse disc layout](pulse-disc-layout.md) | Contents of the Pulse DVD, with findings |

## Key facts

- **`SCES_547.48` is a plain ELF** for the Emotion Engine. No decryption needed.
- `SYSTEM.CNF`'s `BOOT2` line names the boot executable, which is where the
  serial comes from.
- Nine `IOP/*.IRX` modules handle audio, storage and controllers on the I/O
  processor. `SCREAM.IRX` is Sony's audio engine, which means the audio format
  is likely a documented Sony bank format.
- **The disc image is a DVD packed with `chdman createcd`.** The container
  reports CD units; the sectors inside are ordinary 2048-byte user data. See
  [the disc layout](pulse-disc-layout.md#the-container-is-a-dvd-packed-as-a-cd).
- 76% of the disc is a single padding file.

## Related

- [PSP vs PS2 comparison](../comparisons/pulse-psp-vs-ps2.md), and the region
  confound that qualifies all of it.
