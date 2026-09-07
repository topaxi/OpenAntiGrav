# The hover probe takes only the deepest hit

Eight probes, one contact, and no `cross(r, impulse)` angular response. [`oag-trace.md`](../../docs/tools/oag-trace.md) dates when it bites: the two sides agree to a unit for the first 75 ticks, and what fails afterwards is *staying on the track* - `grounded` dropping to `0.5` and then `0` while the original never leaves `1.0`. That is the named next piece of contact work.

## Open

- Only the deepest hit among eight probes is used, with no `cross(r, impulse)` angular response
- After roughly 75 ticks, `grounded` drops to `0.5` and then `0` while the original never leaves `1.0`

## Next Steps

- Fix the hover probe's contact resolution (multi-contact handling plus angular response) - named as the next piece of contact work
