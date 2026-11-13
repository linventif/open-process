//! Native/web free drawing editor. Shared chrome stays in DiagramLayout.
mod interaction;
mod render;
mod session;
pub(crate) use session::register;

use crate::canvas::{self, View};
use crate::editor::layout::{CanvasControls, DiagramLayout, FooterStatus};
use crate::editor::*;
use crate::history::History;
use egui::{Rect, Ui};
use op_core::geom::{Pos, Rect as Bounds};
use op_core::i18n::{tr, trf};
use op_core::whiteboard::{Board, Element, Id, Kind, Paint};
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Select,
    Hand,
    Draw(Kind),
}
#[derive(Clone)]
enum Drag {
    Draw {
        kind: Kind,
        start: Pos,
        current: Pos,
        points: Vec<Pos>,
    },
    Move {
        start: Pos,
        origins: Vec<(Id, Pos)>,
    },
    Resize {
        id: Id,
        original: Bounds,
    },
    Rubber {
        start: Pos,
        current: Pos,
        initial: BTreeSet<Id>,
    },
    Pan,
}
#[derive(Clone)]
pub struct BoardView {
    uid: u64,
    camera: View,
    canvas: Rect,
    fit: bool,
    snap: bool,
    grid: bool,
    inspector: Option<bool>,
    selection: BTreeSet<Id>,
    tool: Tool,
    paint: Paint,
    drag: Option<Drag>,
    export_png: bool,
    focus_text: bool,
    status: String,
}
impl Default for BoardView {
    fn default() -> Self {
        Self {
            uid: canvas::next_uid(),
            camera: View::default(),
            canvas: Rect::NOTHING,
            fit: true,
            snap: false,
            grid: true,
            inspector: None,
            selection: BTreeSet::new(),
            tool: Tool::Select,
            paint: Paint::default(),
            drag: None,
            export_png: false,
            focus_text: false,
            status: String::new(),
        }
    }
}
pub struct WhiteboardEditor {
    pub board: Board,
    history: History<Board>,
    state: BoardView,
}
impl WhiteboardEditor {
    pub fn new(board: Board) -> Self {
        Self {
            history: History::new(&board),
            board,
            state: Default::default(),
        }
    }
    fn delete(&mut self) {
        self.history.sync(&self.board, false);
        self.board.delete(&self.state.selection);
        self.state.selection.clear();
        self.history.sync(&self.board, false);
    }
    fn duplicate(&mut self) {
        self.history.sync(&self.board, false);
        let mut next = self.board.clone();
        let selected = next.duplicate(&self.state.selection);
        if let Err(e) = next.validate() {
            self.state.status = e;
            return;
        }
        self.board = next;
        self.state.selection = selected;
        self.state.status.clear();
        self.history.sync(&self.board, false);
    }
    fn add(&mut self, element: Element) {
        let kind = element.kind;
        let mut next = self.board.clone();
        let id = next.add(element);
        if let Err(e) = next.validate() {
            self.state.status = e;
            return;
        }
        self.board = next;
        self.state.status.clear();
        self.state.selection = BTreeSet::from([id]);
        if kind != Kind::Freehand {
            self.state.tool = Tool::Select;
        }
        if matches!(kind, Kind::Text | Kind::Note) {
            self.state.inspector = Some(true);
            self.state.focus_text = true;
        }
    }
    fn toolbar(&mut self, ui: &mut Ui) {
        for (tool, label) in [(Tool::Select, tr("common.select")), (Tool::Hand, tr("drawing.hand_tool"))]
            .into_iter()
            .chain(Kind::ALL.into_iter().map(|kind| (Tool::Draw(kind), tr(kind.label()))))
        {
            let icon: crate::icons::Icon = match tool {
                Tool::Select => crate::icons::select,
                Tool::Hand => crate::icons::hand,
                Tool::Draw(kind) => match kind {
                    Kind::Rectangle => crate::icons::rectangle,
                    Kind::Ellipse => crate::icons::ellipse,
                    Kind::Diamond => crate::icons::diamond,
                    Kind::Line => crate::icons::line,
                    Kind::Arrow => crate::icons::arrow,
                    Kind::Freehand => crate::icons::freehand,
                    Kind::Text => crate::icons::text,
                    Kind::Note => crate::icons::note,
                },
            };
            if canvas::icon_button(ui, self.state.tool == tool, label, icon).clicked() {
                self.state.tool = tool;
                if matches!(tool, Tool::Draw(_)) {
                    self.state.selection.clear();
                }
            }
        }
    }
    fn properties(&mut self, ui: &mut Ui) {
        DiagramLayout::inspector_tabs(ui, &mut (), &[((), tr("common.properties"))], |_, _| {});
        DiagramLayout::inspector_body(ui, |ui| {
            crate::theme::section_title(ui, tr("drawing.free_drawing"));
            let label = ui.label(tr("common.name"));
            ui.text_edit_singleline(&mut self.board.name).labelled_by(label.id);
            ui.separator();
            ui.label(trf("common.selection_count", &[&self.state.selection.len()]));
            if self.state.selection.len() == 1 {
                let id = *self.state.selection.first().unwrap();
                if let Some(e) = self.board.elements.iter_mut().find(|e| e.id == id) {
                    ui.label(tr(e.kind.label()));
                    let label = ui.label(tr("drawing.element_text"));
                    let r = ui
                        .add(
                            egui::TextEdit::multiline(&mut e.text)
                                .char_limit(16_384)
                                .desired_rows(3)
                                .desired_width(f32::INFINITY),
                        )
                        .labelled_by(label.id);
                    if std::mem::take(&mut self.state.focus_text) {
                        r.request_focus();
                    }
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr("common.width"));
                        ui.add(egui::DragValue::new(&mut e.bounds.w).range(1.0..=10_000.0));
                        ui.label(tr("common.height"));
                        ui.add(egui::DragValue::new(&mut e.bounds.h).range(1.0..=10_000.0));
                    });
                }
            }
            let mut paint = self
                .state
                .selection
                .iter()
                .find_map(|id| self.board.element(*id))
                .map(|e| e.paint.clone())
                .unwrap_or_else(|| self.state.paint.clone());
            let before = paint.clone();
            ui.separator();
            crate::theme::section_title(ui, tr("view.drawing"));
            ui.horizontal(|ui| {
                ui.label(tr("drawing.stroke_color"));
                ui.color_edit_button_srgb(&mut paint.stroke);
            });
            let mut filled = paint.fill.is_some();
            if ui.checkbox(&mut filled, tr("drawing.fill")).changed() {
                paint.fill = filled.then_some([219, 234, 254]);
            }
            if let Some(fill) = &mut paint.fill {
                ui.horizontal(|ui| {
                    ui.label(tr("drawing.fill_color"));
                    ui.color_edit_button_srgb(fill);
                });
            }
            ui.add(egui::Slider::new(&mut paint.width, 0.5..=12.0).text(tr("drawing.stroke_width")));
            ui.add(egui::Slider::new(&mut paint.font_size, 8.0..=96.0).text(tr("drawing.font_size")));
            ui.checkbox(&mut paint.sketch, tr("canvas.hand_drawn_sketch"));
            if paint != before {
                self.state.paint = paint.clone();
                for e in &mut self.board.elements {
                    if self.state.selection.contains(&e.id) {
                        if paint.stroke != before.stroke {
                            e.paint.stroke = paint.stroke;
                        }
                        if paint.fill != before.fill {
                            e.paint.fill = paint.fill;
                        }
                        if paint.width != before.width {
                            e.paint.width = paint.width;
                        }
                        if paint.font_size != before.font_size {
                            e.paint.font_size = paint.font_size;
                        }
                        if paint.sketch != before.sketch {
                            e.paint.sketch = paint.sketch;
                        }
                    }
                }
            }
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        !self.state.selection.is_empty(),
                        egui::Button::new(tr("common.duplicate_selection")),
                    )
                    .clicked()
                {
                    self.duplicate();
                }
                if ui
                    .add_enabled(!self.state.selection.is_empty(), egui::Button::new(tr("common.delete_selection")))
                    .clicked()
                {
                    self.delete();
                }
            });
            if self.state.selection.is_empty() {
                ui.label(tr("drawing.drag_draw_select_element_edit_shift_adds"));
            }
        });
    }
    fn show(&mut self, ui: &mut Ui) {
        self.state.selection.retain(|id| self.board.element(*id).is_some());
        let layout = DiagramLayout::new(ui, self.state.uid, self.state.inspector);
        layout.inspector(ui, |ui| self.properties(ui));
        layout.configuration(
            ui,
            CanvasControls {
                view: &mut self.state.camera,
                canvas: self.state.canvas,
                fit: &mut self.state.fit,
                snap: Some(&mut self.state.snap),
                inspector: &mut self.state.inspector,
                issues: None,
            },
            FooterStatus {
                summary: trf(
                    "drawing.selection_summary",
                    &[&self.board.elements.len(), &self.state.selection.len()],
                ),
                detail: self.state.status.clone(),
                ..Default::default()
            },
            |ui| {
                ui.checkbox(&mut self.state.grid, tr("canvas.show_grid"));
            },
        );
        layout.canvas(ui, |ui| self.canvas(ui));
        self.history.sync(
            &self.board,
            self.state.drag.is_some() || ui.ctx().egui_wants_keyboard_input() || ui.input(|i| i.pointer.any_down()),
        );
    }
    #[cfg(test)]
    pub fn element_screen_center(&self, id: Id) -> egui::Pos2 {
        self.state
            .camera
            .to_screen(self.state.canvas, self.board.element(id).unwrap().bounds.center())
    }
    #[cfg(test)]
    pub fn screen_point(&self, p: Pos) -> egui::Pos2 {
        self.state.camera.to_screen(self.state.canvas, p)
    }
}
