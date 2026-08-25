# The airbrake flap rotation axis is chosen, not recovered

The flaps deploy, and the reason to know this before retrying: the node's payload is zero bytes, its class descriptor carries no handler pointer, no `0x3c5` immediate exists in the binary, and `Vex_FindClassDescriptor`'s only two callers are load-time. **There is no per-class function to decompile** - the way in is a live read under PPSSPP, watching which node matrix moves when the airbrake goes down.

## Open

- The flap rotation axis is chosen, not recovered from data - there is no per-class function to decompile

## Next Steps

- Do a live read under PPSSPP: watch which node matrix moves when the airbrake deploys
