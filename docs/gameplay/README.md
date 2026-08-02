# Gameplay

> **Started.** Milestone M5. The three single-ship modes run; everything that
> needs a grid, opponents or weapons does not.

## Pages

- [race modes](race-modes.md) - time trial, speed lap and Zone: what each one
  does, what is recovered and what is approximated.
- [lap counting](lap-counting.md) - how a lap is decided, and why that is **our
  convention** rather than a recovery.

## Scope

- [x] Laps and finishing conditions, for the single-ship modes
- [x] Game modes: time trial, Zone, speed lap
- [ ] Race rules: grid order, positions
- [ ] Game modes: single race, tournament, Eliminator
- [ ] Speed classes and how they scale handling and top speed
- [ ] Teams and their stat differences
- [ ] Progression and unlocks
- [ ] Shield energy, absorb, and the risk/reward loop around holding a weapon

Shield is the one to watch: the pool exists on the ship and Zone's perfect-zone
recharge writes it, but **nothing depletes it**, so it reads full all race. See
[race modes](race-modes.md).

## Prerequisites

Requires [binary understanding](../reverse-engineering/methodology.md) (M2) and
the [verification harness](../reverse-engineering/verification-protocol.md) (M3).
Rules are cheap to guess at and expensive to guess wrong, so they get read from
the binary rather than inferred from play.
