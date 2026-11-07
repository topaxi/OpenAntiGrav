//! A `<List>` widget's own row: what `List_ParseXml` reads off the element beside the label
//! [`super::Screens::collect_widgets`] folds into a [`super::Text`].
//!
//! HD's settings rows and the pause screens' on/off lists are this widget: a label, a value
//! box and a pair of step arrows (`crate::menu::rows`), the entries named by `idstring` in the
//! order the disc lists them.

use super::{Node, Screens, parse_argb};

/// One `<List>`: where it sits and what it steps through.
#[derive(Debug, Clone, PartialEq)]
pub struct ListWidget {
    /// The widget's `name`, the same one its [`super::Text`] half carries.
    pub name: Option<String>,
    /// `x`, plus every enclosing `Item` offset.
    pub x: f32,
    /// `y`, likewise.
    pub y: f32,
    /// The entries' `idstring`s, in document order.
    pub entries: Vec<String>,
    /// `default`: the entry the list holds before anything is chosen.
    pub default: Option<String>,
    /// `ActiveColor`, the fill while focused.
    pub active_color: Option<u32>,
    /// `InactiveColor`, the fill while not.
    pub inactive_color: Option<u32>,
    /// `arrowcolor`, the tint of the step arrows.
    pub arrow_color: Option<u32>,
}

impl Screens {
    pub(super) fn list_widget_from_node(&self, node: &Node, offset: (f32, f32)) -> ListWidget {
        let argb = |attr: &str| {
            self.resolve(node.value(attr).unwrap_or_default())
                .and_then(parse_argb)
        };
        ListWidget {
            name: node.attr("name").map(str::to_string),
            x: self.number(node.value("x")).unwrap_or(0.0) + offset.0,
            y: self.number(node.value("y")).unwrap_or(0.0) + offset.1,
            entries: node
                .children_named("Entry")
                .filter_map(|entry| entry.value("idstring").map(str::to_string))
                .collect(),
            default: node.attr("default").map(str::to_string),
            active_color: argb("ActiveColor"),
            inactive_color: argb("InactiveColor"),
            arrow_color: argb("arrowcolor"),
        }
    }
}
