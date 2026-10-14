# Pure (EU): the same `MPTimeTrial` collision byte (2026-10-14, `mp-collision-re`)

**Binary:** `/pure/BOOT-psp-pure-eu.BIN`, image base `0x08804000`, relocations not applied.
The reasoning and the evidence are on the USA page,
[../psp-pure-usa/mp-time-trial-collision.md](../psp-pure-usa/mp-time-trial-collision.md); this page
records that the EU pressing carries the same code at these addresses. EU keeps no path string
(its allocation tag reads `file`), so the attribution is by structure, not by name.

| Claim | Evidence | Conf |
| --- | --- | ---: |
| `0x08815c34` is the `MPTimeTrial` race manager constructor | `diff_functions` against USA `0x08815e1c`: 227 instructions each, 164 equal, every difference a register or a relocation constant; same allocation sizes (`0x1e8`, `0x6c`, `0x12f0`), same HUD flag set `0x15f`, `sb zero,0x3664(a0)` equal. `be::MPTimeTrial_RaceManager` is at `0x08a44a50` (the string sits `0x1c` before the `file` tag it passes) and the one caller `0x0880d4c8` mirrors USA's `0x0880d778` | 80 |
| It clears `world + 0x3664` | `sb zero,0x3664(a0)` at `0x08815c90`; the other writer is the world constructor `0x088222c4` (`sb s1,0x3664(s0)` at `0x0882234c`); two hits in the whole program | 80 |
| `0x08824c68` is the pair dispatch and reads the byte | `lbu a0,0x3664(s2)` at `0x08824dbc` (the only `lbu` of the offset); `diff_functions` against USA `0x08824e34`: 96 of 96 instructions equal | 80 |

| Address | Name | Conf |
| --- | --- | ---: |
| `0x08815c34` | `MPTimeTrial_Construct` | 80 |
| `0x08824c68` | `Collision_DispatchPair` | 80 |

No port change; see the USA page.
