use super::*;
use op_core::document::{Document, DocumentCodec, JsonCodec, ReadError};
use op_core::i18n::key;
use serde_json::Value;

const COMMANDS: &[CommandDescriptor] = &[
    CommandDescriptor {
        id: UNDO,
        label: key("common.undo"),
        group: CommandGroup::Edit,
    },
    CommandDescriptor {
        id: REDO,
        label: key("common.redo"),
        group: CommandGroup::Edit,
    },
    CommandDescriptor {
        id: DELETE,
        label: key("common.delete_selection"),
        group: CommandGroup::Edit,
    },
    CommandDescriptor {
        id: FIT,
        label: key("canvas.zoom_fit"),
        group: CommandGroup::View,
    },
    CommandDescriptor {
        id: "database.show-ddl",
        label: key("database.sql_ddl"),
        group: CommandGroup::View,
    },
];
const VIEWS: &[ViewDescriptor] = &[ViewDescriptor {
    id: "database.ddl",
    label: key("database.sql_ddl"),
}];
impl EditorBackend for DatabaseEditor {
    fn thumbnail_rect(&self) -> Option<egui::Rect> {
        Some(self.state.canvas)
    }
    type ViewState = DatabaseView;
    fn capture_view(&self) -> Self::ViewState {
        self.state.clone()
    }
    fn restore_view(&mut self, s: &Self::ViewState) {
        self.state = s.clone();
    }
    fn fresh_view(&self) -> Self::ViewState {
        DatabaseView {
            selected: self.schema.tables.first().map(|t| t.id),
            ..Default::default()
        }
    }
    fn save_view(s: &Self::ViewState) -> Value {
        serde_json::json!({"camera":s.camera,"snap":s.snap,"inspector":s.inspector})
    }
    fn load_view(s: &mut Self::ViewState, v: &Value) {
        s.inspector = v["inspector"].as_bool();
        if let Some(snap) = v["snap"].as_bool() {
            s.snap = snap;
        }
        if let Ok(c) = serde_json::from_value::<View>(v["camera"].clone())
            && c.zoom.is_finite()
            && c.zoom > 0.0
            && c.offset.is_finite()
        {
            s.camera = c;
            s.fit = false;
        }
    }
    fn kind(&self) -> &'static str {
        "database.schema"
    }
    fn name(&self) -> &str {
        &self.schema.name
    }
    fn rename(&mut self, name: String) {
        self.history.sync(&self.schema, false);
        self.schema.name = name;
        self.history.sync(&self.schema, false);
    }
    fn content_generation(&self) -> Option<u64> {
        Some(self.history.generation())
    }
    fn validate_storage(&self) -> Result<(), String> {
        self.schema.check_storage()
    }
    fn document(&self) -> Document {
        JsonCodec::new("database.schema").encode(&self.schema)
    }
    fn semantic_document(&self) -> Document {
        let mut schema = self.schema.clone();
        for t in &mut schema.tables {
            t.bounds = Default::default();
        }
        JsonCodec::new("database.schema").encode(&schema)
    }
    fn presentation_key(&self) -> Value {
        serde_json::json!(self.schema.tables.iter().map(|t| (t.id, t.bounds)).collect::<Vec<_>>())
    }
    fn cancel_edit(&mut self) {
        self.history.cancel(&mut self.schema);
        self.state.drag = None;
        self.state.pending = None;
        self.state.connecting = false;
    }
    fn ui(&mut self, ui: &mut Ui, view: &str) -> Vec<HostEvent> {
        self.show(ui, view == "database.ddl");
        std::mem::take(&mut self.events)
    }
    fn commands(&self) -> &'static [CommandDescriptor] {
        COMMANDS
    }
    fn enabled(&self, c: &str) -> bool {
        match c {
            UNDO => self.history.can_undo(),
            REDO => self.history.can_redo(),
            DELETE => !self.state.sql_view && self.state.selected.is_some_and(|id| self.schema.table(id).is_some()),
            FIT => true,
            PNG => !self.state.sql_view,
            "database.show-ddl" => true,
            _ => false,
        }
    }
    fn execute(&mut self, c: &str, _: f64) -> Vec<HostEvent> {
        match c {
            UNDO => {
                self.history.undo(&mut self.schema);
                self.state.drag = None;
                self.state.pending = None;
            }
            REDO => {
                self.history.redo(&mut self.schema);
                self.state.drag = None;
                self.state.pending = None;
            }
            DELETE => self.delete_selected(),
            FIT => self.state.fit = true,
            PNG => self.state.export_png = true,
            "database.show-ddl" => return vec![HostEvent::OpenView(crate::dock::View::from("database.ddl"))],
            _ => {}
        }
        vec![]
    }
    fn views(&self) -> &'static [ViewDescriptor] {
        VIEWS
    }
    fn export(&self) -> Result<ExportData, String> {
        database::generate(&self.schema, self.schema.dialect)
            .map(|sql| ExportData {
                suffix: "sql",
                bytes: sql.into_bytes(),
            })
            .map_err(|issues| issues.iter().map(|i| i.message.as_str()).collect::<Vec<_>>().join("\n"))
    }
    fn export_label(&self) -> &'static str {
        key("database.export_sql_ddl_sql")
    }
}
pub(crate) fn register(registry: &mut DiagramRegistry) {
    registry.diagrams.push(DiagramDescriptor {
        id: "database.schema",
        aliases: &["database", "erd", "mpd"],
        home_order: 1,
        icon: crate::icons::database,
        new_label: key("database.new_database_schema"),
        title: key("database.design_database"),
        description: key("database.define_tables_columns_relationships_then_generate_sql"),
        badge: "DATABASE",
        color: egui::Color32::from_rgb(34, 139, 157),
        create: |n| JsonCodec::new("database.schema").encode(&Schema::new(trf("database.database_schema", &[&n]))),
        open: |d| {
            let schema: Schema = JsonCodec::new("database.schema").decode(d)?;
            schema.check_storage().map_err(ReadError::Invalid)?;
            Ok(Session::boxed(DatabaseEditor::new(schema)))
        },
    });
    registry.examples.push(ExampleDescriptor {
        id: "database.shop",
        kind: "database.schema",
        label: key("database.database_shop_schema"),
        create: || JsonCodec::new("database.schema").encode(&database::example()),
    });
}
