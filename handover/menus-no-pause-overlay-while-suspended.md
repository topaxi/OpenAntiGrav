# Menus: no pause overlay while a race is suspended

The shell navigates and every row is live ([menus.md](../docs/architecture/menus.md)),
including rebinding - see below.

**Escaping a race now suspends it rather than discarding it, as long as it has
not finished.** `Session::open_menus` parks the outgoing `Stage::Race` in the
new `Session::suspended_race` instead of letting the stage swap drop it, and
backing all the way out of the menus (`MenuEvent::Closed` at the root page,
with nothing behind it in the tree) reaches `Session::resume_race` in place of
quitting - the fourth landing of `escape`'s own "one rule, three places it
lands" rule, documented on `Session::escape` itself. A race that *has*
finished is still discarded on the way to the results table: there is no
track behind that screen to resume into. See
`crates/game/src/main/session/menus.rs`.

**Rebinding landed** (this thread's original open item, formerly titled "the
one thing that does not work"): `oag_input::bindings::Bindings` replaces the
hardcoded `oag_input::keys::map_key` match with a permutation over its closed
eighteen-key candidate set, persisted complete in `[controls] bindings`, and
`Session::maybe_begin_binding` / `crate::rebind::decide` (in
`crates/game/src/main/`) wire a `binding` row's confirm press into a raw-key
capture. Full account, including the three design choices (table location,
conflict-is-a-steal, unknown-entry fallback) and what it deliberately does
not cover - the original's own `Control_Type = "custom"` action-to-button
mapping - in `docs/architecture/menus.md`'s new "Rebinding" section.

One thing a live game still needs is not there:

- **No pause overlay while suspended.** Backing into the menus over a parked
  race draws the ordinary menu screen, not a translucent pause layer over the
  frozen picture - a real "paused" look is a rendering feature of its own, not
  part of what dropping-vs-parking the `World` needed.

## Open

- No pause overlay while a race is suspended behind the menus

## Next Steps

- Draw a translucent layer over the frozen race picture instead of the
  ordinary menu background when `Session::suspended_race` is `Some` - a
  rendering feature over `MenuStage`, not a change to what the `World`'s
  lifetime already does correctly.
