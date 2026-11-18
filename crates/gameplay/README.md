# oag-gameplay

The simulation: one `World` of plain data with fixed-size arrays and no ECS, an
input snapshot in, a state snapshot out, and the tick function between them.

It depends on no renderer, audio, window or network crate (dependency rule 1),
which is what lets a race run with no GPU and compare against a trace. See
[ADR-0003](../../docs/architecture/adr/0003-no-ecs.md).
