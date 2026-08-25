# The `.vex` class-ID table's extent

All ~55 game classes are transcribed into [vex.md](../docs/formats/vex.md) and `vex::CLASS_NAMES`, self-validating at 95. The walk stopped at `0x08ab26a0` without reaching the `id == -1` terminator, so generic Maya classes past `0x3eb` are only partly covered. Cheap to finish; nothing depends on it.

## Open

- The walk stopped at `0x08ab26a0` without reaching the `id == -1` terminator.
- Generic Maya classes past `0x3eb` are only partly covered.

## Next Steps

- Finish walking the class-ID table to the terminator ("cheap to finish; nothing depends on it").
