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
| `talons-junction-time-trial-lap-omega.csv` | **recovered, 2026-09-11**, tracked |
| `venom-straight.csv` | **confirmed lost, 2026-09-11** |

They were never deleted by a policy - `data/` is gitignored, so they only ever
existed in whichever checkout captured them. **Update, 2026-09-11: all
workstations have now been checked** (see the dated sections below for the
outcome per file) - the five were one disk failure from being unrecoverable,
which is the point ADR-0046 answers.

[ADR-0046](../../docs/architecture/adr/0046-test-referenced-traces-are-tracked-in-git.md)
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
[ADR-0033](../../docs/architecture/adr/0033-external-key-material-for-decryption.md)
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
no test names them, so [ADR-0046](../../docs/architecture/adr/0046-test-referenced-traces-are-tracked-in-git.md)
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

## Recovered, 2026-09-11

`data/traces/talons-junction-time-trial-lap-omega.csv` appeared in the working
tree, untracked (`.gitignore` already allowlists it per ADR-0046, so `git
status` showed it as a plain new file rather than ignored). Its provenance -
which workstation, which recapture run - was not recorded by whoever produced
it; this thread only confirms what the file itself demonstrates.

Header matches `crates/game/tests/maglock_ground_truth.rs`'s expectations
exactly (`omega_x`, `right_x`/`up_x`/`fwd_x`, `pos_x`, `vel_x`, `dt`), 3,140
data rows. Verified with `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p
oag-game --run-ignored all -E 'binary(maglock_ground_truth)'`: both tests
pass, including `the_hold_reproduces_the_rotation_the_momentum_column_does_not`,
which previously printed the skip message and returned `None` (this is the
"one place in the workspace where `None` is still the honest answer" the test
file's own doc comment names - it is no longer reached). Staged with `git add`;
not yet committed.

`venom-straight.csv` remains the only one of the six still missing.

**This does not resolve the `## Open` doc-annotation question below - it
adds to it.** `docs/physics/cornering-ground-truth.md` was itself corrected on
2026-09-10 to say `talons-junction-time-trial-lap-omega.csv` "no longer exists
under that name" (its tables were rebased onto `talons-junction-clean-lap.csv`
instead). That statement is now stale in the other direction: the file exists
again, under its original name, and the ground-truth test that names it is
running instead of skipping. Nobody has checked whether the recovered bytes
match what those sections' analysis was written against, or whether the
tables should point back at it. Treat it as a distinct capture until someone
verifies otherwise.

## `venom-straight.csv` is confirmed lost, 2026-09-11

The maintainer checked every workstation as of 2026-09-11: not present
anywhere. This closes the "maintainer has a second workstation and is
checking" line from the opening section - the check is done, and the file did
not come back.

**No live test currently names it**, so this loss does not skip or fail
anything today: `yaw_authority_ground_truth.rs` moved off it in 2026-07-28
(see the correction above), and nothing else in `crates/*/tests/` ever did.
Its only remaining reference is the doc-comment example at the top of
`crates/trace/src/main.rs` (`oag-trace show data/traces/venom-straight.csv`
and two more lines like it), which now names a file that cannot exist again -
not "missing", unrecoverable. That comment should point at a capture that is
actually in the tree (`talons-junction-time-trial-lap.csv` is the obvious
choice) rather than keep citing this one; not fixed here since it is
docs-only and not this thread's main subject.

**No recapture path exists either.** Unlike `talons-junction-pitch-both-ways.csv`,
which came back live because its input script (`verification/scenarios/pitch-both-ways.inputs`)
survived, nothing under `verification/scenarios/` reproduces a straight-line
Venom run - checked directly, `2026-09-06`'s note above already found no
matching `.inputs`. Recapturing it would mean reconstructing the scenario from
scratch (team/class/track/inputs), not replaying a known recipe.

With this, the six-capture table has a final shape: four tracked and present,
`talons-junction-pitch-both-ways.csv` tracked as evidence for a doc comment
rather than a live test, and `venom-straight.csv` gone for good with nothing
depending on it. `.gitignore`'s allowlist still names it per ADR-0046 (that
line is inert, not wrong - it just never matches a file); ADR-0046 itself is
immutable so its own "missing" row for this file stays as written history.

