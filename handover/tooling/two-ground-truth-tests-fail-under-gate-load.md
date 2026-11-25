---
status: ready-to-wire
titles: [pulse, pure]
updated: 2026-11-25
summary: "online two_rooms_race... and ppsspp_pack pure_usa_keys fail under gate load and pass alone; make them load-robust"
categories: [tooling]
size: S (40)
difficulty: medium (40)
---

# Two ground-truth tests fail under gate load

## Open

On 2026-11-25 these failed in full gates or member gates under load (load average 38 to 60) and passed when rerun alone:

- `oag-net::it online::two_rooms_race_one_after_the_other_through_one_slot_and_return_to_their_rooms` (`crates/net/tests/online.rs:259`): `left: Some(Open)`, `right: Some(Queued { position: 1 })`. The second room found the slot open where the test expects it queued, so a wall-clock wait in the test or the server likely let the first race finish first. Seen once in the lead's batch gate on 4ef97a017; it passed alone in 6.5 s.
- `oag-texture ppsspp_pack_ground_truth::pure_usa_keys`: seen twice in member gates (`split-hotspot-files`, `auto-reflection`); it passed alone both times.

## Next Steps

1. For each test, find the wall-clock dependence (sleeps, timeouts, `Instant` deadlines, or a race between two threads). Make it wait on the state it asserts (a condition or a bounded poll with a generous ceiling), not on time.
2. Prove it under load: run each 20 times while `stress-ng --cpu $(nproc)` (or a parallel `cargo build`) loads the machine, before and after. The before must fail at least once, and the after never.
3. If a test cannot be made robust, give it its own nextest test group with `max-threads = 1`, or a retry with a comment naming why, and say so here.
