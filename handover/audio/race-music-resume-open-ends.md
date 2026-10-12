# Race music resume: what is left open

2026-10-12, lane `music-resume`. The playlist's place persists and resumes on all six
releases (`docs/architecture/race-music-resume.md`). Left over:

## Open

- The `--race` developer route (no front-end shell) neither restores nor saves; only
  menu-launched races do. A title key for it would come from `App::race_title`.
- The web build writes `music.toml` through `localStorage` like `records.toml`; not run in
  a browser here.
- The resume is asserted by tests on a null output (position, track); nobody has listened
  to it, by the project's no-audio-from-members rule.
- 10 s periodic save and the 0.5 s same-track tolerance are chosen, not measured.
- A race that ends on the last second of a track saves nothing usable (the second rounds
  to the track's end and the next start falls back to the normal place).

## Next Steps

- Maintainer check by ear on a real device: race 30 s, quit, relaunch, race.
