---
status: re-first
titles: [2048, omega]
updated: 2026-11-02
summary: "2048 and Omega craft laws are inherited, not read: the inertia mass (0.9 against HD's 1.0) and the wall point rate (2048 held world, Omega body-local) need each executable's constructor and contact resolver"
---

# 2048 and Omega craft laws: two inheritances nobody has checked

2026-11-02, from the specs mismatch pass. Both are CLAUDE.md's "A 2048 finding is checked against Omega" census, left open.
Neither is uncheckable: 2048's v1.04 executable and Omega's `eboot.bin` are in Ghidra.

## Open

- **Inertia mass.** `oag_2048::race::CRAFT_LAWS` and `oag_omega::race::CRAFT_LAWS` carry `inertia_mass`
  `inherited(0.9, "Pulse")` but `inertia_rotates` `inherited(true, "Wipeout HD")`. HD builds the box with `1.0`
  (`Body_SetBoxInertia(1.0, 12, 8, 12)`, `0x000debc8`, `docs/ghidra/functions/ps3-hdfury-eu/craft-inertia.md`).
  The project rule for an unmeasured 2048/Omega law is HD's, which would be `1.0`. Settles it: the mass literal in each title's
  ship-entity constructor. Changing the value moves simulation, so it needs its own lane and the AI gates.
- **Wall point rate.** 2048 is `chosen(false)` (a hold for this engine's AI, `docs/ghidra/functions/ps3-hdfury-eu/wall-response.md`),
  Omega `inherited(true, "Wipeout HD")`. Settles it: the `(R^T omega) x r` term of each title's contact resolver (HD's is
  `0x000f7770`). If both have it, 2048's hold is purely an AI-side choice and the doc should say so.

## Next Steps

1. Locate `Body_SetBoxInertia`'s caller in Omega's `eboot.bin` and in 2048 v1.04; record the mass literal on `craft-inertia.md`.
2. Locate each `Body_ResolveContact`; read the point-rate expression; record on `wall-response.md`.
3. Update the `CraftLaws` provenance (`measured` or `inherited`) and `docs/specs/physics.md`'s per-title table; any value change is its own gated lane.
