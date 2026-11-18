use super::lobby_name;
use crate::settings::Player;

fn player(name: &str) -> Player {
    Player {
        name: name.into(),
        tag: String::new(),
    }
}

#[test]
fn the_lobby_name_field_opens_on_the_stored_player_name() {
    assert_eq!(lobby_name("", &player("Zoe-7")), "zoe-7");
}

#[test]
fn the_player_name_wins_over_the_name_the_last_connection_used() {
    assert_eq!(lobby_name("bob", &player("Zoe")), "zoe");
}

#[test]
fn with_no_player_name_the_last_connections_name_is_kept() {
    assert_eq!(lobby_name("bob", &player("")), "bob");
}

#[test]
fn with_no_player_name_the_login_name_is_the_default() {
    assert_eq!(
        lobby_name("", &player("")),
        super::super::lan::player_name()
    );
}
