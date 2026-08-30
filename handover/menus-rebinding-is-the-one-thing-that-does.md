# Menus: rebinding is the one thing that does not work

The shell navigates and every other row is live ([menus.md](../docs/architecture/menus.md)).

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

Two things a live game needs still are not there, deliberately left for
whoever pushes on this next rather than guessed at here:

- **Rebinding does not work in the menu shell** (the original open item below,
  untouched by the suspend/resume work).
- **No pause overlay while suspended.** Backing into the menus over a parked
  race draws the ordinary menu screen, not a translucent pause layer over the
  frozen picture - a real "paused" look is a rendering feature of its own, not
  part of what dropping-vs-parking the `World` needed.

## Open

- Rebinding does not work in the menu shell

## Next Steps

- Wire up rebinding: a binding table where `oag_input::keys::map_key`
  currently has a hardcoded `match`, plus persistence and conflict handling.
  See the `Entry::Binding` row in `crates/game/src/menu.rs`, which already
  draws what a button is bound to and says in its own doc comment that it is
  read-only until this lands.
