//! The "mjolnir" typed-instance database: `data/xml/SP.xml` and `data/xml/MP.xml`
//! in Wipeout 2048's `PSP2/data.psarc`. `data/xml/MjolnirData.xml` beside them
//! (212 bytes) is only the tool's `<WORKSPACES>` list (`SP.xml`/`MP.xml`/
//! `Profile.xml`); it names no schema and this crate reads nothing from it.
//!
//! No other title ships this format: it is 2048's own campaign, not the
//! `PI_Grid`/`PI_Cell` grid of Pulse, Pure and HD ([`crate::race_campaign`]).
//! [`campaign`] is the typed view over this generic reader.
//!
//! # Shape
//!
//! ```xml
//! <mjolnir>
//!   <instance instanceid="-1143582041" typedefid="-1915183557" name="2048 - Event 3" schema="0" version="0" file="SP.xml">
//!     <DATA>
//!       <M_TRACKDEF name="m_trackDef" type="TrackDefinition" length="1" typedefid="205052969" ...>
//!         <ARRAY value="-1892961298" typedefid="205052969"/>
//!       </M_TRACKDEF>
//!       ...
//!     </DATA>
//!     <BASE>...</BASE>
//!     <USER>...</USER>
//!   </instance>
//!   ...
//! </mjolnir>
//! ```
//!
//! Every instance is a typed record: `typedefid` names its shape, and each
//! `<M_FOO>` under `<DATA>` is a field carrying its declared type (`type=`/
//! `typedefid=`) and one or more `<ARRAY>` values. `length=` is the capacity,
//! not the `<ARRAY>` count: most fields carry one, and only a handful of `2048 -
//! Event *` instances' `M_PGRIDSHIPMODELDATA` author all seven.
//!
//! The reader is as forgiving as [`crate::fexml::parse`]: an unrecognised field
//! or instance is skipped, so an unmeasured schema still comes through as raw
//! instances and fields.
//!
//! # The `type=`/`typedefid=` pair is the schema's own name table
//!
//! **The strongest evidence here, and not inferred.** Every field pointing at
//! another instance (`M_TRACKDEF type="TrackDefinition" typedefid="205052969"`,
//! ...) carries the referenced typedef's name in the file. Collecting every
//! `(typedefid, type)` pair across all 288 `SP.xml` instances gives a closed
//! table with no ambiguity:
//! | `typedefid` | name | instances |
//! | --- | --- | --- |
//! | `380278911` | `GameModeObjective` | 96 |
//! | `366306753` | `GameModeBase` | 0 direct (abstract - every event instance is one of the four typedefs below, never this one; only a polymorphic reference field like `M_PNEXTEVENT` declares this as its *static* type) |
//! | `520725191` | `WOShipModelData` | 21 |
//! | `205052969` | `TrackDefinition` | 10 |
//! | `-966434245` | `WeaponSetDefinition` | 20 |
//! | `-1934651500` | `eClass` (an enum, not an instance type - never itself an instance) | - |
//! | `-1959127439` | `bool` | - |
//! | `-1854316044` | `u32` | - |
//! | `351272028` | `int` | - |
//! | `-260549001` | `float` | - |
//! | `1380284284` | `char` | - |
//! | `-1588746963` | `GameModeOptions` | - |
//! | `-1134089044` | `CanvasButtonShape` | - |
//! | `-92967649` | `WOShipCreatorParams` | - |
//! | `-66037811` | `ObjectiveValue` | - |
//! | `856427073` | `I_UnlockData` | - |
//! | `1822158844` | `ActiveWeaponPads` | - |
//! | `2139957613` | `WeaponType` | - |
//! | `-2047583099` | `ObjectiveOptions` | - |
//!
//! Confidence **95** for every row: the file's own text, cross-checked against
//! every field that names it. **This corrects a prior guess** that the twenty
//! single-field `-966434245` instances were `TrackDefinition` ("2048 ships 20
//! track profiles"): they are `WeaponSetDefinition` (`"Rockets Only"`,
//! `"Cannons and Missile"`, ...), and `205052969` (ten instances, `M_TRACKNAME`
//! matching `data/plugins/tracks/Definition.xml`'s ten `<PI_Track name="...">`
//! stems: `square`, `park`, `tower`, `mall`, `bridge`, `arena`, `subway`,
//! `cathedral`, `sol`, `altima`) is `TrackDefinition`.
//!
//! # Four typedefs carry no name in the file
//!
//! `-1915183557` (53 instances), `-1353052320` (52), `1311982788` (26) and
//! `1018671239` (10), the four concrete event shapes, are never a field's
//! declared type: every field pointing at one (`M_PNEXTEVENT`,
//! `M_PBRANCHEVENT`, `M_PEVENTREQUIRED`, `M_PCAMPAIGNUNLOCK`) declares the
//! abstract `GameModeBase`. **The `<ARRAY>` element still carries the pointee's
//! concrete `typedefid`** (`<ARRAY value="-484309551" typedefid="-1353052320"/>`
//! on `2048 - Event 3`'s `M_PNEXTEVENT`), so [`Reference::typedef_id`] recovers
//! it for every edge without a lookup.
//!
//! **2026-09-28: named after all, by hash.** `eboot.elf` carries six
//! `GameMode_*` class names; `oag_formats::wad::hash_name` of each (the
//! case-folded CRC-32 the five in-file names confirm) lands on four `SP.xml`
//! ids with zero misses across ten names: `GameMode_SpeedLapRace`
//! `-1915183557`, `GameMode_ArcadeRace` `-1353052320`, `GameMode_EliminatorRace`
//! `1311982788`, `GameMode_ZoneRace` `1018671239`. `GameMode_CheckPointRace` and
//! `GameMode_ZombieRace` hash to ids `SP.xml` never instantiates. See
//! [`campaign`]'s `typedef` module.
//!
//! # `288` total
//!
//! `380278911`\*96 + `-1915183557`\*53 + `-1353052320`\*52 + `1311982788`\*26 +
//! `520725191`\*21 + `-966434245`\*20 + `1018671239`\*10 + `205052969`\*10 = `288`,
//! the `SP.xml` instance count. `crates/tables/src/mjolnir/tests.rs` and the
//! `#[ignore]`d test in `crates/2048/tests/` assert it, so a patched or regional
//! `SP.xml` that changes the shape fails loudly.

