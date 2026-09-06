# Five reference traces are missing here, and the silent skip is why nobody noticed

2026-09-06. Six trace captures are named by ground-truth tests in this
workspace. **Five of them do not exist here**, and their absence never turned a
build red, because a ground-truth test whose reference is missing *skips*
rather than fails.

| Capture | State |
| --- | --- |
| `talons-junction-time-trial-lap.csv` | present, now tracked in git |
| `pad0-boost.csv` | **gone** |
| `talons-junction-pitch-both-ways.csv` | **gone** |
| `talons-junction-standing-start.csv` | **gone** |
| `talons-junction-time-trial-lap-omega.csv` | **gone** |
| `venom-straight.csv` | **gone** |

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
