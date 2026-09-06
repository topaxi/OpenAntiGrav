# Five reference traces are missing here, and the silent skip is why nobody noticed

2026-09-06. Six trace captures are named by ground-truth tests in this
workspace. **Five of them do not exist here**, and their absence never turned a
build red, because a ground-truth test whose reference is missing *skips*
rather than fails.

| Capture | State |
| --- | --- |
| `talons-junction-time-trial-lap.csv` | present, tracked |
| `pad0-boost.csv` | **recovered from `vimes`**, tracked |
| `talons-junction-standing-start.csv` | **recovered from `vimes`**, tracked |
| `talons-junction-pitch-both-ways.csv` | **recaptured live, 2026-09-06**, tracked |
| `talons-junction-time-trial-lap-omega.csv` | still missing |
| `venom-straight.csv` | still missing |

They were never deleted by a policy - `data/` is gitignored, so they only ever
existed in whichever checkout captured them. **The maintainer has a second
workstation and is checking it as of 2026-09-06**, so "gone" is the working
assumption rather than an established fact; whichever way that lands, the five
were one disk failure from being unrecoverable, which is the point ADR-0046
answers.

[ADR-0046](../docs/architecture/adr/0046-test-referenced-traces-are-tracked-in-git.md)
fixes the recurrence: a trace a test names is now tracked in git, per file by
name. It does not recover the five.

## How this was found, and the measurement worth keeping

`just test-data` reports **2 failures**. The same suite under
`OAG_REQUIRE_GAME_DATA=1` reports **14**, because that variable converts an
absent optional input from a skip into a hard error. Eleven of the twelve extra
are missing-file panics, not behaviour: `data/traces/pad0-boost.csv`,
`data/traces/talons-junction-time-trial-lap-omega.csv` and
`data/keys/pure-dlc-keys.txt` name themselves in the panic message.

The keys file is a different case and **not a loss** -
[ADR-0033](../docs/architecture/adr/0033-external-key-material-for-decryption.md)
designs it as an optional input.

**So `just test-data` being green is weaker evidence than it looks**, and that
is the durable lesson here rather than the file list.

## Recovered, 2026-09-06

**Two of the five came back off the host `vimes`**, and with them six
ground-truth tests: all three `chase_camera_ground_truth` (which wanted
`pad0-boost.csv`) and all three `wall_contact_ground_truth`. Verified under
`OAG_REQUIRE_GAME_DATA=1`: 9 tests in those four binaries, 6 pass, 3 fail, and
the 3 are missing-file panics for the two captures still absent.

`vimes` also holds 27 other captures this checkout did not have - `head-pad*`,
`shield-*`, `talons-junction-steer-*`, `talons-junction-venom-assegai`,
`sideshift-double-tap-clean*` and others. They are copied in but **not tracked**:
no test names them, so [ADR-0046](../docs/architecture/adr/0046-test-referenced-traces-are-tracked-in-git.md)
leaves them excluded, which the allowlist enforced without anyone deciding it
per-file (2 staged, 27 ignored, `just audit-leakage` green).

**One hazard found in the process, and it is exactly the one ADR-0046 exists to
stop.** `vimes` carries its own `talons-junction-time-trial-lap.csv` at 874,185
bytes; this checkout's is 897,897 bytes with a different MD5. **Same filename,
different capture.** Neither is labelled, and nothing recorded which one any
test was written against. The local copy is the one that passes and the one now
in git; the `vimes` copy was deliberately not pulled over it (`rsync
--ignore-existing`). Two divergent captures under one name is how a
ground-truth test silently starts measuring something other than what its
author meant - which is the whole argument for the reference living in git
beside the test.

**`talons-junction-pitch-both-ways.csv` is recapturable and does not need
`vimes` or the other workstation.** Its input script is tracked:
`verification/scenarios/pitch-both-ways.inputs`, one of 18 in git. So the
recipe survived even though the capture did not - which is worth noting as the
thing that *did* work. No matching `.inputs` was found for `venom-straight` or
the `-omega` lap.

**Recaptured, live off `pulse-psp-usa.chd`**, following
`docs/reverse-engineering/ppsspp-debugger.md`'s recipe exactly (`PPSSPPSDL`
under a fresh `Xvfb`, `psp-drive.py preflight` -> `menu` -> `restart`, then
`psp-trace.py --script verification/scenarios/pitch-both-ways.inputs
--script-lead 2`): 360 ticks, `speed/|velocity| == 1.0000` on every tick as
the scenario's own header predicts, craft never leaving Talon's Junction's
start line. Committed at `data/traces/talons-junction-pitch-both-ways.csv`.

**This is the one capture in the table of six that no live Rust test actually
names**, which narrows this thread's opening claim ("six trace captures ...
are named by ground-truth tests") for this one file specifically -
`crates/physics/src/forces.rs` and `crates/physics/src/engine.rs` cite it only
in doc comments as evidence for constants already baked in
(`PITCH_INVERSE_INERTIA`, `ROLL_INVERSE_INERTIA`, the pitch-torque gate), and
`cargo nextest list --workspace --run-ignored all` has nothing pitch-shaped
that opens a `data/traces/` path (checked directly, including the
`format!("data/traces/talons-junction-{name}.csv")` pattern
`yaw_authority_ground_truth.rs` uses for `steer-left`/`-right`, in case a test
built its path that way). So there is no ground-truth test to newly pass or
fail here - the closest thing to one, `scripts/trace-pitch-response.py` and
`scripts/trace-angular-fit.py --joint`, both ran clean against the new
capture: the independently-read `avel_x`/`omega_x` pair (`+0x160`/`+0x150`,
never derived from each other) ratios to a median of `15.6001` across the
capture and a per-axis fit of `-15.619` on `x`, against
`PITCH_INVERSE_INERTIA`'s `15.6` - matching the `-15.620` `forces.rs` already
documented from the original, lost capture to within noise. `cargo run -p
oag-trace -- run ... --script pitch-both-ways.inputs` against it also ran
clean (no panic, no schema mismatch), with the same open hover/contact-gap
divergences past tick ~300 that `docs/physics/angular-velocity-column.md`
already documents elsewhere - not a regression, and not this scenario's
subject (it never leaves the start line).

## Open

- **The docs assert captures this checkout cannot demonstrate.**
  `docs/physics/cornering-ground-truth.md` states "That capture was taken" of
  the `-omega` lap and `docs/physics/angular-velocity-column.md` builds an
  argument on it. Both are honest about history and misleading about the
  present. Whether to annotate them or leave them is a judgement call nobody
  has made.
- Which of the five are still *needed*, as opposed to named by a test that has
  itself gone stale, has not been checked. Recapturing all five without asking
  that first would be wasted work.

## Next Steps

1. **Remove the skip-on-missing path for the tracked set.** Now that ADR-0046
   makes absence impossible for a tracked trace, a missing one should break the
   build rather than reduce coverage quietly. This touches the ground-truth
   test helpers, so it wants a session that is not racing another member
   through the same files.
2. **Recapture, by the recipe in
   [ppsspp-debugger.md](../docs/reverse-engineering/ppsspp-debugger.md)** - the
   same route that produced the sideshift capture. Take them one at a time and
   commit each as it lands; `.gitignore` already names all six, so a recaptured
   file is tracked the moment it appears.
3. Decide the doc-annotation question in `## Open` above, ideally while
   recapturing, since whoever does that will know which claims came back.
