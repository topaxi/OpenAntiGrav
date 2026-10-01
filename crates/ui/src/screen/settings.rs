//! `Touchlist` and `TouchSlider`: Wipeout 2048's own settings-panel widgets,
//! authored on `Team_Definition.xml`'s skin picker and every sub-page of
//! `Options_Definition.xml` - see `docs/formats/2048-frontend.md`'s Options
//! and Team sections.
//!
//! Split out of `screen.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, the way `touch.rs` already is. Neither
//! widget authors a per-entry position: `Touchlist` gives its whole strip one
//! `x`/`y`/`width`/`height` and lists its `Entry` children in document order,
//! `TouchSlider` gives one rect and a `minvalue`/`maxvalue` pair. A reader
//! that wants individual entry rects has to lay them out itself - see
//! `crate::frontend::team`/`crate::frontend::options2048`'s own doc comments
//! for where that chosen spacing lives and what it is bounded against.

use super::{Node, Screens};

/// One `<Entry>` under a `Touchlist`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TouchListEntry {
    /// The string-table id this entry's label reads, when it has one.
    pub idstring: Option<String>,
    /// A literal value instead of, or alongside, the label - `OptionsControls`'
    /// `Motion Sensor` list carries these (`"WIPEOUT"`/`"RACER"`/`"MOTION"`)
    /// and no icon; `CameraP1` carries an icon and no literal value.
    pub value: Option<String>,
    /// The entry's own icon, when it has one.
    pub src: Option<String>,
}

/// A `<Touchlist>` widget: a named, single-focus strip of mutually exclusive
/// entries - Wipeout 2048's own settings picker, and `Team_Definition.xml`'s
/// skin picker.
#[derive(Debug, Clone, PartialEq)]
pub struct TouchList {
    /// The `name` attribute, when it has one.
    pub name: Option<String>,
    /// The string-table id for the list's own caption, read off the
    /// `<Values idstring="...">` the same as any other labelled widget.
    pub idstring: Option<String>,
    /// Left edge, in the screen's own grid.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Authored width, the whole strip's own.
    pub width: f32,
    /// Authored height.
    pub height: f32,
    /// `Vertical="true"`: `Team_Definition.xml`'s own skin list stacks its
    /// three entries rather than running them in a row, unlike every
    /// `Options_Definition.xml` list.
    pub vertical: bool,
    /// The entries, in document order - see [`TouchListEntry`].
    pub entries: Vec<TouchListEntry>,
    /// The `default` attribute: an entry's own `idstring` or `value`,
    /// whichever it carries. **Authored, and not necessarily this build's
    /// own starting point** - `CameraP1` declares `default="OPT_CLOSE"`,
    /// which [`oag_display::display::CameraView::default`] now agrees with;
    /// a reader that seeds the picker from the live setting rather than this
    /// field is choosing continuity with the running session over the disc's
    /// own declared default, and should say so.
    pub default: Option<String>,
}

/// A `<TouchSlider>` widget: one continuously-valued control, `Options`'
/// `OptionsAudio` volumes.
#[derive(Debug, Clone, PartialEq)]
pub struct TouchSlider {
    /// The `name` attribute, when it has one.
    pub name: Option<String>,
    /// The string-table id for the slider's own caption.
    pub idstring: Option<String>,
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Authored width, the whole bar's own.
    pub width: f32,
    /// Authored height.
    pub height: f32,
    /// `minvalue`.
    pub min: f32,
    /// `maxvalue`.
    pub max: f32,
    /// `default`, in the same units as [`Self::min`]/[`Self::max`].
    pub default: f32,
}

impl Screens {
    pub(super) fn touch_list_from_node(&self, node: &Node, offset: (f32, f32)) -> TouchList {
        let entries = node
            .children_named("Entry")
            .map(|entry| TouchListEntry {
                idstring: entry.value("idstring").map(str::to_string),
                value: entry.value("value").map(str::to_string),
                src: entry.value("Src").map(str::to_string),
            })
            .collect();
        TouchList {
            name: node.attr("name").map(str::to_string),
            idstring: node.value("idstring").map(str::to_string),
            x: self.number(node.value("x")).unwrap_or(0.0) + offset.0,
            y: self.number(node.value("y")).unwrap_or(0.0) + offset.1,
            width: self.number(node.value("width")).unwrap_or(0.0),
            height: self.number(node.value("height")).unwrap_or(0.0),
            vertical: node.flag("Vertical").unwrap_or(false),
            entries,
            default: node.value("default").map(str::to_string),
        }
    }

    pub(super) fn touch_slider_from_node(&self, node: &Node, offset: (f32, f32)) -> TouchSlider {
        TouchSlider {
            name: node.attr("name").map(str::to_string),
            idstring: node.value("idstring").map(str::to_string),
            x: self.number(node.value("x")).unwrap_or(0.0) + offset.0,
            y: self.number(node.value("y")).unwrap_or(0.0) + offset.1,
            width: self.number(node.value("width")).unwrap_or(0.0),
            height: self.number(node.value("height")).unwrap_or(0.0),
            min: self.number(node.value("minvalue")).unwrap_or(0.0),
            max: self.number(node.value("maxvalue")).unwrap_or(100.0),
            default: self.number(node.value("default")).unwrap_or(0.0),
        }
    }
}
