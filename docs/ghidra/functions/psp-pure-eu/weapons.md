# `/pure/BOOT-psp-pure-eu.BIN` - the weapon table, Disruptor and Bomb, transferred from `psp-pure-usa`

The functions [`psp-pure-usa/weapons.md`](../psp-pure-usa/weapons.md) reads,
located on the EU pressing (UCES-00001). **The reading is that page's; this
one only establishes that the same function sits at the EU address.** Method
and the one exception to it are below; applied rows are in
[`names.tsv`](names.tsv).

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pure EU), image base `0x08804000` |
| **Related** | [`psp-pure-usa/weapons.md`](../psp-pure-usa/weapons.md) (the evidence), [`../psp-pulse-eu/exact-hash-transfer.md`](../psp-pulse-eu/exact-hash-transfer.md) (the transfer discipline this follows, where it can) |

## Method

The exact-hash pass [`psp-pulse-eu/exact-hash-transfer.md`](../psp-pulse-eu/exact-hash-transfer.md)
ran matches **three** of the forty-one functions here (`WeaponPickup_ArmDisruptor`,
`Disruptor_RollEffect`, `Disruptor_Update`); every other body differs by
exactly the `lui`/`addiu` immediates that build a global's or a string's
address, which the normalised hash does not normalise away and which are
different on a pressing whose data segment sits `0x740` lower (EU
`WeaponStats_Table` is `0x08b11060` against USA `0x08b177a0`). So the pass
here is three mechanical checks per function rather than one:

1. **The anchor.** Each subsystem's entry points were found on EU by the same
   unique strings that found them on USA: `Disruptor` (the class name, at
   `0x08a44000`) for the two parsers, `WO_DISRUPTOR_LAUNCH` (`0x08a46c74`),
   `DISRUPTOREXPSHP` (`0x08a46ca0`), `WO_DISRUPTOR_HEAD` (`0x08a47048`),
   `WO_BOMB_GLOW` (`0x08a46f90`) and `BOMBEXPL` (`0x08a46c24`), each with a
   single referencing function.
2. **The shift.** Every function in one subsystem sits at one constant
   offset from its USA twin - `-0x1c0` for the whole parser block, `-0x220`
   for the Disruptor pool and its eight `Prime*` stubs, `-0x21c` for the Bomb
   pool, `-0x228` for `Disruptor_Update` and `Disruptor_SpeedForClass` - and
   the anchored entry points land exactly where the shift predicts.
3. **The diff.** `diff_functions` USA against EU on every pair: identical
   instruction count and identical body except for pairs of relocated
   immediates (equal `body_added` and `body_removed`, `prologue_changed` and
   `epilogue_changed` both false, no string difference). The eight `Prime*`
   stubs were also decompiled on EU and read back the kind constant their
   USA twin writes (`0` at `0x0884f8f8`, `7` at `0x0884f9d0`) and the table
   offsets the USA page tabulates (`Stall.time` at EU table `+0x58`,
   `Rubber Ship.amount`/`.time` at `+0x80`/`+0x84`).

**The exception**: `Disruptor_Init` and `Bomb_Init` differ by one and two
instructions respectively (EU 161 against USA 160, 188 against 186), with
about a quarter of each body reported changed - the inlined `LinkObj`
allocation sequence, by the position of the differences. Both are still
anchored by their unique strings on EU, and both are transferred at **USA
minus 10** rather than minus 5 for it.

The three ship-side functions on the USA page (`Ship_UpdateWeapons`,
`Ship_UpdateEngine`, `Ship_ApplySteeringTorque`) are **not** transferred: no
EU function of matching size sits within `0x400` of the USA address, so the
shift there is not a constant and nothing was read on EU to place them.

## Confidence

**EU confidence = USA confidence minus 5**, the same chosen-not-derived
discount the Pulse pass applies, and minus 10 for the two `_Init`s above.

