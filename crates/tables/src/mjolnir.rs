//! The "mjolnir" typed-instance database: `data/xml/SP.xml` and `data/xml/MP.xml`
//! inside Wipeout 2048's `PSP2/data.psarc`, and `data/xml/MjolnirData.xml`
//! beside them (212 bytes - only the tool's own `<WORKSPACES>` list of the
//! three files it edits, `SP.xml`/`MP.xml`/`Profile.xml`; it names no schema
//! and this crate reads nothing from it).
//!
//! No other title this project reads ships this format - it is 2048's own
//! campaign, not a `PI_Grid`/`PI_Cell` grid the way Pulse's, Pure's and
//! Wipeout HD's all are (see [`crate::race_campaign`]). [`campaign`] is the
//! typed view built on top of this generic reader.
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
//! Every instance is a typed record: `typedefid` names *which* record shape it
//! is, and each `<M_FOO>` child under `<DATA>` is one field, carrying the
//! field's own declared type (`type=`/`typedefid=` on the field element
//! itself) and one or more `<ARRAY>` children holding the actual value(s) -
//! `length=` on the field is the array's capacity, not necessarily how many
//! `<ARRAY>` children are present (most fields carry exactly one even when
//! `length` says more; a handful of `2048 - Event *` instances' own
//! `M_PGRIDSHIPMODELDATA` are the only fields this crate has seen author all
//! seven).
//!
//! This reader is deliberately as forgiving as [`crate::fexml::parse`], which
//! it is built on: a field or instance this module does not recognise is
//! skipped, never an error, so a schema this pass did not measure still comes
//! through as raw instances and fields rather than failing the whole document.
//!
//! # The `type=`/`typedefid=` pair is the schema's own name table
//!
//! **The strongest evidence in this module, and not inferred.** Every field
//! that points at another instance (`M_TRACKDEF type="TrackDefinition"
//! typedefid="205052969"`, `M_WEAPONSET type="WeaponSetDefinition"
//! typedefid="-966434245"`, ...) carries the referenced typedef's own name
//! right there in the file. Collecting every `(typedefid, type)` pair across
//! all 288 `SP.xml` instances gives a closed name table with no ambiguity:
//!
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
//! Confidence **95** for every row: this is the file's own text, cross-checked
//! against every field that names it, not a structural guess. **This
//! corrects a prior guess.** Reverse-engineering this format started from a
//! hypothesis that the twenty single-field instances of `-966434245` were
//! `TrackDefinition` ("2048 ships 20 track profiles"); the file's own
//! `type=`/`typedefid=` pairs say `-966434245` is `WeaponSetDefinition`
//! (twenty weapon sets: `"Rockets Only"`, `"Cannons and Missile"`, ...) and
//! `205052969` - ten instances, `M_TRACKNAME` matching
//! `data/plugins/tracks/Definition.xml`'s own ten `<PI_Track name="...">`
//! stems exactly (`square`, `park`, `tower`, `mall`, `bridge`, `arena`,
//! `subway`, `cathedral`, `sol`, `altima`) - is `TrackDefinition`. Anyone
//! building on the earlier guess should re-check against this table, not
//! the hypothesis.
//!
//! # Four more typedefs carry no name in the file at all
//!
//! `-1915183557` (53 instances), `-1353052320` (52), `1311982788` (26) and
//! `1018671239` (10) - the four concrete "event" shapes - are never a field's
//! declared type anywhere in `SP.xml`, because every field that points at one
//! (`M_PNEXTEVENT`, `M_PBRANCHEVENT`, `M_PEVENTREQUIRED`,
//! `M_PCAMPAIGNUNLOCK`) declares the *abstract* `GameModeBase` as its static
//! type instead. **The `<ARRAY>` element itself still carries the concrete
//! pointee's own `typedefid`** - `<ARRAY value="-484309551"
//! typedefid="-1353052320"/>` on `2048 - Event 3`'s own `M_PNEXTEVENT` names
//! the *referenced* instance's real typedef even though the field's
//! declaration cannot - so [`Reference::typedef_id`] recovers this for every
//! edge in the event graph without needing to look the target up. See
//! [`campaign`] for what these four typedefs' own field shapes say, and for
//! why this module stops short of naming them after any of the six
//! `GameMode_*` C++ classes `eboot.elf`'s own string table carries
//! (`GameMode_ArcadeRace`, `GameMode_CheckPointRace`, `GameMode_EliminatorRace`,
//! `GameMode_SpeedLapRace`, `GameMode_ZombieRace`, `GameMode_ZoneRace`).
//!
//! # `288` total, one file, one measurement
//!
//! `380278911`\*96 + `-1915183557`\*53 + `-1353052320`\*52 + `1311982788`\*26 +
//! `520725191`\*21 + `-966434245`\*20 + `1018671239`\*10 + `205052969`\*10 =
//! `288`, the exact instance count `SP.xml` carries. `crates/tables/src/mjolnir/tests.rs`
//! and the `#[ignore]`d ground-truth test in `crates/2048/tests/` both assert
//! this so a future SP.xml (a patch, a different region) that changes the
//! shape is a loud failure rather than a silently stale table.

