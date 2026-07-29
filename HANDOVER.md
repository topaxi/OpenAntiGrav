# Handover

State that is **not** inferrable from the repository itself. Everything about
formats, decisions and the plan lives in [`docs/`](docs/README.md); this file
covers what a fresh reader would otherwise have to rediscover.

**Pruned 2026-07-29.** Until then this was an append-only session log that had
reached 4,393 lines, most of it superseded by the docs pages each pass wrote as
it went. Nothing is lost: `git show 0c3e78d:HANDOVER.md` is the last full
edition and `git log -p HANDOVER.md` is every intermediate one. What survives
here is what is still true and still needed. **Keep it that way** - a finding
belongs on a docs page with its evidence and its confidence score, and this file
points at it. Prune again rather than appending indefinitely.

## Read this first

- **The disc images are in place.** `data/images/` holds `pulse-psp-usa.chd`,
  `pulse-ps2-eu.chd` and `pure-psp-usa.chd`, all three hashes matching
  [`source-images.md`](docs/reverse-engineering/source-images.md), so
  `just test-data` works. Earlier sessions have lost them and spent the session
  unable to verify anything - check they are still there before concluding a
  ground-truth test "cannot run".
- **`data/traces/` is a different story, and it has already bitten once.** The
  2026-07-29 pass found `talons-junction-time-trial-lap.csv`, `-lap-omega.csv`
  and `talons-junction-standing-start.csv` all gone, leaving only `steer-left`,
  `steer-right` and `venom-assegai`. `force-balance-ground-truth.md`,
  `contact-response.md` and three citations inside `wall.rs` point at files that
  were not there. **Derived evidence under `data/` is not durable; the scenarios
  and scripts that regenerate it are.**
- **`rg` and `fd` cannot tell you either of the above.** Both respect
  `.gitignore` and `data/` is gitignored, so both return nothing with exit code
  0 whether the directory is full or empty - no error, no warning, and it reads
  exactly like "there is nothing here". Use `--no-ignore`, or `/bin/ls` /
  `find` / `grep`. Note plain `ls` may be shell-aliased (`ls -la` failing with
  `ls:1: command not found: -la` reads like a broken argument and is not), and a
  spawned agent has a second, independent reason to see `data/` as empty: being
  gitignored, it does not travel into a worktree or a restricted sandbox.
- **Check `git status` before assuming the tree is clean.** A whole milestone's
  work once sat uncommitted for a day.
- **Gate status as of `0c3e78d`:** green at 871 tests, `just test-data` at 921,
  `audit-leakage` clean.

## Where the project stands

M4. The force law, the angular-momentum model, the contact response and the
mag-lock attitude hold are all recovered at instruction level and implemented,
and **every fitted constant that once stood in for a recovered one has been
retired** - `YAW_DRIVE_CALIBRATION`, `ALIGNMENT_INERTIA` and the
`<Misc>`-derived box inertia each had a real value found for it. The evidence
lives in [`docs/physics/`](docs/physics/README.md) (the two ground-truth pages
and [`angular-velocity-column.md`](docs/physics/angular-velocity-column.md)) and
under [`docs/ghidra/functions/`](docs/ghidra/functions/README.md) - `engine.md`,
`rigid-body.md`, `contact-response.md`, `craft-update.md`.

**What blocks M4 now is not contact generation** (that diagnosis was stale for
31 commits before being corrected). Running the committed whole-lap scenario
through our own physics, the ship reaches about 40 units/s by tick 400, **stops
dead at one place on the circuit for roughly a thousand ticks**, then reverses:
618 units of path travelled in 3,146 ticks against the original's **1,045** on
the same inputs, grounded on 3,145 of 3,146 ticks and never further than 20.4
units from the spline. It is not falling off the track any more - it is
**wedged**, and too slow round the circuit by a factor of about 1.7.

