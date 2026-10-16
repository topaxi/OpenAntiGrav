//! The `[online]` section: the lobby the ONLINE page last connected to and the name it went
//! by. Remembered automatically once a connection succeeds (modern saves: no save slot).

use serde::{Deserialize, Serialize};

/// What the ONLINE page asks for, kept so the next visit is OK, OK, OK.
///
/// **Ours, chosen, not measured.** Omega's PSN login has no counterpart here: a lobby is a
/// server address, the SHA-256 of its certificate (which `oag-server lobby` prints and writes
/// with `--hash-out`, and which changes on every restart of the server) and a name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Online {
    /// `host:port` of the lobby; the default port is added when none is typed.
    #[serde(default)]
    pub address: String,
    /// The lobby certificate's SHA-256 as 64 hex digits.
    #[serde(default)]
    pub hash: String,
    /// The name to go by in rooms; empty means the login name.
    #[serde(default)]
    pub name: String,
}