use crate::fexml::{self, Node};

/// A parsed mjolnir document: every `<instance>` under `<mjolnir>`, in document
/// order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Document {
    /// Every instance, in the order the file authors them.
    pub instances: Vec<Instance>,
}

impl Document {
    /// An instance by `instanceid`; `None` for a dangling or unauthored reference
    /// (see [`Reference`]).
    #[must_use]
    pub fn instance(&self, instance_id: i64) -> Option<&Instance> {
        self.instances.iter().find(|i| i.instance_id == instance_id)
    }

    /// An instance by `name=`, matched exactly: names are authored text
    /// (`"2048 - Event 3"`).
    #[must_use]
    pub fn instance_named(&self, name: &str) -> Option<&Instance> {
        self.instances.iter().find(|i| i.name == name)
    }

    /// Every instance of one `typedefid`, in document order.
    pub fn by_typedef(&self, typedef_id: i64) -> impl Iterator<Item = &Instance> {
        self.instances
            .iter()
            .filter(move |i| i.typedef_id == typedef_id)
    }

    /// `(typedefid, count)` for every typedef present, by descending count then
    /// id: the census table above, re-derived from the real file by
    /// `crates/2048/tests/campaign_ground_truth.rs`.
    #[must_use]
    pub fn typedef_counts(&self) -> Vec<(i64, usize)> {
        let mut counts: Vec<(i64, usize)> = Vec::new();
        for instance in &self.instances {
            match counts.iter_mut().find(|(id, _)| *id == instance.typedef_id) {
                Some((_, count)) => *count += 1,
                None => counts.push((instance.typedef_id, 1)),
            }
        }
        counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        counts
    }
}

/// One `<instance>`.
#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    /// `instanceid=`. Signed: ids are hash-shaped 32-bit values, as in
    /// `oag_formats::wad::hash_name`.
    pub instance_id: i64,
    /// `typedefid=`: the record shape; see the module docs' name table.
    pub typedef_id: i64,
    /// `name=`, e.g. `"2048 - Event 3"`, `"Bridge"`, `"Rockets Only"`.
    pub name: String,
    /// Every `<DATA>` child, in document order.
    pub fields: Vec<Field>,
}

impl Instance {
    /// A field by tag name (`"M_TRACKDEF"`), case-insensitively, like
    /// [`Node::attr`](crate::fexml::Node::attr): no casing here is load-bearing.
    #[must_use]
    pub fn field(&self, tag: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.tag.eq_ignore_ascii_case(tag))
    }
}

/// One `<DATA>` child: a field, its declared type, and its value(s).
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// The element tag, e.g. `"M_TRACKDEF"`.
    pub tag: String,
    /// `name=`, the mixed-case spelling (`"m_trackDef"`); kept apart from
    /// [`Field::tag`] since their casing sometimes differs.
    pub name: String,
    /// `type=`: the declared type, e.g. `"TrackDefinition"` or, for a polymorphic
    /// reference, the abstract `"GameModeBase"` (not the pointee's real type).
    pub type_name: String,
    /// `typedefid=` on the field, the *declared* type as [`Field::type_name`]; see
    /// [`ArrayValue::typedef_id`]/[`Reference::typedef_id`].
    pub typedef_id: i64,
    /// `length=`: the declared capacity, not `values.len()`.
    pub length: u32,
    /// Every `<ARRAY>` child, in order. Usually one; a few events author all
    /// `length` slots (`M_PGRIDSHIPMODELDATA`, capacity 7).
    pub values: Vec<ArrayValue>,
}

impl Field {
    /// The first array value's text when non-empty: the `length="1"` scalar case.
    #[must_use]
    pub fn value(&self) -> Option<&str> {
        self.values
            .first()
            .map(|v| v.value.as_str())
            .filter(|v| !v.is_empty())
    }

