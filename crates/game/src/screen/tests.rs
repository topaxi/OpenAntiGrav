//! What the catalogue is asserted to do: every built-in compiles, a player's
//! file shadows and outlives a built-in, and a saved edit is picked up.

use std::time::Duration;

use super::*;

/// A directory of its own under the system temp dir, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "oag-screen-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Self(dir)
    }

    fn write(&self, name: &str, body: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, body).expect("a scratch file");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const PASSTHROUGH: &str = "//! name = \"Mine\"\nfn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> { return frame_at(uv); }\n";

/// Pushes a file's modification time forward so a rewrite in the same tick
/// still reads as a change - the filesystem's clock is coarser than a test.
fn touch(path: &Path, seconds_ahead: u64) {
    let file = std::fs::File::options()
        .write(true)
        .open(path)
        .expect("the file");
    let when = std::time::SystemTime::now() + Duration::from_secs(seconds_ahead);
    file.set_modified(when).expect("a modification time");
}

#[test]
fn every_built_in_preset_parses_and_validates() {
    for (id, source) in BUILT_IN {
        let preset = Preset::parse(id, source).unwrap_or_else(|why| panic!("{id}: {why:#}"));
        preset
            .validate()
            .unwrap_or_else(|why| panic!("{id}: {why:#}"));
        assert_ne!(
            preset.name, preset.id,
            "{id} should name itself in its header"
        );
        assert!(
            !preset.description.is_empty(),
            "{id} should describe itself"
        );
    }
}

#[test]
fn every_built_in_id_is_its_file_stem() {
    // The id is spelled beside the `include_str!` rather than derived from it,
    // so this is the check that the two agree.
    let source = include_str!("../screen.rs");
    for (id, _) in BUILT_IN {
        assert!(
            source.contains(&format!("assets/shaders/screen/{id}.wgsl")),
            "{id} is not included from a file of that name"
        );
    }
}

#[test]
fn the_choices_lead_with_off_and_follow_the_shipped_order() {
    let catalogue = Catalogue::built_in();
    let choices = catalogue.choices();
    assert_eq!(choices[0].value, SCREEN_FILTER_OFF);
    let ids: Vec<&str> = choices[1..].iter().map(|c| c.value.as_str()).collect();
    let shipped: Vec<&str> = BUILT_IN.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, shipped);
    assert_eq!(choices[1].label, "PSP-3000 LCD");
    assert!(catalogue.get(SCREEN_FILTER_OFF).is_none());
    assert!(catalogue.get("psp-3000").is_some());
    assert!(catalogue.get("nothing-of-the-kind").is_none());
}

#[test]
fn a_user_file_joins_the_list_and_a_rewrite_bumps_its_revision() {
    let scratch = Scratch::new("user");
    let path = scratch.write("mine.wgsl", PASSTHROUGH);
    let mut catalogue = Catalogue::load(Some(scratch.0.clone()));

    let mine = catalogue.get("mine").expect("the user file").clone();
    assert_eq!(mine.name, "Mine");
    let first = mine.revision;
    assert!(
        catalogue
            .choices()
            .iter()
            .any(|c| c.value == "mine" && c.label == "Mine")
    );

    assert!(!catalogue.poll(), "nothing changed");

    std::fs::write(&path, PASSTHROUGH.replace("Mine", "Mine, edited")).expect("a rewrite");
    touch(&path, 5);
    assert!(catalogue.poll(), "the rewrite is a change");
    let edited = catalogue.get("mine").expect("still there");
    assert_eq!(edited.name, "Mine, edited");
    assert!(edited.revision > first);

    std::fs::remove_file(&path).expect("removed");
    assert!(catalogue.poll(), "the removal is a change");
    assert!(catalogue.get("mine").is_none());
}

#[test]
fn a_user_file_shadows_the_built_in_of_the_same_stem_and_falls_back_when_deleted() {
    let scratch = Scratch::new("shadow");
    let path = scratch.write("psp-3000.wgsl", PASSTHROUGH);
    let mut catalogue = Catalogue::load(Some(scratch.0.clone()));

    assert_eq!(catalogue.get("psp-3000").expect("shadowed").name, "Mine");
    // In the built-in's own place, not appended.
    let choices = catalogue.choices();
    assert_eq!(choices[1].value, "psp-3000");
    assert_eq!(choices[1].label, "Mine");
    assert_eq!(choices.iter().filter(|c| c.value == "psp-3000").count(), 1);

    std::fs::remove_file(&path).expect("removed");
    assert!(catalogue.poll());
    assert_eq!(
        catalogue.get("psp-3000").expect("the built-in again").name,
        "PSP-3000 LCD"
    );
}

#[test]
fn a_file_that_stops_compiling_keeps_its_last_good_revision() {
    let scratch = Scratch::new("broken");
    let path = scratch.write("mine.wgsl", PASSTHROUGH);
    let mut catalogue = Catalogue::load(Some(scratch.0.clone()));
    let good = catalogue.get("mine").expect("good").revision;

    std::fs::write(
        &path,
        "fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> { return +; }\n",
    )
    .expect("a rewrite");
    touch(&path, 5);
    assert!(
        !catalogue.poll(),
        "a broken rewrite is not a change the pass should see"
    );
    assert_eq!(
        catalogue.get("mine").expect("still the good one").revision,
        good
    );
}

#[test]
fn a_file_that_never_compiled_is_not_offered() {
    let scratch = Scratch::new("never");
    scratch.write("nope.wgsl", "this is not wgsl\n");
    scratch.write("notes.txt", PASSTHROUGH);
    let catalogue = Catalogue::load(Some(scratch.0.clone()));
    assert!(catalogue.get("nope").is_none());
    assert!(catalogue.get("notes").is_none());
    assert_eq!(catalogue.choices().len(), 1 + BUILT_IN.len());
}
