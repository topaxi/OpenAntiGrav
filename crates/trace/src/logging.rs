//! The logger `oag-trace` installs, in a file of its own.
//!
//! Belongs to the binary rather than to `oag_trace` the library: a library
//! never picks the sink, it only calls the `log` facade. `main.rs` declares it
//! with `mod logging;`, which is why it is not reachable from `lib.rs`.

/// Installs the sink every `log` call in this run ends up in, and opens the
/// log file (`oag-trace.log`; see `oag_log::tool`).
///
/// **Notes and warnings only.** Every *report* this tool produces - the CSV,
/// the summaries, the tables - is a `println!` on stdout, and is unaffected by
/// anything here or by `RUST_LOG`: a filter must not be able to silence the
/// thing the command was run for.
///
/// The default filter is `warn` globally with our own crates at `info`, and
/// the format carries the level and nothing else. Both match `oag-game`'s
/// `init_logging`, whose doc comment gives the reasoning for each.
pub fn start(args: &oag_log::tool::LogArgs) {
    oag_log::tool::start("oag-trace", "warn,oag=info", args);
}
