# Omega's loyalty award: HD's law, repeated in Omega's own executable

2026-10-16, lane `loyalty-earning`. Static reading of `/omega/eboot-ps4-omega-eu.bin`
(Ghidra bridge); **no live run backs it** (no PS4 emulator in the toolchain), so nothing
here scores above the rubric's 84 ceiling for "decompilation only, consistent call sites".
The reference law is HD's, `docs/ghidra/functions/ps3-hdfury-eu/endrace-loyalty.md`; this
page records that Omega carries the same code and where it differs.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x016544a0` | `Race_ComputeLoyaltyAward` | 80 |

`Race_ComputeLoyaltyAward` (`0x016544a0`, `(store, stats) -> award`) is HD's `0x00023f98`
line for line. Evidence, constant by constant:

- Mode id `DAT_01f999e4`, class masks `0x332318` (class A, lap 15 / perfect 25),
  `0x420` (B, 30 / 50), mode `14` (C, laps `* 0x96` = 150, perfect `* 0x14` = 20, no
  kill term, zone terms added), otherwise D (10 / 20); `< 0x16` guard as in HD.
- Zones `* 5`, perfect zones `* 0xf`; kills `* 0x1e` (30) in modes `8` and `0x14`, else
  `* 0xf` (15).
- Difficulty multiplier only for modes in `0x2318` (3, 4, 8, 9, 13) when
  `DAT_01f999e8 != DAT_01f998f0` (the same "an AI craft is present" test as HD's
  `g_GameState+0xe4 != *g_GameState`): tier `1` -> 3, `2` -> 4, else 2. Forced `x3` when
  `mode - 0x10 < 7`.
- The banked record is found by CRC32 (poly `0xedb88320`, the table built inline) of
  the team string at `DAT_01f999f8 + 0xb8`, in the list at `store + 0x580` (next at
  `+0x50`, key at `+0`), total at `+0x70` (`puVar7[0x1c]`), clamped to **100000**
  (`0x186a1` compare). Same cap as Pulse and HD.

**Second site (the ticker).** The Results screen constructor `FUN_0153ebf0` builds the
same reasons list the award is summed from: `RC_LAP`/`FE_STAGE` lap lines (`+0xf`,
`+0x1e`, `+0x96`, `+10` by class, class read from a 19-entry table at `0x0183e070`:
modes 3, 4 -> A, 5 -> B, 6, 7 -> D, 8, 9 -> A, 10 -> B), `ER_PLAP` (`+0x19`/`+0x32`/
`+0x14`), `ER_ELIM` (`+0x1e` or `+0xf`), `IG_HUD_ZONE` (`+5`), `ER_PZONE` (`+0xf`) and a
difficulty line (`x2`/`x3`/`x4`), then writes `loyalty1.1`/`loyalty1.2`/`loyalty2`
(`0x0182420d`..). The two functions agree on every rate, which is the "two independent
sites" leg. Omega's Results screen therefore **does** show and keep loyalty; the
older note in `omega-status.md` that this build keeps none on Omega rested on the
screen's `LOYALTY` field being empty in our own build, not on the executable.

## Where Omega differs from HD (confidence 65, the object is unnamed)

A second object `DAT_01e31910` has a field at `+0x21c`. When it is `1` or `4` the kill
rate is 30 (as in modes 8/0x14), and in Detonator the kill term is then paid. The ticker
also forces `x3` when a byte at `+0x22c` has bit 2 set, which the award function does not
read. This project has no equivalent object, so neither is modelled; both read as a rule
set of Omega's newer modes (the "Zone Battle"/"Eradication" family), not verified.

## Cross-check against 2048 and HD

- **HD**: ported, identical (HD is the source). `oag_hd::loyalty::award` pays Omega's races
  since 2026-10-16 (`Title::loyalty`, `LoyaltyLaw::Hd`, `Origin::Measured`).
- **2048**: ported as inherited (no loyalty of its own, so it runs this law). Its executable otherwise differs: 2048's executable has no `ER_ELIM`/`ER_PZONE`/`ER_LOY`
  string and its end-of-race page shows XP instead; see
  `docs/ghidra/functions/vita-2048-eu-v104/loyalty-absent.md`. The Omega code is the
  HD lineage, not 2048's, even where the assets are 2048's.
