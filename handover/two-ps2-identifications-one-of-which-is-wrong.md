# Two PS2 identifications, one of which is wrong

`0x0015d058` zeroes the same four accumulators as the already-named `Body_ClearAccumulators` (`0x0015ca48`). Left unnamed per ADR-0005; cheap to resolve.

## Open

- `0x0015d058` zeroes the same four accumulators as the already-named `Body_ClearAccumulators` (`0x0015ca48`) - one of the two identifications is wrong, and it is unresolved which.

## Next Steps

- Determine which of `0x0015d058` / `0x0015ca48` is really `Body_ClearAccumulators` and name the other per ADR-0005.
