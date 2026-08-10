# ADR-0021: Mount downloadable content independently of region

## Status

Accepted.

## Context

Wipeout Pulse sold four downloadable packs for the PSP, each adding a team and
two circuits. In the original, a pack was tied to the territory it was sold
in: the European packs carry a `UCES00465` folder and were bought against the
European disc, and there is no supported way to use them with the American
`UCUS-98712` release.

That coupling is a **storefront** arrangement, not a technical one. What is
actually on the disc says so:

- A pack's payload is an ordinary [WAD](../../formats/wad.md) with no
  encryption and no per-title key - see
  [the DLC pack format](../../formats/dlc-pack.md).
- Assets are addressed by CRC-32 of their name, and a name hash carries no
  territory. `Data\Ships\Harimau\Ship.vex` hashes the same on both discs.
- **The American disc already names all four pack teams**, in its own string
  table, in all five languages it ships. Nothing had to be added for the other
  region's content to read correctly.

This project targets the American PSP disc
([source images](../../reverse-engineering/source-images.md)), so reproducing
the lock would mean a maintainer who owns the packs cannot reach content the
disc in front of them can already name.

There is a further wrinkle specific to reimplementation. Discovery has to decide
what a pack *is*. The obvious signals - the `UCES00465` folder, the
`PACK*.edat` filenames, the disc serial - are exactly the ones that encode a
region, and any of them used as a gate reintroduces the lock by accident.

## Decision

**Mount any Pulse pack against any Pulse source, and give the region nothing to
attach to.**

Concretely:

1. **Discovery matches on file shape.** A candidate is anything that parses as a
   WAD v1 archive. Filenames are a pre-filter for speed and never a requirement,
   so a renamed or repacked download still works.
2. **The title-id folder is dropped** when a download is unpacked into the
   cache, so no later code can come to depend on it.
3. **The booted image's serial is never consulted**, and `pulse::Layout`'s
   other-title deny-list is not applied to packs. That list exists to stop a
   *different game's disc* being opened as Pulse; a pack is not a disc.
4. **A pack is keyed by the team its own manifest declares**, never by its pack
   number, so a differently-numbered territory's pack keys the same and two
   copies of one team deduplicate.
5. **The source always wins a collision.** Packs are searched after `data` and
   `fe`. No shipped pack collides with a disc entry, so this decides nothing for
   real content; it is there so that a modified pack cannot silently change the
   base game.

This is a deliberate divergence from the original, in the same family as
[ADR-0007](0007-fixed-timestep-vs-original.md): the behaviour is documented as a
difference rather than hidden, and the reasoning is recorded so a later
contributor does not "fix" it back.

## Consequences

**A player uses content they own, with the disc they own.** The four European
packs work against the American disc, which is what the implementation is
verified against: twelve teams and thirty-two circuits where the disc alone
offers eight and twenty-four (`crates/game/tests/dlc_ground_truth.rs`).

**The engine has no concept of downloadable content above the asset layer.** A
pack's teams and circuits arrive through the same `PI_Team` and `PI_Track`
schema the disc uses, so the front end, the catalogue and the race loader do not
know which is which. That is the property that keeps the feature small.

**Nothing here bypasses a protection**, because there is none to bypass: the
packs are plaintext and the engine reads files the player already has. This
project ships no content and never will - see
[ADR-0006](0006-no-copyrighted-content.md).

**A partial set of packs is a supported state.** The four packs split each
circuit by direction, so owning one leaves declarations whose geometry is in
another. Those are dropped at boot with a note rather than offered and then
failing.

**What this does not do:** it does not make Pulse packs work with *Pure*, whose
downloadable content is genuinely encrypted and remains unread, and it does not
claim to know how the original itself decided a pack was usable. No runtime
trace of that decision was observed.
