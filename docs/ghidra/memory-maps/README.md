# Memory maps

> **Not yet started.** Milestone M2.

Address space layouts for each binary: where code is loaded, where the heap
lives, where the important globals sit.

| Platform | Document |
| --- | --- |
| PSP | _(pending)_ |
| PS2 | _(pending)_ |

## What belongs here

- Load address and section layout of the main executable
- Heap regions and how they are carved up
- Known global variables and their addresses
- Hardware-mapped regions relevant to reverse engineering, such as the PSP's
  VRAM and scratchpad

Memory maps are what make an address in a
[function page](../functions/README.md) meaningful. Until these exist, an
address is just a number.
