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
                let result = discover_cancellable(&log, &cancelled)?;
                Ok((log, result))
            })
        };
        let soundness = result.as_ref().ok().map(|(_, r)| soundness_cancellable(&r.net, 5000, &cancelled));
        if cancelled() {
            return Err("Cancelled".into());
        }
        serde_json::to_string(&Computed {
            text: input.text,
            lang: input.lang,
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
    fn discovery_transport_preserves_inputs_and_rejects_cancellation() {
        let input = serde_json::to_string(&Input {
            task: "mining.discovery".into(),
            text: "ABCD\nACBD\nAED".into(),
            lang: Lang::En,
        })
        .unwrap();
        let raw = compute_json(input.clone(), Arc::new(AtomicBool::new(false))).unwrap();
        let computed: Computed = serde_json::from_str(&raw).unwrap();
        assert_eq!(computed.text, "ABCD\nACBD\nAED");
        assert_eq!(computed.result.unwrap().1.footprint.activities.len(), 5);
        assert!(compute_json(input, Arc::new(AtomicBool::new(true))).is_err());
        let invalid = serde_json::to_string(&Input {
            task: "mining.discovery".into(),
            text: "case,activity\n1,\n".into(),
            lang: Lang::En,
        })
        .unwrap();
        let raw = compute_json(invalid, Arc::new(AtomicBool::new(false))).unwrap();
        assert!(serde_json::from_str::<Computed>(&raw).unwrap().result.is_err());
    }
}
