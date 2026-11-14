use super::*;
use crate::editor::*;
use op_core::document::{Document, DocumentCodec, JsonCodec, LogDocument};

impl EditorBackend for MiningEditor {
    fn thumbnail_rect(&self) -> Option<egui::Rect> {
        // A canvas-free document uses the pane once discovery is ready. Do not
        // persist a loading spinner as its only preview for an unchanged model.
        if self.computed.as_ref().is_some_and(|c| c.result.is_ok()) {
            None
        } else {
            Some(egui::Rect::NOTHING)
        }
    }
    type ViewState = ();
    fn capture_view(&self) {}
    fn restore_view(&mut self, _: &()) {}
    fn fresh_view(&self) {}
    fn kind(&self) -> &'static str {
        "event-log"
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn rename(&mut self, name: String) {
        self.history.sync(&self.log_document(), false);
        self.name = name;
        self.history.sync(&self.log_document(), false);
    }
    fn content_generation(&self) -> Option<u64> {
        Some(self.history.generation())
    }
    fn document(&self) -> Document {
        JsonCodec::new("event-log").encode(&LogDocument {
            name: self.name.clone(),
            text: self.text.clone(),
            manual: self.manual,
            answers: self.answers.clone(),
        })
    }
    fn cancel_edit(&mut self) {
        let mut doc = self.log_document();
        self.history.cancel(&mut doc);
        self.restore_document(doc);
    }
    fn ui(&mut self, ui: &mut Ui, _: &str) -> Vec<HostEvent> {
        MiningEditor::ui(self, ui);
        match self.action.take() {
            Some(MiningAction::OpenPetri(m)) => vec![HostEvent::Open(JsonCodec::new("petri").encode(&m))],
            Some(MiningAction::OpenBpmn(m)) => vec![HostEvent::Open(JsonCodec::new("bpmn").encode(&m))],
            None => vec![],
        }
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
        ];
        COMMANDS
    }
    fn enabled(&self, command: &str) -> bool {
        match command {
            UNDO => self.history.can_undo(),
            REDO => self.history.can_redo(),
            _ => false,
        }
    }
    fn execute(&mut self, command: &str, _: f64) -> Vec<HostEvent> {
        let mut doc = self.log_document();
        match command {
            UNDO => self.history.undo(&mut doc),
            REDO => self.history.redo(&mut doc),
            _ => return vec![],
        }
        self.restore_document(doc);
        vec![]
    }
    fn export(&self) -> Result<ExportData, String> {
        let computed = self
            .computed
            .as_ref()
            .filter(|c| c.text == self.text && c.lang == op_core::i18n::lang())
            .ok_or_else(|| tr("mining.analysis_pending").to_string())?;
        let (log, result) = computed.result.as_ref().map_err(Clone::clone)?;
        Ok(ExportData {
            suffix: "txt",
            bytes: result.report(log).into_bytes(),
        })
    }
    fn export_label(&self) -> &'static str {
        key("mining.export_alpha_write_up_txt")
    }
}

use crate::theme;

pub(crate) fn register(registry: &mut DiagramRegistry) {
    registry.diagrams.extend([DiagramDescriptor {
        id: "event-log",
        home_order: 6,
        icon: crate::icons::mining,
        aliases: &["log"],
        new_label: key("mining.new_event_log_process_discovery"),
        title: key("mining.discover_process"),
        description: key("mining.turn_event_log_into_relation_matrix_petri"),
        badge: "ALPHA",
        color: theme::KIND_LOG,
        create: |n| {
            JsonCodec::new("event-log").encode(&LogDocument {
                name: trf("mining.log", &[&n]),
                ..Default::default()
            })
        },
        open: |d| Ok(Session::boxed(MiningEditor::new(JsonCodec::new("event-log").decode(d)?))),
    }]);
    registry.examples.extend([
        ExampleDescriptor {
            id: "log.first",
            kind: "event-log",
            label: key("mining.log_abcd_acbd_ef"),
            create: || log_example(EXAMPLE_1),
        },
        ExampleDescriptor {
            id: "log.second",
            kind: "event-log",
            label: key("mining.log_abcd_acbd_aed"),
            create: || log_example(EXAMPLE_2),
        },
    ]);
    registry.imports.extend([
        ImportDescriptor {
            extensions: &["xes"],
            accepts: |s| s.starts_with('<') && op_core::xml::root_name(s).as_deref() == Some("log"),
            decode: |s| {
                op_core::mining::EventLog::from_xes(s)?;
                Ok(log_example(s))
            },
        },
        ImportDescriptor {
            extensions: &["csv", "txt"],
            accepts: |s| !s.starts_with(['<', '{']) && !s.trim().is_empty() && !op_core::mermaid::accepts(s),
            decode: |s| {
                let log = op_core::mining::EventLog::parse(s)?;
                if log.traces.is_empty() {
                    return Err("No trace found".into());
                }
                Ok(log_example(s))
            },
        },
    ]);
}
fn log_example(text: &str) -> Document {
    JsonCodec::new("event-log").encode(&LogDocument {
        name: "Imported log".into(),
        text: text.into(),
        ..Default::default()
    })
}
