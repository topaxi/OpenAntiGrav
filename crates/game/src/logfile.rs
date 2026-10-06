//! What a run writes at the top of its log file, so a launch the player cannot
//! see the terminal of - Steam swallows stderr - is still diagnosable.
//!
//! The sink itself is `oag_log`; this is the game-specific part: the build, the
//! source and the launcher environment. Written to the file only, never to the
//! terminal, so the terminal's output is as it was.

use std::path::Path;

/// The header lines, reading the environment through `env` and asking `exists`
/// whether a path is there (both injected, like `oag_input::prompt::Launch::read`).
///
/// This reports the raw variables; `oag_input::prompt::Launch` is what *decides*
/// from them, and this does not repeat that. `source` is what the command line,
/// settings or `$OAG_IMAGE` named, which on the launcher route is nothing yet:
/// the resolved image arrives as the ordinary `disc image: ...` line.
#[must_use]
pub fn header(
    source: Option<&str>,
    env: &dyn Fn(&str) -> Option<String>,
    exists: &dyn Fn(&Path) -> bool,
) -> Vec<String> {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let var = |key: &str| env(key).map_or_else(|| "unset".to_string(), |v| format!("{v:?}"));
    let info = env("SteamVirtualGamepadInfo").filter(|path| !path.is_empty());
    let info_line = match &info {
        None => "unset".to_string(),
        Some(path) => format!(
            "{path:?} ({})",
            if exists(Path::new(path)) {
                "exists"
            } else {
                "missing"
            }
        ),
    };
    vec![
        format!(
            "oag-game {} ({profile} build, git {}) on {} {}",
            env!("CARGO_PKG_VERSION"),
            env!("OAG_GIT_HASH"),
            std::env::consts::OS,
            std::env::consts::ARCH
        ),
        format!(
            "source: {}",
            source.map_or_else(
                || "none named yet (searched or chosen later)".to_string(),
                |s| format!("{s:?}")
            )
        ),
        format!(
            "steam: SteamDeck={} SteamAppId={} SteamGameId={} SteamVirtualGamepadInfo={info_line}",
            var("SteamDeck"),
            var("SteamAppId"),
            var("SteamGameId")
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_steam_launch_names_its_variables_and_the_pad_file() {
        let env = |key: &str| match key {
            "SteamDeck" => Some("1".to_string()),
            "SteamAppId" => Some("480".to_string()),
            "SteamVirtualGamepadInfo" => Some("/run/pads".to_string()),
            _ => None,
        };
        let lines = header(Some("a.chd"), &env, &|p| p == Path::new("/run/pads"));
        let steam = lines.last().unwrap();
        assert!(
            steam.contains("SteamDeck=\"1\"") && steam.contains("SteamAppId=\"480\""),
            "{steam}"
        );
        assert!(steam.contains("SteamGameId=unset"), "{steam}");
        assert!(steam.contains("\"/run/pads\" (exists)"), "{steam}");
        assert!(lines[0].contains(env!("OAG_GIT_HASH")));
        assert!(lines[1].contains("\"a.chd\""));
        let missing = header(None, &env, &|_| false);
        assert!(missing.last().unwrap().contains("(missing)"));
        assert!(missing[1].contains("none named yet"));
    }
}
