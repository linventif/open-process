//! Discovery and soundness execute on a private snapshot, outside the UI thread.
use super::*;
use op_core::{
    analysis::soundness_cancellable,
    i18n::Lang,
    mining::{discover_with_options_cancellable, replay_cancellable},
};
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Serialize, Deserialize)]
pub(super) struct Input {
    pub task: String,
    pub text: String,
    pub lang: Lang,
    #[serde(default)]
    pub exam: bool,
    #[serde(default)]
    pub discovery: DiscoveryOptions,
}

pub(crate) fn compute_json(input: String, stop: Arc<AtomicBool>) -> Result<String, String> {
    let input: Input = serde_json::from_str(&input).map_err(|e| e.to_string())?;
    op_core::i18n::with_lang(input.lang, || {
        let cancelled = || stop.load(Ordering::Relaxed);
        let result = if input.text.len() > MAX_SOURCE_BYTES {
            Err(tr("mining.analysis_limit").into())
        } else {
            EventLog::parse(&input.text).and_then(|log| {
                if log.traces.is_empty() {
                    return Err(tr("mining.no_trace_found").into());
                }
                if input.exam {
                    let activities = log.activities();
                    if activities.len() > 128 || log.traces.iter().map(Vec::len).sum::<usize>() > 100_000 {
                        return Err(tr("mining.analysis_limit").into());
                    }
                    return Ok(MiningOutput::Exam(activities));
                }
                let result = discover_with_options_cancellable(&log, &input.discovery, &cancelled)?;
                Ok(MiningOutput::Discovery(log, Box::new(result)))
            })
        };
        let soundness = match &result {
            Ok(MiningOutput::Discovery(_, r)) => Some(soundness_cancellable(&r.net, 5000, &cancelled)),
            _ => None,
        };
        let (replays, report) = match &result {
            Ok(MiningOutput::Discovery(log, discovered)) => {
                let mut replays = Vec::new();
                for (trace, _) in log.variants() {
                    if cancelled() {
                        return Err("Cancelled".into());
                    }
                    replays.push(replay_cancellable(&discovered.net, &discovered.project_trace(&trace), &cancelled));
                }
                if cancelled() {
                    return Err("Cancelled".into());
                }
                let report = discovered.report_with_replays(log, &replays);
                (replays, report)
            }
            _ => (Vec::new(), String::new()),
        };
        if cancelled() {
            return Err("Cancelled".into());
        }
        serde_json::to_string(&Computed {
            text: input.text,
            lang: input.lang,
            exam: input.exam,
            discovery: input.discovery,
            result,
            soundness,
            replays,
            report,
        })
        .map_err(|e| e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exam_transport_returns_only_activity_headers_without_solutions() {
        let input = serde_json::to_string(&Input {
            task: "mining.discovery".into(),
            text: "ABCD\nACBD\nAED".into(),
            lang: Lang::En,
            exam: true,
            discovery: DiscoveryOptions::default(),
        })
        .unwrap();
        let raw = compute_json(input.clone(), Arc::new(AtomicBool::new(false))).unwrap();
        let computed: Computed = serde_json::from_str(&raw).unwrap();
        assert!(computed.exam);
        assert!(computed.soundness.is_none());
        assert!(computed.replays.is_empty());
        assert!(computed.report.is_empty());
        let MiningOutput::Exam(activities) = computed.result.unwrap() else {
            panic!("exam must not run discovery")
        };
        assert_eq!(activities, ["A", "B", "C", "D", "E"]);
        assert!(compute_json(input, Arc::new(AtomicBool::new(true))).is_err());
    }

    #[test]
    fn discovery_transport_preserves_inputs_and_rejects_cancellation() {
        let input = serde_json::to_string(&Input {
            task: "mining.discovery".into(),
            text: "ABCD\nACBD\nAED".into(),
            lang: Lang::En,
            exam: false,
            discovery: DiscoveryOptions::default(),
        })
        .unwrap();
        let raw = compute_json(input.clone(), Arc::new(AtomicBool::new(false))).unwrap();
        let computed: Computed = serde_json::from_str(&raw).unwrap();
        assert_eq!(computed.text, "ABCD\nACBD\nAED");
        let MiningOutput::Discovery(log, result) = computed.result.as_ref().unwrap() else {
            panic!("expected discovery")
        };
        assert_eq!(result.footprint.activities.len(), 5);
        assert_eq!(computed.replays.len(), log.variants().len());
        assert!(computed.replays.iter().all(Result::is_ok));
        assert_eq!(computed.report, result.report_with_replays(log, &computed.replays));
        assert!(computed.report.contains("Word check"));
        // Boxing the in-memory result must retain the worker's existing JSON shape.
        let wire: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let unboxed: op_core::mining::AlphaResult = serde_json::from_value(wire["result"]["Ok"]["Discovery"][1].clone()).unwrap();
        assert_eq!(unboxed.net, result.net);
        assert!(compute_json(input, Arc::new(AtomicBool::new(true))).is_err());
        let invalid = serde_json::to_string(&Input {
            task: "mining.discovery".into(),
            text: "case,activity\n1,\n".into(),
            lang: Lang::En,
            exam: false,
            discovery: DiscoveryOptions::default(),
        })
        .unwrap();
        let raw = compute_json(invalid, Arc::new(AtomicBool::new(false))).unwrap();
        assert!(serde_json::from_str::<Computed>(&raw).unwrap().result.is_err());
    }

    #[test]
    fn discovery_projects_routing_markers_without_rewriting_source_or_breaking_replay() {
        let source = "case;activity\n1;Prepare\n1;Vide1\n1;Serve\n2;Prepare\n2;Empty_2\n2;Cancel\n";
        let input = serde_json::to_string(&Input {
            task: "mining.discovery".into(),
            text: source.into(),
            lang: Lang::En,
            exam: false,
            discovery: DiscoveryOptions {
                normalize_boundaries: true,
            },
        })
        .unwrap();
        let raw = compute_json(input, Arc::new(AtomicBool::new(false))).unwrap();
        let computed: Computed = serde_json::from_str(&raw).unwrap();
        assert_eq!(computed.text, source);
        let MiningOutput::Discovery(log, discovered) = computed.result.as_ref().unwrap() else {
            panic!("expected discovery")
        };
        assert!(log.traces.iter().flatten().any(|activity| activity == "Vide1"));
        assert_eq!(discovered.projected_activities.len(), 2);
        assert!(
            !discovered
                .net
                .transitions
                .iter()
                .any(|transition| op_core::mining::is_routing_marker(&transition.name))
        );
        assert_eq!(
            discovered
                .net
                .transitions
                .iter()
                .filter(|transition| transition.silent && transition.name.is_empty())
                .count(),
            2
        );
        assert_eq!(computed.replays, vec![Ok(()), Ok(())]);
        assert!(
            computed
                .report
                .contains(op_core::i18n::with_lang(Lang::En, || tr("mining.report.projected_markers")))
        );
    }
}
