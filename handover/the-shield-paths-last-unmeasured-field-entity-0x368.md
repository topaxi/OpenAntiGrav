# The shield path's last unmeasured field: `entity + 0x368`

2026-08-10. Everything else about the pool now has a runtime leg, including the coefficient (`amount / |p| = 0.035000` on 25 of 25 calls) - see [shield.md](../docs/ghidra/functions/psp-pulse-usa/shield.md). What is left: `entity + 0x368` gates the whole damage path, which touches the pool only when it is 0 or 2, and several sites test `< 0` explicitly so `-1` is a real value. "Local human player slot" fits every site read and would imply AI craft take no damage through this function, which is almost certainly wrong. Confidence 60; the field keeps its offset rather than a name until someone finds the writer. **Two probe facts worth keeping** for whoever works this path: the contact ring is intact at a `Ship_Damage` breakpoint and drained at a `Ship_UpdateCraft` one, and `Ship_Damage` takes its entity in **a0** with the float amount in **f12** - the leading float does not reserve `a0` the way o32 would.

## Open

- `entity + 0x368`'s meaning is unmeasured; it gates the damage path (0 or 2) and some sites test `< 0`, so `-1` is a real value.
- The "local human player slot" hypothesis fits every site read but would imply AI craft take no damage through this function - almost certainly wrong.
- Confidence is only 60; the field has no name until the writer is found.

## Next Steps

- Find the writer of `entity + 0x368` to resolve what it means and name the field.
- Use the two probe facts (contact ring intact at a `Ship_Damage` breakpoint, drained at `Ship_UpdateCraft`; entity in `a0`, float amount in `f12`) as breakpoint starting points.
