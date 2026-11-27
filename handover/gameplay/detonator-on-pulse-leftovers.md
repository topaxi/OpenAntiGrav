---
status: blocked-maintainer
titles: [pulse]
updated: 2026-11-27
summary: "Detonator on Pulse ships as the mod oag.detonator-pulse; no medals, score targets, announcer lines or far-LOD hull"
categories: [gameplay]
---

# Detonator on Pulse: what the backport still lacks

Pulse ships no Detonator mode. The backport is in `main` and its data is the official mod
`oag.detonator-pulse` ([ADR-0100](../../docs/architecture/adr/0100-a-mod-carries-fallback-entries.md)).
What a race has, and the mod split, are on
[detonator-pulse.md](../../docs/gameplay/detonator-pulse.md).

## Open

- Not there: medals, score targets, campaign events, the announcer and EMP voice lines, the
  floating `+N` popups. Pulse ships no stage grade, so none is drawn.
- The own hulls have no far LOD part; they draw at every distance
  ([own-hull.md](../../docs/formats/own-hull.md)).
- On PS2 the cannon still leaves from the disc hull's `cannon_flash` locators, at its nose and
  not at the new hull's pods.
- The AUR packages and the web build ship no official mods, so they have no Pulse Detonator
  (before the move the data was in the binary).
- No record that the maintainer has judged the EMP's look since it borrowed the Repulsor's.

## Next Steps

1. The maintainer plays it and says which of the above matters.