## One of the six is not test-referenced, and ADR-0046's rule does not strictly cover it

Established while recapturing, 2026-09-06. `talons-junction-pitch-both-ways.csv`
is named by **no live test** - checked against `cargo nextest list` and against
the `format!()`-constructed path pattern that some ground-truth tests use, which
a plain grep misses. Its only consumers are doc comments in
`crates/physics/src/forces.rs` and `engine.rs`, citing constants already baked
into the source.

[ADR-0046](../../docs/architecture/adr/0046-test-referenced-traces-are-tracked-in-git.md)
says "a trace a test names is tracked". By that rule this file does not qualify,
and it is tracked anyway. **That is a deliberate call, not an oversight**: the
capture is the standing evidence for `PITCH_INVERSE_INERTIA`, it verified that
constant to within noise on recapture (`avel_x`/`omega_x` ratio to median
`15.6001`; `trace-angular-fit.py` fits `k = -15.619` against a documented
`-15.620`), and a doc comment asserting a measurement whose measurement no
longer exists is the same failure this whole thread is about, one step removed.

If the rule is ever restated, "a trace a test or a doc comment's evidence claim
names" is the shape that matches practice. It is recorded here rather than
edited into the ADR, which is immutable.

**A grep over `crates/*/tests/` under-reports which traces are test-referenced
in general**, because of constructed paths like the `format!()` one
`yaw_authority_ground_truth.rs` uses for `steer-left`/`-right`. That file used
to be `venom-straight.csv`'s reason for counting as test-referenced too, but
`8794b0f3` (2026-07-28, replacing `YAW_DRIVE_CALIBRATION` with the recovered
`YAW_INVERSE_INERTIA`) moved it onto the `steer-left`/`-right` captures
instead - checked directly, `rg venom crates/trace/tests/yaw_authority_ground_truth.rs`
now matches nothing but a doc comment naming the scenario's team. So the
grep-vs-real-run gap this paragraph originally warned about no longer applies
to `venom-straight.csv` specifically; see the confirmed-lost section above for
what does still name it. Use a real run under `OAG_REQUIRE_GAME_DATA=1` to
answer this question for any other capture, not a grep.

## Open

- **The docs assert captures this checkout cannot demonstrate - and, for the
  `-omega` lap, now assert the opposite of what the checkout has.**
  `docs/physics/angular-velocity-column.md` still builds an argument on "that
  capture was taken", while `docs/physics/cornering-ground-truth.md` was
  corrected on 2026-09-10 to say the same file "no longer exists under that
  name". As of the 2026-09-11 recovery above, it exists again and neither doc
  has been reconciled against the recovered bytes. Whether to annotate them,
  re-run the analysis against the recovered file, or leave them is a judgement
  call nobody has made.
- ~~Which of the five are still *needed*...~~ Resolved: all five are
  accounted for. Four came back (`pad0-boost.csv`, `talons-junction-standing-start.csv`,
  `talons-junction-pitch-both-ways.csv`, `talons-junction-time-trial-lap-omega.csv`),
  one is confirmed permanently lost (`venom-straight.csv`, and nothing live
  needs it - see above).

## Next Steps

1. **Remove the skip-on-missing path for the tracked set.** Now that ADR-0046
   makes absence impossible for a tracked trace, a missing one should break the
   build rather than reduce coverage quietly. This touches the ground-truth
   test helpers, so it wants a session that is not racing another member
   through the same files. `venom-straight.csv` is the one tracked-by-`.gitignore`
   name this rule can never apply to - no test names it, so there is nothing
   for the panic to guard.
2. ~~Repoint `crates/trace/src/main.rs`'s doc-comment example off
   `venom-straight.csv`~~ Done, 2026-09-11: now uses
   `talons-junction-time-trial-lap.csv`, a capture actually in the tree.
3. Decide the doc-annotation question in `## Open` above: reconcile
   `docs/physics/cornering-ground-truth.md` and `angular-velocity-column.md`
   against the recovered `-omega` capture.
4. Commit `data/traces/talons-junction-time-trial-lap-omega.csv` (currently
   staged, not committed - see "Recovered, 2026-09-11" above).
