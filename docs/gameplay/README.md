# Gameplay

> **Started.** Milestone M5. Four modes run, a `Weapon Pad` hands out a pickup,
> and a single race fields seven opponents that drive. What they still lack is
> the skill work, laps of their own, weapons and liveries - see [ai](ai.md).

## Pages

- [race modes](race-modes.md) - time trial, speed lap, Zone and single race:
  what each one does, what is recovered and what is approximated.
- [pickups](pickups.md) - what a `Weapon Pad` hands out and what a craft does
  with it. Read its first table before anything else: the machinery around the
  grant is recovered in detail and **the grant itself is ours**.
- [lap counting](lap-counting.md) - how a lap is decided, and why that is **our
  convention** rather than a recovery.
- [opponent AI](ai.md) - what drives the other seven craft in Pulse and Pure,
  recovered from their shipped tuning data, and **why this project keeps the
  steering and throws away the speed schedule**. Read it before touching
  rubberbanding: the original spreads the player coupling across *two* blocks
  and only one of them is called `RubberBanding`.

## Scope

- [x] Laps and finishing conditions, for the single-ship modes
- [x] Game modes: time trial, Zone, speed lap
- [~] Race rules: grid order is recovered and eight craft are **driven**; race
      *positions* are not - `oag_race::RaceState` counts the player's laps only,
      so nobody is placed
- [~] Game modes: **single race** fields a driven grid; tournament and
      Eliminator do not exist
- [~] Weapons: a pad hands out a pickup and Turbo has an effect; the other
      twelve do not
- [ ] Speed classes and how they scale handling and top speed
- [ ] Teams and their stat differences
- [ ] Progression and unlocks
- [ ] Shield energy, absorb, and the risk/reward loop around holding a weapon

The energy pool is live: wall contact spends it, Zone's perfect-zone recharge
and absorbing a pickup pay it back, and the HUD's `ShieldBar` follows. What is
missing from that loop is the *risk* half - no weapon damages anybody yet, so
holding a pickup rather than absorbing it costs nothing. See
[pickups](pickups.md).

## Prerequisites

Requires [binary understanding](../reverse-engineering/methodology.md) (M2) and
the [verification harness](../reverse-engineering/verification-protocol.md) (M3).
Rules are cheap to guess at and expensive to guess wrong, so they get read from
the binary rather than inferred from play.
