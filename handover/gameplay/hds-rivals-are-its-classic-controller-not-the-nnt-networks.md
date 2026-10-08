# HD's rivals are its classic controller, not the `.nnt` networks

2026-10-08, `hd-ai-nnt` lane. The 24 `nnet_*.nnt` / `controlprm_*.txt` files are decoded and
proved dormant in the original: [hd-ai-netconfigs.md](../../docs/formats/hd-ai-netconfigs.md)
(format, invariant) and [ai-net.md](../../docs/ghidra/functions/ps3-hdfury-eu/ai-net.md)
(executable, live RPCS3 read, 14 names). Do not requote from this file.

The upshot for the maintainer's "HD's AI driving line is unknown": the gap is real, but it is
not the networks. HD's opponents drive a classic controller tuned by `AIControlStats.xml` and
`AIRaceStats_<class>.xml` (`DATA02.PSARC`, with a second `airacestats_*.xml` set in
`DATA03.PSARC`), and that controller is what is unread.

2026-10-08, `hd-ai-classic` lane: the tables and the thrust law are read,
[ps3-hdfury-eu/ai-stats.md](../../docs/ghidra/functions/ps3-hdfury-eu/ai-stats.md). HD's
`AIControlStats.xml`/`AIRaceStats_<class>.xml` are Pulse's value for value (so are 2048's and
Omega's), the parser is Pulse's plus `SplitScreenMultiplier`, and `AI_ComputeOpponentThrust`
(`0x000fdf00`) is Pulse's with four changes (finished spread 75, finished wander `8 - place`,
Eliminator drops `WhenBehind`, multi-human reference craft). `oag-ai` ports none of that law for any
title, so nothing changed in code; `crates/tables/tests/ai_stats_lineage_ground_truth.rs` pins
the identity.

## Open

- **The steering law inside `AI_ComputeControls` (`0x00100658`)** is unread past its inputs
  (`LookAheadSecs`, `SteerMul`, `SteerDamp`, `xtrackMul`, header constants). Next address: the call
  to `0x000fe688` at `0x100954`/`0x100a04`. Pulse's own consumer is not identified either.
- **2048 and Omega: checked, applies, not wired.** Same tables as Pulse's (the lineage test); their
  executables were not read for the thrust or steering law.
- **What the byte at `0x9384e1` is**: it gates both the Duel override and the Eliminator change.
- **No live leg**: every row on the page is static. A Racebox race under RPCS3 reading
  `ai+0x170`/`+0x174` and the thrust out of `0xfdf00` would raise `AI_UpdateCraft` and the law.
- **The driver's second net at `+0x298`** (from the `hd-ai-nnt` lane) is cleared by
  `AiNetDriver_Init` and read by nothing found; its live pointers were not recorded.
- ~~Census HD's XML against Pulse's schema~~ and ~~follow `0x103d14..0x104268` for the
  thrust~~: done, above.

## Next Steps

1. Capstone `0x00100658..0x100e60` and `0x000fe688` for the steering law (an afternoon); then
   compare with Pulse's, which first needs Pulse's consumer found (start from the strings at
   `0x08a7ac70`'s neighbours, per `psp-pulse-usa/ai-stats.md`).
2. Only once a law is read: whether `oag-ai` should take the disc's `Controller` values on every
   title is the maintainer's call, and it moves every `ai_*` ground truth.
3. Not recommended: wiring the networks. The original never runs them, so a port would be
   behaviour the original does not have. If it is ever wanted anyway, the activation is
   `s / (sqrt(s*s + 1) + 1)` (`sqrt` only, deterministic under the project's rules), the inputs
   are on [ai-net.md](../../docs/ghidra/functions/ps3-hdfury-eu/ai-net.md) at confidence 65, and
   the result is chosen, not measured.
