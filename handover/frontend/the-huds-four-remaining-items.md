# The HUD's four remaining items

The layout was never an RE problem - Pulse ships five layouts as `Data\XML\*_HUD.xml`. What is left is on [hud.md](../../docs/ui/hud.md).

## Open

- **The mode-code string-key substitution rule is read for one widget pair, not
  generalised.** `TimeTrial_HUD.xml`'s `TotalTimeTxt` (`idstring="IG_HUD_TOTAL"`)
  reads `IG_HUD_RECORD` on a live frame because `Hud_UpdateTimeCluster_q`
  (`0x0881c9d0`) picks its caption key from a five-way table keyed on an ordinal
  "tier" (`IG_HUD_TOTAL` default; `0`-`3` map to `BRONZE`/`SILVER`/`GOLD`/`RECORD`;
  `5` leaves the caption alone). Confidence 84 for the table itself - see
  [hud-time-caption-substitution.md](../../docs/ghidra/functions/psp-pulse-usa/hud-time-caption-substitution.md).
  **Not settled:** what produces the tier value (confidence 50, not renamed), and
  the other 25 of the binary's 38 `IG_HUD_*` keys with no widget in any shipped
  layout - only this one pair was chased.
- 25 of the binary's 38 `IG_HUD_*` keys still appear in no layout and have no
  read mechanism (down from 26, now that `IG_HUD_RECORD` is accounted for).
- Zone, Eliminator, `<Mode3D>` and text outlines are scoped out (details on `hud.md`)

## Next Steps

- **Implementation is blocked on RE, not ready to wire.** Wiring
  `IG_HUD_BRONZE`/`SILVER`/`GOLD`/`RECORD` into `oag_game::hud::draw::caption`
  without a real source for the tier value would invent the medal/record
  evaluation this thread did not recover - the stand-in this project's rules
  forbid. It waits on lap-timing and medal-progression data
  ([hud.md](../../docs/ui/hud.md#lap-counting-was-the-one-real-blocker)'s existing
  blocker, and its "Medal targets" deferred item).
- If picked up again: `Hud_UpdateTimeCluster_q` has no confirmed caller (see its
  evidence page) - reads on `param_1 + 0x30`/`+ 0x34`/`+ 0x5a` and on what widgets
  land at `+ 0x200`/`+ 0x204` are the next things to chase. **The PSP relocation
  patch landed 2026-09-07** (`HANDOVER.md`, "Traps that are live") - `get_xrefs_to`
  and `get_function_callers` should no longer be invisible on `psp-pulse-usa`,
  so this is unblocked and not yet retried.
