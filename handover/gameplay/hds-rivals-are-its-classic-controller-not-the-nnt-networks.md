# HD's rivals are its classic controller, not the `.nnt` networks

2026-10-08, `hd-ai-nnt` lane. The 24 `nnet_*.nnt` / `controlprm_*.txt` files are decoded and
proved dormant in the original: [hd-ai-netconfigs.md](../../docs/formats/hd-ai-netconfigs.md)
(format, invariant) and [ai-net.md](../../docs/ghidra/functions/ps3-hdfury-eu/ai-net.md)
(executable, live RPCS3 read, 14 names). Do not requote from this file.

The upshot for the maintainer's "HD's AI driving line is unknown": the gap is real, but it is
not the networks. HD's opponents drive a classic controller tuned by `AIControlStats.xml` and
`AIRaceStats_<class>.xml` (`DATA02.PSARC`, with a second `airacestats_*.xml` set in
`DATA03.PSARC`), and that controller is what is unread.

## Open

- **HD's classic controller is unread.** Its config strings sit at `0x780c31` in `EBOOT.elf`
  with `LookAhead`, `RubberBanding`, `PosBalancing`, `SkillScale`, `RaceBalancing` beside them.
  Whether HD's XML is Pulse's schema (already parsed, [ai.md](../../docs/gameplay/ai.md)) with
  other numbers, or a different one, is not checked.
- **What feeds `AiNetDriver_GetControls`'s classic hand-through.** `0x0010b6e0` stores three
  values from the AI update (`0x103d2c`), but live they read `0.0` on six of eight drivers 16 s
  into a race, and the caller at `0x104254..0x104264` was not followed. That AI update
  (`0x102e00..0x104268`) is the classic controller's likely home.
- **Only one mode was sampled live** (Racebox single race). The static case does not depend on
  mode, but a Campaign or Zone sample would close it.
- **The driver's second net at `+0x298`** is cleared by `AiNetDriver_Init` and read by nothing
  found; its live pointers were not recorded.

## Next Steps

1. Census HD's `aicontrolstats.xml` and `airacestats_<class>.xml` against Pulse's schema in
   `oag_tables` (a reader run, about 30 minutes). If it parses, HD's numbers can feed `oag-ai`
   through `Title` data the way Pulse's do.
2. Follow `0x103d14..0x104268` with capstone to find what reads `LookAhead` and turns it into
   steer, thrust and airbrake (an afternoon; Ghidra halts at the AltiVec).
3. Not recommended: wiring the networks. The original never runs them, so a port would be
   behaviour the original does not have. If it is ever wanted anyway, the activation is
   `s / (sqrt(s*s + 1) + 1)` (`sqrt` only, deterministic under the project's rules), the inputs
   are on [ai-net.md](../../docs/ghidra/functions/ps3-hdfury-eu/ai-net.md) at confidence 65, and
   the result is chosen, not measured.
