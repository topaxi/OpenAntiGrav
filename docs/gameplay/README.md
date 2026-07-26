# Gameplay

> **Not yet started.** Milestone M5. This directory will hold documentation of
> race rules, modes, progression and the things that make Pulse Pulse rather
> than a generic racer.

## Scope

- Race rules: grid order, laps, positions, finishing conditions
- Game modes: single race, tournament, time trial, Zone, Eliminator
- Speed classes and how they scale handling and top speed
- Teams and their stat differences
- Progression and unlocks
- Shield energy, absorb, and the risk/reward loop around holding a weapon

## Prerequisites

Requires [binary understanding](../reverse-engineering/methodology.md) (M2) and
the [verification harness](../reverse-engineering/verification-protocol.md) (M3).
Rules are cheap to guess at and expensive to guess wrong, so they get read from
the binary rather than inferred from play.
