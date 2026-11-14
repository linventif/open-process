//! Mermaid source editor and live preview, integrated through the notation registry.
pub(crate) mod preview;

use crate::canvas::{self, View};
use crate::editor::layout::{CanvasControls, DiagramLayout, FooterStatus};
use crate::editor::*;
use crate::history::History;
use crate::platform::jobs::Job;
use egui::{Rect, Ui};
use op_core::document::{Document, DocumentCodec, JsonCodec};
use op_core::geom::Pos;
use op_core::i18n::{key, tr, trf};
use op_core::mermaid::{Diagram, TEMPLATES};
use serde_json::Value;

#[derive(Clone)]
pub struct MermaidView {
    uid: u64,
    camera: View,
    canvas: Rect,
    fit: bool,
    inspector: Option<bool>,
    templates: bool,
}

impl Default for MermaidView {
    fn default() -> Self {
        Self {
            uid: canvas::next_uid(),
            camera: View::default(),
            canvas: Rect::NOTHING,
            fit: true,
            inspector: None,
            templates: false,
        }
    }
}

struct Rendered {
    svg: String,
    texture: egui::TextureHandle,
    size: egui::Vec2,
}

pub struct MermaidEditor {
    diagram: Diagram,
    history: History<Diagram>,
    state: MermaidView,
    requested_source: Option<String>,
    requested_lang: Option<op_core::i18n::Lang>,
    changed_at: f64,
    pending: Option<Job>,
    result: Option<Result<Rendered, String>>,
    convert_uml: bool,
}

impl MermaidEditor {
    fn new(diagram: Diagram) -> Self {
        Self {
            history: History::new(&diagram),
            diagram,
            state: MermaidView::default(),
            requested_source: None,
            requested_lang: None,
            changed_at: 0.0,
            pending: None,
            result: None,
            convert_uml: false,
        }
    }

    fn update_preview(&mut self, ui: &Ui) {
        let now = ui.input(|i| i.time);
        let language = op_core::i18n::lang();
        if self.requested_source.as_ref() != Some(&self.diagram.source) || self.requested_lang != Some(language) {
            // Dropping a stale job terminates its web worker. Its result cannot replace this source.
            self.pending = None;
            self.result = None;
            self.requested_source = Some(self.diagram.source.clone());
            self.requested_lang = Some(language);
            self.changed_at = now;
        }
        if self.pending.is_none() && self.result.is_none() && now - self.changed_at >= 0.3 {
            let input = serde_json::to_string(&preview::Input {
                task: "mermaid.preview".into(),
                source: self.diagram.source.clone(),
                lang: language,
            })
            .unwrap();
            match Job::spawn(input, preview::compute_json, ui.ctx()) {
                Ok(job) => self.pending = Some(job),
                Err(e) => self.result = Some(Err(e)),
            }
        }
        if let Some(result) = self.pending.as_mut().and_then(Job::poll) {
            self.pending = None;
            self.result = Some(result.and_then(|raw| {
                let preview: preview::Preview = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                let image = image::load_from_memory(&preview.png).map_err(|e| e.to_string())?.into_rgba8();
                let image = egui::ColorImage::from_rgba_unmultiplied([image.width() as usize, image.height() as usize], image.as_raw());
                Ok(Rendered {
                    svg: preview.svg,
                    size: egui::vec2(preview.width, preview.height),
                    texture: ui.ctx().load_texture("mermaid-preview", image, egui::TextureOptions::LINEAR),
                })
            }));
        }
        if self.result.is_none() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
        }
    }

    fn show(&mut self, ui: &mut Ui) {
        let layout = DiagramLayout::new(ui, self.state.uid, self.state.inspector);
        layout.tools(ui, |ui| {
            if ui
                .add_enabled(
                    op_core::mermaid::is_class_source(&self.diagram.source),
                    egui::Button::new(tr("conversion.to_uml_classes")),
                )
                .on_hover_text(tr("conversion.mermaid_classes_only"))
                .clicked()
            {
                self.convert_uml = true;
            }
        });
        let mut editing = false;
        layout.inspector(ui, |ui| {
            DiagramLayout::inspector_tabs(
                ui,
                &mut self.state.templates,
                &[(false, tr("mermaid.source")), (true, tr("mermaid.templates"))],
                |_, _| {},
            );
            DiagramLayout::inspector_body(ui, |ui| {
                if self.state.templates {
                    ui.label(tr("mermaid.template_hint"));
                    for template in TEMPLATES {
                        if ui.button(tr(template.label)).clicked() {
                            self.history.sync(&self.diagram, false);
                            self.diagram.source = template.source.into();
                            self.history.sync(&self.diagram, false);
                            self.state.templates = false;
                            self.state.fit = true;
                        }
                    }
                } else {
                    ui.label(tr("mermaid.edit_hint"));
                    egui::ScrollArea::horizontal().id_salt("mermaid-code-scroll").show(ui, |ui| {
                        let response = ui
                            .add(
                                egui::TextEdit::multiline(&mut self.diagram.source)
                                    .id_salt("mermaid-source")
                                    .hint_text(tr("mermaid.code"))
                                    .code_editor()
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(24),
                            )
                            .accessible_name(tr("mermaid.code"));
                        editing = response.has_focus();
                    });
                }
            });
        });
        self.history.sync(&self.diagram, editing);
        self.update_preview(ui);
        let status = match &self.result {
            Some(Ok(_)) => tr("mermaid.preview"),
            Some(Err(_)) => tr("mermaid.preview_error"),
            None => tr("mermaid.rendering"),
        };
        layout.configuration(
            ui,
            CanvasControls {
                view: &mut self.state.camera,
                canvas: self.state.canvas,
                fit: &mut self.state.fit,
                snap: None,
                inspector: &mut self.state.inspector,
                issues: None,
            },
            FooterStatus {
                summary: status.into(),
                ..Default::default()
            },
            |_| {},
        );
        layout.canvas(ui, |ui| {
            let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::drag());
            let rect = response.rect;
            self.state.canvas = rect;
            painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
            self.state.camera.handle_navigation(ui, &response, rect);
            match &self.result {
                Some(Ok(preview)) => {
                    if self.state.fit && rect.is_positive() {
                        self.state
                            .camera
                            .fit(rect, Pos::default(), Pos::new(preview.size.x, preview.size.y));
                        self.state.fit = false;
                    }
                    let image_rect = Rect::from_min_size(
                        self.state.camera.to_screen(rect, Pos::default()),
                        preview.size * self.state.camera.zoom,
                    );
                    painter.image(
                        preview.texture.id(),
                        image_rect,
                        Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                result => {
                    let text = match result {
                        Some(Err(e)) => format!("{}\n\n{e}", tr("mermaid.preview_error")),
                        _ => tr("mermaid.rendering").into(),
                    };
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(20.0)));
                    egui::ScrollArea::vertical().show(&mut child, |ui| {
                        ui.label(text);
                    });
                }
            }
        });
    }
}

