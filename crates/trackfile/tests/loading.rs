//! `[loading]`: a track's own loading screen, in the manifest.

use oag_trackfile::fixture::oval;
use oag_trackfile::{Error, Loading, from_parts, to_parts};

fn with(loading: Loading) -> oag_trackfile::Track {
    let mut track = oval();
    track.loading = Some(loading);
    track
}

#[test]
fn a_loading_table_round_trips_and_the_schema_takes_it() {
    let loading = Loading {
        layout: Some("run.loading.toml".into()),
        caption: Some("ASTEROID RUN".into()),
        feature_image: Some("art/run.png".into()),
        feature_title: Some("NO WALLS".into()),
        feature_heading: Some("WARNING".into()),
        feature_text: Some("The outer ring has no barrier.".into()),
    };
    let parts = to_parts(&with(loading.clone()), "t.glb").expect("writable");
    assert!(parts.manifest.contains("[loading]"), "{}", parts.manifest);
    let loaded = from_parts(&parts.manifest, &parts.glb).expect("reads");
    assert_eq!(loaded.track.loading, Some(loading));
    assert!(loaded.unread.is_empty(), "{:?}", loaded.unread);
    oag_schema::validate::accepts("track", 1, &parts.manifest).expect("the schema takes [loading]");
}

#[test]
fn a_track_without_the_table_is_what_it_was() {
    let parts = to_parts(&oval(), "t.glb").expect("writable");
    assert!(!parts.manifest.contains("[loading]"));
    assert_eq!(
        from_parts(&parts.manifest, &parts.glb)
            .unwrap()
            .track
            .loading,
        None
    );
}

#[test]
fn a_file_outside_the_tracks_folder_or_an_empty_text_is_refused_by_name() {
    for (what, loading, wanted) in [
        (
            "parent",
            Loading {
                layout: Some("../x.toml".into()),
                ..Loading::default()
            },
            "loading.layout",
        ),
        (
            "absolute",
            Loading {
                feature_image: Some("/etc/x.png".into()),
                ..Loading::default()
            },
            "loading.feature_image",
        ),
        (
            "blank text",
            Loading {
                caption: Some("  ".into()),
                ..Loading::default()
            },
            "loading.caption is empty",
        ),
    ] {
        let written = to_parts(&with(loading), "t.glb").unwrap_err().to_string();
        assert!(written.contains(wanted), "{what}: {written}");
    }
    // the reader refuses what a hand-written manifest says
    let parts = to_parts(&oval(), "t.glb").unwrap();
    let hand = format!(
        "{}\n[loading]\nlayout = \"../x.toml\"\n",
        parts.manifest.trim_end()
    );
    let read = from_parts(&hand, &parts.glb).unwrap_err();
    assert!(
        matches!(&read, Error::Manifest(why) if why.contains("loading.layout")),
        "{read}"
    );
}

#[test]
fn a_key_this_reader_does_not_know_is_named_as_unread() {
    let parts = to_parts(&oval(), "t.glb").unwrap();
    let hand = format!(
        "{}\n[loading]\ncaption = \"X\"\nmusic = \"y.ogg\"\n",
        parts.manifest.trim_end()
    );
    let loaded = from_parts(&hand, &parts.glb).unwrap();
    assert!(
        loaded.unread.iter().any(|u| u.contains("loading.music")),
        "{:?}",
        loaded.unread
    );
}

#[test]
fn naming_a_feature_text_or_picture_makes_the_track_the_feature() {
    assert!(
        !Loading {
            caption: Some("X".into()),
            ..Loading::default()
        }
        .supplies_feature()
    );
    assert!(
        Loading {
            feature_text: Some("X".into()),
            ..Loading::default()
        }
        .supplies_feature()
    );
}