| EU address | Name | Confidence | USA address |
| --- | --- | --- | --- |
| `0x08808604` | `WeaponStats_Load` | 75 | `0x088087c4` |
| `0x08809518` | `WeaponStats_Parse` | 79 | `0x088096d8` |
| `0x08808620` | `WeaponStats_ParseRocket` | 79 | `0x088087e0` |
| `0x0880880c` | `WeaponStats_ParseMissile` | 79 | `0x088089cc` |
| `0x08808a24` | `WeaponStats_ParseQuake` | 79 | `0x08808be4` |
| `0x08809bdc` | `WeaponStats_ParseDisruptor` | 79 | `0x08809d9c` |
| `0x08808b84` | `WeaponStats_ParseTurbo` | 79 | `0x08808d44` |
| `0x08808c84` | `WeaponStats_ParseShield` | 79 | `0x08808e44` |
| `0x08808d84` | `WeaponStats_ParseAutopilot` | 79 | `0x08808f44` |
| `0x08808e84` | `WeaponStats_ParsePlasma` | 79 | `0x08809044` |
| `0x08809070` | `WeaponStats_ParseBomb` | 79 | `0x08809230` |
| `0x0880925c` | `WeaponStats_ParseMine` | 79 | `0x0880941c` |
| `0x08809448` | `WeaponStats_ParseGlobal` | 79 | `0x08809608` |
| `0x0880a340` | `WeaponStats_ParsePickupOdds` | 79 | `0x0880a500` |
| `0x08b11060` | `WeaponStats_Table` (data) | 79 | `0x08b177a0` |
| `0x0884cf84` | `WeaponPickup_Grant` | 75 | `0x0884d19c` |
| `0x0884f7cc` | `WeaponPickup_ArmDisruptor` | 75 | `0x0884f9ec` |
| `0x0884f7f0` | `Disruptor_RollEffect` | 77 | `0x0884fa10` |
| `0x0884f8f8` | `Disruptor_PrimeStall` | 75 | `0x0884fb18` |
| `0x0884f910` | `Disruptor_PrimeMirror` | 75 | `0x0884fb30` |
| `0x0884f92c` | `Disruptor_PrimeNoAirbrakes` | 75 | `0x0884fb4c` |
| `0x0884f948` | `Disruptor_PrimeAutopilotSlow` | 75 | `0x0884fb68` |
| `0x0884f96c` | `Disruptor_PrimeAutopilotFast` | 75 | `0x0884fb8c` |
| `0x0884f990` | `Disruptor_PrimeDrunk` | 75 | `0x0884fbb0` |
| `0x0884f9ac` | `Disruptor_PrimeDrunkCamera` | 75 | `0x0884fbcc` |
| `0x0884f9d0` | `Disruptor_PrimeRubberShip` | 75 | `0x0884fbf0` |
| `0x0884f9f4` | `DisruptorPool_Fire` | 77 | `0x0884fc14` |
| `0x08858de4` | `Disruptor_Init` | 74 | `0x08859010` |
| `0x088590e4` | `Disruptor_Update` | 77 | `0x0885930c` |
| `0x088590b0` | `Disruptor_SpeedForClass` | 77 | `0x088592d8` |
| `0x088504a4` | `DisruptorPool_Update` | 77 | `0x088506c4` |
| `0x08850054` | `Disruptor_TestHit` | 75 | `0x08850274` |
| `0x08850ca8` | `Disruptor_ApplyEffect` | 79 | `0x08850ec8` |
| `0x08857ea4` | `Bomb_Init` | 74 | `0x088580d8` |
| `0x088581ac` | `Bomb_UpdateSpin` | 73 | `0x088583d8` |
| `0x0884e74c` | `BombPool_Update` | 75 | `0x0884e968` |
| `0x0884ed5c` | `Bomb_UpdateTrigger` | 77 | `0x0884ef78` |
| `0x0884f204` | `Bomb_ApplyHit` | 77 | `0x0884f420` |
| `0x0884eff8` | `BombPool_Detonate` | 77 | `0x0884f214` |

`WeaponStats_Table`'s EU address is read off `Disruptor_PrimeStall`'s
decompile (`DAT_08b110b8` is `Stall.time`, `+0x58`) and cross-checked against
`Disruptor_SpeedForClass` (`DAT_08b110b4` is `speed`, `+0x54`).