impl EditorBackend for MermaidEditor {
    type ViewState = MermaidView;
    fn capture_view(&self) -> MermaidView {
        self.state.clone()
    }
    fn restore_view(&mut self, state: &MermaidView) {
        self.state = state.clone();
    }
    fn fresh_view(&self) -> MermaidView {
        MermaidView::default()
    }
    fn save_view(state: &MermaidView) -> Value {
        serde_json::json!({"camera":state.camera,"inspector":state.inspector,"templates":state.templates})
    }
    fn load_view(state: &mut MermaidView, value: &Value) {
        state.inspector = value["inspector"].as_bool();
        state.templates = value["templates"].as_bool().unwrap_or(false);
        if let Ok(camera) = serde_json::from_value::<View>(value["camera"].clone())
            && camera.offset.is_finite()
            && camera.zoom.is_finite()
            && (0.1..=5.0).contains(&camera.zoom)
        {
            state.camera = camera;
            state.fit = false;
        }
    }
    fn kind(&self) -> &'static str {
        "mermaid"
    }
    fn name(&self) -> &str {
        &self.diagram.name
    }
    fn rename(&mut self, name: String) {
        self.history.sync(&self.diagram, false);
        self.diagram.name = name;
        self.history.sync(&self.diagram, false);
    }
    fn content_generation(&self) -> Option<u64> {
        Some(self.history.generation())
    }
    fn has_content(&self) -> bool {
        !self.diagram.source.trim().is_empty()
    }
    fn document(&self) -> Document {
        JsonCodec::new("mermaid").encode(&self.diagram)
    }
    fn cancel_edit(&mut self) {
        self.history.cancel(&mut self.diagram);
    }
    fn thumbnail_rect(&self) -> Option<Rect> {
        Some(self.state.canvas)
    }
    fn ui(&mut self, ui: &mut Ui, _: &str) -> Vec<HostEvent> {
        self.show(ui);
        if std::mem::take(&mut self.convert_uml) {
            return self.execute("mermaid.convert-to-uml", 0.0);
        }
        vec![]
    }
    fn commands(&self) -> &'static [CommandDescriptor] {
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
                id: FIT,
                label: key("canvas.zoom_fit"),
                group: CommandGroup::View,
            },
            CommandDescriptor {
                id: "mermaid.convert-to-uml",
                label: key("conversion.to_uml_classes"),
                group: CommandGroup::Tools,
            },
        ];
        COMMANDS
    }
    fn enabled(&self, command: &str) -> bool {
        match command {
            UNDO => self.history.can_undo(),
            REDO => self.history.can_redo(),
            FIT => true,
            "mermaid.convert-to-uml" => op_core::mermaid::is_class_source(&self.diagram.source),
            _ => false,
        }
    }
    fn execute(&mut self, command: &str, _: f64) -> Vec<HostEvent> {
        match command {
            "mermaid.convert-to-uml" => {
                return match op_core::mermaid::conversion::to_uml(&self.diagram) {
                    Ok(converted) => {
                        let mut events = vec![HostEvent::Open(JsonCodec::new("uml.class").encode(&converted.diagram))];
                        if !converted.warnings.is_empty() {
                            events.push(HostEvent::Message(trf(
                                "conversion.mermaid_warnings",
                                &[&converted.warnings.join(" · ")],
                            )));
                        }
                        events
                    }
                    Err(error) => vec![HostEvent::Message(error)],
                };
            }
            UNDO => self.history.undo(&mut self.diagram),
            REDO => self.history.redo(&mut self.diagram),
            FIT => self.state.fit = true,
            _ => {}
        }
        vec![]
    }
    fn export(&self) -> Result<ExportData, String> {
        Ok(ExportData {
            suffix: "mmd",
            bytes: self.diagram.source.as_bytes().to_vec(),
        })
    }
    fn export_label(&self) -> &'static str {
        key("mermaid.export_source")
    }
    fn exports(&self) -> Vec<ExportDescriptor> {
        vec![
            ExportDescriptor {
                id: "standard",
                label: key("mermaid.export_source"),
            },
            ExportDescriptor {
                id: "svg",
                label: key("mermaid.export_svg"),
            },
        ]
    }
    fn export_as(&self, format: &str) -> Result<ExportData, String> {
        match format {
            "standard" => self.export(),
            "svg" if self.requested_source.as_ref() == Some(&self.diagram.source) => match &self.result {
                Some(Ok(preview)) => Ok(ExportData {
                    suffix: "svg",
                    bytes: preview.svg.as_bytes().to_vec(),
                }),
                Some(Err(e)) => Err(e.clone()),
                None => Err(tr("mermaid.rendering").into()),
            },
            "svg" => Err(tr("mermaid.rendering").into()),
            _ => Err(format!("Unknown export format: {format}")),
        }
    }
}