**One hypothesis is already dead, and it was mine.** The run report's angular
velocity averages 21.2 rad/s over the lap, which reads like a craft spinning
three and a half times a second and was written up as the likely cause. The
fresh capture kills it: **the original reads 22.96 rad/s on the same scenario**,
slightly *higher* than ours. Whatever that column is, both sides do the same
thing with it and it is not the wedge. Recorded because it is exactly the kind
of plausible number a next pass would otherwise spend a day on.

**Read lap fidelity off the *seeded* comparison** (`oag-trace run --reseed N`),
never the open-loop one, and note the roadmap's exit criterion is now stated as
two decidable numbers for exactly this reason. The original integrates the frame
duration it measured
([ADR-0007](docs/architecture/adr/0007-fixed-timestep-vs-original.md)), those
durations follow host load, and **two runs of the original itself, from a start
pose pinned to 0.0000 degrees, are 100 units apart by tick 495** with `dt`
agreeing on about 1 % of ticks. A 3,146-tick single-seeded trajectory comparison
measures the emulator's scheduler, not the force law, and a byte-exact
implementation would fail it.

**The lap was recaptured on 2026-07-29 and both comparisons run.** Numbers, so
the next pass has a baseline rather than a re-run:

| | single-seeded | `--reseed 60` |
| --- | ---: | ---: |
| Position, max error | 148.7 at tick 435 | **10.3** at tick 299 |
| Position, mean first/last quarter | 100.8 -> 22.9 | 1.89 -> 1.65 |
| Orientation, worst axis | 0.66 rad | **0.114** rad |
| Velocity, max | 97.8 | 54.5 |
| `grounded` | **exact on 3,146 of 3,146** | exact |
| Trend, every field | shrinking or bounded; **nothing growing** | same |

`grounded` agreeing exactly over a whole lap is the strongest single result in
the table: the hover model decides the same thing about surface contact as the
original on every one of 3,146 ticks. And the reseeded position error - a couple
of units per second of simulation, never above ten - is a very different
statement from the single-seeded 148, which is mostly one early divergence being
carried forward.

**Two things about the capture itself that the next person needs.** First, the
committed lap script **no longer flies a clean lap in the emulator**: this
capture fails the `speed/|velocity| == 1.0000` cleanliness test on **67.3 %** of
its ticks (36.8 % over the first 600), where the 2026-07-28 capture of the same
script was 95.2 % wall-free. The script was recorded *closed-loop* by
`psp-autopilot.py`, and replaying it open-loop drifts into the walls - which is
the same negative as above, seen from the authoring side rather than the
measuring side. **A clean lap scenario has to be re-derived from a fresh
autopilot run, not replayed from the committed file.** Second, the capture
timed out at tick 561 on its first attempt (`never stopped at 0x08849618`) and
completed on a straight retry after restarting PPSSPP, so a stalled capture is
worth one retry before it is worth diagnosing.

## Open threads

Each is a real, named next step. The Task numbers are the ones the agent passes
used, kept because commits and docs cite them.

