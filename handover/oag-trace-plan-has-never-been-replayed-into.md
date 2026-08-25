# `oag-trace plan` has never been replayed into the emulator

The subcommand works in our simulation and `scripts/psp-autopilot.py` gained `--gate/--gate-dir/--gate-after` for the same target driven live. **The only thing that settles it**: `psp-trace.py --script verification/scenarios/talons-junction-pad-0.inputs --script-lead 2`, compared against `plan --trace-out`. Design and the full untested list: [`autopilot-planning.md`](../docs/tools/autopilot-planning.md). Watch the start-pose trap - `drive`'s default `Start Position` on `16_Track` is ~138 units behind the time-trial line.

## Open

- `oag-trace plan` has never been replayed into the emulator to confirm it against real PSP behaviour
- `drive`'s default Start Position on `16_Track` is ~138 units behind the time-trial line - a trap for anyone comparing traces

## Next Steps

- Run `psp-trace.py --script verification/scenarios/talons-junction-pad-0.inputs --script-lead 2` and compare against `plan --trace-out`
