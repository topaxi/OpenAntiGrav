---
status: measure
titles: [all]
updated: 2026-11-16
summary: "Pad seats landed; open: shadPS4 unverified (root-only install), RPCS3 two-seat run untested, members not yet given seats"
---

# Pad seats: what is not yet proven

`OAG_PAD_SEAT=N` gives a member's uinput pads their own name and USB product id and its SDL emulators a hint that
shows them nothing else (`docs/tools/compare-page.md`, "Pad seats"). Two wipeout_pc ports on seats 1 and 2 at once did not
cross; the bundled SDL3 of RPCS3 and PCSX2 and the system's honour the hint (`python3 -I scripts/pad_seat.py probe`).

## Open

- **shadPS4 is unverified.** `/usr/lib/shadps4/{bin,lib,optional}` are `drwx------ root` on the maintainer's workstation, so no
  member can start or inspect it. `shadps4-omega-pair.py` passes it the same environment as the port. Needs
  `chmod -R a+rX /usr/lib/shadps4`, then two shadPS4 instances on two seats, or one beside the port.
- **RPCS3 under two seats is untested.** Only the name resolution was run (`rpcs3_pad.py pad` under `OAG_PAD_SEAT=3` made
  `OAG seat 3 pad rpcs3`). `docs/reverse-engineering/emulator-recipes.md` keeps its confidence-40 note that RPCS3 once
  read another pad when the profile's name did not match.
- **Members are not told a seat.** `member-rules.md` names no pad rule; the lead has to hand out `OAG_PAD_SEAT` numbers in
  briefs and add a line to the "Emulators" bullet.

## Next Steps

1. Fix the shadPS4 install permission, then run the two-seat proof (port on seat 1, shadPS4 on seat 2; press on one, grab both).
2. Boot two RPCS3 instances on seats 1 and 2 (`scripts/emu-env.sh` with `OAG_PAD_SEAT` set) and walk one menu on one.
3. Add the seat line to `member-rules.md`, then delete this thread.
