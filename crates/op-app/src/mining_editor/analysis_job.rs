//! Discovery and soundness execute on a private snapshot, outside the UI thread.
use super::*;
use op_core::{analysis::soundness_cancellable, i18n::Lang, mining::discover_cancellable};
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
}

pub(crate) fn compute_json(input: String, stop: Arc<AtomicBool>) -> Result<String, String> {
    let input: Input = serde_json::from_str(&input).map_err(|e| e.to_string())?;
    op_core::i18n::with_lang(input.lang, || {
        let cancelled = || stop.load(Ordering::Relaxed);
        let result = if input.text.len() > 2 * 1024 * 1024 {
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
                let result = discover_cancellable(&log, &cancelled)?;
                Ok(MiningOutput::Discovery(log, Box::new(result)))
            })
        };
        let soundness = match &result {
            Ok(MiningOutput::Discovery(_, r)) => Some(soundness_cancellable(&r.net, 5000, &cancelled)),
            _ => None,
        };
        if cancelled() {
            return Err("Cancelled".into());
        }
        serde_json::to_string(&Computed {
            text: input.text,
            lang: input.lang,
            exam: input.exam,
            result,
            soundness,
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
        })
        .unwrap();
        let raw = compute_json(input.clone(), Arc::new(AtomicBool::new(false))).unwrap();
        let computed: Computed = serde_json::from_str(&raw).unwrap();
        assert!(computed.exam);
        assert!(computed.soundness.is_none());
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
        })
        .unwrap();
        let raw = compute_json(input.clone(), Arc::new(AtomicBool::new(false))).unwrap();
        let computed: Computed = serde_json::from_str(&raw).unwrap();
        assert_eq!(computed.text, "ABCD\nACBD\nAED");
        let MiningOutput::Discovery(_, result) = computed.result.unwrap() else {
            panic!("expected discovery")
        };
        assert_eq!(result.footprint.activities.len(), 5);
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
        })
        .unwrap();
        let raw = compute_json(invalid, Arc::new(AtomicBool::new(false))).unwrap();
        assert!(serde_json::from_str::<Computed>(&raw).unwrap().result.is_err());
    }
}
