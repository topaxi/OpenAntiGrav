# `just` recipes eat the backslashes in a WAD path

2026-09-10. Reported by the member working the alpha-test cutout thread, which
needed to open a specific mesh in the viewer and could not name one. The
mechanism is confirmed here directly; only the blast radius is estimated.

## What happens

Every WAD-internal path on these discs is authored with backslashes -
`Data\Environments\01_Vineta_K\track.vex`, `Data\Ships\Feisar\ship.vex`,
`Data\Psys\WO_SHIP_COLL_SPARK_DAMAGE.POB`. That is the separator the original
uses and the one `oag-wad` and `oag-assets` expect.

A `just` recipe interpolates `{{ARGS}}` **unquoted** into a shell, and the shell
treats a backslash as an escape character. So the separators are silently
deleted before the binary ever sees them.

Confirmed directly, both interpolation forms this justfile uses:

```console
$ sh -cu 'printf "[%s]\n" Data\Ships\x.vex'
[DataShipsx.vex]

$ bash -c 'args=(Data\Ships\x.vex); printf "[%s]\n" "${args[@]}"'
[DataShipsx.vex]
```

There is no error. The path simply arrives with its separators removed, and the
failure surfaces much later as "no such entry", which reads like a wrong path
rather than a mangled one.

## Which recipes are affected

- **`view`** (`justfile:436`) - `cargo run -q -p oag-view -- {{ARGS}}`, a plain
  recipe, so `just` runs it through `sh` and the first form above applies. This
  is the one that was reported, via `just view <image>:<path> --mesh ...`.
- **`play`** (`justfile:268`) - a `#!/usr/bin/env bash` recipe whose first line is
  `args=({{ARGS}})`, the second form above. Same deletion.
- Every other `*ARGS` passthrough recipe has the same shape: `unpack`, `wad`,
  `launch`, `appimage`, `deploy-deck` and friends. `unpack` and `wad` take
  WAD-internal paths routinely, so they are as exposed as `view` is.

**A quoted argument does not save you** in the plain-recipe case, because the
quotes are consumed by the caller's shell before `just` ever sees them, and what
`just` interpolates is the already-unquoted word.

## The workaround that works today

Run the binary directly and let your own shell's quoting reach it:

```sh
cargo run -q -p oag-view -- 'data/images/pulse-psp-eu.chd:Data\Ships\Feisar\ship.vex' --mesh ...
```

Single quotes, so the backslashes survive. This is what to use until the recipes
are fixed, and it is worth knowing regardless: it is also how to pass a path
containing a space.

## Open

- Whether any *committed* documentation, script or test invokes one of these
  recipes with a backslash path and has therefore been silently broken - not
  swept. `docs/` examples are the likely place.
- Whether `oag-wad`/`oag-assets` should additionally accept `/` as a separator
  and normalise, which would make the whole class of mistake harmless rather than
  merely fixed in the recipes. That is a design call, not a bug fix, and it has a
  real argument against it: the disc's own separator is the backslash, and
  accepting both makes a path that works in one place and not another.

## Next Steps

1. Quote the interpolation in the affected recipes. For the plain recipes that is
   `cargo run -q -p oag-view -- "{{ARGS}}"` only if a single argument is intended -
   which it is not, since `--mesh` follows the path - so the honest fix is to give
   each affected recipe a `#!/usr/bin/env bash` body and build the array with
   `args=("${@}")`-style handling, or to switch the passthrough to `just`'s own
   `positional-arguments` setting, which makes `$@` available and quoted.
   `positional-arguments` is the smallest change that fixes every recipe at once
   and should be tried first.
2. Add one regression test that a WAD-internal path survives a recipe - the
   cheapest form is a recipe that echoes its argument and a test asserting the
   backslashes are still there.
3. Sweep `docs/` for backslash paths inside `just` invocations and correct them to
   whichever form ends up working.
