---
status: ready-to-wire
titles: [2048, omega]
updated: 2026-10-27
summary: "ui's campaign map selects the first event by the literal name '2048 - Event 1'; take the start event from the campaign data"
size: XS (40)
difficulty: easy (40)
kind: debt
---

# The campaign map picks its first event by a 2048 name

2026-10-27. `crates/ui/src/frontend/campaign_map.rs:543` `set_campaign` selects the initial event with
`.position(|event| event.name == "2048 - Event 1").unwrap_or(0)`. A title's event name is data, and
`oag-ui` is a generic crate (ADR-0058: per-title behaviour is `Title` data). For any other campaign
the lookup silently falls back to index 0; for 2048 it equals index 0 only if the event list is
ordered that way. Tests in `crates/ui/src/frontend/tests/wipeout2048.rs` call `set_campaign`.

## Open

- Whether "Event 1" is simply the first element of every shipped list (then `0` is correct and the
  name can go) or the campaign data names a start event. Check `oag_tables` campaign data for 2048 and Omega.

## Next Steps

1. Read the 2048 and Omega event order; if index 0 always matches, drop the name test.
2. Otherwise pass the start event in from the caller.
3. Run `cargo nextest run -p oag-ui` and the 2048 campaign ground truth tests.
