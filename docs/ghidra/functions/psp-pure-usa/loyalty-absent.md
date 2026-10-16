# Pure pays no loyalty: what the executable and the data carry

2026-10-16, lane `loyalty-earning`. `BOOT.BIN` (ELF, not in the Ghidra bridge; searched
as bytes) and the three WADs of `pure-psp-usa.chd`. No rename comes of this page; a
negative finding, confidence **85** for "the original has none".

- `grep -a -i 'loyal|ER_LOY|ER_TOT_LOY|ER_POINTS'` over `BOOT.BIN` finds nothing.
- Over `Data.wad`, `FE.wad` and `FEData.wad` the only hit is the credits line "Director
  of Loyalty and Channel Marketing" (also no UTF-16 hit). No loyalty widget or string.
- The Unlock record in the executable has no `Loyalty` field where Pulse's does
  (`campaign.md`, "Confirmed shape"): 17 shipped rows gate on class, team, track, medal,
  none on a total.
- Conclusion: **Pure has no loyalty system.** Per the maintainer rule this project pays
  Pure's races Pulse's law into the shared total, labelled inherited
  (`Origin::InheritedFrom("Wipeout Pulse")`); nothing in Pure reads it back, so it only
  feeds the Pulse/HD-gated unlocks and skins that read the shared total.
