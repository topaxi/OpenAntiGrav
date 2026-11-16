//! A choice option worded from the string table: `Definition::relabel_choice_from`.

use super::*;
use crate::language::StringTable;

const SETTING: &str = "audio.spatial_audio";

fn labels(definition: &Definition) -> Vec<String> {
    definition
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .filter(|entry| entry.setting() == Some(SETTING))
        .filter_map(|entry| match entry {
            Entry::Choice { values, .. } => Some(values.iter().map(|c| c.label.clone())),
            _ => None,
        })
        .flatten()
        .collect()
}

#[test]
fn auto_takes_the_words_of_the_table_and_the_others_keep_theirs() {
    let mut definition = Definition::built_in(&StringTable::default()).expect("built-in menu");
    let before = labels(&definition);
    assert_eq!(before.len(), 4, "{before:?}");
    let mut strings = StringTable::default();
    strings.merge(
        [(
            "OAG_AUDIO_SPATIAL_AUTO".to_string(),
            "AUTOMATISCH".to_string(),
        )]
        .into_iter()
        .collect(),
    );
    definition.relabel_choice_from(SETTING, "auto", &strings, "OAG_AUDIO_SPATIAL_AUTO");
    let after = labels(&definition);
    assert_eq!(after[0], "AUTOMATISCH");
    assert_eq!(after[1..], before[1..]);
    definition.relabel_choice_from(SETTING, "auto", &StringTable::default(), "NO_SUCH_ID");
    assert_eq!(
        labels(&definition)[0],
        "AUTOMATISCH",
        "a missing id keeps the label"
    );
}
