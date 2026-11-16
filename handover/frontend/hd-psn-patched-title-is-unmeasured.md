---
status: blocked-maintainer
titles: [hd]
updated: 2026-11-05
summary: "A patched PSN HD mounts its chain but keeps the 3.00 Title rows; what 02.51 offers needs its EBOOT read with the player's licence"
size: M (50)
difficulty: medium (50)
---

# The patched PSN install keeps the 3.00 title data

`docs/formats/hd-updates.md`, "The two sources after patching"; `docs/formats/hd-psn.md`, "A patched install".
The chain mounts; `oag_hd::psn::PSN` still answers, measured on the 3.00 executable.

## Open

- Whether the PSN 02.51 `EBOOT.BIN` has a body behind the call the disc build stubs (`0x002c5d5c`, Fury
  `update-0251.md`, "Archive load order"), and what gates `data07` or Fury content on it. The PSN file is
  7,690,576 bytes against Fury's 7,690,704, and it needs the player's `.rap` to decrypt; none is on
  this machine.
- Which `PSN` rows flip on 02.51: `file.gtf` or `file2.gtf`, the plain or `bluered` exhaust, Team and
  Track Selection, `Campaign Selection`, `_c1` and `_n1` variants, `data05`'s end-race `Podium`.
- Whether the "Unlock Key" of `hd-psn.md` unlocks the base or Fury. No package held ships `data00`,
  so Fury's four circuits and the Zone circuits are absent either way.

## Next Steps

1. Maintainer: put the `.rap` under a private RPCS3 profile's `dev_hdd0/home/00000001/exdata/`.
2. `rpcs3 --decrypt` the patched `EBOOT.BIN`; diff strings and `0x002c5d5c` against Fury 02.51.
3. Add a variant of `PSN` for a patched install only for what that proves (`title_of` can tell by `data08`).
