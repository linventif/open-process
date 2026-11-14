//! Cached process evaluation uses the same bounded transport as other analyses.
use crate::platform::jobs::Job;
use op_core::{
    bpmn::BpmnDiagram,
    evaluation::Evaluation,
    i18n::{Lang, lang},
};
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Serialize, Deserialize)]
struct Input {
    task: String,
    diagram: BpmnDiagram,
    lang: Lang,
}

#[derive(Default)]
pub(super) struct EvaluationState {
    stamp: Option<(u64, Lang)>,
    pending: Option<Job>,
    pub result: Option<Result<Evaluation, String>>,
}

impl EvaluationState {
    pub fn invalidate(&mut self) {
        *self = Self::default();
    }

    pub fn update(&mut self, diagram: &BpmnDiagram, generation: u64, ctx: &egui::Context) {
        let stamp = (generation, lang());
        if self.stamp != Some(stamp) {
            self.invalidate();
            self.stamp = Some(stamp);
            let input = serde_json::to_string(&Input {
                task: "bpmn.evaluation".into(),
                diagram: diagram.clone(),
                lang: stamp.1,
            })
            .unwrap();
            match Job::spawn(input, compute_json, ctx) {
                Ok(job) => self.pending = Some(job),
                Err(error) => self.result = Some(Err(error)),
            }
        }
        if let Some(result) = self.pending.as_mut().and_then(Job::poll) {
            self.pending = None;
            self.result = Some(result.and_then(|raw| serde_json::from_str(&raw).map_err(|e| e.to_string())));
        }
        if self.pending.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }
}

pub(crate) fn compute_json(input: String, stop: Arc<AtomicBool>) -> Result<String, String> {
    let input: Input = serde_json::from_str(&input).map_err(|e| e.to_string())?;
    op_core::i18n::with_lang(input.lang, || {
        input.diagram.check_storage()?;
        let result = op_core::evaluation::evaluate(&input.diagram)?;
        if stop.load(Ordering::Relaxed) {
            return Err("Cancelled".into());
        }
        serde_json::to_string(&result).map_err(|e| e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn evaluation_discards_old_results_and_reuses_an_unchanged_snapshot() {
        let ctx = egui::Context::default();
        let mut state = EvaluationState::default();
        let mut diagram = op_core::examples::evaluation_process();
        state.update(&diagram, 0, &ctx);
        diagram
            .nodes
            .iter_mut()
            .filter(|n| matches!(n.kind, op_core::bpmn::NodeKind::Task(_)))
            .for_each(|n| n.duration = 17.0);
        state.update(&diagram, 1, &ctx);
        let expected = op_core::evaluation::evaluate(&diagram).unwrap().mean_duration;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while state.result.is_none() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(1));
            state.update(&diagram, 1, &ctx);
        }
        assert_eq!(state.result.as_ref().unwrap().as_ref().unwrap().mean_duration, expected);
        state.update(&diagram, 1, &ctx);
        assert!(state.pending.is_none());
        let input = serde_json::to_string(&Input {
            task: "bpmn.evaluation".into(),
            diagram,
            lang: Lang::En,
        })
        .unwrap();
        assert!(compute_json(input, Arc::new(AtomicBool::new(true))).is_err());
    }
}
