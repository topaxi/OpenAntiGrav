# Omega's Eliminator: HD's Fury Eliminator on its own executable

The score law, the L1 flip and the weapon pads' colours, each read on Omega rather than carried
over from HD. HD's law and evidence are on
[ps3-hdfury-eu/eliminator-score.md](../ps3-hdfury-eu/eliminator-score.md) and
[eliminator-flip.md](../ps3-hdfury-eu/eliminator-flip.md).

**Every address on this page is in the 1.07 patch's `eboot.bin`**
(`/omega/eboot-ps4-omega-eu-v107.bin`), not in the base executable the rest of this directory and
`names.tsv` key ([patch-v107-addresses.md](patch-v107-addresses.md)). No function here is renamed.
Static reading only, 2026-11-25.

## The score

The same awards as HD, into the pilot's score at `+0x7c98`.

| Award | Where | Law | Confidence |
| --- | --- | --- | ---: |
| Damage | `FUN_01309ad0` | mode `DAT_0201280c == 8`, damage source `2`, attacker not the victim: `score += scale * damage * 0.1` | 85 |
| Kill | `FUN_01309ad0` | kills `+0x7c94` up by one and `score += 20 * scale` | 85 |
| Lap | `FUN_0133bce0` | `score += 5`, or `5 * scale` for an opponent | 85 |
| Death | `FUN_01310650`, jump table `0x0186f228` to `0x01310cca` | `score -= min(score, 5)` (`5` at `0x0185fdc8`), unscaled | 85 |

`scale` is `1` for the person. An opponent (role `+0x6dfc == 2`) takes one off the difficulty
table at `DAT_01a78e98 + 0x1926c + difficulty * 4`, which `FUN_0134c5a0` fills from the
`<EliminatorAIStats>` row (`+0xe4`/`+0xe8`/`+0xec`). Omega's `weaponaistats.xml` authors no such
row, so every opponent scales at `1.0`.

The HUD's target line is `"%s  ( %d )"` (`FUN_0126b030`), HD's two-space form.

The Racebox slider is Omega's own: `<Slider name="Eliminations">` in `DATA09`'s
`racebox_definition.xml`, `50` to `600` by `25`. HD's is `200` to `600` by `20`.

## The L1 flip

`FUN_012f1c50` reads `<Misc eliminator_flip_time>` into `handling+0x4b4` and
`race_eliminator_flip` into the byte at `+0x4b8`; the constructor `FUN_014aba00` defaults the time
to `0.2`. The player's path is in `FUN_01302b50`: the turn rate is `pi` (`0x0185fc18`) over
`handling+0x4b4` (`0x013035cd`). Confidence 80.

The gate (`0x0130342f`-`0x01303496`) passes in mode `8` (Elimination) or `0x14` (Multiplayer
Elimination). Otherwise it reads the event object `DAT_01ea7bc0` (`+0x14c` its mode name, from the
table at `0x0198f890`, constructed by `FUN_015cb2e0`): `+0x21c` in `{1, 4}` passes, and `+0x21c ==
2` passes where the craft's `race_eliminator_flip` is set. **Open:** what writes `+0x21c`; ours
flips only in an Eliminator.

The opponents' path is `FUN_012d9190` (divide at `0x012dd8fd`, role `2` required). Not wired, as
on HD.

## The weapon pads

Colour is by the pad's node name, in every mode, and carries no Eliminator test.
[pads.md](../../../rendering/pads.md), "2048 and Omega", has the tables and the shader.

| What | Where |
| --- | --- |
| Update: the key position advances `3` a second, scale `1/255` (`DAT_02198ab0`), cooling `(0.025, 0, 0.01)` (`DAT_02198aa0`) | `FUN_0177bf90` |
| Init: a name holding `offensive` takes yellow, else `defensive` takes green, else red; `elim_` sets `+0x1ac` | `FUN_0177c3b0` |
| Constructor: the start key is the object's address mod `6` | `FUN_0177c170` |
| Static init | `FUN_0177c4a0`, vtable `0x019957b8` |
| Tables of six RGB keys: red `0x018a47c0`, yellow `0x018a4810`, green `0x018a4860` | data |

## What ours does

`oag_omega::TITLE` states the score rule, the flip and the three cycles as measured.
`crates/raceplay/tests/omega_eliminator_ground_truth.rs` guards the score law, the target of `50`,
the HUD's header and rows on both layouts, the pad cycles and a prototype craft's `0.5 s` turn.

A 2048-era circuit reads Omega's `2048_hud` layout, which authors `SCORE`, an empty `Score` value
and no target line. Ours fills it by 2048's law (**inherited from 2048**,
[eliminator-score.md](../ps3-hdfury-eu/eliminator-score.md), "Other titles").

**Open:**

- Omega's own writer of the 2048-era `Score` and rows is unread.
- The event object's `+0x21c`, the second way into the flip.
