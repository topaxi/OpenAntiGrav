//! The `<TagInput>` widget: Pulse and Pure's own text-entry cell row.
//!
//! Split out under the 1,000-line rule the same way `widgets.rs`'s own
//! builders were - see that file's doc. See `docs/formats/fexml.md`'s
//! `TagInput` section for the schema and the evidence.

use super::{Node, Screens};

/// One `<TagInput>` widget's own attributes.
///
/// **Not the cell backgrounds, the confirm label or its bars** - those are
/// ordinary `Image`/`Text` siblings, already collected as [`super::Fill`]s
/// and [`super::Text`]s wherever they sit under a named `Screen`. This is
/// only the glyph row's own position, cell count and colour. See
/// `docs/formats/fexml.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct TagInput {
    /// The `name` attribute - `Name`, `Tag`, or one of the four online
    /// fields (`GameNameTag`, `GamePasswordTag`, `UsernameTag`,
    /// `PasswordTag`).
    pub name: String,
    /// Left edge of the first cell's text.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// How many cells the row holds.
    pub length: u32,
    /// ARGB glyph colour.
    pub color: u32,
    /// Scale multiplier. `1.0` when the widget authors none - `Create
    /// Profile Setup`'s own `Tag` is the measured case.
    pub scale: f32,
    /// Whether the `Encrypt` attribute was present and true - the two online
    /// password fields mask their glyphs.
    pub encrypt: bool,
    /// The `AllowBlank` attribute, when the widget states one.
    /// `None` when absent, which is every field but the two online username/
    /// password ones.
    pub allow_blank: Option<bool>,
    /// The `focus` attribute, verbatim - `"true"` on every profile field,
    /// `"force"` on the two online fields the screen will not proceed past
    /// unfilled.
    pub focus: Option<String>,
}

impl Screens {
    /// `pub(crate)` for `oag_ui_screens::tag_entry`, which reads a `TagInput` off
    /// an anonymous `Screen` this module's own `collect_widgets` never
    /// registers - see that function's own doc and `oag_ui_screens::tag_entry`'s.
    pub fn tag_input_from_node(&self, node: &Node) -> TagInput {
        TagInput {
            name: node.attr("name").unwrap_or_default().to_string(),
            x: self.number(node.value("x")).unwrap_or(0.0),
            y: self.number(node.value("y")).unwrap_or(0.0),
            length: self
                .number(node.value("length"))
                .map(|v| v.max(0.0) as u32)
                .unwrap_or(0),
            color: self
                .resolve(node.value("color").unwrap_or_default())
                .and_then(super::parse_argb)
                .unwrap_or(0xffff_ffff),
            scale: self.number(node.value("scale")).unwrap_or(1.0),
            encrypt: node.flag("Encrypt").unwrap_or(false),
            allow_blank: node.flag("AllowBlank"),
            focus: node.attr("focus").map(str::to_string),
        }
    }
}
