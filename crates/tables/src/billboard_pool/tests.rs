use super::*;

fn entry(name: &str, kind: &str, colours: &str) -> Entry {
    Entry {
        name: name.to_string(),
        kind: kind.to_string(),
        location: format!("/Data/{name}.vex"),
        colours: colour_mask(colours),
    }
}

/// The shuffle is fixed arithmetic: a three-entry pool walks the six swaps by
/// hand. Counter 1 swaps slots 1 and 0, 2 swaps 2 and 0, 3 swaps 0 and 0, 4
/// swaps 1 and 0, 5 swaps 2 and 0, 6 swaps 0 and 0.
#[test]
fn the_shuffle_is_the_engines_swap_loop() {
    let pool = Pool::new(vec![
        entry("a", "portrait", "red"),
        entry("b", "portrait", "red"),
        entry("c", "portrait", "red"),
    ]);
    let names: Vec<_> = pool.entries().iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["b", "c", "a"]);
}

/// Dropping the rotation makes two slots of one colour pick the same advert.
#[test]
fn a_drawn_entry_goes_to_the_back() {
    let mut pool = Pool::new(vec![
        entry("a", "portrait", "blue"),
        entry("b", "portrait", "blue"),
        entry("c", "landscape", "blue"),
    ]);
    let first = pool.draw("portrait", "blue").map(|e| e.name.clone());
    let second = pool.draw("portrait", "blue").map(|e| e.name.clone());
    assert_ne!(first, second);
    assert_eq!(pool.entries().last().map(|e| e.name.clone()), second);
}

#[test]
fn a_draw_needs_the_type_and_a_shared_colour() {
    let mut pool = Pool::new(vec![entry("a", "landscape", "red white")]);
    assert!(pool.draw("portrait", "red").is_none());
    assert!(pool.draw("landscape", "blue").is_none());
    assert_eq!(
        pool.draw("landscape", "white").map(|e| e.name.as_str()),
        Some("a")
    );
}

#[test]
fn colour_names_are_the_engines_bits() {
    assert_eq!(colour_mask("blue white"), 0x88);
    assert_eq!(colour_mask("orange white yellow"), 0xd0);
    assert_eq!(colour_mask("grey green"), 0x05);
}

/// Pulse's catalogue is shortened XML: `PI_Billboard`, `Values`, `type`,
/// `location` and `color` all arrive as one-letter codes.
#[test]
fn the_shortened_catalogue_reads_in_file_order() {
    let xml = br#"<code as="PI_Billboard" bs="Values" cs="name" ds="type" es="location" fs="color" gs="Screen"></code>
<g c="Top">
<a c="Portrait2"><b d="Portrait" e="Data\Billboards\ags.vex" f="red"></b></a>
<a c="Landscape1"><b d="Landscape" e="Data\Billboards\x.vex" f="blue white"></b></a>
</g>"#;
    let entries = parse(xml);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "Portrait2");
    assert_eq!(entries[0].kind, "portrait");
    assert_eq!(entries[0].location, "/Data/Billboards/ags.vex");
    assert_eq!(entries[1].colours, 0x88);
}

/// A colour slot with no quad still takes its entry: skipping it would shift
/// every later pick.
#[test]
fn every_colour_slot_draws_in_manifest_order() {
    let manifest = TrackStartup::parse(
        r#"<TrackStartup>
<Billboard num="3" type="portrait" color="blue"></Billboard>
<Billboard num="3" type="portrait" color="red"></Billboard>
<Billboard num="1" type="portrait" color="blue"></Billboard>
</TrackStartup>"#,
    );
    let fills = colour_fills(
        &manifest,
        vec![
            entry("a", "portrait", "blue"),
            entry("b", "portrait", "blue"),
        ],
    );
    let nums: Vec<u32> = fills.iter().map(|(n, _)| *n).collect();
    assert_eq!(nums, [3, 1]);
    assert_ne!(fills[0].1, fills[1].1);
}
