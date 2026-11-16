# ADR-0091: A modded race records its inputs and its header names the modules

## Status

Accepted (2026-11-16, the maintainer's decision for screenshots). Supersedes the first bullet of
[ADR-0087](0087-a-replay-and-photo-mode-share-one-timeline.md) decision 7 ("modded races record
nothing and stay that way") and the phase-2 "no replay" rule in
[ADR-0079](0079-extensions-are-sandboxed-webassembly-guests-behind-two-worlds.md) decision 9 and
`docs/architecture/wasm-extensions/README.md` section 4.4.

## Context

A screenshot carries the input stream that makes its frame again (`docs/formats/shot-png.md`).
In a dev checkout `oag.energy-turbo` is found by default, so every race there is modded and,
under ADR-0087 decision 7, recorded nothing: no shot had a stream. ADR-0087 left it open as "a
feature of its own": the module's bytes have to be named and matched by the file.

## Decision

1. A modded race records its human input stream, events and hashes like any race.
2. The replay header carries a `mods` list (`oag_replay::header::ModUse`, `[[mods]]`): per
   simulation module the mod id, version, module path, SHA-256 of the module's bytes, ABI major,
   layout revision, state bytes and the player's `sim` picks. A vanilla race writes no table.
3. There is no separate "blob layout version" field. The layout is in the module's code, which
   the SHA-256 pins; the ABI, revision and state bytes are recorded so a refusal can say what
   moved.
4. A rebuild runs exactly the listed modules with the listed picks (`Options::recorded_mods`) and
   `apply_recorded` refuses, by name, a module that is missing, another build, another layout,
   other picks, or an extra one. The stored state hashes still decide validity.
5. A modded race sets no ghost and offers none to save: a module changes the run, so no lap of it
   is a lap to race against.
6. Not decided here: the tune bytes (the ABI has no `setup.tune`), the lobby fingerprint's
   SHA-256, the cross-engine conformance test.

## Consequences

- A screenshot of a modded race is made again by `--from-png` with the same modules, to a hash
  MATCH, and refused by name without them.
- The REPLAYS list now holds modded recordings; playing one needs the same modules on.
- Host-local failures (an instance-pool or mmap failure, a native stack overflow) still become a
  hashed `Fault::Trap`, so a recording whose module faulted that way desyncs on a machine that did
  not. It is reported at the next stored hash, never silently, and is open until the failure is
  moved out of the hash.
- A replay across engines (Cranelift, Pulley, the browser) is unproven until the conformance
  test exists.
- Manifest tables a race reads (pickup odds, memory pages) are not hashed; the mod's `version`
  stands for them, and an author who changes one without a version bump gets a desync, not a
  refusal.
- The module's file is hashed whole, so any rebuild of an unchanged guest that differs in a
  byte is "another build", though it may behave the same.