use crate::fexml::{self, Node};

/// A parsed mjolnir document: every `<instance>` under the `<mjolnir>` root,
/// in document order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Document {
    /// Every instance, in the order the file authors them.
    pub instances: Vec<Instance>,
}

impl Document {
    /// An instance by its own `instanceid`. `None` if the document was never
    /// asked about an id it does not carry, which is the shape a dangling or
    /// unauthored reference field takes - see [`Reference`].
    #[must_use]
    pub fn instance(&self, instance_id: i64) -> Option<&Instance> {
        self.instances.iter().find(|i| i.instance_id == instance_id)
    }

    /// An instance by its own `name=`, matched exactly - names are authored
    /// text (`"2048 - Event 3"`), not case-folded anywhere else in this
    /// module, so this does not fold case either.
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

    /// `(typedefid, count)` for every typedef this document actually carries,
    /// sorted by descending count then by id - what this module's own doc
    /// comment's census table was read off, and what
    /// `crates/2048/tests/campaign_ground_truth.rs` re-derives from the real
    /// file rather than trusting the table not to drift.
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
    /// `instanceid=`. Signed - every id in this format is a hash-shaped
    /// 32-bit value, some of them negative, the same convention
    /// `oag_formats::wad::hash_name` already uses elsewhere on this project.
    pub instance_id: i64,
    /// `typedefid=`: which record shape this is. See this module's own doc
    /// comment for the name table `SP.xml`'s own fields resolve most of
    /// these against.
    pub typedef_id: i64,
    /// `name=`, e.g. `"2048 - Event 3"`, `"Bridge"`, `"Rockets Only"`.
    pub name: String,
    /// Every `<DATA>` child, in document order.
    pub fields: Vec<Field>,
}

impl Instance {
    /// A field by its own tag name (`"M_TRACKDEF"`), case-insensitively -
    /// the same convention [`Node::attr`](crate::fexml::Node::attr) uses,
    /// kept here for the same reason: nothing in this format's own casing is
    /// load-bearing and a future file spelling it differently should still
    /// resolve.
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
    /// `name=`, the mixed-case spelling (`"m_trackDef"`) - kept separately
    /// from [`Field::tag`] since the two sometimes disagree in casing and
    /// nothing here has needed to reconcile them.
    pub name: String,
    /// `type=`: the field's own declared type name, e.g. `"TrackDefinition"`
    /// or, for a polymorphic reference, the abstract `"GameModeBase"` - see
    /// this module's own doc comment on why that is not always the
    /// referenced instance's *real* type.
    pub type_name: String,
    /// `typedefid=` on the field element itself - the same caveat as
    /// [`Field::type_name`] applies: this is the field's *declared* type,
    /// not necessarily what a reference inside it points at. See
    /// [`ArrayValue::typedef_id`]/[`Reference::typedef_id`].
    pub typedef_id: i64,
    /// `length=`: the field's declared capacity. Not the same as
    /// `values.len()` - see this module's own doc comment.
    pub length: u32,
    /// Every `<ARRAY>` child, in document order. Usually one; a handful of
    /// events author all `length` slots (`M_PGRIDSHIPMODELDATA`, capacity 7).
    pub values: Vec<ArrayValue>,
}

