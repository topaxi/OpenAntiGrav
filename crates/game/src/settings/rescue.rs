//! Fallback to default on unparsable values in a settings table.

use super::Settings;

pub(super) struct RejectedValue {
    pub value: toml::Value,
    pub path: Vec<String>,
    pub message: String,
}

impl std::fmt::Display for RejectedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} = {}: {}",
            self.path.join("."),
            self.value,
            self.message
        )
    }
}

/// What survived a settings table that does not deserialise as a whole.
pub(super) struct Rescued {
    pub settings: Settings,
    pub rejected: Vec<RejectedValue>,
}

pub(super) fn rescue(table: &toml::Table) -> Rescued {
    let mut built = toml::Table::try_from(Settings::default())
        .expect("the defaults are serialisable, `load_from` writes them");
    let mut rejected = Vec::new();
    let mut leaves = Vec::new();

    collect(table, &mut Vec::new(), &mut leaves);

    for (path, value) in leaves {
        let mut candidate = built.clone();
        set(&mut candidate, &path, value.clone());
        match candidate.clone().try_into::<Settings>() {
            Ok(_) => built = candidate,
            Err(why) => rejected.push(RejectedValue {
                value,
                path,
                message: why.message().to_string(),
            }),
        }
    }

    let settings = built
        .try_into()
        .expect("every step above deserialised, and the defaults do");

    Rescued { settings, rejected }
}

fn collect(
    table: &toml::Table,
    path: &mut Vec<String>,
    leaves: &mut Vec<(Vec<String>, toml::Value)>,
) {
    for (key, value) in table {
        path.push(key.clone());
        match value {
            toml::Value::Table(inner) => collect(inner, path, leaves),
            _ => leaves.push((path.clone(), value.clone())),
        }
        path.pop();
    }
}

fn set(table: &mut toml::Table, path: &[String], value: toml::Value) {
    let Some((key, rest)) = path.split_first() else {
        return;
    };

    if rest.is_empty() {
        table.insert(key.clone(), value);
        return;
    }

    let entry = table
        .entry(key.clone())
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));

    if !entry.is_table() {
        *entry = toml::Value::Table(toml::Table::new());
    }

    if let Some(inner) = entry.as_table_mut() {
        set(inner, rest, value);
    }
}
