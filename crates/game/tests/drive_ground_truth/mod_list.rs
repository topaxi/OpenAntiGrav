//! What the MODS list showed over the live switching run.

use serde_json::Value;

use super::{Run, text};

/// The MODS list's rows each time the state shows it: `(mod id, on or off)`, in order, one
/// entry per distinct listing.
fn mod_lists(run: &Run) -> Vec<Vec<(String, String)>> {
    let mut seen: Vec<Vec<(String, String)>> = Vec::new();
    for state in run
        .states
        .iter()
        .filter(|s| text(s, &["page"]) == Some("mods"))
    {
        let rows: Vec<(String, String)> = (state.pointer("/screen/rows"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|row| Some((text(row, &["id"])?.to_string(), text(row, &["value"])?)))
            .map(|(id, value)| (id, value.to_string()))
            .collect();
        if !rows.is_empty() && seen.last() != Some(&rows) {
            seen.push(rows);
        }
    }
    seen
}

/// Square turns Hillside off where it stands, and the next entry of the list puts it after
/// every mod that is on.
pub(super) fn check_the_list_order(run: &Run) {
    let lists = mod_lists(run);
    let ids = |list: &[(String, String)]| -> Vec<String> {
        list.iter().map(|(id, _)| id.clone()).collect()
    };
    let value = |list: &[(String, String)], mod_id: &str| {
        (list.iter().find(|(id, _)| id == mod_id)).map(|(_, value)| value.clone())
    };
    let off_at = lists
        .iter()
        .position(|list| value(list, "oag.hillside").as_deref() == Some("off"))
        .unwrap_or_else(|| panic!("Hillside never read off: {lists:?}"));
    assert!(off_at > 0, "{lists:?}");
    assert_eq!(
        ids(&lists[off_at]),
        ids(&lists[off_at - 1]),
        "toggled in place"
    );
    let on_first = |list: &[(String, String)]| {
        let first_off = list.iter().position(|(_, v)| v == "off");
        first_off.is_none_or(|at| list[at..].iter().all(|(_, v)| v == "off"))
    };
    assert!(on_first(&lists[0]), "{:?}", lists[0]);
    assert!(
        lists[off_at + 1..]
            .iter()
            .any(|list| value(list, "oag.hillside").as_deref() == Some("off") && on_first(list)),
        "entered again, Hillside is among the mods that are off: {lists:?}"
    );
}
