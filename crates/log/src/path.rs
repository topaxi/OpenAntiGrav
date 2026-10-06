//! Where the file lives, and which of the three ways to say so wins.

use std::path::{Path, PathBuf};

/// The platforms with their own convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    MacOs,
    Windows,
}

impl Os {
    /// The one this binary was built for.
    #[must_use]
    pub const fn here() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else if cfg!(windows) {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

/// What the default location is read from, injected so a test needs no
/// process environment.
#[derive(Debug, Clone, Default)]
pub struct Env {
    /// `$XDG_STATE_HOME`; the spec ignores a relative one.
    pub xdg_state_home: Option<PathBuf>,
    pub home: Option<PathBuf>,
    /// The per-user local application data directory (Windows).
    pub local_app_data: Option<PathBuf>,
}

impl Env {
    /// The running process's.
    #[must_use]
    pub fn from_process() -> Self {
        Self {
            xdg_state_home: std::env::var_os("XDG_STATE_HOME").map(PathBuf::from),
            home: dirs::home_dir(),
            local_app_data: dirs::data_local_dir(),
        }
    }
}

/// The default file for `app`: `$XDG_STATE_HOME/oag/logs/<app>.log` on Linux
/// (`~/.local/state` when unset), `~/Library/Logs/oag/` on macOS, the local
/// application data directory's `oag\logs` on Windows. `None` when the
/// platform names no home to put it under.
#[must_use]
pub fn default_path(os: Os, env: &Env, app: &str) -> Option<PathBuf> {
    let file = format!("{app}.log");
    match os {
        Os::Linux => {
            let state = env
                .xdg_state_home
                .clone()
                .filter(|dir| dir.is_absolute())
                .or_else(|| env.home.as_ref().map(|home| home.join(".local/state")))?;
            Some(state.join("oag").join("logs").join(file))
        }
        Os::MacOs => Some(env.home.as_ref()?.join("Library/Logs/oag").join(file)),
        Os::Windows => Some(
            env.local_app_data
                .as_ref()?
                .join("oag")
                .join("logs")
                .join(file),
        ),
    }
}

/// Which file a run writes: the command line's, else the settings file's, else
/// `default`. **An empty value at either level means no file at all**, which is
/// the disable switch, so there is no second option for it. `None` is "write
/// nothing".
#[must_use]
pub fn resolve(
    cli: Option<&str>,
    setting: Option<&str>,
    default: Option<PathBuf>,
) -> Option<PathBuf> {
    match cli.or(setting) {
        Some("") => None,
        Some(path) => Some(Path::new(path).to_path_buf()),
        None => default,
    }
}
