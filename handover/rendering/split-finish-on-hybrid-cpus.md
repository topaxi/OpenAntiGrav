---
status: measure
titles: [all]
updated: 2026-10-27
summary: "Split finish now only on uniform cores (S24 flyby -1 ms); probe reads Linux sysfs only, hybrid Windows/macOS unprobed"
categories: [rendering, tooling]
size: S (40)
difficulty: easy (40)
---

# The split encoder finish on hybrid CPUs

The S24's reported CPU row rise (about 9 to 12-13 ms, HD campaign event 1 flyby) was measured
on 2026-10-27: the threaded encoder finish adds about 1 ms of `scene` on the phone and saves no
`submit`; the rest did not reproduce. The split is now on only where every core reports the same
`cpu_capacity` (`oag_gpu::split_finish::wanted`). Numbers: `docs/rendering/frame-thread-cpu.md`,
"The maintainer's scene: the split finish costs the phone about 1 ms".

## Open

1. The probe reads Linux/Android sysfs only. Windows on an Intel hybrid part and macOS on Apple
   silicon keep the split unprobed; measure one of each (`OAG_SPLIT_FINISH=0` against `=1`, the
   `frame-phases` `scene` and `submit` phases) before deciding their default.
2. An Intel hybrid part on Linux reports `cpu_capacity` only on kernels with asymmetric capacity
   support; unreadable means the split stays on.
3. The parked indirect depth prepass (`park/indirect-depth-prepass`) slightly regressed together
   with the split (maintainer); re-measure per platform if either changes.
4. Confirm on the S24 with an installed build that the probe picks one encoder (the CPU row should
   match the `OAG_SPLIT_FINISH=0` row).

## Next Steps

1. On the S24: `just deploy android`, then the campaign route by taps, or
   `scripts/android-race-phases.sh --serial R3CX707JA9T --out run.log --cool 287` for the scripted
   race, with and without `-- OAG_SPLIT_FINISH=1`.
