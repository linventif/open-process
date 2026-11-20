//! Sales aggregation is calculated on immutable snapshots outside the UI.
use op_core::i18n::Lang;
use op_core::sales::{SalesAnalysis, SalesFilter, SalesStudy};
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Serialize, Deserialize)]
pub(super) struct Input {
    pub task: String,
    pub study: SalesStudy,
    pub filter: SalesFilter,
    pub generation: u64,
    pub lang: Lang,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Output {
    pub generation: u64,
    pub filter: SalesFilter,
    pub lang: Lang,
    pub result: Result<SalesAnalysis, String>,
}

pub(crate) fn compute_json(input: String, stop: Arc<AtomicBool>) -> Result<String, String> {
    if stop.load(Ordering::Relaxed) {
        return Err("Cancelled".into());
    }
    let input: Input = serde_json::from_str(&input).map_err(|error| error.to_string())?;
    let result = op_core::i18n::with_lang(input.lang, || input.study.analyze(&input.filter));
    if stop.load(Ordering::Relaxed) {
        return Err("Cancelled".into());
    }
    serde_json::to_string(&Output {
        generation: input.generation,
        filter: input.filter,
        lang: input.lang,
        result,
    })
    .map_err(|error| error.to_string())
}
