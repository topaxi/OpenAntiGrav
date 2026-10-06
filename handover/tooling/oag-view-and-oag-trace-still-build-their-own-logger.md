# oag-view and oag-trace still build their own logger

`oag-game` logs through `oag-log` (`crates/log`): the terminal as it always was
plus an appended, pruned log file (`docs/architecture/logging.md`, "The log
file"). `oag-view` (`crates/view/src/logging.rs`) and `oag-trace`
(`crates/trace/src/logging.rs`) still call `env_logger` directly, so a launch of
either under a launcher that swallows stderr leaves no file.

## Open

- Move both onto `oag_log::install` / `attach`. Each needs its own app name for
  the default path (`oag_log::path::default_path(os, env, "oag-view")`) and
  probably its own `[log]` handling or none: they read no settings file today,
  so `--log-file` and the default path are the whole surface. Their terminal
  defaults (see the two `logging.rs` files) must carry over unchanged.
- Two processes appending to `oag-game.log` is safe line by line; the startup
  prune can lose a line another process appended between its read and rename.
  Left, because the only line at risk is a startup line.
- `Tee::flush_file` waits one second at most; a writer stuck on a hung network
  mount delays exit by that, never the frame loop.

## Next Steps

1. Replace the two `init_logging` bodies with `oag_log::install(...)`.
2. Drop `env_logger` from `oag-view` and `oag-trace`, and from the workspace
   `Cargo.toml`'s comment block if `oag-log` is then its only user.