pub(crate) fn register(registry: &mut DiagramRegistry) {
    registry.diagrams.push(DiagramDescriptor {
        id: "mermaid",
        aliases: &[],
        home_order: 11,
        icon: crate::icons::mermaid,
        new_label: key("mermaid.new"),
        title: key("mermaid.title"),
        description: key("mermaid.description"),
        badge: "MERMAID",
        color: egui::Color32::from_rgb(190, 58, 125),
        create: |n| JsonCodec::new("mermaid").encode(&Diagram::new(trf("mermaid.name", &[&n]))),
        open: |d| Ok(Session::boxed(MermaidEditor::new(JsonCodec::new("mermaid").decode(d)?))),
    });
    for template in TEMPLATES {
        // Descriptors use function pointers. Each factory is selected statically below.
        registry.examples.push(ExampleDescriptor {
            id: template.id,
            kind: "mermaid",
            label: template.label,
            create: match template.id {
                "mermaid.sequence" => || example(1),
                "mermaid.class" => || example(2),
                "mermaid.state" => || example(3),
                "mermaid.er" => || example(4),
                _ => || example(0),
            },
        });
    }
    registry.imports.push(ImportDescriptor {
        extensions: &["mmd", "mermaid"],
        accepts: op_core::mermaid::accepts,
        decode: |source| {
            Ok(JsonCodec::new("mermaid").encode(&Diagram {
                name: tr("mermaid.imported").into(),
                source: source.into(),
            }))
        },
    });
}

