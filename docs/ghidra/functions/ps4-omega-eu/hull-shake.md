# Omega's drawn-hull shake is HD's

`/omega/eboot-ps4-omega-eu.bin` (base v1.00), read from the decompile 2026-10-26, lane `2048-queens-offset`. The law and its
constants are on [`ps3-hdfury-eu/hull-shake.md`](../ps3-hdfury-eu/hull-shake.md); the 2048 counterpart, with what was
checked live there, is [`vita-2048-eu-v104/hull-shake.md`](../vita-2048-eu-v104/hull-shake.md).

| Address | Name | Confidence | Evidence |
| --- | --- | --- | --- |
| `0x012f7200` | `HullShake_Update` | 80 | `(dt, f2, obj)` in SSE/AVX: `0.99 * obj[8] + 0.01 * f2`, `clamp((80 - s) * 0.02, 0, 1)`, a uniform cubic B-spline, a 60-slot ring (`imul 0x88888889`, `% 0x3c`) with history at `obj+0x90` and its index at `obj+0x450` (HD's own layout), the `0.025`, `-0.01`, `0.025` angle terms, `5.0 * (p - q)` and `0.8 * s`; new points `(4.0, 1.8, 1.8) * (rand() * 2^-30 - 0.5)` (`3.7252903e-09` is `4 * 2^-30`, `1.6763806e-09` is `1.8 * 2^-30`). The active byte is written at `obj+0x454` |

Found from the ship constructor `0x01309e90`'s callee list and the `0x88888889` divide, with the constructor reached from the string
`"%s internal tripod"` (`0x0181533b`). Six callers: `0x0130ebdf`, `0x012ec8b8`, `0x0130a16f` (the constructor), `0x014923f4`, `0x0157e1a7`,
`0x01764c81`. Nothing was run; the `Apply` function and the running build (patch v1.07, not in Ghidra) were not read.
