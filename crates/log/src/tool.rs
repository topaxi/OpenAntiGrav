//! The whole setup a command-line tool needs: install, resolve the file, attach.
//!
//! `oag-game` has a settings file and wires [`crate::install`] and
//! [`crate::attach`] itself. `oag-view` and `oag-trace` have none, so for them
//! the file is `--log-file`, else the default path for the tool's name, and the
//! file's filter is `OAG_LOG_FILE_FILTER` (`RUST_LOG` syntax), else
//! [`FILE_FILTER`].

use crate::path::{self, Env, Os};

/// The file's filter when nothing replaces it: the terminal's default with our
/// own crates at `debug`. Shared with `oag-game`'s `[log] filter` default.
pub const FILE_FILTER: &str = "warn,oag=debug,calloop=error";

/// The environment variable that replaces [`FILE_FILTER`] for a tool.
pub const FILTER_ENV: &str = "OAG_LOG_FILE_FILTER";

/// Where this run's log file goes.
#[derive(clap::Args, Debug)]
pub struct LogArgs {
    /// Write this run's log to FILE instead of the default
    /// (`$XDG_STATE_HOME/oag/logs/<tool>.log`). An empty value
    /// (`--log-file ''`) writes no file. The terminal output is unchanged.
    #[arg(long = "log-file", value_name = "FILE")]
    pub file: Option<String>,
}

/// The file a run of `app` writes: `--log-file`, else `<app>.log` in the
/// default directory; `None` for the empty value that turns it off.
#[must_use]
pub fn file_for(app: &str, args: &LogArgs, os: Os, env: &Env) -> Option<std::path::PathBuf> {
    path::resolve(args.file.as_deref(), None, path::default_path(os, env, app))
}

/// Installs the logger for the tool `app` (its default file is `<app>.log`)
/// and opens the file. Call once, right after parsing the command line.
///
/// An unwritable location is a `warn` on the terminal, not a failed run.
pub fn start(app: &str, terminal_default: &str, args: &LogArgs) {
    crate::install(terminal_default, FILE_FILTER);
    crate::install_panic_hook();
    let Some(file) = file_for(app, args, Os::here(), &Env::from_process()) else {
        crate::detach();
        return;
    };
    let filter = std::env::var(FILTER_ENV).ok();
    if let Err(why) = crate::attach(&file, filter.as_deref(), &[]) {
        log::warn!("not writing a log file at {}: {why}", file.display());
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn env() -> Env {
        Env {
            xdg_state_home: Some(PathBuf::from("/state")),
            ..Env::default()
        }
    }

    fn args(file: Option<&str>) -> LogArgs {
        LogArgs {
            file: file.map(str::to_owned),
        }
    }

    #[test]
    fn each_tool_gets_a_file_named_for_itself() {
        for app in ["oag-view", "oag-trace", "oag-game"] {
            assert_eq!(
                file_for(app, &args(None), Os::Linux, &env()),
                Some(PathBuf::from(format!("/state/oag/logs/{app}.log")))
            );
        }
    }

    #[test]
    fn the_flag_replaces_the_default_and_empty_turns_it_off() {
        assert_eq!(
            file_for("oag-view", &args(Some("/x/v.log")), Os::Linux, &env()),
            Some(PathBuf::from("/x/v.log"))
        );
        assert_eq!(
            file_for("oag-view", &args(Some("")), Os::Linux, &env()),
            None
        );
    }

    #[test]
    fn the_tools_parse_the_flag_through_clap() {
        use clap::Parser;
        #[derive(Parser)]
        struct Cli {
            #[command(flatten)]
            log: LogArgs,
        }
        let cli = Cli::try_parse_from(["t", "--log-file", ""]).unwrap();
        assert_eq!(cli.log.file.as_deref(), Some(""));
        assert!(Cli::try_parse_from(["t"]).unwrap().log.file.is_none());
    }
}
