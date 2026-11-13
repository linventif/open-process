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
        id: "whiteboard.duplicate",
        label: key("common.duplicate_selection"),
        group: CommandGroup::Tools,
    },
];
impl EditorBackend for WhiteboardEditor {
    type ViewState = BoardView;
    fn capture_view(&self) -> BoardView {
        self.state.clone()
    }
    fn restore_view(&mut self, s: &BoardView) {
        self.state = s.clone();
    }
    fn fresh_view(&self) -> BoardView {
        BoardView::default()
    }
    fn save_view(s: &BoardView) -> Value {
        serde_json::json!({"camera":s.camera,"snap":s.snap,"grid":s.grid,"inspector":s.inspector,"paint":s.paint})
    }
    fn load_view(s: &mut BoardView, v: &Value) {
        s.inspector = v["inspector"].as_bool();
        if let Some(snap) = v["snap"].as_bool() {
            s.snap = snap;
        }
        if let Some(grid) = v["grid"].as_bool() {
            s.grid = grid;
        }
        if let Ok(camera) = serde_json::from_value::<View>(v["camera"].clone())
            && camera.zoom.is_finite()
            && (0.1..=5.0).contains(&camera.zoom)
            && camera.offset.is_finite()
        {
            s.camera = camera;
            s.fit = false;
        }
        if let Ok(paint) = serde_json::from_value::<Paint>(v["paint"].clone()) {
            let mut probe = Board::new("");
            probe.add(Element::new(Kind::Rectangle, Pos::default(), Pos::new(1.0, 1.0), paint.clone()));
            if probe.validate().is_ok() {
                s.paint = paint;
            }
        }
    }
    fn kind(&self) -> &'static str {
        "whiteboard"
    }
    fn name(&self) -> &str {
        &self.board.name
    }
    fn rename(&mut self, name: String) {
        self.history.sync(&self.board, false);
        self.board.name = name;
        self.history.sync(&self.board, false);
    }
    fn document(&self) -> Document {
        JsonCodec::new("whiteboard").encode(&self.board)
    }
    // Geometry is content in a free drawing; moving a shape changes the drawing's meaning.
    fn presentation_key(&self) -> Value {
        serde_json::to_value(&self.board.elements).unwrap()
    }
    fn cancel_edit(&mut self) {
        self.history.cancel(&mut self.board);
        self.state.drag = None;
        self.state.tool = Tool::Select;
        self.state.selection.retain(|id| self.board.element(*id).is_some());
    }
    fn ui(&mut self, ui: &mut Ui, _: &str) -> Vec<HostEvent> {
        self.show(ui);
        vec![]
    }
    fn commands(&self) -> &'static [CommandDescriptor] {
        COMMANDS
    }
    fn enabled(&self, command: &str) -> bool {
        match command {
            UNDO => self.history.can_undo(),
            REDO => self.history.can_redo(),
            DELETE | "whiteboard.duplicate" => !self.state.selection.is_empty(),
            FIT | PNG => true,
            _ => false,
        }
    }
    fn execute(&mut self, command: &str, _: f64) -> Vec<HostEvent> {
        match command {
            UNDO => {
                self.history.undo(&mut self.board);
                self.state.drag = None;
            }
            REDO => {
                self.history.redo(&mut self.board);
                self.state.drag = None;
            }
            DELETE => self.delete(),
            "whiteboard.duplicate" => self.duplicate(),
            FIT => self.state.fit = true,
            PNG => self.state.export_png = true,
            _ => {}
        }
        self.state.selection.retain(|id| self.board.element(*id).is_some());
        vec![]
    }
    fn export(&self) -> Result<ExportData, String> {
        Ok(ExportData {
            suffix: "svg",
            bytes: op_core::whiteboard::svg::export(&self.board)?.into_bytes(),
        })
    }
    fn export_label(&self) -> &'static str {
        key("flow.export_svg_svg")
    }
}
pub(crate) fn register(registry: &mut DiagramRegistry) {
    registry.diagrams.push(DiagramDescriptor {
        id: "whiteboard",
        aliases: &["free-drawing"],
        home_order: 10,
        icon: crate::icons::freehand,
        new_label: key("drawing.new_free_drawing"),
        title: key("drawing.sketch_freely"),
        description: key("drawing.draw_shapes_arrows_notes_freehand_strokes_shared"),
        badge: "DRAW",
        color: egui::Color32::from_rgb(192, 75, 48),
        create: |n| JsonCodec::new("whiteboard").encode(&Board::new(trf("drawing.drawing", &[&n]))),
        open: |d| {
            let board: Board = JsonCodec::new("whiteboard").decode(d)?;
            board.validate().map_err(ReadError::Invalid)?;
            Ok(Session::boxed(WhiteboardEditor::new(board)))
        },
    });
    registry.examples.push(ExampleDescriptor {
        id: "whiteboard.ideas",
        kind: "whiteboard",
        label: key("drawing.free_drawing_shared_idea"),
        create: || JsonCodec::new("whiteboard").encode(&op_core::whiteboard::example()),
    });
}
