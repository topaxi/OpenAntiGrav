# Next drive lanes, as left by the 2026-10-09 drive

The 2026-10-09 `/oag-drive` session ended with an empty approved queue. These are
the candidate lanes it proposed and the decisions still open. None is approved
yet: propose them to the maintainer before spawning.

## Open

Candidate lanes, ranked by what a player notices:

1. **Firefox runs the web build at 9-14 fps** (sonnet or opus). Measured by
   `web-load` before and after threads (12.2 vs 12.4 fps), so threads are not
   the cause. Profile it. Evidence: [the web thread](the-web-build-races-in-chromium-and-firefox.md).
2. **The web build's remaining freezes** (sonnet): about 2 s at boot, 0.7-0.85 s
   at the end of an HD race load (GPU scene build, mostly `writeBuffer`), 0.5 s on
   escape to the menus. Spreading GPU uploads over frames is the likely fix. Same
   thread.
3. **HD's AI field scrapes Talon's Junction** (opus): 808 wall-contact ticks over
   five seeds against a target of 642 or less; the lone Ace is fixed. Evidence:
   [the HD handling thread](../gameplay/hd-handling-against-rpcs3.md) and
   `docs/gameplay/ai.md`.
4. **A vertical clip for menu drawing** (sonnet), so long menu pages scroll by
   the pixel under a finger instead of a whole row (`oag_ui::kinetic` exists).
   Evidence: [pointer input](../frontend/pointer-input-what-is-left.md).
5. ~~**CI chore**~~ taken and done (lane `ci-node24-ubuntu26`): actions bumped to
   their Node 24 majors (checkout v7, setup-python v7, upload-artifact v7,
   download-artifact v8, wrangler-action v4) and every Linux job pinned to
   `ubuntu-24.04`. See `docs/tools/releases.md`, "Runner images and action runtimes".

Decisions waiting on the maintainer:

- **ATRAC3+ decoder (deferred).** Pulse/Pure music (and their movie audio) needs
  it in the browser; natively it goes through `ffmpeg`. ADR-0024 forbids writing
  a codec we did not invent in this repo, so the options are: a separate crate
  ported from ffmpeg's decoder (LGPL-2.1+, so LGPL lands in shipped binaries),
  leaving it, or ffmpeg.wasm for the web only. Open questions: where the crate
  would live, and whether LGPL in the binaries is acceptable.
- **GitHub Pages is removed from the workflows but still live**: the maintainer
  disables it with `gh api -X DELETE repos/topaxi/OpenAntiGrav/pages`.

## Next Steps

1. Ask the maintainer which of lanes 1-5 to queue, and the ATRAC3+ answer.
2. Delete this file once each item is either spawned or moved into its own thread.
