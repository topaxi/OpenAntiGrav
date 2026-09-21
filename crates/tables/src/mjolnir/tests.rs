use super::*;

/// A trimmed fixture in the real file's own shape: one plain field, one
/// polymorphic reference field whose `ARRAY` carries a concrete pointee
/// typedef different from the field's own declared type, and one field with
/// no `ARRAY` child at all (self-closed, the way an unauthored optional
/// field is stored).
const FIXTURE: &str = r#"<?xml version="1.0"?>
<mjolnir version="1">
<instance instanceid="-1143582041" typedefid="-1915183557" name="2048 - Event 3" schema="0" version="0" file="SP.xml"><DATA>
<M_TRACKDEF name="m_trackDef" type="TrackDefinition" length="1" typedefid="205052969"><ARRAY value="-1892961298" typedefid="205052969"/></M_TRACKDEF>
<M_PNEXTEVENT name="m_pNextEvent" type="GameModeBase" length="1" typedefid="366306753"><ARRAY value="-484309551" typedefid="-1353052320"/></M_PNEXTEVENT>
<M_NUMOFLAPS name="m_numOfLaps" type="int" length="1" typedefid="351272028"><ARRAY value="2" typedefid="351272028"/></M_NUMOFLAPS>
<M_PEVENTREQUIRED name="m_pEventRequired" type="GameModeBase" length="1" typedefid="366306753" value="0"><ARRAY value="" typedefid="366306753" /></M_PEVENTREQUIRED>
<M_CANVASTWEAK_X name="m_canvasTweak_x" type="int" length="1" typedefid="351272028" value="0" />
</DATA></instance>
<instance instanceid="-405306526" typedefid="205052969" name="Bridge" schema="0" version="0" file="SP.xml"><DATA>
<M_TRACKNAME name="m_trackName" type="char" length="64" typedefid="1380284284"><ARRAY value="bridge" typedefid="1380284284"/></M_TRACKNAME>
<M_DISPLAYNAME name="m_displayName" type="char" length="64" typedefid="1380284284"><ARRAY value="CAPITAL REACH" typedefid="1380284284"/></M_DISPLAYNAME>
</DATA></instance>
</mjolnir>
"#;

#[test]
fn parses_every_instance_in_document_order() {
    let doc = parse(FIXTURE);
    assert_eq!(doc.instances.len(), 2);
    assert_eq!(doc.instances[0].name, "2048 - Event 3");
    assert_eq!(doc.instances[1].name, "Bridge");
}

#[test]
fn instance_and_instance_named_find_by_id_or_name() {
    let doc = parse(FIXTURE);
    assert_eq!(
        doc.instance(-1143582041).map(|i| i.name.as_str()),
        Some("2048 - Event 3")
    );
    assert!(doc.instance(999).is_none());
    assert_eq!(
        doc.instance_named("Bridge").map(|i| i.instance_id),
        Some(-405306526)
    );
}

#[test]
fn by_typedef_filters_to_the_requested_shape() {
    let doc = parse(FIXTURE);
    let tracks: Vec<_> = doc.by_typedef(205052969).collect();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].name, "Bridge");
}

#[test]
fn typedef_counts_matches_a_hand_count() {
    let doc = parse(FIXTURE);
    let counts = doc.typedef_counts();
    assert_eq!(counts, vec![(-1915183557, 1), (205052969, 1)]);
}

#[test]
fn a_plain_field_reads_its_scalar_value() {
    let doc = parse(FIXTURE);
    let event = doc.instance(-1143582041).unwrap();
    assert_eq!(event.field("M_NUMOFLAPS").and_then(Field::int), Some(2));
    assert_eq!(
        event.field("m_numoflaps").and_then(Field::int),
        Some(2),
        "case-insensitive tag"
    );
}

#[test]
fn a_reference_field_carries_its_own_pointee_typedef_not_the_fields_declared_one() {
    let doc = parse(FIXTURE);
    let event = doc.instance(-1143582041).unwrap();

    let track_field = event.field("M_TRACKDEF").unwrap();
    assert_eq!(track_field.type_name, "TrackDefinition");
    assert_eq!(track_field.typedef_id, 205052969);
    let track_ref = track_field.reference().unwrap();
    assert_eq!(track_ref.instance_id, -1892961298);
    assert_eq!(track_ref.typedef_id, Some(205052969));

    // The field's own declared type is the abstract GameModeBase, but the
    // ARRAY names the concrete pointee - -1353052320, not 366306753.
    let next_field = event.field("M_PNEXTEVENT").unwrap();
    assert_eq!(next_field.type_name, "GameModeBase");
    assert_eq!(next_field.typedef_id, 366306753);
    let next_ref = next_field.reference().unwrap();
    assert_eq!(next_ref.instance_id, -484309551);
    assert_eq!(next_ref.typedef_id, Some(-1353052320));
}

#[test]
fn an_empty_array_value_is_not_a_reference() {
    let doc = parse(FIXTURE);
    let event = doc.instance(-1143582041).unwrap();
    assert_eq!(event.field("M_PEVENTREQUIRED").unwrap().reference(), None);
}

#[test]
fn a_self_closed_field_with_no_array_child_has_no_value() {
    let doc = parse(FIXTURE);
    let event = doc.instance(-1143582041).unwrap();
    let field = event.field("M_CANVASTWEAK_X").unwrap();
    assert_eq!(field.values.len(), 0);
    assert_eq!(field.value(), None);
    assert_eq!(field.int(), None);
}

#[test]
fn a_document_with_no_mjolnir_root_parses_empty() {
    let doc = parse("<not-mjolnir/>");
    assert!(doc.instances.is_empty());
}

#[test]
fn multiple_array_children_are_all_kept_in_order() {
    let xml = r#"<mjolnir><instance instanceid="1" typedefid="2" name="grid"><DATA>
<M_PGRIDSHIPMODELDATA name="m_pGridShipModelData" type="WOShipModelData" length="3" typedefid="520725191">
<ARRAY value="10" typedefid="520725191"/>
<ARRAY value="" typedefid="520725191"/>
<ARRAY value="30" typedefid="520725191"/>
</M_PGRIDSHIPMODELDATA>
</DATA></instance></mjolnir>"#;
    let doc = parse(xml);
    let field = doc.instances[0].field("M_PGRIDSHIPMODELDATA").unwrap();
    assert_eq!(field.values.len(), 3);
    let refs = field.references();
    assert_eq!(refs.len(), 2, "the empty middle slot is skipped");
    assert_eq!(refs[0].instance_id, 10);
    assert_eq!(refs[1].instance_id, 30);
}
