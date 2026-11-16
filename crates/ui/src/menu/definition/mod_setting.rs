//! The rule for a setting key a mod's row names (ADR-0082, decision 2).

use super::{Definition, Error};

impl Definition {
    /// Refuses a row of mod `mod_id` whose `setting` is not `mod.<mod_id>.<key>`: a mod
    /// saves under its own id only, never another mod's and never an engine setting.
    ///
    /// A mod id carries dots and a key never does, so the key is what follows the last
    /// dot of the setting.
    ///
    /// # Errors
    ///
    /// [`Error::Mod`] naming the mod, the row and the setting.
    pub fn check_mod_setting(mod_id: &str, row_id: &str, setting: &str) -> Result<(), Error> {
        let own = setting
            .strip_prefix("mod.")
            .and_then(|rest| rest.rsplit_once('.'))
            .is_some_and(|(id, key)| id == mod_id && !key.is_empty());
        if own {
            return Ok(());
        }
        Err(Error::Mod {
            mod_id: mod_id.to_string(),
            key: format!("entry {row_id:?} setting"),
            problem: format!(
                "{setting:?} is not mod.{mod_id}.<key>: a mod saves under its own id only"
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mod_may_name_only_its_own_keys() {
        assert!(Definition::check_mod_setting("oag.example", "r", "mod.oag.example.track").is_ok());
        for bad in [
            "mod.other.track",
            "mod.oag.example",
            "mod.oag.example.",
            "race.track",
            "mod.oag.example.sub.track",
        ] {
            let e = Definition::check_mod_setting("oag.example", "r", bad).expect_err(bad);
            let text = e.to_string();
            assert!(text.contains("oag.example") && text.contains(bad), "{text}");
        }
    }
}