impl Field {
    /// The first array value's own text, when it is non-empty. The common
    /// case for a `length="1"` scalar field.
    #[must_use]
    pub fn value(&self) -> Option<&str> {
        self.values
            .first()
            .map(|v| v.value.as_str())
            .filter(|v| !v.is_empty())
    }

    /// [`Field::value`], parsed as a signed integer - every numeric field in
    /// this format (`int`, `u32`, an enum ordinal like `eClass`) fits.
    #[must_use]
    pub fn int(&self) -> Option<i64> {
        self.value()?.trim().parse().ok()
    }

    /// [`Field::value`], parsed as a float.
    #[must_use]
    pub fn float(&self) -> Option<f32> {
        self.value()?.trim().parse().ok()
    }

    /// [`Field::value`], parsed as `"true"`/`"false"` case-insensitively.
    /// `None` for anything else, including empty - never a silent `false`.
    #[must_use]
    pub fn bool(&self) -> Option<bool> {
        match self.value()?.trim() {
            v if v.eq_ignore_ascii_case("true") => Some(true),
            v if v.eq_ignore_ascii_case("false") => Some(false),
            _ => None,
        }
    }

    /// The first array value read as an instanceid reference - [`Field::int`]
    /// plus the `ARRAY` element's own `typedefid=`, which is the concrete
    /// pointee type for a polymorphic field (see [`Reference`] and this
    /// module's own doc comment). `None` when the field's own value is empty,
    /// which is how an unauthored reference (`M_PEVENTREQUIRED` on an event
    /// with no prerequisite) is stored.
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

    /// Every non-empty array value read as an instanceid reference, in
    /// document order - the `length > 1` counterpart to [`Field::reference`],
    /// for a field like `M_PGRIDSHIPMODELDATA` that names more than one
    /// opponent slot.
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
    /// `value=`. Empty for an unauthored slot - this format leaves the
    /// attribute present but blank rather than omitting the element, so an
    /// empty string is the "nothing here" state throughout this module.
    pub value: String,
    /// `typedefid=` on the `ARRAY` element itself, when present. For a
    /// reference field this is the referenced instance's own concrete
    /// typedef - see this module's own doc comment and [`Reference`].
    pub typedef_id: Option<i64>,
}

/// An instanceid reference recovered from a field, carrying the pointee's own
/// concrete typedef alongside the id - see [`Field::reference`] and this
/// module's own doc comment on why the field's *declared* type
/// ([`Field::type_name`]) is not enough on its own for a polymorphic field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reference {
    /// The referenced instance's `instanceid`.
    pub instance_id: i64,
    /// The referenced instance's own `typedefid`, read off the `ARRAY`
    /// element rather than looked up - so this is available even when the
    /// referenced instance itself is not in the same document (`MP.xml`
    /// referencing an id `SP.xml` alone does not carry, for instance).
    /// `None` on the rare `ARRAY` that omits it.
    pub typedef_id: Option<i64>,
}

/// Parses a mjolnir document from already-decoded XML text.
///
/// As forgiving as [`fexml::parse`], which this is built on: an `<instance>`
/// missing `instanceid`/`typedefid`/`name` is skipped rather than failing the
/// whole document, and a `<DATA>` child that is not a recognisable field
/// (none seen in `SP.xml`/`MP.xml`, but nothing here assumes there is none)
/// is skipped the same way. An empty or non-mjolnir document parses to an
/// empty [`Document`] rather than erroring - callers that need to tell "no
/// campaign" from "not a mjolnir file" should check `!xml.is_empty()`
/// themselves first.
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