| Thread | What is known, and the next step |
| --- | --- |
| **The wedge: 618 units of path against the original's 1,045** | See above. The blocker, and the cheapest thing in this table to reproduce (`just scripted-sim`, fifteen seconds). The 21 rad/s spin is **not** the cause - the original reads 22.96 on the same scenario. |
| **The 13 % roll-stiffness gap** | The recovered tensor runs the roll oscillator 13 % stiff against measurement. Untouched by the downforce fix by construction (the probes are on the centreline), so it is now the only gap of its kind. |
| **A clean lap scenario** | The committed lap script is a closed-loop autopilot recording and no longer flies clean open-loop (67.3 % of the recapture is in wall contact). Re-derive one from `just autopilot` rather than replaying the file. |
| **Nothing airborne has ever been captured** | `grip_air`, the airborne pitch gain and the `-0.3` airborne weathervane consequently have no runtime leg at all. One capture with a real jump in it closes several at once. |
| **Task #33: `oag-trace` cannot exercise the mag-lock hold** | `replay`/`drive` take one `Environment` for the whole run and the track samples change per tick, so a replay's blend is 0 by construction. Needs per-tick locator plumbing. It also carries the locator-fidelity lead: the hold explains 49.5 % of the inverted-section residual with nothing fitted, and the remaining magnitude points at the *locator* - our 4-per-segment resampled spline may not be the original's evaluated curve. |
| **Ship-to-ship collision stun** | `stun_timer`, its constant and the engine gate all exist and nothing arms them, deliberately: track contact does not arm it in the original. Ship-to-ship needs the writer of the pending impulse at `entity->0x4c + 0x110`, plus `0x0884ef30` (the two-body resolver) and the `+0xb8 == 6` zero-friction entity class - all unread. |
| **Sideshift's input side** | The force half is recovered and pinned (world-space, row-0-aligned, 0.2 s per-side timer, grounded-gated; the direction is evidence now rather than a coin flip). The tap history that triggers it is unread; the capture recipe is written down in the physics docs. |
| **Task #38: the penetration-escape surface gate** | The original gates escape on surface type 1 only; the crate uses 1-or-3. Noticed, not acted on. |
| **Track art's texture pairing** | The delta-(-1) directory rule is proven for ships (11 of 11 exact, position and texture-node count) and **must not be extended to track models** - a looser grouping reaches only 796 of 975, consistent with track meshes sharing sets. A separate, harder problem: [`ps2-texture.md`](docs/formats/ps2-texture.md). |
| **Task #37: the PS2 `.ipf` backdrop renders but is not wired as the looping FE Screen backdrop** | Boot never reaches FE Screen. Front-end work, not format work - [`ipf.md`](docs/formats/ipf.md) is closed at 92. |
| **Which movie cut plays, and what plays the three 260-frame reels** | The reel set is region-invariant, so the disc's region cannot be the picker, and nothing in the boot path plays them at all. [`pmf.md`](docs/formats/pmf.md). |
| **The PS2 PAL/NTSC selector's ultimate trigger** | `Movie_ResolveSourcePath` picks on global `0x0027a85c`; what decides that value (a numbered case in dispatcher `FUN_00186ed8`) was time-boxed away deliberately and nothing needs it yet. |
| **Task #31: `oag-view --collision` panics** | On an empty vertex buffer, for any file with no recognised collision class. Found against Pure; a shipped tool bug. |
| **Two PS2 identifications, one of which is wrong** | `0x0015d058` zeroes the same four accumulators as the already-named `Body_ClearAccumulators` (`0x0015ca48`). Left unnamed per ADR-0005; cheap to resolve. |
| **The SAP clamp globals** | Both are all-zero in shipped `.data` with no writer anywhere in the image (xrefs, operand scans over all 635,898 instructions, `.ctors` - all checked), so taken literally the clamp is degenerate. Confidence 45, and nothing depends on which way it resolves. |
| **An unverified sRGB lead** | `crates/render/src/mesh_render.rs` has the shape that cost the front-end sprite sheet two thirds of its brightness: `Rgba8Unorm` target, `Rgba8UnormSrgb` textures. A textured `oag-view` capture next to a window settles it in a minute. |
| **Front-end gaps behind `Image`** | A `Viewport` is an undecoded clipping rectangle; `screen.rs` reads only a screen's direct children, so `Show Logo`'s `BOOT_LEGAL` line is silently absent; an `Image` with no `x` is centred on a guess (16 pixels either way, on the one case that exists). |
| **Pure's dangling `FEGlobals->TextColor`** | Referenced at seven sites in Pure's own `Skin.xml` and defined nowhere in that file. Either a base skin XML that `Screens::from_xml` does not merge across `LoadXML` includes defines it, or the original engine has a genuine dangling reference. **Not a parser bug on this side** - worth knowing before someone debugs one. |
| **SteamOS's own glibc version** | Unestablished. `just appimage-portable`'s floor is `GLIBC_2.34` (`objdump -T` and `readelf -V` agree, and the determinism test passes inside that container, so the two builds are interchangeable for the simulation). If `GLIBC_... not found` ever appears on a Deck, the version it names is the missing datum. [`packaging.md`](docs/tools/packaging.md). |
| **The chase camera's 3/4 factor** | The eye sits at exactly 0.75 of the authored external offset, at rest and at speed alike, and where that factor comes from was not found - do not hardcode it. [`camera.md`](docs/ghidra/functions/psp-pulse/camera.md) has the samples. The behaviour is right; only `chase.rs`'s prose is wrong, and the tidy-up (one sign flip in `oag_render::camera::chase::anchor` plus deleting `race::chase_pos_length`) is behaviour-neutral and undone. |

