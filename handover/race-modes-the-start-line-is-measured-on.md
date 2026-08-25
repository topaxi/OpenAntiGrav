# Race modes: the start line is measured on one circuit only

Time trial, speed lap and Zone run. `Course::START_LINE_OFFSET` is fitted on `16_Track`; the authored `Start Position` frame would replace it at confidence 88 across all 40. Two guards exist because both failed in practice: `RaceState::lap_gate` requires the near half *then* the far half of a lap (the ship spawns behind the line, so its first crossing is a wrap, and rocking over the line would otherwise record an unbeatable best lap), and lap 1's clock starts at the line rather than at the standing start.

## Open

- `Course::START_LINE_OFFSET` is fitted on `16_Track` only, not verified across the other 40 circuits
- Lap 1's clock starts at the line rather than at the standing start

## Next Steps

- Read the authored `Start Position` frame to replace `START_LINE_OFFSET` (confidence 88) across all 40 circuits
