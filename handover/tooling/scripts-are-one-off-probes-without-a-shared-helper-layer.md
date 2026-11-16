---
status: ready-to-wire
titles: [all]
updated: 2026-11-02
summary: "scripts/ holds 347 files, 291 Python; 144 psp/rpcs3 probes, 27 redefine u32 helpers, 6 are cited nowhere; the justfile is 1,351 lines"
size: M (30)
difficulty: easy (40)
kind: debt
---

# `scripts/` is one-off probes without a shared helper layer

2026-11-02, `debt-survey`. `ls scripts | wc -l` is 347 (291 `.py`). By prefix: `psp-` 83, `rpcs3-` 61,
`hd-` 34, `check-` 18, `compare` 14, `pcsx2-` 10. Most are single-question capture or probe scripts
written for one finding. Shared modules exist (`emu_guard.py`, `rpcs3_debugger.py`, `ppsspp_debugger.py`,
`pcsx2_pine.py`; 135 scripts import one of a handful), but:

- 27 scripts define their own `def u32`/`u16`/`rd32`-style helper (`rg '^def (u32|u16|rd32|read_u32|rd|r32|u8|f32)\b' scripts`),
  for example `hd-flare-tuning-dump.py`, `rpcs3-hd-trail-colours.py`, `rpcs3-hd-sound-tick.py`.
- 90 files set their own `REPO`/`ROOT`.
- Six are cited by no other file in the repo (`justfile`, `docs/`, `handover/`, `crates/`, other scripts):
  `hd-pose-render.py`, `hd-tone-stats.py`, `hd-zone-stage-source-break.py`, `psp-check-sap-clamp.py`,
  `rpcs3-hd-glow-callers.py`, `rpcs3-hd-zoom-frames.py` (last touched 2026-09-07 to 2026-10-15).
- The `justfile` is 1,351 lines with 147 recipes.

## Why it hurts

A contributor looking for "how do I read the PSP RAM" finds 83 candidates with no index; a fix to a
memory-reading quirk lands in one copy of 27. The unreferenced scripts cost reading time and are
possibly stale against current memory maps.

## Open

- Whether the unreferenced six still run. Each needs a hand check: cite it from its evidence page or delete.
- Which probe scripts are evidence-bearing (cited from `docs/`) and must stay, as opposed to throwaway.
- Whether 147 recipes want a `just --list` grouping; unmeasured.

## Next Steps

1. Add `scripts/oag_mem.py` (`u32`, `u16`, `f32`, repo root) and switch the 27 duplicates; no behaviour change.
2. Hand-check the six unreferenced scripts: delete or cite them.
3. Add a one-screen `scripts/README.md` grouping by purpose (capture, probe, gate, build, ghidra).

Verify: `python3 -I scripts/check-just-args.py`, `just check-docs`, run one converted script with `--help`.
