# Memory maps

Address space layouts for each binary: where code is loaded, where the heap
lives, where the important globals sit.

| Platform | Document | Covers |
| --- | --- | --- |
| PSP (Pulse, USA) | [psp-pulse-usa.md](psp-pulse-usa.md) | The console's regions, `BOOT.BIN` section by section, where the named globals sit, and the heap arithmetic after boot |
| PS2 (Pulse, EU) | [ps2-pulse-eu.md](ps2-pulse-eu.md) | The EE's regions, `SCES_547.48` section by section, the VU overlays, and why the PS2's allocator is not the PSP's |

The allocator itself - the front door, the small-block pools, the heap
descriptor and block format, and the two heap sizes - is on
[memory.md](../functions/psp-pulse-usa/memory.md) beside the functions it
names; the maps here are where those regions land.

## What belongs here

- Load address and section layout of the main executable
- Heap regions and how they are carved up
- Known global variables and their addresses
- Hardware-mapped regions relevant to reverse engineering, such as the PSP's
  VRAM and scratchpad

Memory maps are what make an address in a
[function page](../functions/README.md) meaningful.
