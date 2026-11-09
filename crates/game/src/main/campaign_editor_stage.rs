//! The campaign editor open over the Race Campaign's screens: the model, the pulse of
//! its selector, what the last save said, and the save in flight. The flow that opens,
//! answers and closes it is `crate::session::campaign_editor`; the drawing is
//! `oag_ui_screens::campaign::editor`.

use oag_game::campaign::editor::{Editor, Field};
use oag_ui_screens::campaign::editor::{FIELD_ROWS, Row, View};
use oag_ui_screens::campaign::pulse::SelectorPulse;

/// What a save came to: the folder written and the rebuilt menus, or why not.
pub(crate) type Saved = Result<
    (
        std::path::PathBuf,
        Result<crate::session::Rebuilt, String>,
    ),
    String,
>;

/// The screen, open.
pub(crate) struct EditorScreen {
    pub(crate) model: Editor,
    pub(crate) pulse: SelectorPulse,
    /// What the last save did, shown under the rows.
    pub(crate) status: String,
    /// The campaign being written and the menus rebuilt to list it, off the render thread.
    pub(crate) saving: Option<oag_thread::Task<Saved>>,
}

/// What the screen shows, owned so a [`View`] can borrow it.
pub(crate) struct Shown {
    title: String,
    name: String,
    heading: String,
    placed: Vec<(u32, u32)>,
    fields: Vec<Row>,
    actions: Vec<Row>,
    selected_row: Option<usize>,
    status: String,
    legend: [String; 2],
    hint: String,
}

impl Shown {
    pub(crate) fn view<'a>(&'a self, pulse: &'a SelectorPulse, cursor: (u32, u32)) -> View<'a> {
        View {
            title: &self.title,
            name: &self.name,
            heading: &self.heading,
            cursor,
            placed: &self.placed,
            fields: &self.fields,
            actions: &self.actions,
            selected_row: self.selected_row,
            status: &self.status,
            legend: [&self.legend[0], &self.legend[1]],
            hint: &self.hint,
            pulse,
        }
    }
}

impl EditorScreen {
    pub(crate) fn new(model: Editor) -> Self {
        Self {
            model,
            pulse: SelectorPulse::default(),
            status: String::new(),
            saving: None,
        }
    }

    /// The panel's rows, as the model stands.
    pub(crate) fn shown(&self) -> Shown {
        let model = &self.model;
        let cursor = model.cursor();
        let fields: Vec<Row> = if model.has(cursor) {
            Field::ALL
                .iter()
                .map(|&field| Row {
                    label: model.field_name(field),
                    value: model.label(cursor, field).unwrap_or_default(),
                })
                .collect()
        } else {
            Vec::new()
        };
        debug_assert!(fields.len() <= FIELD_ROWS);
        let actions = vec![
            Row {
                label: model.say("OAG_EDITOR_ACTION_NAME", "NAME"),
                value: model.name().to_string(),
            },
            Row {
                label: model.say("OAG_EDITOR_ACTION_SAVE", "SAVE CAMPAIGN"),
                value: String::new(),
            },
        ];
        let heading = if model.has(cursor) {
            format!("{}, {}", cursor.0, cursor.1)
        } else {
            model.say("OAG_EDITOR_EMPTY", "EMPTY SLOT")
        };
        Shown {
            title: model.say("OAG_EDITOR_TITLE", "CAMPAIGN EDITOR"),
            name: model.name().to_string(),
            heading: format!(
                "{heading}   {} {}",
                model.say("OAG_EDITOR_PLACED", "EVENTS PLACED"),
                model.count()
            ),
            placed: model.placed().collect(),
            fields,
            actions,
            selected_row: model
                .editing()
                .then(|| Field::ALL.iter().position(|&f| f == model.field()))
                .flatten(),
            status: if self.status.is_empty() {
                model.say("OAG_EDITOR_HINT_SAVE", "START SAVES")
            } else {
                self.status.clone()
            },
            hint: model.say("OAG_EDITOR_HINT_PLACE", "CROSS PLACES AN EVENT"),
            legend: [
                model.say("OAG_EDITOR_LEGEND_CLEAR", "Clear slot"),
                model.say("OAG_EDITOR_LEGEND_NAME", "Name"),
            ],
        }
    }
}
