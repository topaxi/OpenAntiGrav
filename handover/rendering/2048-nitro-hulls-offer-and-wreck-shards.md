---
status: measure
titles: [2048, omega]
updated: 2026-11-13
summary: "2048/Omega nitro hulls paint their diffuse; open: 2048 offer and unlock, wreck shards, iridescent overlay"
difficulty: medium (50)
---

# 2048 and Omega nitro hulls: what is open after the diffuse fix

2026-11-13. The fix, its census and its test are on
[hd-ship-materials.md](../../docs/rendering/hd-ship-materials.md) ("The nitro
hull's picture was a ramp and a mask"). The maintainer's expectation was
"2048 should have nitro skins via the Fury DLC"; the hulls are in the base
package, not `dlc2.psarc`.

## Open

- **Offer.** `oag_2048::GUEST_ROSTER.offered` is `false`: the twelve HD-derived
  teams and their `_c1`/`_n1` variants are reachable through RACE REMIX's craft
  title only, not 2048's own Ship Select and not gated on the Fury pack. What
  the original's team list does, and whether `_n1` waits behind the Fury
  campaign (`grid8`, as in HD), is unmeasured: needs an original frame from
  Vita3K or the user's recollection.
- **Wreck shards.** `ship_damage*/nitro_perspex_debris*` carry the same ramp
  and mask hashes in 2048 and Omega and are left alone (no role measured
  there); they likely draw the ramp.
- **Iridescent overlay.** `diffuse` overlaid with `ramp(-dot(V, N))` is not
  drawn on any title; the white-violet wing rims are `nitro_emissive_outlines`.

## Next Steps

1. Look at a nitro hull wreck in 2048 (`--team Feisar_n1`, destroy the craft)
   and apply `NITRO_LOOKUPS` to the debris families if it shows the ramp.
2. Ask the maintainer which Vita3K frame shows a 2048 `_n1` hull, if any.
