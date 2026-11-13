//! Typed Petri analysis task. Snapshots and limits identify all input dependencies.
use super::*;
use crate::platform::jobs::Job;
use op_core::i18n::{Lang, lang};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(serde::Serialize, serde::Deserialize)]
struct Input {
    net: PetriNet,
    limit: usize,
    lang: Lang,
}

pub(super) struct PendingAnalysis {
    pub net: PetriNet,
    pub limit: usize,
    pub lang: Lang,
    pub job: Job,
}

pub(crate) fn compute_json(input: String, stop: Arc<AtomicBool>) -> Result<String, String> {
    let input: Input = serde_json::from_str(&input).map_err(|e| e.to_string())?;
    #[cfg(target_arch = "wasm32")]
    op_core::i18n::set_lang(input.lang);
    let cancelled = || stop.load(Ordering::Relaxed);
    let ss = StateSpace::explore_cancellable(&input.net, input.limit, &cancelled);
    if cancelled() {
        return Err("Cancelled".into());
    }
    let beh = BehaviouralReport::compute(&input.net, &ss);
    let st = StructuralReport::compute(&input.net);
    if cancelled() {
        return Err("Cancelled".into());
    }
    let pinv = analysis::p_invariants(&input.net);
    if cancelled() {
        return Err("Cancelled".into());
    }
    let tinv = analysis::t_invariants(&input.net);
    if cancelled() {
        return Err("Cancelled".into());
    }
    let sound = analysis::soundness_cancellable(&input.net, input.limit, &cancelled);
    if cancelled() {
        return Err("Cancelled".into());
    }
    serde_json::to_string(&AnalysisCache {
        net: input.net,
        limit: input.limit,
        lang: input.lang,
        ss,
        beh,
        st,
        pinv,
        tinv,
        sound,
    })
    .map_err(|e| e.to_string())
}

impl PetriEditor {
    pub(super) fn ensure_analysis(&mut self, ui: &mut Ui) -> bool {
        let net = self.net.analysis_model();
        let limit = self.state_limit;
        let language = lang();
        if self
            .analysis
            .as_ref()
            .is_some_and(|a| a.net == net && a.limit == limit && a.lang == language)
        {
            return true;
        }
        self.analysis = None;
        if self.analysis_cancelled.as_ref().is_some_and(|(n, l)| *n == net && *l == limit) {
            ui.label(tr("Analysis cancelled"));
            if ui.button(tr("Recompute")).clicked() {
                self.analysis_cancelled = None;
            }
            return false;
        }
        if self
            .analysis_job
            .as_ref()
            .is_some_and(|p| p.net != net || p.limit != limit || p.lang != language)
        {
            self.analysis_job = None;
        }
        if self.analysis_job.is_none() {
            let input = serde_json::to_string(&Input {
                net: net.clone(),
                limit,
                lang: language,
            })
            .unwrap();
            match Job::spawn(input, compute_json, ui.ctx()) {
                Ok(job) => {
                    self.analysis_job = Some(PendingAnalysis {
                        net: net.clone(),
                        limit,
                        lang: language,
                        job,
                    })
                }
                Err(e) => {
                    self.status = e.clone();
                    self.analysis_cancelled = Some((net, limit));
                    ui.colored_label(ui.visuals().error_fg_color, e);
                    return false;
                }
            }
        }
        if let Some(result) = self.analysis_job.as_mut().and_then(|p| p.job.poll()) {
            self.analysis_job = None;
            match result.and_then(|raw| serde_json::from_str::<AnalysisCache>(&raw).map_err(|e| e.to_string())) {
                Ok(cache) if cache.net == net && cache.limit == limit && cache.lang == language => {
                    self.analysis = Some(cache);
                    return true;
                }
                Ok(_) => {}
                Err(e) => {
                    self.status = e.clone();
                    ui.colored_label(ui.visuals().error_fg_color, e);
                    self.analysis_cancelled = Some((net.clone(), limit));
                }
            }
        }
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(tr("Computing analysis…"));
            if ui.button(tr("Cancel")).clicked() {
                self.analysis_job = None;
                self.analysis_cancelled = Some((net, limit));
            }
        });
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
        false
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    fn wait(editor: &mut PetriEditor, ctx: &egui::Context) {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let mut ready = false;
            let mut output = ctx.run_ui(Default::default(), |ui| {
                ready = editor.ensure_analysis(ui);
            });
            output.textures_delta.clear();
            if ready {
                return;
            }
            assert!(std::time::Instant::now() < until, "analysis did not complete: {}", editor.status);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    #[test]
    fn stale_results_are_replaced_and_layout_preserves_the_cache() {
        let mut editor = PetriEditor::new(op_core::examples::mutual_exclusion());
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(Default::default(), |ui| {
            editor.ensure_analysis(ui);
        });
        output.textures_delta.clear();
        editor.net.places[0].tokens += 1;
        let expected = editor.net.analysis_model();
        wait(&mut editor, &ctx);
        assert_eq!(editor.analysis.as_ref().unwrap().net, expected);
        editor.net.places[0].pos.x += 100.0;
        let mut ready = false;
        let mut output = ctx.run_ui(Default::default(), |ui| {
            ready = editor.ensure_analysis(ui);
        });
        output.textures_delta.clear();
        assert!(ready);
        assert!(editor.analysis_job.is_none());
        assert_eq!(editor.analysis.as_ref().unwrap().net, expected);
    }
    #[test]
    fn graph_camera_survives_workspace_restore_and_cache_rebuild() {
        use crate::editor::EditorBackend;
        let ctx = egui::Context::default();
        let net = op_core::examples::mutual_exclusion();
        let mut first = PetriEditor::new(net.clone());
        wait(&mut first, &ctx);
        let mut output = ctx.run_ui(Default::default(), |ui| first.graph_window(ui));
        output.textures_delta.clear();
        first.graph_view = View {
            offset: vec2(80.0, 120.0),
            zoom: 1.8,
        };
        let saved = PetriEditor::save_view(&first.capture_view());
        let mut restored = PetriEditor::new(net);
        let mut state = restored.fresh_view();
        PetriEditor::load_view(&mut state, &saved);
        restored.restore_view(&state);
        wait(&mut restored, &ctx);
        let mut output = ctx.run_ui(Default::default(), |ui| restored.graph_window(ui));
        output.textures_delta.clear();
        assert_eq!(restored.graph_view.zoom, 1.8);
        assert_eq!(restored.graph_view.offset, vec2(80.0, 120.0));
    }
}
