# DLC packs are mounted; two things inside them are not read

2026-08-09. The four Pulse packs load, region-independently: `just play --race --team Mantis` off `pulse-psp-usa.chd` draws a European pack's ship, and the catalogue goes from 8 teams / 24 circuits to **12 / 32**. Format, evidence and per-claim confidences on [`docs/formats/dlc-pack.md`](../docs/formats/dlc-pack.md); the divergence from the original's region lock is [ADR-0021](../docs/architecture/adr/0021-region-independent-dlc.md). **Left unread, both deliberate.** (1) `downloadNN.xml`, entry 1 of each `PACKn.edat`: a `PI_Grid` championship ladder, which is progression work rather than asset work and belongs with the campaign, not here. (2) `PI_TeamModel`/`PI_ModelSkin` - the concept, zone and unlockable liveries with their `loyalty` thresholds - are declared in every team's manifest, disc and pack alike, and `catalogue::Team` deliberately does not collect them while nothing draws a second hull. That is the same list the roadmap's "livery and team variants" item wants, so whoever takes that item gets the parse for free. **The trap worth knowing**: a pack's ship is in `PACKn.edat` and its `handlingstats.xml` is in `PACKn_UI1.edat`, so mounting only the main archive gives a ship that loads and cannot race.

## Open

- `downloadNN.xml` (the `PI_Grid` championship ladder) is deliberately left unparsed - it is progression work, not asset work.
- `PI_TeamModel`/`PI_ModelSkin` (unlockable liveries and their `loyalty` thresholds) are declared in every manifest but not collected, since nothing draws a second hull yet.

## Next Steps

- Parse `PI_TeamModel`/`PI_ModelSkin` when picking up the roadmap's "livery and team variants" item.
