# ADR-0086: A lead session may push to the private repository

## Status

**Accepted** (2026-11-05). Replaces point 5 of
[ADR-0085](0085-comparison-originals-live-in-a-private-repository.md). ADR-0085 is not edited.

## Context

ADR-0085 held every push to `OpenAntiGrav/private` for the maintainer's say-so. The maintainer
lifted that the same day: the repository publishes nothing, so a push there has little impact.

## Decision

1. **A lead session pushes to it without asking**, on either workstation.
2. **The allowlist is the guard, and only the maintainer widens what it admits.** Narrowing it is
   a lead's call. The never-list in ADR-0085's point 4 holds, and a rule may not be loosened to
   get a file in.
3. **Drive members and CI still do not push.** A member's data goes through its lead.

## Consequences

- A re-render's `pair.json` and manifest stamps can be committed or discarded as a lead sees fit.
- A wrong push is a history rewrite on a repository with no other users, and an LFS object stays
  until GitHub support or a new repository removes it, so a lead reads the staged list first.
