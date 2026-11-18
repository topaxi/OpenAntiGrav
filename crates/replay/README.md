# oag-replay

Replays and ghosts. A replay is the per-slot input stream plus periodic state
hashes that name the first tick a desync appears; a ghost is a pose track drawn
from a recorded lap.

On the simulation's side of the line: it links nothing that draws. Design:
[ADR-0055](../../docs/architecture/adr/0055-replays-are-inputs-and-a-ghost-is-poses.md).
