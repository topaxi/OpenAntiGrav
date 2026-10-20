# ADR-0074: Curated screenshots may be published, game content may not be shipped

## Status

**Accepted (2026-10-20)** by the maintainer, proposed the same day.

It does not supersede [ADR-0006](0006-no-copyrighted-content.md), which stands
in full: no game content in the repository. It adds a separate, narrower
permission about what the project may show on its public pages, and the
[legal policy](../../overview/legal.md) is amended to say both.

## Context

[`docs/overview/legal.md`](../../overview/legal.md) says "No game content is in
this repository, and none ever will be." That sentence was written about
distribution, and ADR-0006 gives its reason: a stray asset in git history is
effectively irreversible. It was never meant to forbid showing the project
working, but read plainly it does, and it leaves the maintainer unable to put
the original beside this engine's output on a project page, which is the most
direct evidence that the reimplementation is faithful.

A screenshot is not an asset. It cannot be loaded by the original or by this
engine, it carries no model, table or sound, and one frame does not
reconstitute the work. Publishing it is commentary on behaviour, which is what
the rest of `docs/` already is.

### Precedent (background, not advice)

The following was supplied and discussed by the maintainer. It is **not legal
advice and has not been independently verified here**; no case details are
added to what the maintainer gave.

- OpenMW, OpenRCT2, OpenRA and DevilutionX show their engine running the
  original assets on their sites.
- phoboslab's `wipeout-rewrite` (2023) was written up with screenshots and
  video, and Sony did not act.
- Takedowns elsewhere (the SM64 port, AM2R, Pokémon fan games) are understood to
  have targeted distributed assets and builds, not screenshots.

This is context for a decision the maintainer takes, not a guarantee. A rights
holder can still object, which is why removal on request is part of the
decision.

## Decision

Curated, credited screenshots and comparison frames of the original games, and
of this engine running a player's own copy, **may be published on the project's
pages** (the project website and its documentation site), as illustration and
commentary on behaviour.

### Never

- Assets or extracted files of any kind, in any form.
- Bulk or automatic galleries. Every published image is picked by a person.
- Anything that reconstitutes the original: full-length video of a race,
  complete sets of frames that could be replayed as the game, texture sheets,
  model renders at working resolution, soundtracks.
- Any of the above in the repository or in a build. ADR-0006 and the
  repository rule in `legal.md` are unchanged.

### Credit

Each image names the title and platform it shows, says whether it is the
original or this engine, and carries the rights-holder statement (below) on its
page.

### Removal on request

A rights holder who asks for an image to be removed gets it removed, without
argument and usually within a few days, through the channel in the statement.
The same channel covers any other concern about the project.

## Mechanics

- **Published images never enter git.** `--screenshot` output and `data/` stay
  gitignored, `scripts/check-leakage.py` and its extension list are unchanged,
  and CI is not touched. The leakage check keeps meaning "no reproduction in the
  repository".
- **Publishing is a manual step** the maintainer runs from the workstation
  (a future `just publish-compare`), never from CI. CI has no disc images and
  must not gain a way to hold or upload them.
- **This ADR must be Accepted before that command is allowed to publish.** Until
  then the command, and the comparison page that feeds it, produce local output
  only.
- The comparison frames come from the tooling documented under `docs/tools/`;
  this ADR decides whether they may be published, not how they are made.

## Rights-holder statement

Published pages carry this text, and `legal.md` holds the same wording:

> **Screenshots and rights holders.** OpenAntiGrav is an independent,
> non-commercial reimplementation. It is not affiliated with or endorsed by Sony
> Interactive Entertainment, Studio Liverpool or any other rights holder;
> *Wipeout* and related names are trademarks of their owners. This project ships
> no game content. Some pages show screenshots of the original games and of this
> engine running a player's own copy, to document and compare behaviour; they are
> credited and shown for commentary. If you hold rights to anything shown here
> and want it removed, contact us through GitHub's private vulnerability
> reporting on this repository (fallback: open a GitHub issue). Name the page or
> image. We will take it down without argument, usually within a few days. The
> same channel works for any other concern about this project.

The maintainer chose the channel (private reporting, issue as fallback), chose
to publish no email address, and kept "within a few days" and the trademark
line.

## Alternatives considered

**Keep the rule as written and publish nothing.** Safest, and the project would
have no public evidence of fidelity beyond prose. Rejected by the maintainer's
wish to show comparisons.

**Commit the images to the repository or build them in CI.** Rejected. It puts
reproductions in git history, which ADR-0006 exists to prevent, and it would
force CI to hold disc images.

**Automatic galleries generated from every test run.** Rejected. Curation is
what keeps this commentary and not a mirror of the game.

## Consequences

**Good.** The project can show the engine next to the original, which is the
clearest argument that it is faithful. The repository rule stays absolute and
mechanically checked.

**Bad.** The permission rests on a judgement that curated screenshots are
commentary, supported only by precedent the maintainer supplied. A rights
holder may disagree, and the project then removes the image rather than
contests it. Removal only reaches pages the project controls; a copy someone
else saved stays. The leakage check cannot see published images, because they
are outside git, so keeping the "never" list honest rests on the maintainer's
curation by hand.

**Needs doing.** `legal.md` is amended (done alongside this ADR). Private
vulnerability reporting must be enabled on the GitHub repository for the
statement to be true; it was **off** when this was written, and enabling it is a
repository setting only the maintainer can change. Until it is on, the
statement's fallback (a GitHub issue) is the working channel and the wording
should be checked before anything is published.