fn example(index: usize) -> Document {
    let t = &TEMPLATES[index];
    JsonCodec::new("mermaid").encode(&Diagram {
        name: tr(t.label).into(),
        source: t.source.into(),
    })
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    fn frame(editor: &mut MermaidEditor, ctx: &egui::Context) {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0))),
                ..Default::default()
            },
            |ui| editor.show(ui),
        );
        output.textures_delta.clear();
    }

    fn wait(editor: &mut MermaidEditor, ctx: &egui::Context) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        frame(editor, ctx);
        while editor.result.is_none() {
            frame(editor, ctx);
            assert!(std::time::Instant::now() < deadline, "preview timed out");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    #[test]
    fn registry_imports_mermaid_without_stealing_logs_and_opx_preserves_invalid_drafts() {
        let registry = crate::composition::registry();
        let descriptor = registry.find("mermaid").unwrap();
        assert_eq!(registry.examples.iter().filter(|e| e.kind == "mermaid").count(), 5);
        assert!(registry.extensions().contains(&"mmd"));
        assert!(registry.extensions().contains(&"mermaid"));
        for template in TEMPLATES {
            let source = registry.load(template.source.as_bytes()).unwrap();
            assert_eq!(source.kind, "mermaid");
            assert_eq!(registry.open(&source).unwrap().export().unwrap().bytes, template.source.as_bytes());
            assert!(registry.open(&registry.example(template.id).unwrap()).is_ok());
        }
        assert_eq!(registry.load(b"a b c\na c b").unwrap().kind, "event-log");
        let draft = registry.load(b"flowchart TD\n A[").unwrap();
        let session = registry.open(&Document::read(draft.to_json().as_bytes()).unwrap()).unwrap();
        assert_eq!(session.export().unwrap().bytes, b"flowchart TD\n A[");
        assert!(registry.open(&(descriptor.create)(1)).is_ok());
    }

    #[test]
    fn stale_jobs_and_svg_exports_follow_current_source_and_errors_keep_source_exportable() {
        let ctx = egui::Context::default();
        let mut editor = MermaidEditor::new(Diagram::new("Test"));
        frame(&mut editor, &ctx);
        editor.changed_at = -1.0;
        frame(&mut editor, &ctx);
        assert!(editor.pending.is_some() || editor.result.is_some());
        editor.diagram.source = "flowchart LR\n Current --> Result\n".into();
        assert!(
            editor.export_as("svg").is_err(),
            "a previous preview must never be exported for changed code"
        );
        wait(&mut editor, &ctx);
        let svg = String::from_utf8(editor.export_as("svg").unwrap().bytes).unwrap();
        assert!(svg.contains("Current"));
        assert!(!svg.contains("Process order"));
        editor.diagram.source = "not Mermaid".into();
        frame(&mut editor, &ctx);
        wait(&mut editor, &ctx);
        assert!(matches!(editor.result, Some(Err(_))));
        assert!(editor.export_as("svg").is_err());
        assert_eq!(editor.export().unwrap().bytes, b"not Mermaid");
        editor.diagram.source = TEMPLATES[1].source.into();
        frame(&mut editor, &ctx);
        wait(&mut editor, &ctx);
        assert!(editor.export_as("svg").is_ok(), "corrected code recovers from a render error");
    }

    #[test]
    fn shared_history_and_independent_views_roundtrip_without_copying_the_document() {
        let ctx = egui::Context::default();
        let original = Diagram::new("Test");
        let mut editor = MermaidEditor::new(original.clone());
        editor.diagram.source = TEMPLATES[1].source.into();
        editor.history.sync(&editor.diagram, false);
        editor.execute(UNDO, 0.0);
        assert_eq!(editor.diagram, original);
        editor.execute(REDO, 0.0);
        assert_eq!(editor.diagram.source, TEMPLATES[1].source);
        editor.history.sync(&editor.diagram, true);
        editor.diagram.source = "aborted".into();
        editor.cancel_edit();
        assert_eq!(editor.diagram.source, TEMPLATES[1].source);
        let mut session = Session::boxed(editor);
        let document = session.snapshot();
        let mut output = ctx.run_ui(Default::default(), |ui| {
            session.render(ui, 1, "");
            session.render(ui, 2, "");
        });
        output.textures_delta.clear();
        let mut workspace = session.workspace();
        assert_eq!(workspace.as_array().unwrap().len(), 2);
        workspace[0][1]["camera"]["zoom"] = serde_json::json!(1.8);
        workspace[0][1]["inspector"] = serde_json::json!(true);
        workspace[1][1]["camera"]["zoom"] = serde_json::json!(0.5);
        session.restore_workspace(&workspace);
        let mut restored = crate::composition::registry()
            .open(&match document.clone() {
                StoredDocument::Editable(d) => d,
                _ => unreachable!(),
            })
            .unwrap();
        restored.restore_workspace(&session.workspace());
        let zooms: Vec<_> = restored
            .workspace()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v[1]["camera"]["zoom"].as_f64().unwrap())
            .collect();
        assert!(zooms.iter().any(|z| (z - 1.8).abs() < 0.001) && zooms.contains(&0.5));
        assert_eq!(restored.snapshot(), document);
        session.rename("Renamed".into());
        session.execute(UNDO, 0.0);
        assert_eq!(session.name(), "Test");
    }
}
