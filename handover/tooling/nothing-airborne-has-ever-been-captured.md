# Nothing airborne has ever been captured

`grip_air`, the airborne pitch gain and the `-0.3` airborne weathervane have no runtime leg at all. One capture with a real jump in it closes several at once.

## Open

- `grip_air` has no runtime leg
- The airborne pitch gain has no runtime leg
- The `-0.3` airborne weathervane term has no runtime leg

## Next Steps

- **`13_Track`, the circuit with the clean authored jump, is not reachable from a fresh profile - measured, not assumed.** 2026-08-26. `Data\Plugins\PI001\Definition.xml`'s `PI_Track name="13_Track"` carries `<Unlock Grid="Grid6">`, and Race Campaign's own progression screen showed only Grid 1 of 16 unlocked, needing 12 of 24 points to open Grid 2 - so five more grids stand between a fresh profile and 13_Track. Reaching it means either grinding five Race Campaign grids first (real races, real placements, not a Time Trial capture) or editing the profile's own progression state, neither of which is a next step to take unattended.
- **RaceBox's own Custom Race → Track Select is not a free track picker.** Confirmed live (PPSSPP SDL under Xvfb, `47811`/websocket debugger, screenshots via `magick import -window root`): it is a wrapping list of exactly **three** entries, moved with **up/down**, not left/right (`docs/reverse-engineering/ppsspp-debugger.md` already said so; this session re-derived it by watching `right` do nothing). Those three are exactly the `PI_Track` entries with **no `<Unlock>` element at all** - `16_Track` (Talon's Junction, always first), `03_Track` (Moa Therma) and `18_Track` (Metropia reversed) - everything else needs its own `<Unlock Grid="N">` cleared first. None of the three free tracks has a clean authored jump: `16_Track` and `03_Track` are two of the three circuits `race_ground_truth::the_racing_line_has_track_under_it_where_it_is_known_to` measured with **zero** unsupported line samples, and `18_Track`'s non-reversed pair (`02_Track`) is the ambiguous "racing line running above the track" case, not the "nothing within sixty reaches" case `13_Track` is.
- **The nearer target is `10_Track`, not `13_Track`.** Its own `<Unlock Grid="Grid0">` is the shallowest gate of any locked track (`Grid0` is the currently-active grid, not a future one), and `ai.md`'s own table already flags 40 of its 72 unsupported-line samples as the same "nothing within sixty reaches" shape as `13_Track`'s jump - a partial rather than a clean case, but real and much closer. Reaching it still needs at least one Race Campaign cell cleared (a real race, opponents and weapons on, placed well enough for the cell's medal), which `scripts/psp-autopilot.py` was not built or tested for - it flies a Time Trial line, not a contested race.
- Whichever track is reached, the loop is: `oag-trace track --source <image> --track 'Data\Environments\<NN>_Track\track.vex'` for the spline (accepts any track, not just the default), `scripts/psp-autopilot.py --spline ... --trace-out data/traces/<name>.csv` for the capture, then read `grip_air`/the pitch gain/the weathervane term off the airborne ticks the same way `docs/physics/cornering-ground-truth.md`'s `scripts/trace-cornering.py` reads the grounded ones.
