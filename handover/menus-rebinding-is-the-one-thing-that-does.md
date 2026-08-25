# Menus: rebinding is the one thing that does not work

The shell navigates and every other row is live ([menus.md](../docs/architecture/menus.md)). Separately, **escape backs out of a race into the menus, and that is deliberately *not* a pause menu**: the `World` is dropped rather than suspended, so re-entering loads a fresh race. Suspending it is the remaining work, and the menu stage already builds its own renderer so it can be opened from somewhere that is not the front end.

## Open

- Rebinding does not work in the menu shell
- Escaping a race drops the `World` instead of suspending it, so re-entering loads a fresh race rather than resuming

## Next Steps

- Suspend the `World` instead of dropping it when escaping into menus during a race (the menu stage already builds its own renderer, so it can be opened from outside the front end)