## Pending maintainer decision: shipped design data in tracked docs

[`handling-stats.md`](docs/formats/handling-stats.md)'s own rule, per
[ADR-0006](docs/architecture/adr/0006-no-copyrighted-content.md) - "a field name is a
description of the format, a tuning table is the content itself" - is breached
in places by earlier passes. `angular-velocity-column.md`'s shipped
`Turning.amount` has been redacted, but it still quotes the Assegai `Misc` hull
dimensions in its box-tensor refutation, and `force-balance-ground-truth.md`
carries XML-side values from the force-law derivations (`accelcap`, an
`amount`-derived figure, `ride_height`, `Physical.mass`), some of them
load-bearing in the arithmetic.

**Whether to sweep those historically is a maintainer decision, not an
agent's**, which is why it is flagged rather than done. Git history retains
everything regardless, so a sweep would clean the distributable snapshot, not
the past.

## Scope decisions that look like gaps

| Decision | Why |
| --- | --- |
| **The Memory Stick and profile system** | One automatic save slot, the way a modern PC game does it. The disc's boot chain between `Language Selection` and `Main Menu` is eight screens, and all but two exist to serve Memory Stick mechanics. When M6 opens, model `LogoFMV`, `LogoFMVRedirectScreen` and `Show Logo`, then go straight to `Main Menu`. Do not read the omitted screens as unimplemented work. |
| **PSP over PS2 where they diverge** | User directive, now policy in [goals.md](docs/overview/goals.md#scope). PS2 stays the corroboration leg for confidence scoring; genuine behavioural divergences implement the PSP side and document the PS2 one in a table. |
| **The collision stun is not armed by track contact** | The original's is gated on a pending impulse the contact path never writes, and `stun_timer` reads 0.0 on all 3,446 recorded ticks across both captures. The timer and its gate stay; nothing arms them yet. |
| **Sweep and prune is not reimplemented** | The original's packing clamps world space rather than rebasing it, so it never drops a genuinely overlapping pair - it is an accelerator, and exact AABB overlap is a legal and strictly better substitute. The 1-unit quantisation and 1024-id cap are properties of the original's index, not of the format. |
| **The four-corner hover variant** | Not implemented. Its selector is known (`DAT_08ab07e3 == 0 && DAT_08b31048 == 6`, which also disables the brakes and swaps in an auto-speed law), and it is why the PS2's bank-to-yaw `50` was never a contradiction with the two-point path's `30`. |
| **Pure asset work** | Deferred behind M4's exit per [ADR-0009](docs/architecture/adr/0009-multi-game-fanout.md); the probe ran and [`pure-status.md`](docs/formats/pure-status.md) has the table. Two render blockers: the `.vex` class-ID table is renumbered wholesale (five constants, but the right shape is a per-title table keyed off the version word, which is a design call), and Pure's embedded model textures ship pre-swizzled behind a flag byte at `+0x06` that `vex::textures` never reads. |
| Everything milestone-scoped | Weapons, AI, audio, netcode, the shell: [roadmap](docs/overview/roadmap.md), not here. |

## Working rules that were learned expensively

- **A claim that the crate is missing a term is a claim about the crate.** It
  has been wrong three times - the airbrake drag term, the hover downforce
  magnitude and the roll-oscillator note were each recorded as unimplemented in
  prose nobody had checked against the code, and twice the correct value was
  sitting in the same file a hundred lines above the claim.
- **A page can go stale while a sibling page in the same tree already carries
  the correction.** It has now happened three times (the front-end font claim,
  the airbrake drag term in four places, M4's blocker for 31 commits).
  `git log -S <symbol>` against the doc's own last-touched commit settles it in
  one command.
- **Never write a fitted constant in as a recovered value.** Every one that has
  been chased turned out to be something real - an inverse-inertia entry, a box
  tensor, a friction coefficient - and writing the fit in would have closed the
  question at exactly the wrong moment.
- **Check a capture is clean before fitting anything to it.** `speed` and
  `|velocity|` agree to 1e-6 in free flight, so `speed/|velocity| == 1.0000` is
  a free contact detector. Two reference captures were recorded scraping a wall
  for their whole length, and the resulting "missing linear resistance" stood as
  the M4 blocker for a session and a half.
- **A fidelity fix that changes nothing measurable is still a finding.**
  `raycast_all` moved the whole-lap scenario by not one digit (617.538 units
  either way) and single-sided rejection moved it by 1.1; both were real
  divergences from `Collision_BoxAgainstMesh` that `16_Track`'s geometry never
  exercises. Written down, that is knowledge; unwritten, it gets re-litigated.
- **The ramped columns (`craft+0x2c0`-`0x2c8`) lag the frame that used them by
  one tick** - ramp-then-consume in one call, captures sample at entry. Naive
  pairing multiplies a yaw fit's rms by 5.6.
- **The recorded basis columns must be read as Left-Up-Forward.** Taken at their
  column names the basis is a reflection, and a 1.25 rad/s residual reads as 32.
- **Never read timing off the HUD under breakpoints.** The race clock does not
  count emulated frames; laps of identical tick counts timed `0.50.25` and
  `1.11.08`. The lap *counter* is unaffected.
- **"Read far enough to rule X out" is how 80 instructions of a different
  mechanism stayed unread for three passes.** `Ship_UpdateMagLock` had been
  opened twice, each time only to the depth that day's question needed; its tail
  turned out to rewrite the basis directly and was the whole of the remaining
  attitude gap.
- **Time-boxing pays.** Two unread leads (the PAL/NTSC selector's trigger, the
  `.IPF` container) were left alone deliberately and the video got playing
  anyway, because the measured facts did not need them.
- **Diff the output, not the compile.** A shader uniform field existing is not
  the same as a shader reading it: `Draw::Video`'s `rect` was computed and
  discarded on the way to the GPU, and the first "fix" produced a
  pixel-identical screenshot while silently doing nothing.

## Traps that are live

**Process, in a shared tree:**

- **`git-commit` does not isolate a pathspec - it commits everything staged.**
  `git add <paths> && git-commit` silently sweeps whatever another writer staged
  in between; it has happened twice. Check `git diff --cached --name-only`
  immediately before every commit.
- **Do not run `just` or `cargo fmt --all` while anything else is editing.** A
  gate run taken mid-edit reports failures that do not exist, and `cargo fmt
  --all` rewrites files another writer holds open. Scope to `-p <crate>`.
- **A doc two parties are editing wants committing promptly**, not left dirty:
  concurrent uncommitted edits are indistinguishable from data loss to whoever
  looks second.

**This machine:**

- `data/` is on `ecryptfs`. **`cp --reflink` does not work** (copying the three
  images is a real 4 GB copy), and `ls`/`rg` intermittently fail with
  "permission denied" or "no such file" on directories that plainly exist. `fd`,
  `find` and absolute paths work.
- **Backticks in a justfile are command substitution at parse time**, even
  inside an `@echo` string - an echoed example command once ran a full
  trace-compare on every `just` invocation, `just --list` included.

**Tools that fail silently rather than erroring:**

- **`ffprobe -show_entries` does not print fields in the order requested** - it
  uses its own. Parse `key=value` pairs by name (`-of
  default=noprint_wrappers=1`, keys kept); positional parsing reads the wrong
  field and says nothing.
- **`ffmpeg -lossless 1` on its own is silently a lossy encode.** The quantiser
  must be pinned and rate control disabled too: `-b:v 0 -crf 0 -qmin 0 -qmax 0
  -aom-params lossless=1`. Even then `-cpu-used` below 6 is reproducibly not
  bit-exact; 6 and 8 are exact, and faster.
- **A niri screenshot piped through `wl-paste` can return the PREVIOUS clipboard
  image.** One lap "verification" shot came out as a menu from an earlier
  session. Check the image is what it claims before citing it.

**PPSSPP** - the rest is in
[`ppsspp-debugger.md`](docs/reverse-engineering/ppsspp-debugger.md), worth
reading before a capture session rather than after:

- `pkill -f PPSSPPSDL` inside a shell command whose own text contains that
  string kills the calling shell.
- A `memory.write` with an empty base64 payload kills the emulator outright.
- `psp-drive.py restart` fails about one time in three with a transient
  `Invalid address` (the craft is read while the race loads) - retry.
- `replay.flush` crashes v1.20.4 whenever the recording spans a screen
  transition, three reproductions. The replay API is closed as an option.
- Restore the savedata backup (move, do not delete) before concluding a session
  that touched `~/.config/ppsspp/PSP/SAVEDATA/UCES00465P0000`.
- **The USA PSP disc never shows the Language Selection picker** - the build is
  English-only and the front end skips it when there is one language to show.
  Proving that costs about an hour. A live capture of it needs a EU PSP disc or
  a PS2 BIOS in PCSX2, neither of which is available here; do not suggest
  sourcing one.

## Verification status: what to lean on

**Measured against the original running.** The forward force law end to end (a
standing start fits from `fs = 0.56` to `47.67` at rms 0.127 on a one-parameter
fit); lateral grip at 0.9985 of the disc value, 99.95 % explained; the yaw
accumulator term by term at racing speed, 99.94 %; the inertia tensor on all
three axes, pitch to 0.13 % and the `I * omega` identity to 0.01 %; the pitch
step response at 1.006x; the contact friction coefficient as a one-sided bound
approached from above; the mag-lock probe re-finding the magstrip from geometry
that knows nothing about the capture (91 % of inverted poses against 0.3 % of
upright ones). Method and numbers in the two ground-truth pages under
[`docs/physics/`](docs/physics/README.md).

**Instruction-level reading with no runtime leg yet.** Mag-lock's blend weights
and its `|h - d| > 5.0` fallback; the swept collision path; everything airborne.
Single-sided wall rejection is measured against the *data* rather than against
the original running: **2,076 of 2,084 wall triangles on `16_Track` (99.6 %)**
are wound toward the nearest point of the track's own spline, which is what
makes reproducing the original's `dot(boxCentre - sample, n) > 0` rejection
faithful rather than merely literal.

**The determinism gate reaches the simulation now** (`dd53c1f`).
`oag_physics::probe` steps the same `oag_physics::step` the race loop steps,
over a scripted input and a synthetic corridor built in the file, so it runs in
CI on all three OSes rather than needing a disc image. Two mechanisms keep the
hashed field list from rotting and **both are needed**: exhaustive destructuring
makes a new `ShipState`/`Body` field a compile error, and a perturbation test
proves each field actually reaches the hash rather than merely being bound -
that second test caught a real hole on its first run, since `time_since_landing`
defaults to `1.0` and perturbing it *to* `1.0` was a vacuous no-op. What it does
not cover is stated in its module docs: maglock, reset, and the swept tunnelling
path.

**Two standing rules.** When the determinism test fails, find the bug - never
update the reference constants to make it pass. And never raise a confidence
score on the strength of the simulation agreeing with itself.

## Reference

The user pointed at <https://www.youtube.com/watch?v=lGAWYHmgo7o> as a
reasonably high-quality capture of Wipeout Pulse, useful for checking a reading
against the real thing without setting up an emulator capture. It is not an
authority over the disc images and Ghidra: it is a recording of someone else's
playthrough, subject to its own encode and possibly the wrong regional cut.
Whether Pure has an equivalent is not checked.