    /// [`Field::value`] as a signed integer (`int`, `u32` and enum ordinals fit).
    #[must_use]
    pub fn int(&self) -> Option<i64> {
        self.value()?.trim().parse().ok()
    }

    /// [`Field::value`], parsed as a float.
    #[must_use]
    pub fn float(&self) -> Option<f32> {
        self.value()?.trim().parse().ok()
    }

    /// [`Field::value`] as `"true"`/`"false"` case-insensitively; `None` for
    /// anything else, including empty, never a silent `false`.
    #[must_use]
    pub fn bool(&self) -> Option<bool> {
        match self.value()?.trim() {
            v if v.eq_ignore_ascii_case("true") => Some(true),
            v if v.eq_ignore_ascii_case("false") => Some(false),
            _ => None,
        }
    }

    /// The first array value as an instanceid reference: [`Field::int`] plus the
    /// `ARRAY`'s `typedefid=`, the pointee's concrete type for a polymorphic
    /// field. `None` when the value is empty, how an unauthored reference
    /// (`M_PEVENTREQUIRED` with no prerequisite) is stored.
    #[must_use]
    pub fn reference(&self) -> Option<Reference> {
        let first = self.values.first()?;
        if first.value.is_empty() {
            return None;
        }
        Some(Reference {
            instance_id: first.value.trim().parse().ok()?,
            typedef_id: first.typedef_id,
        })
    }

    /// Every non-empty array value as a reference, in order: the `length > 1`
    /// counterpart to [`Field::reference`] (`M_PGRIDSHIPMODELDATA`'s opponent
    /// slots).
    #[must_use]
    pub fn references(&self) -> Vec<Reference> {
        self.values
            .iter()
            .filter(|v| !v.value.is_empty())
            .filter_map(|v| {
                Some(Reference {
                    instance_id: v.value.trim().parse().ok()?,
                    typedef_id: v.typedef_id,
                })
            })
            .collect()
    }
}

/// One `<ARRAY>` child of a field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayValue {
    /// `value=`. Empty for an unauthored slot: the attribute stays present but
    /// blank, so empty is "nothing here" throughout.
    pub value: String,
    /// `typedefid=` on the `ARRAY`, when present: for a reference field, the
    /// pointee's concrete typedef; see [`Reference`].
    pub typedef_id: Option<i64>,
}

/// An instanceid reference carrying the pointee's concrete typedef, since the
/// field's *declared* type ([`Field::type_name`]) is not enough for a
/// polymorphic field; see [`Field::reference`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reference {
    /// The referenced instance's `instanceid`.
    pub instance_id: i64,
    /// The referenced instance's `typedefid`, read off the `ARRAY` rather than
    /// looked up, so it is available when the target is in another document
    /// (`MP.xml` naming an id `SP.xml` lacks). `None` on an `ARRAY` omitting it.
    pub typedef_id: Option<i64>,
}

/// Parses a mjolnir document from decoded XML text.
///
/// As forgiving as [`fexml::parse`]: an `<instance>` missing `instanceid`/
/// `typedefid`/`name`, or a `<DATA>` child that is not a field, is skipped. An
/// empty or non-mjolnir document parses to an empty [`Document`]; callers
/// needing "no campaign" vs "not a mjolnir file" check `!xml.is_empty()` first.
#[must_use]
pub fn parse(xml: &str) -> Document {
    let root = fexml::parse(xml);
    let Some(mjolnir) = root.children_named("mjolnir").next() else {
        return Document::default();
    };

    let instances = mjolnir
        .children_named("instance")
        .filter_map(parse_instance)
        .collect();

    Document { instances }
}

fn parse_instance(node: &Node) -> Option<Instance> {
    let instance_id = node.attr("instanceid")?.trim().parse().ok()?;
    let typedef_id = node.attr("typedefid")?.trim().parse().ok()?;
    let name = node.attr("name").unwrap_or_default().to_string();

    let fields = node
        .children_named("DATA")
        .next()
        .map(|data| data.children.iter().filter_map(parse_field).collect())
        .unwrap_or_default();

    Some(Instance {
        instance_id,
        typedef_id,
        name,
        fields,
    })
}

fn parse_field(node: &Node) -> Option<Field> {
    let type_name = node.attr("type").unwrap_or_default().to_string();
    let typedef_id = node.attr("typedefid").and_then(|v| v.trim().parse().ok())?;
    let length = node
        .attr("length")
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(1);
    let name = node.attr("name").unwrap_or_default().to_string();

    let values = node
        .children_named("ARRAY")
        .map(|array| ArrayValue {
            value: array.attr("value").unwrap_or_default().to_string(),
            typedef_id: array.attr("typedefid").and_then(|v| v.trim().parse().ok()),
        })
        .collect();

    Some(Field {
        tag: node.name.clone(),
        name,
        type_name,
        typedef_id,
        length,
        values,
    })
}

pub mod campaign;

#[cfg(test)]
mod tests;
