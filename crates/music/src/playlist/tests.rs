use super::*;

/// Shaped like `Data\Plugins\PI001\Definition.xml`, with the soundtrack beside
/// other nodes the reader must walk past.
const DEFINITION: &str = r#"
<Screen name="Top">
  <PI_Track name="16_Track">
    <Values type="Race" location="Data\Environments\16_Track"/>
  </PI_Track>
  <PI_Music name="A Piece Of Music">
    <Values location="Data\Music\SomeArtist"></Values>
    <Entry Artist="Some Artist"></Entry>
    <Entry Label="Some Label"></Entry>
  </PI_Music>
  <PI_Music name="Another Piece">
    <Values location="Data\Music\OtherArtist"></Values>
    <Entry Artist="Other Artist"></Entry>
    <Entry Label=""></Entry>
  </PI_Music>
</Screen>
"#;

/// A pack manifest: the same schema, holding no music.
const PACK_MANIFEST: &str = r#"
<Screen name="Top">
  <PI_Track name="99_Track">
    <Values type="Race" location="Data\Environments\99_Track"/>
  </PI_Track>
  <LoadXML><Values Src="download09.xml"/></LoadXML>
</Screen>
"#;

/// The soundtrack is declared beside the circuits, in the same schema and
/// in the order the disc's own music screen walks. **The fixture's titles
/// and artists are invented**, unlike its `16_Track` and `Assegai` ids,
/// which are folder names: an id is not shipped text and a song title is.
#[test]
fn the_soundtrack_is_declared_in_file_order_and_addressed_by_location() {
    let music = music(DEFINITION);
    assert_eq!(music.len(), 2, "{music:?}");

    assert_eq!(music[0].location, r"Data\Music\SomeArtist");
    assert_eq!(
        music[0].entry_name("music.at3"),
        r"Data\Music\SomeArtist\music.at3",
        "MusicManager.cpp's own `%s\\%s` join"
    );
    assert_eq!(music[1].location, r"Data\Music\OtherArtist");

    // An `Entry` sitting beside `Values` must not be mistaken for one: the
    // node's own `name` is the id, and `Values` is the only child read.
    assert_ne!(music[0].id, music[1].id);
}

/// A definition with no `PI_Music` at all yields nothing rather than
/// guessing - the case a caller distinguishes to fall back on finding the
/// entries by what they are.
#[test]
fn a_definition_that_declares_no_music_yields_none() {
    assert!(music(PACK_MANIFEST).is_empty());
}
