# Pure: the collider byte a destroyed craft clears (static)

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pure, PSP, USA) |
| **Subsystem** | collision, ship state machine |
| **Related** | [`../psp-pulse-usa/wreck-collision.md`](../psp-pulse-usa/wreck-collision.md) (the measured law) |

Read statically 2026-10-12 (lane `wreck-faithful`), by the same `lbu`/`sb` search on `0x68(` used on Pulse.
Pulse's box-against-box test refuses a craft pair when either collider's byte `+0x68` is zero, and
`Ship_SetState` clears it in states 6 and 7. Pure has the same shape. Not run live.

| Address | Name | What | Confidence |
| --- | --- | --- | ---: |
| `0x08824fbc` | `Collision_BoxAgainstBox` | `lbu 0x68(t0)` at `0x08824fe4` and `lbu 0x68(t2)` at `0x08824ff0`: offset `0x28` into the function, where Pulse's twin (`0x0881702c`) reads them at `0x08817054`/`0x08817060` | 80 |
| `0x08928824` | `Ship_SetState` | an 8-entry jump table; two cases store `0` through `0x68(*(*(entity+0x8b4)+0x3fc))` at `0x08928dcc` and `0x08928dec` (Pulse: cases 6 and 7, `0x088445d0`/`0x088445f8`, `entity+0x794 -> +0x3bc`); the first writes the `2.0`/`0.8` s timer, as Pulse's case 6 does | 75 |
| `0x088272f0` | collider initialiser | `sb 1, 0x68(a0)` at `0x08827314`, as Pulse's `0x08818b78` | 70 (not renamed) |

Confidence is below Pulse's because the two other cases were not mapped to state numbers and nothing ran.
