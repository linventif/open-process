//! Native/web free drawing editor. Shared chrome stays in DiagramLayout.
mod editing;
mod interaction;
mod render;
mod session;
pub(crate) use session::register;

use crate::canvas::{self, View};
use crate::editor::layout::{CanvasControls, DiagramLayout, FooterStatus};
use crate::editor::*;
use crate::history::History;
use editing::ClipboardAction;
use egui::{Rect, Ui};
use op_core::geom::Pos;
use op_core::i18n::{key, tr, trf};
use op_core::whiteboard::{Board, Element, Fragment, Id, Kind, Paint, StackOrder};
use std::collections::BTreeSet;

const SIZE_LABELS: [&str; 5] = [
    key("drawing.size_xs"),
    key("drawing.size_s"),
    key("drawing.size_m"),
    key("drawing.size_l"),
    key("drawing.size_xl"),
];
const STROKE_SIZES: [f32; 5] = [0.5, 1.0, 2.0, 4.0, 8.0];
const FONT_SIZES: [f32; 5] = [12.0, 16.0, 20.0, 32.0, 48.0];

fn size_presets(ui: &mut Ui, label: &str, value: &mut f32, sizes: [f32; 5], mixed: bool) -> bool {
    ui.horizontal_wrapped(|ui| {
        ui.label(label);
        if mixed {
            ui.weak(tr("drawing.mixed_sizes"));
        } else if !sizes.contains(value) {
            ui.weak(tr("drawing.custom_size"));
        }
    });
    let mut chosen = false;
    ui.horizontal_wrapped(|ui| {
        for (name, size) in SIZE_LABELS.into_iter().zip(sizes) {
            let name = tr(name);
            let selected = !mixed && *value == size;
            let response = ui.selectable_label(selected, name);
            let accessible_name = trf("drawing.size_preset", &[&label, &name]);
            response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, ui.is_enabled(), selected, &accessible_name));
            if response.on_hover_text(accessible_name).clicked() {
                *value = size;
                chosen = true;
            }
        }
    });
    chosen
}

fn exact_size_row(ui: &mut Ui, label: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>) -> bool {
    let label = ui.label(label);
    let changed = ui
        .add(
            egui::DragValue::new(value)
                .range(range)
                .clamp_existing_to_range(false)
                .speed(0.1)
                .custom_formatter(|value, _| (value as f32).to_string()),
        )
        .labelled_by(label.id)
        .changed();
    ui.end_row();
    changed
}

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
        origins: std::collections::HashMap<Id, Pos>,
        original_selection: Option<BTreeSet<Id>>,
    },
    Resize {
        id: Id,
        gesture: canvas::resize::ResizeGesture,
    },
    Endpoint {
        id: Id,
        start: bool,
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
    pending_clipboard: Option<ClipboardAction>,
    pending_paste: Option<(Pos, f64)>,
    paste_count: u32,
    context_at: Pos,
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
            pending_clipboard: None,
            pending_paste: None,
            paste_count: 0,
            context_at: Pos::default(),
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
    fn drawing_element(&self, kind: Kind, start: Pos, end: Pos) -> Element {
        let mut element = Element::new(kind, self.snap_point(start), self.snap_point(end), self.state.paint.clone());
        self.board.attach_connector(&mut element, start, end, 12.0 / self.state.camera.zoom);
        element
    }
    fn delete(&mut self) {
        self.history.sync(&self.board, false);
        self.board.delete(&self.state.selection);
        self.state.selection.clear();
        self.history.sync(&self.board, false);
    }
    fn duplicate(&mut self) {
        if self.state.selection.is_empty() {
            return;
        }
        self.history.sync(&self.board, false);
        match self.board.paste(&self.board.fragment(&self.state.selection), Pos::new(20.0, 20.0)) {
            Ok(selected) => {
                self.state.selection = selected;
                self.state.status.clear();
            }
            Err(error) => self.state.status = error,
        }
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
            let mixed_width = self
                .state
                .selection
                .iter()
                .filter_map(|id| self.board.element(*id))
                .any(|e| e.paint.width != paint.width);
            let mixed_font = self
                .state
                .selection
                .iter()
                .filter_map(|id| self.board.element(*id))
                .any(|e| e.paint.font_size != paint.font_size);
            let mut width_changed = size_presets(ui, tr("drawing.stroke_width"), &mut paint.width, STROKE_SIZES, mixed_width);
            let mut font_changed = size_presets(ui, tr("drawing.font_size"), &mut paint.font_size, FONT_SIZES, mixed_font);
            ui.checkbox(&mut paint.sketch, tr("canvas.hand_drawn_sketch"));
            egui::CollapsingHeader::new(tr("drawing.advanced_sizes"))
                .id_salt("advanced_sizes")
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("exact_sizes").num_columns(2).show(ui, |ui| {
                        width_changed |= exact_size_row(ui, tr("drawing.stroke_width"), &mut paint.width, 0.5..=12.0);
                        font_changed |= exact_size_row(ui, tr("drawing.font_size"), &mut paint.font_size, 8.0..=96.0);
                        if self.state.selection.len() == 1 {
                            let id = *self.state.selection.first().unwrap();
                            let geometry = self.board.geometry();
                            let e = self.board.element(id).unwrap();
                            let bounds = geometry.element_bounds(e);
                            let points = e.kind.is_connector().then(|| geometry.points(e));
                            let (mut width, mut height) = (bounds.w, bounds.h);
                            exact_size_row(ui, tr("common.width"), &mut width, 1.0..=10_000.0);
                            exact_size_row(ui, tr("common.height"), &mut height, 1.0..=10_000.0);
                            if width != bounds.w || height != bounds.h {
                                let e = self.board.elements.iter_mut().find(|e| e.id == id).unwrap();
                                if let Some(points) = points {
                                    e.set_endpoints(points[0], points[1]);
                                    e.start_attachment = None;
                                    e.end_attachment = None;
                                }
                                e.bounds.w = width;
                                e.bounds.h = height;
                            }
                        }
                    });
                });
            if paint != before || width_changed || font_changed {
                self.state.paint = paint.clone();
                for e in &mut self.board.elements {
                    if self.state.selection.contains(&e.id) {
                        if paint.stroke != before.stroke {
                            e.paint.stroke = paint.stroke;
                        }
                        if paint.fill != before.fill {
                            e.paint.fill = paint.fill;
                        }
                        if width_changed {
                            e.paint.width = paint.width;
                        }
                        if font_changed {
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
        if let Some(action) = self.state.pending_clipboard.take() {
            self.clipboard_action(ui.ctx(), action, None);
        }
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
